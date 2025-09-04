// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Error handling module
//!
//! This module provides comprehensive error types for server operations,
//! including proper HTTP response mapping and error propagation.

use std::net::SocketAddr;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use shared_types::{ChainCapability, ChainId, ChainStatus};
use thiserror::Error;

/// Comprehensive error types for server operations
#[derive(Error, Debug)]
pub enum ServerError {
    /// Configuration validation errors
    #[error("Configuration error: {message}")]
    Config {
        /// Error message
        message: String,
    },

    /// Network binding errors
    #[error("Failed to bind to {address}: {source}")]
    Bind {
        /// Socket address that failed to bind
        address: SocketAddr,
        /// Underlying IO error
        source: std::io::Error,
    },

    /// Server startup errors
    #[error("Server startup failed: {source}")]
    Startup {
        /// Underlying IO error
        source: std::io::Error,
    },

    /// Server shutdown errors
    #[error("Server shutdown failed: {source}")]
    Shutdown {
        /// Underlying IO error
        source: std::io::Error,
    },

    /// Runtime errors during server operation
    #[error("Runtime error: {message}")]
    Runtime {
        /// Error message
        message: String,
    },

    /// Dependency injection errors
    #[error("Dependency error: {message}")]
    Dependency {
        /// Error message
        message: String,
    },

    /// Task join errors for async operations
    #[error("Task join error: {source}")]
    TaskJoin {
        /// Underlying tokio join error
        #[source]
        source: tokio::task::JoinError,
    },

    /// Timeout errors for operations that exceed time limits
    #[error("Operation timed out after {timeout_seconds} seconds")]
    Timeout {
        /// Timeout duration in seconds
        timeout_seconds: u64,
    },

    /// Signal handling errors
    #[error("Signal handling error: {message}")]
    Signal {
        /// Error message
        message: String,
    },

    /// Input validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// JSON parsing errors with detailed context
    #[error("Invalid JSON request: {message}")]
    JsonError {
        /// Detailed error message
        message: String,
    },

    /// Chain validation errors with detailed information
    #[error("Chain validation error: {0}")]
    ChainValidation(#[from] ChainValidationError),
}

/// Result type for server operations
pub type ServerResult<T> = Result<T, ServerError>;

/// Detailed chain validation error types with specific context
#[derive(Error, Debug)]
pub enum ChainValidationError {
    /// Chain is not supported at all
    #[error("Chain {chain_name} (ID: {chain_id}) is not supported")]
    UnsupportedChain {
        /// Chain ID that is unsupported
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
    },

    /// Chain is planned but not yet implemented
    #[error("Chain {chain_name} (ID: {chain_id}) is not yet implemented")]
    PlannedChain {
        /// Chain ID that is planned
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
        /// Current status of the chain
        status: ChainStatus,
        /// Estimated availability if known
        estimated_availability: Option<String>,
    },

    /// Chain has limited functionality available
    #[error("Chain {chain_name} (ID: {chain_id}) has limited functionality")]
    PartialChain {
        /// Chain ID with partial support
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
        /// List of supported capabilities
        supported_features: Vec<ChainCapability>,
        /// List of unsupported features
        unsupported_features: Vec<ChainCapability>,
        /// List of limitation descriptions
        limitations: Vec<String>,
    },

    /// Chain support is deprecated and being phased out
    #[error("Chain {chain_name} (ID: {chain_id}) support is deprecated")]
    DeprecatedChain {
        /// Chain ID that is deprecated
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
        /// Deprecation reason
        reason: String,
        /// Alternative chain suggestion if available
        alternative: Option<ChainId>,
    },

    /// Specific capability is not available for this chain
    #[error("Capability {capability} is not available for chain {chain_name} (ID: {chain_id})")]
    UnsupportedCapability {
        /// Chain ID
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
        /// Missing capability
        capability: ChainCapability,
        /// List of supported capabilities for this chain
        supported_capabilities: Vec<ChainCapability>,
    },

