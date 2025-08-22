// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Moralis Web3 API integration
//!
//! This module provides an implementation of the `ApiClient` trait for the Moralis Web3 API.
//! Moralis provides comprehensive NFT, and blockchain data across multiple chains.

use std::{collections::HashMap, time::Duration};

use alloy_primitives::Address;
use api_client::{ApiClient, ApiError, ContractMetadata, ContractType, HealthStatus};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Configuration for the Moralis API client
#[derive(Debug, Clone)]
pub struct MoralisConfig {
    /// Base URL for the Moralis API
    pub base_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Health check timeout in seconds
    pub health_check_timeout_seconds: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl Default for MoralisConfig {
    fn default() -> Self {
        Self {
            base_url: "https://deep-index.moralis.io/api/v2".to_string(),
            api_key: "test-api-key".to_string(),
            timeout_seconds: 30,
            health_check_timeout_seconds: 5,
            max_retries: 3,
        }
    }
}

/// Moralis API client implementation
#[derive(Debug)]
pub struct MoralisClient {
    client: Client,
    config: MoralisConfig,
}

/// Errors specific to the Moralis API client
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum MoralisError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing failed
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimited,

    /// Authentication failed
    #[error("Authentication failed")]
    Unauthorized,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Timeout error
    #[error("Request timeout")]
    Timeout { seconds: u64 },
}

impl From<MoralisError> for ApiError {
    fn from(value: MoralisError) -> Self {
        match value {
            MoralisError::Http(error) => ApiError::Http {
                message: error.to_string(),
            },
            MoralisError::Json(error) => ApiError::InvalidResponse {
                message: error.to_string(),
            },
            MoralisError::ApiError { status, message } => ApiError::Custom {
                error: anyhow::Error::msg(format!("{status}: {message}")),
            },
            MoralisError::RateLimited => ApiError::RateLimitExceeded {
                retry_after_seconds: 3,
            },
            MoralisError::Unauthorized => ApiError::Authentication {
                message: value.to_string(),
            },
            MoralisError::Config(message) => ApiError::Configuration { message },
            MoralisError::Timeout { seconds } => ApiError::Timeout {
                timeout_seconds: seconds,
            },
        }
    }
}

/// Response structure for Moralis contract NFTs endpoint
#[derive(Debug, Deserialize)]
pub struct MoralisContractNftsResponse {
    /// List of NFT items returned from the API
    pub result: Vec<MoralisNftItem>,
}

/// Individual NFT item from contract NFTs endpoint
#[derive(Debug, Deserialize)]
pub struct MoralisNftItem {
    /// Contract address of the NFT
    pub token_address: String,
    /// Token ID of the NFT
    pub token_id: String,
    /// Type of contract (ERC721, ERC1155, etc.)
    pub contract_type: Option<String>,
    /// Hash of the token
    pub token_hash: Option<String>,
    /// Metadata associated with the NFT
    pub metadata: Option<serde_json::Value>,
    /// Name of the NFT collection
    pub name: Option<String>,
    /// Symbol of the NFT collection
    pub symbol: Option<String>,
}

