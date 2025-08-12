// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! NFT API Server Implementation
//! # Module Structure
//!
//! - [`config`]: Server configuration and environment management
//! - [`error`]: Error types and HTTP response handling
//! - [`dependencies`]: Dependency injection framework
//! - [`state`]: Shared application state management with cancellation token support
//! - [`server`]: Main server implementation, lifecycle, and coordinated shutdown
//! - [`handlers`]: HTTP request handlers with cancellation awareness

pub mod config;
pub mod dependencies;
pub mod error;
pub mod handlers;
pub mod server;
pub mod state;

pub use config::{Environment, ServerConfig};
pub use dependencies::{DefaultDependencies, Dependencies, HealthCheck};
pub use error::{ServerError, ServerResult};
pub use server::{Server, ShutdownConfig};
pub use state::ServerState;
