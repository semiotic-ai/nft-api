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
use spam_predictor::SpamPredictor;
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
    /// Spam predictor for contract analysis
    spam_predictor: Arc<SpamPredictor>,
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
    /// * `spam_predictor` - Spam predictor for contract analysis
    /// * `cancellation_token` - Token for coordinated cancellation
    pub fn new(
        config: ServerConfig,
        api_registry: Arc<ApiRegistry>,
        spam_predictor: Arc<SpamPredictor>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            config,
            api_registry,
            spam_predictor,
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

    /// Get the spam predictor for contract analysis
    pub fn spam_predictor(&self) -> &Arc<SpamPredictor> {
        &self.spam_predictor
    }

    /// Perform health check operations
    pub async fn health_check(&self) -> ServerResult<HealthCheck> {
        let external_api_clients = self.api_registry.get_overall_health().await;
        let spam_predictor_health = self.get_spam_predictor_health().await;

        let mut api_clients = external_api_clients
            .into_iter()
            .map(|(name, status)| (name, Self::convert_health_status(status)))
            .collect::<HashMap<String, HealthStatus>>();

        // Add spam predictor health as a separate internal service
        api_clients.extend(spam_predictor_health);

        Ok(HealthCheck {
            status: HealthStatus::Up,
            version: Box::from(env!("CARGO_PKG_VERSION")),
            environment: Environment::Development,
            timestamp: chrono::Utc::now().to_rfc3339(),
            api_clients,
        })
    }

    /// Get spam predictor health status as a separate internal service
    async fn get_spam_predictor_health(&self) -> HashMap<String, HealthStatus> {
        let mut services = HashMap::new();

        match self.spam_predictor.health_check().await {
            Ok(health_status) => {
                let status = if health_status.overall_healthy {
                    HealthStatus::Up
                } else {
                    let reasons = vec![
                        if health_status.openai_healthy {
                            None
                        } else {
                            Some("OpenAI API unavailable")
                        },
                        if health_status.config_healthy {
                            None
                        } else {
                            Some("Configuration issues")
                        },
                        if health_status.cache_healthy {
                            None
                        } else {
                            Some("Cache issues")
                        },
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(", ");

                    HealthStatus::Down {
                        reason: if reasons.is_empty() {
                            "Unknown issue".into()
                        } else {
                            reasons.into()
                        },
                    }
                };
                services.insert("spam-predictor".to_string(), status);
            }
            Err(e) => {
                services.insert(
                    "spam-predictor".to_string(),
                    HealthStatus::Down {
                        reason: format!("Health check failed: {e}").into(),
                    },
                );
            }
        }

        services
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
#[schema(
    examples(
        json!({
            "status": "Up",
            "version": "0.1.0",
            "environment": "Development",
            "timestamp": "2025-01-22T10:30:00Z",
            "api_clients": {
                "moralis": "Up",
                "pinax": "Up",
                "spam-predictor": "Up"
            }
        }),
        json!({
            "status": "Up",
            "version": "0.1.0",
            "environment": "Production",
            "timestamp": "2025-01-22T10:30:00Z",
            "api_clients": {
                "moralis": {"Degraded": {"reason": "High response times"}},
                "pinax": "Up",
                "spam-predictor": "Up"
            }
        }),
        json!({
            "status": "Up",
            "version": "0.1.0",
            "environment": "Production",
            "timestamp": "2025-01-22T10:30:00Z",
            "api_clients": {
                "moralis": "Up",
                "pinax": {"Down": {"reason": "API authentication failed"}},
                "spam-predictor": "Up"
            }
        })
    )
)]
pub struct HealthCheck {
    /// Overall service status
    pub status: HealthStatus,
    /// Service version from Cargo.toml
    pub version: Box<str>,
    /// Current deployment environment
    pub environment: Environment,
    /// ISO 8601 formatted timestamp of health check
    pub timestamp: String,
    /// Status of external API clients and internal services
    #[schema(value_type = HashMap<String, HealthStatus>)]
    pub api_clients: HashMap<String, HealthStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_state_creation() {
        let config = ServerConfig::for_testing();
        let api_registry = Arc::new(ApiRegistry::new());

        // Create a test spam predictor configuration
        let spam_predictor_config = spam_predictor::SpamPredictorConfig::from_files(
            &config.spam_predictor.model_registry_path,
            &config.spam_predictor.prompt_registry_path,
            spam_predictor::config::OpenAiConfig::new(
                config.spam_predictor.openai_api_key.value().to_string(),
            ),
        )
        .await
        .expect("Failed to create spam predictor config");

        let spam_predictor = Arc::new(
            SpamPredictor::new(spam_predictor_config)
                .await
                .expect("Failed to create spam predictor"),
        );

        let state = ServerState::new(
            config,
            api_registry,
            spam_predictor,
            CancellationToken::new(),
        );

        assert!(!state.cancellation_token.is_cancelled());
        // SpamPredictor is now always present
        assert!(state.spam_predictor().health_check().await.is_ok());
    }

    #[tokio::test]
    async fn server_state_with_cancellation_token() {
        let config = ServerConfig::for_testing();
        let api_registry = Arc::new(ApiRegistry::new());

        // Create a test spam predictor configuration
        let spam_predictor_config = spam_predictor::SpamPredictorConfig::from_files(
            &config.spam_predictor.model_registry_path,
            &config.spam_predictor.prompt_registry_path,
            spam_predictor::config::OpenAiConfig::new(
                config.spam_predictor.openai_api_key.value().to_string(),
            ),
        )
        .await
        .expect("Failed to create spam predictor config");

        let spam_predictor = Arc::new(
            SpamPredictor::new(spam_predictor_config)
                .await
                .expect("Failed to create spam predictor"),
        );

        let token = CancellationToken::new();
        let state = ServerState::new(config, api_registry, spam_predictor, token.clone());

        assert!(!state.cancellation_token.is_cancelled());
        // SpamPredictor is now always present
        assert!(state.spam_predictor().health_check().await.is_ok());

        // Test that the tokens are linked
        token.cancel();
        assert!(state.cancellation_token.is_cancelled());
    }
}