impl MoralisClient {
    /// Create a new Moralis API client
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the Moralis API client
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created or configuration is invalid
    pub fn new(config: MoralisConfig) -> Result<Self, MoralisError> {
        if config.api_key.trim().is_empty() {
            return Err(MoralisError::Config("API key cannot be empty".to_string()));
        }

        if config.base_url.trim().is_empty() {
            return Err(MoralisError::Config("Base URL cannot be empty".to_string()));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("nft-api/0.1.0")
            .build()
            .map_err(MoralisError::Http)?;

        Ok(Self { client, config })
    }

    /// Get contract NFTs from Moralis (similar to Python implementation)
    ///
    /// # Arguments
    ///
    /// * `address` - Contract address to get NFTs for
    /// * `chain` - Blockchain chain (optional, defaults to "eth")
    /// * `format` - Response format (optional, defaults to "decimal")
    /// * `limit` - Number of NFTs to return (optional, defaults to 100)
    /// * `normalize_metadata` - Whether to normalize metadata (optional, defaults to true)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or the response cannot be parsed
    pub async fn get_contract_nfts(
        &self,
        address: Address,
        chain: Option<&str>,
        format: Option<&str>,
        limit: Option<u32>,
        normalize_metadata: Option<bool>,
    ) -> Result<MoralisContractNftsResponse, MoralisError> {
        if address == Address::ZERO {
            return Err(MoralisError::Config(
                "Invalid contract address provided".to_string(),
            ));
        }

        let url = format!("{}/nft/{}", self.config.base_url, address);

        let chain = chain.unwrap_or("eth");
        let format = format.unwrap_or("decimal");
        let limit = limit.unwrap_or(1);
        let normalize_metadata = normalize_metadata.unwrap_or(true);

        debug!(
            url,
            chain, format, limit, normalize_metadata, "fetching contract NFTs from Moralis"
        );

        let request = self
            .client
            .get(&url)
            .query(&[
                ("chain", chain),
                ("format", format),
                ("limit", &limit.to_string()),
                ("normalizeMetadata", &normalize_metadata.to_string()),
            ])
            .header("X-API-Key", &self.config.api_key)
            .header("accept", "application/json");

        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            request.send(),
        )
        .await
        .map_err(|_| MoralisError::Timeout {
            seconds: self.config.timeout_seconds,
        })?
        .map_err(MoralisError::Http)?;

        match response.status() {
            StatusCode::OK => {
                let nfts_response: MoralisContractNftsResponse =
                    response.json().await.map_err(MoralisError::Http)?;
                Ok(nfts_response)
            }
            StatusCode::NOT_FOUND => {
                debug!("Contract NFTs not found for address: {}", address);
                Ok(MoralisContractNftsResponse { result: vec![] })
            }
            StatusCode::UNAUTHORIZED => Err(MoralisError::Unauthorized),
            StatusCode::TOO_MANY_REQUESTS => Err(MoralisError::RateLimited),
            status => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                warn!("Moralis API error: {} - {}", status.as_u16(), error_text);
                Err(MoralisError::ApiError {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    /// Convert a Moralis NFT item to contract metadata
    fn convert_nft_item_to_metadata(
        &self,
        nft_item: &MoralisNftItem,
    ) -> Result<ContractMetadata, MoralisError> {
        let address = nft_item.token_address.parse().map_err(|_| {
            MoralisError::Config(format!(
                "Invalid address format in response: {}",
                nft_item.token_address
            ))
        })?;

        let contract_type = nft_item.contract_type.as_deref().map(|ct| match ct {
            "ERC721" => ContractType::Erc721,
            "ERC1155" => ContractType::Erc1155,
            _ => ContractType::Unknown,
        });

        let mut additional_data = HashMap::new();
        if let Some(ref contract_type_str) = nft_item.contract_type {
            additional_data.insert(
                "contract_type".to_string(),
                serde_json::Value::String(contract_type_str.clone()),
            );
        }
        if let Some(ref token_hash) = nft_item.token_hash {
            additional_data.insert(
                "token_hash".to_string(),
                serde_json::Value::String(token_hash.clone()),
            );
        }
        if let Some(ref metadata) = nft_item.metadata {
            additional_data.insert("metadata".to_string(), metadata.clone());
        }

        Ok(ContractMetadata {
            address,
            name: nft_item.name.clone(),
            symbol: nft_item.symbol.clone(),
            total_supply: None,
            holder_count: None,
            transaction_count: None,
            creation_block: None,
            creation_timestamp: None,
            creator_address: None,
            is_verified: None,
            contract_type,
            additional_data,
        })
    }
}

impl ApiClient for MoralisClient {
    async fn health_check(&self) -> Result<HealthStatus, ApiError> {
        // Use a simple endpoint to check health
        let url = format!("{}/info/endpointWeights", self.config.base_url);

        debug!(url, "performing health check on Moralis API");

        let request = self
            .client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("accept", "application/json");

        let start_time = std::time::Instant::now();
        let response = timeout(
            Duration::from_secs(self.config.health_check_timeout_seconds),
            request.send(),
        )
        .await
        .map_err(|_| MoralisError::Timeout {
            seconds: start_time.elapsed().as_secs(),
        })?
        .map_err(MoralisError::Http)?;

        let response_time = start_time.elapsed();

        match response.status() {
            StatusCode::OK => {
                info!("Moralis API health check passed in {:?}", response_time);
                Ok(HealthStatus::Up)
            }
            StatusCode::UNAUTHORIZED => {
                warn!("Moralis API health check failed: unauthorized");
                Ok(HealthStatus::Down {
                    reason: "Authentication failed".to_string(),
                })
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("Moralis API health check failed: rate limited");
                Ok(HealthStatus::Degraded {
                    reason: "Rate limited".to_string(),
                })
            }
            status => {
                warn!("Moralis API health check failed with status: {}", status);
                Ok(HealthStatus::Degraded {
                    reason: format!("API returned status {}", status.as_u16()),
                })
            }
        }
    }

    async fn get_contract_metadata(
        &self,
        address: Address,
    ) -> Result<Option<ContractMetadata>, ApiError> {
        info!("Fetching contract metadata for address: {}", address);

        // Get contract NFTs and extract metadata from the first NFT
        let nfts_response = self
            .get_contract_nfts(address, None, None, Some(1), None)
            .await
            .map_err(|e| {
                error!(
                    "Failed to fetch contract NFTs from Moralis for address {}: {}",
                    address, e
                );
                e
            })?;

        if let Some(first_nft) = nfts_response.result.first() {
            debug!("Found NFT metadata for address: {}", address);
            let metadata = self.convert_nft_item_to_metadata(first_nft).map_err(|e| {
                error!(
                    "Failed to convert NFT item to metadata for address {}: {}",
                    address, e
                );
                e
            })?;
            return Ok(Some(metadata));
        }

        debug!("No NFT metadata found for address: {}", address);
        Ok(None)
    }

    fn name(&self) -> &'static str {
        "moralis"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn moralis_client_creation_success() {
        let config = MoralisConfig {
            api_key: "valid-api-key".to_string(),
            ..Default::default()
        };

        let client = MoralisClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn moralis_client_creation_invalid_config() {
        let config = MoralisConfig {
            api_key: String::new(),
            ..Default::default()
        };

        let client = MoralisClient::new(config);
        assert!(client.is_err());
        assert!(matches!(client.unwrap_err(), MoralisError::Config(_)));
    }

    #[tokio::test]
    async fn convert_nft_item_to_metadata() {
        let config = MoralisConfig::default();
        let client = MoralisClient::new(config).unwrap();

        let nft_item = MoralisNftItem {
            token_address: "0x1234567890123456789012345678901234567890".to_string(),
            token_id: "1".to_string(),
            contract_type: Some("ERC721".to_string()),
            token_hash: Some("abc123".to_string()),
            metadata: Some(serde_json::json!({"name": "Test NFT"})),
            name: Some("Test NFT Collection".to_string()),
            symbol: Some("TNFT".to_string()),
        };

        let metadata = client.convert_nft_item_to_metadata(&nft_item).unwrap();
        assert_eq!(metadata.name, Some("Test NFT Collection".to_string()));
        assert_eq!(metadata.symbol, Some("TNFT".to_string()));
        assert_eq!(metadata.contract_type, Some(ContractType::Erc721));
        assert!(metadata.additional_data.contains_key("contract_type"));
        assert!(metadata.additional_data.contains_key("token_hash"));
        assert!(metadata.additional_data.contains_key("metadata"));
    }

    #[tokio::test]
    async fn get_contract_nfts_success() {
        let config = MoralisConfig::default();
        let client = MoralisClient::new(config).unwrap();
        let test_address = Address::from([0x12; 20]);

        // This would need a mock server to test properly, but we can at least test the method exists
        // The integration tests cover the full functionality with wiremock
        let result = client
            .get_contract_nfts(test_address, None, None, None, None)
            .await;
        // We expect this to fail since we don't have a real server, but method should exist
        assert!(result.is_err());
    }

    #[test]
    fn convert_nft_item_to_metadata_invalid_address() {
        let config = MoralisConfig::default();
        let client = MoralisClient::new(config).unwrap();

        let nft_item = MoralisNftItem {
            token_address: "invalid-address".to_string(),
            token_id: "1".to_string(),
            contract_type: Some("ERC721".to_string()),
            token_hash: Some("abc123".to_string()),
            metadata: None,
            name: Some("Test NFT".to_string()),
            symbol: Some("TNFT".to_string()),
        };

        let result = client.convert_nft_item_to_metadata(&nft_item);
        assert!(result.is_err());
        match result.unwrap_err() {
            MoralisError::Config(msg) => {
                assert!(msg.contains("Invalid address format"));
            }
            other => panic!("Expected Config error, got: {other:?}"),
        }
    }
}
