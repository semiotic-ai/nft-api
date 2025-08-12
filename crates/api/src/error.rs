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
}

/// Result type for server operations
pub type ServerResult<T> = Result<T, ServerError>;

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Config { .. }
            | ServerError::Bind { .. }
            | ServerError::Startup { .. }
            | ServerError::Shutdown { .. }
            | ServerError::Runtime { .. }
            | ServerError::TaskJoin { .. }
            | ServerError::Signal { .. } => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ServerError::Dependency { .. } => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            ServerError::Timeout { .. } => (StatusCode::REQUEST_TIMEOUT, self.to_string()),
            ServerError::ValidationError(..) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

/// Convenient From implementations for common async error types
impl From<tokio::task::JoinError> for ServerError {
    fn from(source: tokio::task::JoinError) -> Self {
        Self::TaskJoin { source }
    }
}
