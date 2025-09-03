// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Generic API client traits and utilities for external integrations
//!
//! This crate provides common abstractions for external API clients, designed for
//! use with blockchain data providers.
//!
//! # Core Abstractions
//!
//! - **`ApiClient` Trait**: Common interface for all external API clients with async support
//! - **Health Check System**: Standardized health status reporting across all clients
//! - **Error Handling**: Comprehensive `ApiError` types for different failure scenarios
//! - **Data Types**: Common structures for contract metadata and blockchain data
//!
//! # Key Features
//!
//! - **Async-First Design**: All operations return `impl Future` for efficient async execution
//! - **Health Monitoring**: Built-in health check with `Up`, `Degraded`, and `Down` statuses
//! - **Error Classification**: Detailed error types for authentication, rate limiting, network issues
//! - **Type Safety**: Strong typing prevents runtime errors from invalid configurations

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use shared_types::ChainId;
use thiserror::Error;

pub mod health;
pub mod types;

pub use health::*;
pub use types::*;

/// Generic trait for external API clients
///
/// This trait provides a common interface for all external API integrations,
/// enabling consistent error handling, health checks, and contract data retrieval.
pub trait ApiClient: Send + Sync {
    /// Check the health of this API client
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails
    fn health_check(&self) -> impl Future<Output = Result<HealthStatus, ApiError>> + Send;

    /// Get contract metadata for the given address on a specific blockchain chain
    ///
    /// # Arguments
    ///
    /// * `address` - The contract address to retrieve metadata for
    /// * `chain_id` - The blockchain chain to query for the contract
    ///
    /// # Returns
    ///
    /// * `Ok(Some(metadata))` if the contract exists and metadata was retrieved
    /// * `Ok(None)` if the contract doesn't exist or no metadata is available
    /// * `Err(error)` if there was an error retrieving the data
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails, rate limits are exceeded,
    /// the chain is not supported, or there are network/authentication issues
    fn get_contract_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
    ) -> impl Future<Output = Result<Option<ContractMetadata>, ApiError>> + Send;

    /// Get the name/identifier of this API client
    fn name(&self) -> &'static str;
}

/// Configuration for rate limiting behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Burst capacity for handling traffic spikes
    pub burst_capacity: u32,
    /// How long to wait before retrying after rate limit
    pub retry_after_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_capacity: 20,
            retry_after_seconds: 60,
        }
    }
}

/// Common errors that can occur when working with API clients
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum ApiError {
    /// HTTP request failed
    #[error("HTTP request failed: {message}")]
    Http { message: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after_seconds} seconds")]
    RateLimitExceeded { retry_after_seconds: u64 },

    /// Authentication failed
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Invalid response format
    #[error("Invalid response format: {message}")]
    InvalidResponse { message: String },

    /// Service unavailable
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Network timeout
    #[error("Request timeout after {timeout_seconds} seconds")]
    Timeout { timeout_seconds: u64 },

    /// Client independent error
    #[error(transparent)]
    Custom { error: anyhow::Error },
}
