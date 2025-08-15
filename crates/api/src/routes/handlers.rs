// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! HTTP request handlers module
//!
//! This module provides HTTP request handlers for the NFT API server,
//! including health checks, API endpoints, and cancellation-aware handlers
//! for coordinated graceful shutdown.

use std::collections::HashMap;

use alloy_primitives::Address;
use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{dependencies::HealthCheck, error::ServerError, state::ServerState};

/// Health check endpoint handler
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    summary = "Health check endpoint",
    description = "Returns the current health status of the API service including version and environment information.",
    responses(
        (status = 200, description = "Service is healthy", body = HealthCheck),
        (status = 503, description = "Service unavailable", body = String)
    )
)]
pub async fn health_handler(
    State(state): State<ServerState>,
) -> Result<impl IntoResponse, ServerError> {
    let health = state.dependencies().health_check()?;
    Ok(Json(health))
}

/// Contract status analysis request
///
/// Contains the contract address(es) to analyze for spam classification.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ContractStatusRequest {
    /// Contract addresses to analyze (must not be empty)
    #[schema(value_type = Vec<String>, example = json!(["0x1234567890abcdef1234567890abcdef12345678"]))]
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

/// Individual contract analysis result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ContractStatusResult {
    /// Whether the contract is identified as spam
    pub contract_spam_status: bool,
    /// Human-readable message explaining the classification result
    pub message: String,
}

/// Response from the contract status endpoint
/// Maps contract addresses to their analysis results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ContractStatusResponse {
    /// Analysis results keyed by contract address
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub results: HashMap<Address, ContractStatusResult>,
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
#[utoipa::path(
    post,
    path = "/v1/contract/status",
    tag = "contracts",
    summary = "Analyze contract spam status",
    description = "Analyzes one or more blockchain contract addresses to determine if they are spam. Returns classification results with explanatory messages.",
    request_body = ContractStatusRequest,
    responses(
        (status = 200, description = "Contract analysis completed successfully", body = ContractStatusResponse),
        (status = 400, description = "Invalid request - addresses list cannot be empty", body = String),
        (status = 500, description = "Internal server error during analysis", body = String)
    )
)]
pub async fn contract_status_handler(
    State(_state): State<ServerState>,
    Json(contract_status): Json<ContractStatusRequest>,
) -> Result<Json<ContractStatusResponse>, ServerError> {
    contract_status
        .validate()
        .map_err(|msg| ServerError::ValidationError(msg.to_string()))?;

    let results = contract_status
        .addresses
        .into_iter()
        .map(|address| {
            (
                address,
                ContractStatusResult {
                    contract_spam_status: true,
                    message: "testing".to_owned(),
                },
            )
        })
        .collect();

    Ok(Json(ContractStatusResponse { results }))
}
