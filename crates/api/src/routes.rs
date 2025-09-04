// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Routes module
//!
//! This module provides route configuration and handlers for the NFT API server.

pub mod handlers;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use handlers::{contract_status_handler, health_handler};

use crate::{
    middleware::{RateLimiter, chain_validation_middleware, rate_limiting_middleware},
    openapi::{openapi_spec, swagger_ui},
    state::ServerState,
};

/// Create application routes with conditional rate limiting
#[allow(clippy::needless_pass_by_value)] // We need to clone the rate limiter for middleware
pub fn create_routes(rate_limiter: RateLimiter) -> Router<ServerState> {
    // Health endpoint is not rate limited for monitoring purposes
    let health_routes = Router::new().route("/health", get(health_handler));

    // Documentation endpoints are not rate limited
    let docs_routes = Router::new()
        .route("/api-doc/openapi.json", get(openapi_spec))
        .route("/swagger-ui", get(swagger_ui));

    // API endpoints - conditionally apply rate limiting
    let mut api_routes = Router::new().route("/contract/status", post(contract_status_handler));

    // Add chain validation middleware (always enabled for chain-specific endpoints)
    api_routes = api_routes.layer(middleware::from_fn(chain_validation_middleware));

    // Only apply rate limiting middleware if enabled
    if rate_limiter.is_enabled() {
        api_routes = api_routes.layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limiting_middleware,
        ));
    }

    let v1 = Router::new().nest("/v1", api_routes);

    Router::new()
        .merge(health_routes)
        .merge(docs_routes)
        .merge(v1)
}
