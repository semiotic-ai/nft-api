// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Error types for spam prediction operations
//!
//! This module provides comprehensive error handling for all spam prediction
//! operations, including configuration errors, API failures, and validation issues.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use thiserror::Error;

/// Result type alias for spam prediction operations
pub type SpamPredictorResult<T> = Result<T, SpamPredictorError>;

/// Enhanced error context with request correlation
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Request ID for correlation across logs
    pub request_id: Option<String>,
    /// Operation that failed
    pub operation: Option<String>,
    /// Timestamp when error occurred
    pub timestamp: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ErrorContext {
    /// Create new error context
    pub fn new() -> Self {
        Self {
            request_id: None,
            operation: None,
            timestamp: Some(Utc::now()),
            metadata: HashMap::new(),
        }
    }

    /// Set request ID for correlation
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set operation name
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Add metadata key-value pair
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive error types for spam prediction operations
#[derive(Debug, Error)]
pub enum SpamPredictorError {
    /// Configuration file not found or invalid
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Model registry error (model not found, invalid format)
    #[error("Model registry error: {message}")]
    ModelRegistry { message: String },

    /// Prompt registry error (prompt not found, invalid format)
    #[error("Prompt registry error: {message}")]
    PromptRegistry { message: String },

    /// OpenAI API error
    #[error("OpenAI API error: {message}")]
    OpenAi { message: String },

    /// HTTP request failed
    #[error("HTTP request failed: {message}")]
    Http { message: String },

    /// Authentication failed
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after_seconds} seconds")]
    RateLimitExceeded { retry_after_seconds: u64 },

    /// Request timeout
    #[error("Request timeout after {timeout_seconds} seconds")]
    Timeout { timeout_seconds: u64 },

    /// Invalid response format from OpenAI
    #[error("Invalid response format: {message}")]
    InvalidResponse { message: String },

    /// Service unavailable
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// JSON serialization/deserialization error
    #[error("JSON error: {message}")]
    Json { message: String },

    /// YAML parsing error
    #[error("YAML error: {message}")]
    Yaml { message: String },

    /// I/O error (file operations)
    #[error("I/O error: {message}")]
    Io { message: String },

    /// Cache error
    #[error("Cache error: {message}")]
    Cache { message: String },

    /// Validation error
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Custom error with context
    #[error("Custom error: {0}")]
    Custom(#[from] anyhow::Error),
}

impl SpamPredictorError {
    /// Create a configuration error
    pub fn config<T: ToString>(message: T) -> Self {
        Self::Configuration {
            message: message.to_string(),
        }
    }

    /// Create a model registry error
    pub fn model_registry<T: ToString>(message: T) -> Self {
        Self::ModelRegistry {
            message: message.to_string(),
        }
    }

    /// Create a prompt registry error
    pub fn prompt_registry<T: ToString>(message: T) -> Self {
        Self::PromptRegistry {
            message: message.to_string(),
        }
    }

    /// Create an OpenAI API error
    pub fn openai<T: ToString>(message: T) -> Self {
        Self::OpenAi {
            message: message.to_string(),
        }
    }

    /// Create an HTTP error
    pub fn http<T: ToString>(message: T) -> Self {
        Self::Http {
            message: message.to_string(),
        }
    }

    /// Create an authentication error
    pub fn authentication<T: ToString>(message: T) -> Self {
        Self::Authentication {
            message: message.to_string(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit(retry_after_seconds: u64) -> Self {
        Self::RateLimitExceeded {
            retry_after_seconds,
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout_seconds: u64) -> Self {
        Self::Timeout { timeout_seconds }
    }

    /// Create an invalid response error
    pub fn invalid_response<T: ToString>(message: T) -> Self {
        Self::InvalidResponse {
            message: message.to_string(),
        }
    }

    /// Create a service unavailable error
    pub fn service_unavailable<T: ToString>(message: T) -> Self {
        Self::ServiceUnavailable {
            message: message.to_string(),
        }
    }

    /// Create a JSON error
    pub fn json<T: ToString>(message: T) -> Self {
        Self::Json {
            message: message.to_string(),
        }
    }

    /// Create a YAML error
    pub fn yaml<T: ToString>(message: T) -> Self {
        Self::Yaml {
            message: message.to_string(),
        }
    }

    /// Create an I/O error
    pub fn io<T: ToString>(message: T) -> Self {
        Self::Io {
            message: message.to_string(),
        }
    }

    /// Create a cache error
    pub fn cache<T: ToString>(message: T) -> Self {
        Self::Cache {
            message: message.to_string(),
        }
    }

    /// Create a validation error
    pub fn validation<T: ToString>(message: T) -> Self {
        Self::Validation {
            message: message.to_string(),
        }
    }

    /// Create an internal error
    pub fn internal<T: ToString>(message: T) -> Self {
        Self::Internal {
            message: message.to_string(),
        }
    }

    /// Create an OpenAI error with context
    pub fn openai_with_context<T: ToString>(message: T, context: ErrorContext) -> Self {
        let mut enhanced_message = message.to_string();

        if let Some(request_id) = &context.request_id {
            enhanced_message.push_str(&format!(" [request_id: {}]", request_id));
        }

        if let Some(operation) = &context.operation {
            enhanced_message.push_str(&format!(" [operation: {}]", operation));
        }

        if let Some(timestamp) = &context.timestamp {
            enhanced_message.push_str(&format!(" [timestamp: {}]", timestamp));
        }

        Self::OpenAi {
            message: enhanced_message,
        }
    }

    /// Create an HTTP error with context
    pub fn http_with_context<T: ToString>(message: T, context: ErrorContext) -> Self {
        let mut enhanced_message = message.to_string();

        if let Some(request_id) = &context.request_id {
            enhanced_message.push_str(&format!(" [request_id: {}]", request_id));
        }

        Self::Http {
            message: enhanced_message,
        }
    }

    /// Check if this error indicates a temporary failure that could be retried
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SpamPredictorError::Http { .. }
                | SpamPredictorError::Timeout { .. }
                | SpamPredictorError::ServiceUnavailable { .. }
                | SpamPredictorError::RateLimitExceeded { .. }
        )
    }

    /// Get retry delay suggestion in seconds for rate limited requests
    pub fn retry_delay_seconds(&self) -> Option<u64> {
        match self {
            SpamPredictorError::RateLimitExceeded {
                retry_after_seconds,
            } => Some(*retry_after_seconds),
            SpamPredictorError::Timeout { .. } => Some(5), // 5 second backoff for timeouts
            SpamPredictorError::ServiceUnavailable { .. } => Some(30), // 30 second backoff for server errors
            _ => None,
        }
    }

    /// Check if this error indicates a permanent failure that should not be retried
    pub fn is_permanent_failure(&self) -> bool {
        matches!(
            self,
            SpamPredictorError::Authentication { .. }
                | SpamPredictorError::Configuration { .. }
                | SpamPredictorError::ModelRegistry { .. }
                | SpamPredictorError::PromptRegistry { .. }
                | SpamPredictorError::Validation { .. }
        )
    }

    /// Check if this error indicates an authentication problem
    pub fn is_auth_error(&self) -> bool {
        matches!(self, SpamPredictorError::Authentication { .. })
    }

    /// Check if this error indicates a configuration problem
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            SpamPredictorError::Configuration { .. }
                | SpamPredictorError::ModelRegistry { .. }
                | SpamPredictorError::PromptRegistry { .. }
                | SpamPredictorError::Validation { .. }
        )
    }
}

/// Convert from reqwest errors
impl From<reqwest::Error> for SpamPredictorError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout {
                timeout_seconds: 30, // Default timeout assumption
            }
        } else if err.is_status() {
            let status = err.status().map(|s| s.as_u16()).unwrap_or(0);
            if status == 401 || status == 403 {
                Self::Authentication {
                    message: err.to_string(),
                }
            } else if status == 429 {
                Self::RateLimitExceeded {
                    retry_after_seconds: 60, // Default retry after
                }
            } else if status >= 500 {
                Self::ServiceUnavailable {
                    message: err.to_string(),
                }
            } else {
                Self::Http {
                    message: err.to_string(),
                }
            }
        } else {
            Self::Http {
                message: err.to_string(),
            }
        }
    }
}

