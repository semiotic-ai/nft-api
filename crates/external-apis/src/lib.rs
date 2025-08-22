// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! External API integrations for blockchain data providers
//!
//! This crate provides implementations of the `ApiClient` trait for various external
//! services that provide blockchain and NFT data, along with sophisticated orchestration
//! and failover capabilities through the registry pattern.
//!
//! # Architecture
//!
//! - **Client Implementations**: [`moralis`], [`pinax`] - specific API integrations
//! - **Registry Pattern**: [`registry::ApiRegistry`] - orchestrates multiple clients with failover
//! - **Validation Utilities**: [`non_empty_string::NonEmptyString`] - ensures non-empty string constraints
//!
//! # Features
//!
//! - **Automatic Failover**: Registry tries clients in order, skipping unhealthy ones
//! - **Concurrent Health Checks**: Uses `tokio::join!` for efficient health monitoring
//! - **Robust Error Handling**: Comprehensive error types for different failure scenarios
//! - **Configuration Validation**: Strong typing prevents invalid configurations
//! - **Testing Support**: Comprehensive test coverage using wiremock for HTTP simulation

pub mod moralis;
pub mod non_empty_string;
pub mod pinax;
pub mod registry;

pub use moralis::*;
pub use non_empty_string::NonEmptyString;
pub use pinax::*;
pub use registry::*;
