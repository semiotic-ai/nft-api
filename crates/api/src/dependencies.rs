// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Dependency injection module
//!
//! This module provides the dependency injection framework for the NFT API server,
//! including traits for injectable dependencies and default implementations.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{config::Environment, error::ServerResult};

/// Trait for dependency injection
pub trait Dependencies: Send + Sync + fmt::Debug {
    /// Get service name for health checks
    fn service_name(&self) -> &str;

    /// Perform health check operations
    fn health_check(&self) -> ServerResult<HealthCheck>;
}

/// Default dependencies implementation
#[derive(Debug)]
pub struct DefaultDependencies {
    service_name: String,
}

/// Health status of a service or dependency
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Service status
    pub status: HealthStatus,
    /// Service version
    pub version: Box<str>,
    /// Environment
    pub environment: Environment,
    /// Timestamp
    pub timestamp: String,
}

impl DefaultDependencies {
    /// Create new default dependencies
    pub fn new() -> Self {
        Self {
            service_name: "nft-api".to_string(),
        }
    }
}

impl Default for DefaultDependencies {
    fn default() -> Self {
        Self::new()
    }
}

impl Dependencies for DefaultDependencies {
    fn service_name(&self) -> &str {
        &self.service_name
    }

    fn health_check(&self) -> ServerResult<HealthCheck> {
        Ok(HealthCheck {
            status: HealthStatus::Up,
            version: Box::from(env!("CARGO_PKG_VERSION")),
            environment: Environment::Development,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_dependencies() -> ServerResult<()> {
        let deps = DefaultDependencies::new();
        assert_eq!(deps.service_name(), "nft-api");

        let health = deps.health_check()?;
        assert_eq!(health.status, HealthStatus::Up);
        assert_eq!(&*health.version, env!("CARGO_PKG_VERSION"));

        Ok(())
    }
}
