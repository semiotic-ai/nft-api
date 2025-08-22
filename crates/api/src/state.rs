// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Server state management module
//!
//! This module provides shared application state for the NFT API server,
//! including configuration, dependency management, and coordinated cancellation.

use std::{collections::HashMap, sync::Arc};

use external_apis::ApiRegistry;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

use crate::{
    config::{Environment, ServerConfig},
    error::ServerResult,
};

/// Shared application state with cancellation token support
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Server configuration
    config: ServerConfig,
    /// API registry for external API operations
    api_registry: Arc<ApiRegistry>,
    /// Cancellation token for coordinated shutdown
    pub cancellation_token: CancellationToken,
}

impl ServerState {
    /// Create new server state
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `api_registry` - API registry for external API operations
    /// * `cancellation_token` - Token for coordinated cancellation
    pub fn new(
        config: ServerConfig,
        api_registry: Arc<ApiRegistry>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            config,
            api_registry,
            cancellation_token,
        }
    }

    /// Server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get the API registry for external API operations
    pub fn api_registry(&self) -> &Arc<ApiRegistry> {
        &self.api_registry
    }

    /// Perform health check operations
    pub async fn health_check(&self) -> ServerResult<HealthCheck> {
        let external_api_clients = self.api_registry.get_overall_health().await;

        let api_clients = external_api_clients
            .into_iter()
            .map(|(name, status)| (name, Self::convert_health_status(status)))
            .collect();

        Ok(HealthCheck {
            status: HealthStatus::Up,
            version: Box::from(env!("CARGO_PKG_VERSION")),
            environment: Environment::Development,
            timestamp: chrono::Utc::now().to_rfc3339(),
            api_clients,
        })
    }

    /// Convert external API health status to internal health status
    fn convert_health_status(external_status: api_client::HealthStatus) -> HealthStatus {
        match external_status {
            api_client::HealthStatus::Up => HealthStatus::Up,
            api_client::HealthStatus::Degraded { reason } => HealthStatus::Degraded {
                reason: reason.into_boxed_str(),
            },
            api_client::HealthStatus::Down { reason } => HealthStatus::Down {
                reason: reason.into_boxed_str(),
            },
        }
    }
}

/// Health status of a service or dependency
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum HealthStatus {
    /// Service is fully operational and responding normally
    Up,

    /// Service is not operational or has critical failures
    Down {
        /// Human-readable explanation of why the service is down
        reason: Box<str>,
    },

    /// Service is operational but experiencing performance issues or partial failures
    Degraded {
        /// Human-readable explanation of the degradation condition
        reason: Box<str>,
    },
}

/// Health check status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthCheck {
    /// Service status
    pub status: HealthStatus,
    /// Service version
    pub version: Box<str>,
    /// Environment
    pub environment: Environment,
    /// Timestamp
    pub timestamp: String,
    /// Status of individual API clients
    #[schema(value_type = Object)]
    pub api_clients: HashMap<String, HealthStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_state_creation() {
        let config = ServerConfig::default();
        let api_registry = Arc::new(ApiRegistry::new());
        let state = ServerState::new(config, api_registry, CancellationToken::new());

        assert!(!state.cancellation_token.is_cancelled());
    }

    #[test]
    fn server_state_with_cancellation_token() {
        let config = ServerConfig::default();
        let api_registry = Arc::new(ApiRegistry::new());
        let token = CancellationToken::new();
        let state = ServerState::new(config, api_registry, token.clone());

        assert!(!state.cancellation_token.is_cancelled());

        // Test that the tokens are linked
        token.cancel();
        assert!(state.cancellation_token.is_cancelled());
    }
}
