// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Routes module
//!
//! This module provides route configuration and handlers for the NFT API server.

pub mod handlers;

use axum::{
    Router,
    routing::{get, post},
};
use handlers::{contract_status_handler, health_handler};

use crate::state::ServerState;

/// Create application routes
pub fn create_routes() -> Router<ServerState> {
    let contract_status = Router::new().route("/contract/status", post(contract_status_handler));
    let v1 = Router::new().nest("/v1", contract_status);

    Router::new()
        .route("/health", get(health_handler))
        .merge(v1)
}
