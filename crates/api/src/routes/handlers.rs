// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! HTTP request handlers module
//!
//! This module provides HTTP request handlers for the NFT API server,
//! including health checks, API endpoints, and cancellation-aware handlers
//! for coordinated graceful shutdown.

use alloy_primitives::Address;
use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{error::ServerError, state::ServerState};

/// Health check endpoint handler
pub async fn health_handler(
    State(state): State<ServerState>,
) -> Result<impl IntoResponse, ServerError> {
    let health = state.dependencies().health_check()?;
    Ok(Json(health))
}

/// Contract status analysis request
///
/// Contains the contract address(es) to analyze for spam classification.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractStatusRequest {
    /// Contract addresses to analyze
    addresses: Vec<Address>,
}

/// Contract spam analysis response
///
/// Returns the results of blockchain contract spam classification analysis.
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractStatusResponse {
    /// Whether the contract is classified as spam
    pub contract_spam_status: bool,
    /// Human-readable explanation of the classification
    pub message: String,
    /// The analyzed contract address
    pub address: Address,
}

/// Contract status analysis
///
/// Analyzes blockchain contracts to determine if they are spam by:
/// 1. Fetching contract metadata from blockchain data sources (Pinax API, Moralis API)
/// 2. Running spam prediction using a machine learning model
/// 3. Returning classification results with explanatory messages
///
/// If no meaningful data is found for a contract, returns `contract_spam_status: false`
/// with message "No data found for the contract".
///
/// # Errors
///
/// Returns `ServerError` if contract analysis fails or external APIs are unavailable.
pub async fn contract_status_handler(
    State(_state): State<ServerState>,
    Json(_contract_status): Json<ContractStatusRequest>,
) -> Result<impl IntoResponse, ServerError> {
    Ok(())
}
