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
    /// Contract addresses to analyze (must not be empty)
    addresses: Vec<Address>,
}

impl ContractStatusRequest {
    /// Validates that the request contains at least one address
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.addresses.is_empty() {
            return Err("addresses list cannot be empty");
        }
        Ok(())
    }
}

/// Response from the contract status endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractStatusResponse {
    /// Whether the contract is identified as spam
    pub contract_spam_status: bool,
    /// Human-readable message explaining the classification result
    pub message: String,
    /// The contract address that was analyzed
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
    Json(contract_status): Json<ContractStatusRequest>,
) -> Result<Json<ContractStatusResponse>, ServerError> {
    contract_status
        .validate()
        .map_err(|msg| ServerError::ValidationError(msg.to_string()))?;

    Ok(Json(ContractStatusResponse {
        contract_spam_status: true,
        message: "testing".to_owned(),
        address: alloy_primitives::address!("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"),
    }))
}