    /// Chain configuration is missing or invalid
    #[error("Chain {chain_name} (ID: {chain_id}) configuration is invalid: {reason}")]
    ConfigurationError {
        /// Chain ID with configuration issues
        chain_id: u64,
        /// Chain name for display
        chain_name: String,
        /// Reason for configuration failure
        reason: String,
    },
}

impl ChainValidationError {
    /// Create an error for an unsupported chain
    pub fn unsupported_chain(chain_id: ChainId) -> Self {
        Self::UnsupportedChain {
            chain_id: chain_id.chain_id(),
            chain_name: chain_id.name().to_string(),
        }
    }

    /// Create an error for a planned chain
    pub fn planned_chain(chain_id: ChainId) -> Self {
        Self::PlannedChain {
            chain_id: chain_id.chain_id(),
            chain_name: chain_id.name().to_string(),
            status: chain_id.support_status(),
            estimated_availability: chain_id.estimated_availability().map(ToString::to_string),
        }
    }

    /// Create an error for a partially supported chain
    pub fn partial_chain(chain_id: ChainId) -> Self {
        let all_capabilities = vec![
            ChainCapability::MoralisMetadata,
            ChainCapability::PinaxAnalytics,
            ChainCapability::SpamPrediction,
            ChainCapability::RealTimeUpdates,
        ];

        let supported = chain_id.capabilities();
        let unsupported: Vec<_> = all_capabilities
            .into_iter()
            .filter(|cap| !supported.contains(cap))
            .collect();

        Self::PartialChain {
            chain_id: chain_id.chain_id(),
            chain_name: chain_id.name().to_string(),
            supported_features: supported,
            unsupported_features: unsupported,
            limitations: chain_id
                .limitations()
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        }
    }

    /// Create an error for unsupported capability
    pub fn unsupported_capability(chain_id: ChainId, capability: ChainCapability) -> Self {
        Self::UnsupportedCapability {
            chain_id: chain_id.chain_id(),
            chain_name: chain_id.name().to_string(),
            capability,
            supported_capabilities: chain_id.capabilities(),
        }
    }

    /// Create a configuration error
    pub fn configuration_error(chain_id: ChainId, reason: impl Into<String>) -> Self {
        Self::ConfigurationError {
            chain_id: chain_id.chain_id(),
            chain_name: chain_id.name().to_string(),
            reason: reason.into(),
        }
    }

