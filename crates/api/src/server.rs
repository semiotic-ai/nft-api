// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Server implementation module
//!
//! This module provides the main server struct and implementation for the NFT API server,
//! including server lifecycle management, router configuration, and coordinated graceful
//! shutdown using `CancellationToken`.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, http::HeaderName};
use hyper::Request;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info, info_span, warn};

use crate::{
    config::ServerConfig,
    dependencies::{DefaultDependencies, Dependencies},
    error::{ServerError, ServerResult},
    routes::create_routes,
    state::ServerState,
};

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Configuration for server shutdown behavior
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for graceful shutdown before forcing termination
    pub graceful_timeout: Duration,
    /// Maximum time to wait for all tasks to complete after graceful shutdown
    pub force_timeout: Duration,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_secs(30),
            force_timeout: Duration::from_secs(5),
        }
    }
}

/// Main server struct
#[derive(Debug)]
#[allow(dead_code)]
pub struct Server {
    /// Server configuration
    config: ServerConfig,
    /// Application router
    router: Router,
    /// Server state
    state: ServerState,
    /// Cancellation token for coordinated shutdown
    cancellation_token: CancellationToken,
    /// Configuration for coordinated shutdown
    graceful_shutdown_config: ShutdownConfig,
}

impl Server {
    /// Create new server instance
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Config` if the configuration is invalid.
    pub fn new(config: ServerConfig, shutdown_config: ShutdownConfig) -> ServerResult<Self> {
        Self::with_dependencies(
            config,
            shutdown_config,
            Arc::new(DefaultDependencies::new()),
        )
    }

    /// Create server with custom dependencies for dependency injection
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Config` if the configuration is invalid.
    pub fn with_dependencies(
        config: ServerConfig,
        graceful_shutdown_config: ShutdownConfig,
        dependencies: Arc<dyn Dependencies>,
    ) -> ServerResult<Self> {
        // Configuration validation is now built into the types

        let cancellation_token = CancellationToken::new();
        let state = ServerState::new(
            config.clone(),
            dependencies,
            cancellation_token.child_token(),
        );
        let router = Self::create_router(state.clone());

        Ok(Self {
            config,
            router,
            state,
            cancellation_token,
            graceful_shutdown_config,
        })
    }

    /// Create application router with middleware
    fn create_router(state: ServerState) -> Router {
        let timeout_duration = state.config().timeout_seconds.value();

        let middleware = ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
            .layer(
                TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
                    if let Some(request_id) = req.headers().get(REQUEST_ID_HEADER) {
                        info_span!("http_request", ?request_id)
                    } else {
                        tracing::error!("failed to extract id from request");
                        info_span!("http_request", request_id = "unknown")
                    }
                }),
            )
            .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
            .layer(CorsLayer::permissive())
            .layer(TimeoutLayer::new(timeout_duration));

        create_routes().layer(middleware).with_state(state)
    }

    /// Run the server with coordinated graceful shutdown
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Bind` if unable to bind to the configured address,
    /// or `ServerError::Startup` if the server fails to start.
    pub async fn run(self) -> ServerResult<()> {
        let addr = self.config.socket_addr();
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|source| ServerError::Bind {
                address: addr,
                source,
            })?;

        let actual_addr = listener
            .local_addr()
            .map_err(|source| ServerError::Startup { source })?;

        info!(
            address = %actual_addr,
            environment = %self.config.environment,
            "NFT API server starting",
        );

        let cancellation_token = self.cancellation_token.clone();
        let shutdown_token = cancellation_token.clone();
        tokio::spawn(async move {
            info!("spawning the graceful shutdown task");
            Self::shutdown_signal_handler(shutdown_token).await;
        });

        let server_result = axum::serve(listener, self.router)
            .with_graceful_shutdown(async move {
                cancellation_token.cancelled().await;
                info!("NFT API server shut down gracefully");
            })
            .await;

        if let Err(e) = server_result {
            error!(error = ?e, "Server error during shutdown");
            Err(ServerError::Shutdown { source: e })
        } else {
            Ok(())
        }
    }

    /// Handle shutdown signals and trigger coordinated cancellation
    ///
    /// This function listens for SIGINT (Ctrl+C) and SIGTERM signals,
    /// and cancels the provided cancellation token when received.
    ///
    /// # Arguments
    ///
    /// * `cancellation_token` - Token to cancel when shutdown signal is received
    async fn shutdown_signal_handler(cancellation_token: CancellationToken) {
        let signal_received = async {
            #[cfg(unix)]
            #[allow(clippy::expect_used)]
            {
                use tokio::signal::unix::{SignalKind, signal};

                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
                let mut sigint =
                    signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        warn!("Received SIGTERM signal, initiating coordinated shutdown");
                        "SIGTERM"
                    },
                    _ = sigint.recv() => {
                        warn!("Received SIGINT signal, initiating coordinated shutdown");
                        "SIGINT"
                    },
                }
            }

            #[cfg(not(unix))]
            #[allow(clippy::expect_used)]
            {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to install CTRL+C signal handler");
                warn!("Received CTRL+C signal, initiating coordinated shutdown");
                "CTRL+C"
            }
        };

        // Wait for either a signal or existing cancellation
        tokio::select! {
            signal_name = signal_received => {
                warn!("Shutdown signal {} received, cancelling all operations...", signal_name);
                cancellation_token.cancel();
            },
            () = cancellation_token.cancelled() => {
                warn!("Cancellation token already cancelled, shutdown signal handler exiting");
            }
        }
    }

    /// Returns a clone of the cancellation token for coordinated shutdown
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Initiates graceful shutdown by cancelling the server's cancellation token
    pub fn shutdown(&self) {
        info!("programmatic shutdown requested");
        self.cancellation_token.cancel();
    }

    /// Run server for testing, returns the bound address
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Bind` if unable to bind to the configured address.
    pub async fn run_for_testing(self) -> ServerResult<(SocketAddr, CancellationToken)> {
        let addr = self.config.socket_addr();

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|source| ServerError::Bind {
                address: addr,
                source,
            })?;

        let actual_addr = listener
            .local_addr()
            .map_err(|source| ServerError::Startup { source })?;

        let token = self.cancellation_token.child_token();
        let task = token.child_token();
        tokio::spawn(async move {
            let _ = axum::serve(listener, self.router)
                .with_graceful_shutdown(async move { task.cancelled().await })
                .await;
        });

        Ok((actual_addr, token))
    }

    /// Get server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get server state for testing
    pub fn state(&self) -> &ServerState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;

    #[tokio::test]
    async fn server_creation() -> ServerResult<()> {
        let config = ServerConfig::for_testing();
        let server = Server::new(config, ShutdownConfig::default())?;
        assert_eq!(server.config().environment, Environment::Testing);
        assert!(!server.cancellation_token().is_cancelled());
        Ok(())
    }

    #[tokio::test]
    async fn programmatic_shutdown() -> ServerResult<()> {
        let config = ServerConfig::for_testing();
        let server = Server::new(config, ShutdownConfig::default())?;

        assert!(!server.cancellation_token().is_cancelled());

        server.shutdown();

        assert!(server.cancellation_token().is_cancelled());
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(config.graceful_timeout, Duration::from_secs(30));
        assert_eq!(config.force_timeout, Duration::from_secs(5));
    }
}
