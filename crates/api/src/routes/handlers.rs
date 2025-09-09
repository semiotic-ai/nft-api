// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! HTTP request handlers module
//!
//! This module provides HTTP request handlers for the NFT API server,
//! including health checks, API endpoints, and cancellation-aware handlers
//! for coordinated graceful shutdown.

use std::{collections::HashMap, sync::Arc};

use alloy_primitives::Address;
use axum::{Json, extract::State, response::IntoResponse};
use external_apis::ApiRegistry;
use serde::{Deserialize, Serialize};
use shared_types::{ChainId, ChainImplementationStatus, ContractSpamStatus};
use spam_predictor::SpamPredictor;
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
    /// Spam classification status
    status: ContractSpamStatus,
    /// Human-readable analysis message
    message: String,
    /// Optional reasoning from AI analysis
    reasoning: Option<String>,
    /// Processing time for analysis in milliseconds
    processing_time_ms: Option<u64>,
    /// Whether result was cached
    cached: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(
    examples(
        json!({
            "chain_id": 1,
            "status": "legitimate",
            "message": "contract metadata found on Ethereum, AI analysis classified as legitimate",
            "reasoning": "AI analysis classified as legitimate",
            "processing_time_ms": 150,
            "cached": false
        }),
        json!({
            "chain_id": 137,
            "status": "spam",
            "message": "contract metadata found on Polygon, AI analysis classified as spam",
            "reasoning": "exhibits known scam patterns",
            "processing_time_ms": 221,
            "cached": false
        }),
        json!({
            "chain_id": 8453,
            "status": "legitimate",
            "message": "contract metadata found on Base, AI analysis classified as legitimate",
            "reasoning": null,
            "processing_time_ms": 181,
            "cached": true
        }),
        json!({
            "chain_id": 43114,
            "status": "inconclusive",
            "message": "contract metadata found on Avalanche, AI analysis was inconclusive, defaulting to not spam",
            "reasoning": "insufficient data for reliable classification",
            "processing_time_ms": 96,
            "cached": false
        }),
        json!({
            "chain_id": 42161,
            "status": "no_data",
            "message": "no data found for the contract on Arbitrum",
            "reasoning": null,
            "processing_time_ms": null,
            "cached": false
        }),
        json!({
            "chain_id": 1,
            "status": "error",
            "message": "unable to retrieve contract data from external services for Ethereum",
            "reasoning": "External API error: timeout",
            "processing_time_ms": null,
            "cached": false
        })
    )
)]
pub struct ContractStatusResult {
    /// Blockchain chain identifier
    pub chain_id: ChainId,
    /// Contract spam classification status
    pub status: ContractSpamStatus,
    /// Human-readable message explaining the classification result
    pub message: String,
    /// Optional reasoning from AI analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Processing time for analysis in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,
    /// Whether result was cached
    pub cached: bool,
}

