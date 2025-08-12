// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! HTTP request handlers module
//!
//! This module provides HTTP request handlers for the NFT API server,
//! including health checks, API endpoints, and cancellation-aware handlers
//! for coordinated graceful shutdown.

use axum::{
    Json,
    extract::{Request, State},
    response::IntoResponse,
};

use crate::{error::ServerError, state::ServerState};

/// Health check endpoint handler
pub async fn health_handler(
    State(state): State<ServerState>,
    _request: Request,
) -> Result<impl IntoResponse, ServerError> {
    let health = state.dependencies().health_check()?;
    Ok(Json(health))
}
