// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! NFT API Server Implementation
//!
//! This crate provides the main HTTP server for the NFT API service, built with Axum
//! and designed for production use with comprehensive configuration, middleware, and
//! graceful shutdown capabilities.
//!
//! # Module Structure
//!
//! - [`config`]: Server configuration and environment management with hierarchical loading
//! - [`error`]: Error types and HTTP response handling with proper status codes
//! - [`state`]: Shared application state management with cancellation token support
//! - [`server`]: Main server implementation, lifecycle, and coordinated shutdown
//! - [`routes`]: Route configuration and HTTP request handlers with cancellation awareness
//! - [`middleware`]: Rate limiting, request tracing, and cross-cutting concerns
//! - [`openapi`]: `OpenAPI` specification and Swagger UI endpoints for API documentation
//!
//! # Key Features
//!
//! - **External API Integration**: Orchestrates Moralis and Pinax clients via registry pattern
//! - **Graceful Shutdown**: Coordinated termination using `CancellationToken` with timeouts
//! - **Rate Limiting**: IP-based request limiting with configurable requests per minute
//! - **Health Monitoring**: Aggregated health checks across all external API providers
//! - **Production Safety**: Validates credentials, enforces security policies
//! - **Comprehensive Middleware**: Request tracing, CORS, timeouts, and error handling

pub mod config;
pub mod docs;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod server;
pub mod state;

pub use config::{Environment, ServerConfig};
pub use error::{ServerError, ServerResult};
pub use server::{Server, ShutdownConfig};
pub use shared_types::{ChainId, ChainImplementationStatus};
pub use state::{HealthCheck, ServerState};