/// Response from the contract status endpoint
/// Maps contract addresses to their analysis results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(
    examples(
        json!({
            "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d": {
                "chain_id": 1,
                "status": "legitimate",
                "message": "contract metadata found on Ethereum, AI analysis classified as legitimate",
                "reasoning": "AI analysis classified as legitimate",
                "processing_time_ms": 150,
                "cached": false
            }
        }),
        json!({
            "0x1234567890abcdef1234567890abcdef12345678": {
                "chain_id": 137,
                "status": "legitimate",
                "message": "contract metadata found on Polygon, AI analysis classified as legitimate",
                "reasoning": null,
                "processing_time_ms": 120,
                "cached": true
            },
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd": {
                "chain_id": 137,
                "status": "spam",
                "message": "contract metadata found on Polygon, AI analysis classified as spam",
                "reasoning": "exhibits scam contract patterns",
                "processing_time_ms": 301,
                "cached": false
            }
        }),
        json!({
            "0x60e4d786628fea6478f785a6d7e704777c86a7c6": {
                "chain_id": 8453,
                "status": "legitimate",
                "message": "contract metadata found on Base, AI analysis classified as legitimate",
                "reasoning": "AI analysis classified as legitimate",
                "processing_time_ms": 181,
                "cached": false
            }
        }),
        json!({
            "0x071126cbec1c5562530ab85fd80dd3e3a42a70b8": {
                "chain_id": 43114,
                "status": "legitimate",
                "message": "contract metadata found on Avalanche, AI analysis classified as legitimate",
                "reasoning": "AI analysis classified as legitimate",
                "processing_time_ms": 200,
                "cached": false
            },
            "0xa7d7079b0fead91f3e65f86e8915cb59c1a4c664": {
                "chain_id": 43114,
                "status": "no_data",
                "message": "no data found for the contract on Avalanche",
                "reasoning": null,
                "processing_time_ms": null,
                "cached": false
            }
        }),
        json!({
            "0x32400084c286cf3e17e7b677ea9583e60a000324": {
                "chain_id": 42161,
                "status": "legitimate",
                "message": "contract metadata found on Arbitrum, AI analysis classified as legitimate",
                "reasoning": "AI analysis classified as legitimate",
                "processing_time_ms": 160,
                "cached": false
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

/// Process a single contract address for spam analysis
///
/// Handles the complete analysis pipeline for a single address including:
/// - Fetching metadata from external APIs
/// - Running spam prediction analysis
/// - Building the appropriate result based on chain implementation status
#[instrument(skip(api_registry, spam_predictor), fields(
    address = %address,
    chain_id = %chain_id,
    implementation_status = %implementation_status
))]
async fn process_single_address(
    address: Address,
    chain_id: ChainId,
    implementation_status: ChainImplementationStatus,
    api_registry: &ApiRegistry,
    spam_predictor: &Arc<SpamPredictor>,
) -> ContractStatusResult {
    debug!(
        address = %address,
        chain_id = %chain_id,
        "processing contract address"
    );

    match implementation_status {
        ChainImplementationStatus::Full => {
            process_with_full_implementation(address, chain_id, api_registry, spam_predictor).await
        }
        ChainImplementationStatus::Partial => {
            process_with_partial_implementation(address, chain_id, api_registry, spam_predictor)
                .await
        }
        ChainImplementationStatus::Planned => ContractStatusResult {
            chain_id,
            status: ContractSpamStatus::NoData,
            message: format!(
                "contract analysis for {} is {}",
                chain_id.name(),
                chain_id.status_message()
            ),
            reasoning: None,
            processing_time_ms: None,
            cached: false,
        },
    }
}

async fn process_with_full_implementation(
    address: Address,
    chain_id: ChainId,
    api_registry: &ApiRegistry,
    spam_predictor: &Arc<SpamPredictor>,
) -> ContractStatusResult {
    let start = std::time::Instant::now();

    match api_registry.get_contract_metadata(address, chain_id).await {
        Ok(Some(metadata)) => {
            crate::metrics::observe_metadata_api_duration(
                "external_api",
                "found",
                start.elapsed().as_secs_f64(),
            );
            let analysis_result = perform_spam_analysis(&metadata, spam_predictor, address).await;

            ContractStatusResult {
                chain_id,
                status: analysis_result.status.clone(),
                message: format!(
                    "contract metadata found on {}, {}",
                    chain_id.name(),
                    analysis_result.message
                ),
                reasoning: analysis_result.reasoning.clone(),
                processing_time_ms: analysis_result.processing_time_ms,
                cached: analysis_result.cached,
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
                status: ContractSpamStatus::NoData,
                message: format!("no data found for the contract on {}", chain_id.name()),
                reasoning: None,
                processing_time_ms: None,
                cached: false,
            }
        }
        Err(e) => {
            crate::metrics::observe_metadata_api_duration(
                "external_api",
                "error",
                start.elapsed().as_secs_f64(),
            );
            error!(
                %address,
                chain = chain_id.name(),
                error = %e,
                "failed to fetch contract metadata"
            );
            ContractStatusResult {
                chain_id,
                status: ContractSpamStatus::Error,
                message: format!(
                    "unable to retrieve contract data from external services for {}",
                    chain_id.name()
                ),
                reasoning: Some(format!("External API error: {e}")),
                processing_time_ms: None,
                cached: false,
            }
        }
    }
}