/// Convert from JSON errors
impl From<serde_json::Error> for SpamPredictorError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            message: err.to_string(),
        }
    }
}

/// Convert from YAML errors
impl From<serde_yaml::Error> for SpamPredictorError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Yaml {
            message: err.to_string(),
        }
    }
}

/// Convert from I/O errors
impl From<std::io::Error> for SpamPredictorError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_constructors() {
        let config_err = SpamPredictorError::config("test message");
        assert!(matches!(
            config_err,
            SpamPredictorError::Configuration { .. }
        ));

        let openai_err = SpamPredictorError::openai("test error");
        assert!(matches!(openai_err, SpamPredictorError::OpenAi { .. }));

        let rate_limit_err = SpamPredictorError::rate_limit(60);
        assert!(matches!(
            rate_limit_err,
            SpamPredictorError::RateLimitExceeded {
                retry_after_seconds: 60
            }
        ));
    }

    #[test]
    fn error_classification() {
        let auth_error = SpamPredictorError::authentication("invalid key");
        assert!(auth_error.is_auth_error());
        assert!(!auth_error.is_retryable());
        assert!(!auth_error.is_config_error());

        let timeout_error = SpamPredictorError::timeout(30);
        assert!(timeout_error.is_retryable());
        assert!(!timeout_error.is_auth_error());
        assert!(!timeout_error.is_config_error());

        let config_error = SpamPredictorError::config("bad config");
        assert!(config_error.is_config_error());
        assert!(!config_error.is_auth_error());
        assert!(!config_error.is_retryable());
    }

    #[test]
    fn error_display() {
        let error = SpamPredictorError::openai("API failed");
        let display = format!("{}", error);
        assert!(display.contains("OpenAI API error"));
        assert!(display.contains("API failed"));
    }

    #[test]
    fn error_context_creation() {
        let context = ErrorContext::new()
            .with_request_id("req-123".to_string())
            .with_operation("predict_spam".to_string())
            .with_metadata("model".to_string(), "gpt-4".to_string());

        assert_eq!(context.request_id, Some("req-123".to_string()));
        assert_eq!(context.operation, Some("predict_spam".to_string()));
        assert!(context.timestamp.is_some());
        assert_eq!(context.metadata.get("model"), Some(&"gpt-4".to_string()));
    }

    #[test]
    fn error_with_context() {
        let context = ErrorContext::new()
            .with_request_id("req-456".to_string())
            .with_operation("api_call".to_string());

        let error = SpamPredictorError::openai_with_context("Rate limited", context);
        let error_str = error.to_string();

        assert!(error_str.contains("Rate limited"));
        assert!(error_str.contains("request_id: req-456"));
        assert!(error_str.contains("operation: api_call"));
    }

    #[test]
    fn retry_delay_suggestions() {
        let rate_limit_error = SpamPredictorError::rate_limit(120);
        assert_eq!(rate_limit_error.retry_delay_seconds(), Some(120));

        let timeout_error = SpamPredictorError::timeout(30);
        assert_eq!(timeout_error.retry_delay_seconds(), Some(5));

        let service_error = SpamPredictorError::service_unavailable("Down for maintenance");
        assert_eq!(service_error.retry_delay_seconds(), Some(30));

        let auth_error = SpamPredictorError::authentication("Invalid key");
        assert_eq!(auth_error.retry_delay_seconds(), None);
    }

    #[test]
    fn permanent_failure_classification() {
        let auth_error = SpamPredictorError::authentication("Invalid key");
        assert!(auth_error.is_permanent_failure());
        assert!(!auth_error.is_retryable());

        let config_error = SpamPredictorError::config("Bad config");
        assert!(config_error.is_permanent_failure());
        assert!(!config_error.is_retryable());

        let timeout_error = SpamPredictorError::timeout(30);
        assert!(!timeout_error.is_permanent_failure());
        assert!(timeout_error.is_retryable());
    }
}
