// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Server implementation module
//!
//! This module provides the main server struct and implementation for the NFT API server,
//! including server lifecycle management, router configuration, and coordinated graceful
//! shutdown using `CancellationToken`.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{Router, http::HeaderName, routing::get};
use external_apis::{
    ApiRegistry, MoralisClient, MoralisConfig as ExternalMoralisConfig, PerChainMoralisConfig,
    PerChainPinaxConfig, PinaxClient, PinaxConfig as ExternalPinaxConfig,
};
use hyper::Request;
use spam_predictor::{SpamPredictor, SpamPredictorConfig};
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
    error::{ServerError, ServerResult},
    metrics::metrics_handler,
    middleware::RateLimiter,
    routes::create_routes,
    state::ServerState,
};

// Server constants
const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_FORCE_SHUTDOWN_TIMEOUT_SECONDS: u64 = 5;

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
            graceful_timeout: Duration::from_secs(DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS),
            force_timeout: Duration::from_secs(DEFAULT_FORCE_SHUTDOWN_TIMEOUT_SECONDS),
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
    pub async fn new(config: ServerConfig, shutdown_config: ShutdownConfig) -> ServerResult<Self> {
        let api_registry = Self::create_api_registry_from_config(&config);
        Self::with_api_registry(config, shutdown_config, Arc::new(api_registry)).await
    }

    /// Create API registry from server configuration
    fn create_api_registry_from_config(config: &ServerConfig) -> ApiRegistry {
        // Initialize MoralisClient if enabled
        let moralis_client = if config.external_apis.moralis.enabled {
            let moralis_config = ExternalMoralisConfig {
                base_url: config.external_apis.moralis.base_url.to_string(),
                api_key: config.external_apis.moralis.api_key.value().to_string(),
                timeout_seconds: config
                    .external_apis
                    .moralis
                    .timeout_seconds
                    .value()
                    .as_secs(),
                health_check_timeout_seconds: config
                    .external_apis
                    .moralis
                    .health_check_timeout_seconds
                    .value()
                    .as_secs(),
                max_retries: config.external_apis.moralis.max_retries,
            };

            // Build chain-specific Moralis overrides from configuration
            let mut chain_overrides = std::collections::HashMap::new();
            for (chain_id, chain_config) in &config.chains {
                if let Some(moralis_override) = &chain_config.moralis {
                    chain_overrides.insert(
                        *chain_id,
                        PerChainMoralisConfig {
                            base_url: moralis_override.base_url.clone(),
                            timeout_seconds: moralis_override
                                .timeout_seconds
                                .as_ref()
                                .map(|t| t.value().as_secs()),
                            max_retries: moralis_override.max_retries,
                        },
                    );
                }
            }

            Some(
                MoralisClient::with_chain_overrides(moralis_config, chain_overrides)
                    .expect("Failed to create Moralis client"),
            )
        } else {
            None
        };

        // Initialize PinaxClient if enabled
        let pinax_client = if config.external_apis.pinax.enabled {
            let pinax_config = ExternalPinaxConfig::new(
                config.external_apis.pinax.endpoint.as_str(),
                config.external_apis.pinax.api_user.value(),
                config.external_apis.pinax.api_auth.value(),
                &config.external_apis.pinax.db_name,
                config.external_apis.pinax.timeout_seconds.value().as_secs(),
                config
                    .external_apis
                    .pinax
                    .health_check_timeout_seconds
                    .value()
                    .as_secs(),
                config.external_apis.pinax.max_retries,
            )
            .expect("Failed to create Pinax config");

            // Build chain-specific Pinax overrides from configuration
            let mut chain_overrides = std::collections::HashMap::new();
            for (chain_id, chain_config) in &config.chains {
                if let Some(pinax_override) = &chain_config.pinax {
                    chain_overrides.insert(
                        *chain_id,
                        PerChainPinaxConfig {
                            db_name: Some(pinax_override.db_name.clone()),
                            timeout_seconds: pinax_override
                                .timeout_seconds
                                .as_ref()
                                .map(|t| t.value().as_secs()),
                            max_retries: pinax_override.max_retries,
                        },
                    );
                }
            }

            Some(
                PinaxClient::with_chain_overrides(pinax_config, chain_overrides)
                    .expect("Failed to create Pinax client"),
            )
        } else {
            None
        };

        ApiRegistry::with_clients(moralis_client, pinax_client)
    }

    /// Create spam predictor from server configuration
    async fn create_spam_predictor_from_config(
        config: &ServerConfig,
    ) -> ServerResult<SpamPredictor> {
        info!("initializing spam predictor");

        // Create OpenAI configuration
        let openai_config = spam_predictor::config::OpenAiConfig::new(
            config.spam_predictor.openai_api_key.value().to_string(),
        )
        .with_timeout(config.spam_predictor.timeout_seconds.value().as_secs())
        .with_max_tokens(config.spam_predictor.max_tokens.unwrap_or(10))
        .with_temperature(config.spam_predictor.temperature.unwrap_or(0.0));

        // Set base URL if configured
        let openai_config = if let Some(base_url) = &config.spam_predictor.openai_base_url {
            openai_config.with_base_url(base_url.clone())
        } else {
            openai_config
        };

        // Set organization ID if configured
        let openai_config = if let Some(org_id) = &config.spam_predictor.openai_organization_id {
            openai_config.with_organization(org_id.clone())
        } else {
            openai_config
        };

        // Create SpamPredictorConfig
        let predictor_config = SpamPredictorConfig::from_files(
            &config.spam_predictor.model_registry_path,
            &config.spam_predictor.prompt_registry_path,
            openai_config,
        )
        .await
        .map_err(|e| ServerError::Config {
            message: format!("Failed to create spam predictor configuration: {e}"),
        })?;

        // Create SpamPredictor
        let predictor =
            SpamPredictor::new(predictor_config)
                .await
                .map_err(|e| ServerError::Config {
                    message: format!("Failed to initialize spam predictor: {e}"),
                })?;

        info!("spam predictor initialized successfully");
        Ok(predictor)
    }

    /// Create server with custom API registry for dependency injection
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Config` if the configuration is invalid.
    pub async fn with_api_registry(
        config: ServerConfig,
        graceful_shutdown_config: ShutdownConfig,
        api_registry: Arc<ApiRegistry>,
    ) -> ServerResult<Self> {
        // Configuration validation is now built into the types

        // Initialize spam predictor (always required)
        let spam_predictor = Self::create_spam_predictor_from_config(&config).await?;
        let spam_predictor = Arc::new(spam_predictor);

        let cancellation_token = CancellationToken::new();
        let state = ServerState::new(
            config.clone(),
            api_registry,
            spam_predictor,
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

        // Create rate limiter from configuration
        let rate_limiter = RateLimiter::new(state.config().rate_limiting.clone());

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

        create_routes(rate_limiter)
            .layer(middleware)
            .with_state(state)
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

        let metrics_addr = SocketAddr::new(self.config.host, self.config.metrics.port);
        let metrics_listener =
            TcpListener::bind(&metrics_addr)
                .await
                .map_err(|source| ServerError::Bind {
                    address: metrics_addr,
                    source,
                })?;

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

        // Run metrics server
        let metrics_cancel = cancellation_token.clone();
        let metrics_server = tokio::spawn(async move {
            let metrics_router = Router::new()
                .route(&self.config.metrics.endpoint_path, get(metrics_handler))
                .with_state(self.state);
            if let Err(e) = axum::serve(
                metrics_listener,
                metrics_router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                metrics_cancel.cancelled().await;
                info!("Prometheus metrics server shut down gracefully");
            })
            .await
            {
                error!(error = ?e, "Metrics server error during shutdown");
            }
        });

        // Run main server
        let app_server = axum::serve(
            listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            cancellation_token.cancelled().await;
            info!("NFT API server shut down gracefully");
        })
        .await;

        let _ = metrics_server.await;

        if let Err(e) = app_server {
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
            let _ = axum::serve(
                listener,
                self.router
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
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
        let server = Server::new(config, ShutdownConfig::default()).await?;
        assert_eq!(server.config().environment, Environment::Testing);
        assert!(!server.cancellation_token().is_cancelled());
        Ok(())
    }

    #[tokio::test]
    async fn programmatic_shutdown() -> ServerResult<()> {
        let config = ServerConfig::for_testing();
        let server = Server::new(config, ShutdownConfig::default()).await?;

        assert!(!server.cancellation_token().is_cancelled());

        server.shutdown();

        assert!(server.cancellation_token().is_cancelled());
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(
            config.graceful_timeout,
            Duration::from_secs(DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS)
        );
        assert_eq!(
            config.force_timeout,
            Duration::from_secs(DEFAULT_FORCE_SHUTDOWN_TIMEOUT_SECONDS)
        );
    }
}