async fn process_with_partial_implementation(
    address: Address,
    chain_id: ChainId,
    api_registry: &ApiRegistry,
    spam_predictor: &Arc<SpamPredictor>,
) -> ContractStatusResult {
    let start = std::time::Instant::now();

    match api_registry.get_contract_metadata(address, chain_id).await {
        Ok(Some(metadata)) => {
            crate::metrics::observe_metadata_api_duration(
                "external_api",
                "found",
                start.elapsed().as_secs_f64(),
            );
            let analysis_result = perform_spam_analysis(&metadata, spam_predictor, address).await;

            ContractStatusResult {
                chain_id,
                status: analysis_result.status.clone(),
                message: format!(
                    "contract metadata found on {} - {} - {}",
                    chain_id.name(),
                    chain_id.status_message(),
                    analysis_result.message
                ),
                reasoning: analysis_result.reasoning.clone(),
                processing_time_ms: analysis_result.processing_time_ms,
                cached: analysis_result.cached,
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
                status: ContractSpamStatus::NoData,
                message: format!(
                    "no data found for the contract on {} - {}",
                    chain_id.name(),
                    chain_id.status_message()
                ),
                reasoning: None,
                processing_time_ms: None,
                cached: false,
            }
        }
        Err(e) => {
            crate::metrics::observe_metadata_api_duration(
                "external_api",
                "error",
                start.elapsed().as_secs_f64(),
            );
            error!(
                %address,
                chain = chain_id.name(),
                error = %e,
                "failed to fetch contract metadata"
            );
            ContractStatusResult {
                chain_id,
                status: ContractSpamStatus::Error,
                message: format!(
                    "unable to retrieve contract data for {} - {}",
                    chain_id.name(),
                    chain_id.status_message()
                ),
                reasoning: Some(format!("External API error: {e}")),
                processing_time_ms: None,
                cached: false,
            }
        }
    }
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

    for address in &contract_status.addresses {
        let result = process_single_address(
            *address,
            chain_id,
            implementation_status,
            api_registry,
            state.spam_predictor(),
        )
        .await;
        results.insert(*address, result);
    }

    let duration = start_time.elapsed();
    let spam_count = results.values().filter(|r| r.status.is_spam()).count();
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
        results_summary = ?results.iter().map(|(addr, result)| (addr, &result.status)).collect::<Vec<_>>(),
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
    spam_predictor: &Arc<SpamPredictor>,
    contract_address: Address,
) -> SpamAnalysisResult {
    let start_time = std::time::Instant::now();
    debug!(contract_address = %contract_address, "starting ai spam prediction");

    // Create typed prediction request
    let request = spam_predictor::SpamPredictionRequest::spam_classification(metadata.clone());

    let result = match spam_predictor.predict_spam_typed(request).await {
        Ok(prediction_result) => {
            let duration = start_time.elapsed();
            let duration_f64 = duration.as_secs_f64();
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let duration_ms = (duration_f64 * 1000.0) as u128;

            crate::metrics::observe_spam_predictor_duration("success", duration_f64);

            let status = ContractSpamStatus::from(prediction_result.classification());

            info!(
                contract_address = %contract_address,
                ?duration_ms,
                status = ?status,
                "ai analysis completed"
            );

            let message = status.default_message().to_owned();
            SpamAnalysisResult {
                status,
                message,
                reasoning: prediction_result.reasoning().map(ToString::to_string),
                processing_time_ms: Some(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)),
                cached: prediction_result.is_cached(),
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            let duration_f64 = duration.as_secs_f64();
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let duration_ms = (duration_f64 * 1000.0) as u128;
            warn!(
                contract_address = %contract_address,
                ?duration_ms,
                error = %e,
                "spam prediction failed"
            );
            crate::metrics::observe_spam_predictor_duration("error", duration_f64);
            SpamAnalysisResult {
                status: ContractSpamStatus::Error,
                message: "prediction failed".to_string(),
                reasoning: Some(format!("Prediction error: {e}")),
                processing_time_ms: Some(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)),
                cached: false,
            }
        }
    };

    debug!(
        contract_address = %contract_address,
        status = ?result.status,
        message = result.message,
        total_duration_ms = start_time.elapsed().as_millis(),
        "spam analysis completed"
    );

    result
}
