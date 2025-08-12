// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Server state management module
//!
//! This module provides shared application state for the NFT API server,
//! including configuration, dependency management, and coordinated cancellation.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::{config::ServerConfig, dependencies::Dependencies};

/// Shared application state with cancellation token support
#[derive(Debug, Clone)]
pub struct ServerState {
    /// Server configuration
    config: ServerConfig,
    /// Injected dependencies
    dependencies: Arc<dyn Dependencies>,
    /// Cancellation token for coordinated shutdown
    pub cancellation_token: CancellationToken,
}

impl ServerState {
    /// Create new server state
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `dependencies` - Injected dependencies
    /// * `cancellation_token` - Token for coordinated cancellation
    pub fn new(
        config: ServerConfig,
        dependencies: Arc<dyn Dependencies>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            config,
            dependencies,
            cancellation_token,
        }
    }

    /// Server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Server configuration
    pub fn dependencies(&self) -> &Arc<dyn Dependencies> {
        &self.dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependencies::DefaultDependencies;

    #[test]
    fn server_state_creation() {
        let config = ServerConfig::default();
        let deps = Arc::new(DefaultDependencies::new());
        let state = ServerState::new(config, deps, CancellationToken::new());

        assert!(!state.cancellation_token.is_cancelled());
    }

    #[test]
    fn server_state_with_cancellation_token() {
        let config = ServerConfig::default();
        let deps = Arc::new(DefaultDependencies::new());
        let token = CancellationToken::new();
        let state = ServerState::new(config, deps, token.clone());

        assert!(!state.cancellation_token.is_cancelled());

        // Test that the tokens are linked
        token.cancel();
        assert!(state.cancellation_token.is_cancelled());
    }
}