    /// Get the appropriate HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::UnsupportedChain { .. } => StatusCode::BAD_REQUEST,
            Self::PlannedChain { .. } | Self::UnsupportedCapability { .. } => {
                StatusCode::NOT_IMPLEMENTED
            }
            Self::PartialChain { .. } => StatusCode::OK, // Will add warning headers
            Self::DeprecatedChain { .. } => StatusCode::GONE,
            Self::ConfigurationError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Convert error to detailed JSON response body
    #[allow(clippy::too_many_lines)]
    pub fn to_json_response(&self) -> serde_json::Value {
        match self {
            Self::UnsupportedChain {
                chain_id,
                chain_name,
            } => {
                serde_json::json!({
                    "error": "chain_not_supported",
                    "message": format!("Chain {} (ID: {}) is not supported", chain_name, chain_id),
                    "details": {
                        "chain_id": chain_id,
                        "chain_name": chain_name,
                        "status": "unsupported"
                    }
                })
            }
            Self::PlannedChain {
                chain_id,
                chain_name,
                status,
                estimated_availability,
            } => {
                let mut details = serde_json::json!({
                    "chain_id": chain_id,
                    "chain_name": chain_name,
                    "status": status.to_string()
                });

                if let Some(availability) = estimated_availability {
                    details["estimated_availability"] =
                        serde_json::Value::String(availability.clone());
                }

                serde_json::json!({
                    "error": "chain_not_implemented",
                    "message": format!("Chain {} (ID: {}) is not yet implemented", chain_name, chain_id),
                    "details": details
                })
            }
            Self::PartialChain {
                chain_id,
                chain_name,
                supported_features,
                unsupported_features,
                limitations,
            } => {
                serde_json::json!({
                    "error": "chain_partially_supported",
                    "message": format!("Chain {} (ID: {}) has limited functionality", chain_name, chain_id),
                    "details": {
                        "chain_id": chain_id,
                        "chain_name": chain_name,
                        "status": "partially_supported",
                        "supported_features": supported_features.iter().map(ToString::to_string).collect::<Vec<_>>(),
                        "unsupported_features": unsupported_features.iter().map(ToString::to_string).collect::<Vec<_>>(),
                        "limitations": limitations
                    }
                })
            }
            Self::DeprecatedChain {
                chain_id,
                chain_name,
                reason,
                alternative,
            } => {
                let mut details = serde_json::json!({
                    "chain_id": chain_id,
                    "chain_name": chain_name,
                    "status": "deprecated",
                    "reason": reason
                });

                if let Some(alt) = alternative {
                    details["alternative"] = serde_json::json!({
                        "chain_id": alt.chain_id(),
                        "chain_name": alt.name()
                    });
                }

                serde_json::json!({
                    "error": "chain_deprecated",
                    "message": format!("Chain {} (ID: {}) support is deprecated", chain_name, chain_id),
                    "details": details
                })
            }
            Self::UnsupportedCapability {
                chain_id,
                chain_name,
                capability,
                supported_capabilities,
            } => {
                serde_json::json!({
                    "error": "capability_not_supported",
                    "message": format!("Capability {} is not available for chain {} (ID: {})", capability, chain_name, chain_id),
                    "details": {
                        "chain_id": chain_id,
                        "chain_name": chain_name,
                        "requested_capability": capability.to_string(),
                        "supported_capabilities": supported_capabilities.iter().map(ToString::to_string).collect::<Vec<_>>()
                    }
                })
            }
            Self::ConfigurationError {
                chain_id,
                chain_name,
                reason,
            } => {
                serde_json::json!({
                    "error": "chain_configuration_error",
                    "message": format!("Chain {} (ID: {}) configuration is invalid: {}", chain_name, chain_id, reason),
                    "details": {
                        "chain_id": chain_id,
                        "chain_name": chain_name,
                        "reason": reason
                    }
                })
            }
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, json_body) = match &self {
            ServerError::Config { .. }
            | ServerError::Bind { .. }
            | ServerError::Startup { .. }
            | ServerError::Shutdown { .. }
            | ServerError::Runtime { .. }
            | ServerError::TaskJoin { .. }
            | ServerError::Signal { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": self.to_string(),
                    "status": StatusCode::INTERNAL_SERVER_ERROR.as_u16()
                }),
            ),
            ServerError::Dependency { .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                serde_json::json!({
                    "error": self.to_string(),
                    "status": StatusCode::SERVICE_UNAVAILABLE.as_u16()
                }),
            ),
            ServerError::Timeout { .. } => (
                StatusCode::REQUEST_TIMEOUT,
                serde_json::json!({
                    "error": self.to_string(),
                    "status": StatusCode::REQUEST_TIMEOUT.as_u16()
                }),
            ),
            ServerError::ValidationError(..) | ServerError::JsonError { .. } => (
                StatusCode::BAD_REQUEST,
                serde_json::json!({
                    "error": self.to_string(),
                    "status": StatusCode::BAD_REQUEST.as_u16()
                }),
            ),
            ServerError::ChainValidation(chain_err) => {
                let status = chain_err.status_code();
                let mut json_response = chain_err.to_json_response();
                json_response["status"] = serde_json::Value::Number(status.as_u16().into());
                (status, json_response)
            }
        };

        let body = Json(json_body);
        (status, body).into_response()
    }
}

/// Convenient From implementations for common async error types
impl From<tokio::task::JoinError> for ServerError {
    fn from(source: tokio::task::JoinError) -> Self {
        Self::TaskJoin { source }
    }
}
