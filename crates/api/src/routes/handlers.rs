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
use shared_types::{ChainId, ChainImplementationStatus};
use tracing::error;
use utoipa::ToSchema;

use crate::{
    error::ServerError,
    extractors::JsonExtractor,
    state::{HealthCheck, ServerState},
};

/// Health check endpoint handler
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    summary = "Health check endpoint",
    description = "Returns the current health status of the API service including version, environment information, and status of all registered API clients (Moralis, Pinax, etc.).",
    responses(
        (status = 200, description = "Service is healthy", body = HealthCheck),
        (status = 503, description = "Service unavailable", body = String)
    )
)]
pub async fn health_handler(
    State(state): State<ServerState>,
) -> Result<impl IntoResponse, ServerError> {
    let health = state.health_check().await?;
    Ok(Json(health))
}

/// Contract status analysis request
///
/// Contains the contract address(es) to analyze for spam classification
/// and the blockchain chain to analyze them on.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ContractStatusRequest {
    /// Blockchain chain identifier
    #[schema(example = 137)]
    chain_id: ChainId,
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
    /// Blockchain chain identifier
    pub chain_id: ChainId,
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
    description = "Analyzes one or more blockchain contract addresses on a specific chain to determine if they are spam. Returns classification results with explanatory messages and chain implementation status.",
    request_body = ContractStatusRequest,
    responses(
        (status = 200, description = "Contract analysis completed successfully", body = ContractStatusResponse),
        (status = 400, description = "Invalid request - addresses list cannot be empty or unsupported chain", body = String),
        (status = 500, description = "Internal server error during analysis", body = String)
    )
)]
pub async fn contract_status_handler(
    State(state): State<ServerState>,
    JsonExtractor(contract_status): JsonExtractor<ContractStatusRequest>,
) -> Result<Json<ContractStatusResponse>, ServerError> {
    contract_status
        .validate()
        .map_err(|msg| ServerError::ValidationError(msg.to_string()))?;

    let chain_id = contract_status.chain_id;
    let implementation_status = chain_id.implementation_status();
    let api_registry = state.api_registry();
    let mut results = HashMap::new();

    for address in contract_status.addresses {
        let result = match implementation_status {
            ChainImplementationStatus::Full => {
                // Full implementation - perform normal analysis
                match api_registry.get_contract_metadata(address, chain_id).await {
                    Ok(Some(_metadata)) => ContractStatusResult {
                        chain_id,
                        contract_spam_status: false,
                        message: format!(
                            "contract metadata found on {}, analysis indicates not spam",
                            chain_id.name()
                        ),
                    },
                    Ok(None) => ContractStatusResult {
                        chain_id,
                        contract_spam_status: false,
                        message: format!("no data found for the contract on {}", chain_id.name()),
                    },
                    Err(e) => {
                        error!(
                            "failed to fetch contract metadata for {} on {}: {}",
                            address,
                            chain_id.name(),
                            e
                        );
                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: false,
                            message: format!(
                                "unable to retrieve contract data from external services for {}",
                                chain_id.name()
                            ),
                        }
                    }
                }
            }
            ChainImplementationStatus::Partial => {
                // Partial implementation - limited analysis with warning
                match api_registry.get_contract_metadata(address, chain_id).await {
                    Ok(Some(_metadata)) => ContractStatusResult {
                        chain_id,
                        contract_spam_status: false,
                        message: format!(
                            "contract metadata found on {} - {}",
                            chain_id.name(),
                            chain_id.status_message()
                        ),
                    },
                    Ok(None) => ContractStatusResult {
                        chain_id,
                        contract_spam_status: false,
                        message: format!(
                            "no data found for the contract on {} - {}",
                            chain_id.name(),
                            chain_id.status_message()
                        ),
                    },
                    Err(e) => {
                        error!(
                            "failed to fetch contract metadata for {} on {}: {}",
                            address,
                            chain_id.name(),
                            e
                        );
                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: false,
                            message: format!(
                                "unable to retrieve contract data for {} - {}",
                                chain_id.name(),
                                chain_id.status_message()
                            ),
                        }
                    }
                }
            }
            ChainImplementationStatus::Planned => {
                // Planned implementation - not yet available
                ContractStatusResult {
                    chain_id,
                    contract_spam_status: false,
                    message: format!(
                        "contract analysis for {} is {}",
                        chain_id.name(),
                        chain_id.status_message()
                    ),
                }
            }
        };

        results.insert(address, result);
    }

    Ok(Json(ContractStatusResponse { results }))
}
