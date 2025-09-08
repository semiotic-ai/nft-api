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
use tracing::{debug, error, info, instrument, warn};
use utoipa::ToSchema;

use crate::{
    error::ServerError,
    extractors::JsonExtractor,
    state::{HealthCheck, ServerState},
};

/// Result of spam analysis operation
#[derive(Debug, Clone)]
struct SpamAnalysisResult {
    /// Whether the contract is classified as spam
    is_spam: bool,
    /// Human-readable analysis message
    message: &'static str,
}

/// Health check endpoint handler
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    summary = "System health check",
    description = "Returns comprehensive health status of the API service including version, environment, timestamp, and status of all external API clients and internal services (spam-predictor).",
    responses(
        (status = 200, description = "Health check completed successfully", body = HealthCheck),
        (status = 503, description = "Service unavailable due to critical system failures", body = String,
            example = json!("Critical system failure: Unable to initialize core services")
        )
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
#[schema(
    examples(
        json!({
            "chain_id": 1,
            "addresses": ["0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d"]
        }),
        json!({
            "chain_id": 137,
            "addresses": ["0x1234567890abcdef1234567890abcdef12345678", "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"]
        }),
        json!({
            "chain_id": 8453,
            "addresses": ["0x60e4d786628fea6478f785a6d7e704777c86a7c6"]
        }),
        json!({
            "chain_id": 43114,
            "addresses": ["0x071126cbec1c5562530ab85fd80dd3e3a42a70b8", "0xa7d7079b0fead91f3e65f86e8915cb59c1a4c664"]
        }),
        json!({
            "chain_id": 42161,
            "addresses": ["0x32400084c286cf3e17e7b677ea9583e60a000324"]
        })
    )
)]
pub struct ContractStatusRequest {
    /// Blockchain chain identifier
    #[schema(example = 1)]
    chain_id: ChainId,
    /// Contract addresses to analyze
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
#[schema(
    examples(
        json!({
            "chain_id": 1,
            "contract_spam_status": false,
            "message": "contract metadata found on Ethereum, AI analysis classified as legitimate"
        }),
        json!({
            "chain_id": 137,
            "contract_spam_status": true,
            "message": "contract metadata found on Polygon, AI analysis classified as spam"
        }),
        json!({
            "chain_id": 8453,
            "contract_spam_status": false,
            "message": "contract metadata found on Base, AI analysis classified as legitimate"
        }),
        json!({
            "chain_id": 43114,
            "contract_spam_status": false,
            "message": "contract metadata found on Avalanche, AI analysis classified as legitimate"
        }),
        json!({
            "chain_id": 42161,
            "contract_spam_status": false,
            "message": "no data found for the contract on Arbitrum"
        }),
        json!({
            "chain_id": 1,
            "contract_spam_status": false,
            "message": "unable to retrieve contract data from external services for Ethereum"
        })
    )
)]
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
#[schema(
    examples(
        json!({
            "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d": {
                "chain_id": 1,
                "contract_spam_status": false,
                "message": "contract metadata found on Ethereum, AI analysis classified as legitimate"
            }
        }),
        json!({
            "0x1234567890abcdef1234567890abcdef12345678": {
                "chain_id": 137,
                "contract_spam_status": false,
                "message": "contract metadata found on Polygon, AI analysis classified as legitimate"
            },
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd": {
                "chain_id": 137,
                "contract_spam_status": true,
                "message": "contract metadata found on Polygon, AI analysis classified as spam"
            }
        }),
        json!({
            "0x60e4d786628fea6478f785a6d7e704777c86a7c6": {
                "chain_id": 8453,
                "contract_spam_status": false,
                "message": "contract metadata found on Base, AI analysis classified as legitimate"
            }
        }),
        json!({
            "0x071126cbec1c5562530ab85fd80dd3e3a42a70b8": {
                "chain_id": 43114,
                "contract_spam_status": false,
                "message": "contract metadata found on Avalanche, AI analysis classified as legitimate"
            },
            "0xa7d7079b0fead91f3e65f86e8915cb59c1a4c664": {
                "chain_id": 43114,
                "contract_spam_status": false,
                "message": "no data found for the contract on Avalanche"
            }
        }),
        json!({
            "0x32400084c286cf3e17e7b677ea9583e60a000324": {
                "chain_id": 42161,
                "contract_spam_status": false,
                "message": "contract metadata found on Arbitrum, AI analysis classified as legitimate"
            }
        })
    )
)]
pub struct ContractStatusResponse {
    /// Analysis results keyed by contract address
    #[serde(flatten)]
    #[schema(value_type = HashMap<String, ContractStatusResult>)]
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
    description = "Analyzes one or more blockchain contract addresses on a specific chain to determine if they are spam. Supports all major blockchain networks including Ethereum (1), Polygon (137), Base (8453), Avalanche (43114), and Arbitrum (42161). Uses AI-powered classification with external blockchain data sources (Moralis API, Pinax Analytics).",
    request_body = ContractStatusRequest,
    responses(
        (status = 200, description = "Contract analysis completed successfully", body = ContractStatusResponse),
        (status = 400, description = "Invalid request - addresses list cannot be empty, unsupported chain, or malformed addresses", body = String),
        (status = 429, description = "Rate limit exceeded - too many requests", body = String,
            example = json!("Rate limit exceeded.")
        ),
        (status = 500, description = "Internal server error during analysis", body = String)
    )
)]
#[allow(clippy::too_many_lines)] // Complex handler with multiple analysis steps
#[instrument(skip(state, contract_status), fields(
    chain_id = %contract_status.chain_id,
    addresses_count = contract_status.addresses.len(),
    chain_implementation = %contract_status.chain_id.implementation_status()
))]
pub async fn contract_status_handler(
    State(state): State<ServerState>,
    JsonExtractor(contract_status): JsonExtractor<ContractStatusRequest>,
) -> Result<Json<ContractStatusResponse>, ServerError> {
    let start_time = std::time::Instant::now();
    info!(
        chain_id = %contract_status.chain_id,
        addresses_count = contract_status.addresses.len(),
        "starting contract status analysis"
    );
    contract_status
        .validate()
        .map_err(|msg| ServerError::ValidationError(msg.to_string()))?;

    let chain_id = contract_status.chain_id;
    crate::metrics::inc_requests_by_chain(chain_id);
    let implementation_status = chain_id.implementation_status();
    let api_registry = state.api_registry();
    let mut results = HashMap::new();

    for (index, address) in contract_status.addresses.iter().enumerate() {
        debug!(
            address = %address,
            index = index,
            chain_id = %chain_id,
            "processing contract address"
        );

        let start = std::time::Instant::now();
        let result = match implementation_status {
            ChainImplementationStatus::Full => {
                // Full implementation - perform normal analysis
                match api_registry.get_contract_metadata(*address, chain_id).await {
                    Ok(Some(metadata)) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "found",
                            start.elapsed().as_secs_f64(),
                        );
                        // Perform spam prediction if spam predictor is available
                        let analysis_result =
                            perform_spam_analysis(&metadata, state.spam_predictor(), *address)
                                .await;

                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: analysis_result.is_spam,
                            message: format!(
                                "contract metadata found on {}, {}",
                                chain_id.name(),
                                analysis_result.message
                            ),
                        }
                    }
                    Ok(None) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "missing",
                            start.elapsed().as_secs_f64(),
                        );
                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: false,
                            message: format!(
                                "no data found for the contract on {}",
                                chain_id.name()
                            ),
                        }
                    }
                    Err(e) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "error",
                            start.elapsed().as_secs_f64(),
                        );
                        error!(
                            "failed to fetch contract metadata for {} on {}: {}",
                            *address,
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
                match api_registry.get_contract_metadata(*address, chain_id).await {
                    Ok(Some(metadata)) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "found",
                            start.elapsed().as_secs_f64(),
                        );
                        // Perform spam prediction if spam predictor is available
                        let analysis_result =
                            perform_spam_analysis(&metadata, state.spam_predictor(), *address)
                                .await;

                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: analysis_result.is_spam,
                            message: format!(
                                "contract metadata found on {} - {} - {}",
                                chain_id.name(),
                                chain_id.status_message(),
                                analysis_result.message
                            ),
                        }
                    }
                    Ok(None) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "missing",
                            start.elapsed().as_secs_f64(),
                        );
                        ContractStatusResult {
                            chain_id,
                            contract_spam_status: false,
                            message: format!(
                                "no data found for the contract on {} - {}",
                                chain_id.name(),
                                chain_id.status_message()
                            ),
                        }
                    }
                    Err(e) => {
                        crate::metrics::observe_metadata_api_duration(
                            "external_api",
                            "error",
                            start.elapsed().as_secs_f64(),
                        );
                        error!(
                            "failed to fetch contract metadata for {} on {}: {}",
                            *address,
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

        results.insert(*address, result);
    }

    let duration = start_time.elapsed();
    let spam_count = results.values().filter(|r| r.contract_spam_status).count();
    let total_addresses = results.len();

    info!(
        chain_id = %chain_id,
        total_addresses = total_addresses,
        spam_contracts = spam_count,
        legitimate_contracts = total_addresses - spam_count,
        duration_ms = duration.as_millis(),
        "contract status analysis completed"
    );

    debug!(
        results_summary = ?results.iter().map(|(addr, result)| (addr, result.contract_spam_status)).collect::<Vec<_>>(),
        "detailed results summary"
    );

    Ok(Json(ContractStatusResponse { results }))
}

/// Perform spam analysis on contract metadata
///
/// Returns a `SpamAnalysisResult` containing the spam classification and analysis message
#[instrument(skip(metadata, spam_predictor), fields(
    contract_address = %contract_address
))]
async fn perform_spam_analysis(
    metadata: &api_client::ContractMetadata,
    spam_predictor: &std::sync::Arc<spam_predictor::SpamPredictor>,
    contract_address: Address,
) -> SpamAnalysisResult {
    let start_time = std::time::Instant::now();
    debug!(contract_address = %contract_address, "starting ai spam prediction");

    // Create typed prediction request
    let request = spam_predictor::SpamPredictionRequest::spam_classification(metadata.clone());

    let result = match spam_predictor.predict_spam_typed(request).await {
        Ok(prediction_result) => {
            let is_spam = prediction_result.classification().is_spam();
            let duration = start_time.elapsed().as_secs_f64();
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let duration_ms = (duration * 1000.0) as u128;

            crate::metrics::observe_spam_predictor_duration("success", duration);

            if is_spam {
                info!(
                    contract_address = %contract_address,
                    ?duration_ms,
                    "ai analysis classified contract as spam"
                );
                SpamAnalysisResult {
                    is_spam: true,
                    message: "AI analysis classified as spam",
                }
            } else {
                info!(
                    contract_address = %contract_address,
                    ?duration_ms,
                    "ai analysis classified contract as legitimate"
                );
                SpamAnalysisResult {
                    is_spam: false,
                    message: "AI analysis classified as legitimate",
                }
            }
        }
        Err(e) => {
            let duration = start_time.elapsed().as_secs_f64();
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let duration_ms = (duration * 1000.0) as u128;
            warn!(
                contract_address = %contract_address,
                ?duration_ms,
                error = %e,
                "spam prediction failed"
            );
            crate::metrics::observe_spam_predictor_duration("error", duration);
            SpamAnalysisResult {
                is_spam: false,
                message: "spam prediction unavailable, defaulting to not spam",
            }
        }
    };

    debug!(
        contract_address = %contract_address,
        is_spam = result.is_spam,
        message = result.message,
        total_duration_ms = start_time.elapsed().as_millis(),
        "spam analysis completed"
    );

    result
}
