// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Pinax API integration
//!
//! This module provides an implementation of the `ApiClient` trait for the Pinax API.
//! Pinax provides blockchain data through SQL-like queries via HTTP endpoints.

use std::{collections::HashMap, time::Duration};

use alloy_primitives::Address;
use api_client::{ApiClient, ApiError, ContractMetadata, ContractType, HealthStatus};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use shared_types::ChainId;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::non_empty_string::NonEmptyString;

// Pinax API constants
const DEFAULT_PINAX_TIMEOUT_SECONDS: u64 = 20;
const DEFAULT_PINAX_HEALTH_CHECK_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_PINAX_MAX_RETRIES: u32 = 3;

/// Configuration for the Pinax API client
/// This type is always valid by construction.
#[derive(Debug, Clone)]
pub struct PinaxConfig {
    /// Base URL for the Pinax API endpoint
    pub endpoint: NonEmptyString,
    /// Username for Pinax API authentication
    pub api_user: NonEmptyString,
    /// Password/token for Pinax API authentication
    pub api_auth: NonEmptyString,
    /// Database name in the Pinax environment
    pub db_name: NonEmptyString,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Health check timeout in seconds
    pub health_check_timeout_seconds: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl PinaxConfig {
    /// Create a new `PinaxConfig` with validation
    pub fn new(
        endpoint: impl Into<String>,
        api_user: impl Into<String>,
        api_auth: impl Into<String>,
        db_name: impl Into<String>,
        timeout_seconds: u64,
        health_check_timeout_seconds: u64,
        max_retries: u32,
    ) -> Result<Self, String> {
        Ok(Self {
            endpoint: NonEmptyString::new(endpoint)?,
            api_user: NonEmptyString::new(api_user)?,
            api_auth: NonEmptyString::new(api_auth)?,
            db_name: NonEmptyString::new(db_name)?,
            timeout_seconds,
            health_check_timeout_seconds,
            max_retries,
        })
    }

    /// Create default configuration for testing
    #[allow(clippy::missing_panics_doc)]
    pub fn default_test() -> Self {
        Self {
            endpoint: NonEmptyString::new("https://api.pinax.network/sql")
                .expect("known to be non-empty"),
            api_user: NonEmptyString::new("test-user").expect("known to be non-empty"),
            api_auth: NonEmptyString::new("test-auth").expect("known to be non-empty"),
            db_name: NonEmptyString::new("mainnet:evm-nft-tokens@v0.6.2")
                .expect("known to be non-empty"),
            timeout_seconds: DEFAULT_PINAX_TIMEOUT_SECONDS,
            health_check_timeout_seconds: DEFAULT_PINAX_HEALTH_CHECK_TIMEOUT_SECONDS,
            max_retries: DEFAULT_PINAX_MAX_RETRIES,
        }
    }
}

/// Per-chain Pinax configuration override
#[derive(Debug, Clone)]
pub struct PerChainPinaxConfig {
    /// Chain-specific database name override
    pub db_name: Option<String>,
    /// Chain-specific timeout override
    pub timeout_seconds: Option<u64>,
    /// Chain-specific max retries override
    pub max_retries: Option<u32>,
}

/// Resolved effective configuration for a specific chain
#[derive(Debug, Clone)]
struct ChainPinaxEffectiveConfig {
    /// Effective database name (base config or chain override)
    db_name: String,
    /// Effective timeout (base config or chain override)
    timeout_seconds: u64,
    /// Effective max retries (base config or chain override)
    /// Note: Currently unused but prepared for future retry logic implementation
    #[allow(dead_code)]
    max_retries: u32,
}

/// Pinax API client implementation with chain-specific support
#[derive(Debug)]
pub struct PinaxClient {
    client: Client,
    config: PinaxConfig,
    /// Chain-specific configuration overrides
    chain_overrides: HashMap<ChainId, PerChainPinaxConfig>,
}

/// Errors specific to the Pinax API client
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum PinaxError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing failed
    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    /// Authentication failed
    #[error("Authentication failed")]
    Unauthorized,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Timeout error
    #[error("Request timeout")]
    Timeout { seconds: u64 },

    /// SQL query error
    #[error("SQL query error: {0}")]
    SqlError(String),

    /// Unsupported chain
    #[error(
        "Unsupported chain: {chain_name} (ID: {chain_id}). This chain is not fully supported by Pinax integration"
    )]
    UnsupportedChain { chain_id: u64, chain_name: String },
}

impl From<PinaxError> for ApiError {
    fn from(value: PinaxError) -> Self {
        match value {
            PinaxError::Http(error) => ApiError::Http {
                message: error.to_string(),
            },
            PinaxError::Json(error) => ApiError::InvalidResponse {
                message: error.to_string(),
            },
            PinaxError::ApiError { status, message } => ApiError::Custom {
                error: anyhow::Error::msg(format!("{status}: {message}")),
            },
            PinaxError::Unauthorized => ApiError::Authentication {
                message: value.to_string(),
            },
            PinaxError::Config(message) => ApiError::Configuration { message },
            PinaxError::Timeout { seconds } => ApiError::Timeout {
                timeout_seconds: seconds,
            },
            PinaxError::SqlError(message) => ApiError::InvalidResponse { message },
            PinaxError::UnsupportedChain {
                chain_id,
                chain_name,
            } => ApiError::Configuration {
                message: format!(
                    "Chain {chain_name} ({chain_id}) is not supported by Pinax integration"
                ),
            },
        }
    }
}

/// Response structure for Pinax NFT metadata query
#[derive(Debug, Deserialize)]
struct PinaxNftMetadata {
    symbol: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

/// Response structure for Pinax API
#[derive(Debug, Deserialize)]
struct PinaxResponse {
    data: Option<Vec<PinaxNftMetadata>>,
    error: Option<String>,
}

impl PinaxClient {
    /// Create a new Pinax API client
    ///
    /// # Arguments
    ///
    /// * `config` - Base configuration for the Pinax API client
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created
    pub fn new(config: PinaxConfig) -> Result<Self, PinaxError> {
        Self::with_chain_overrides(config, HashMap::new())
    }

    /// Create a new Pinax API client with chain-specific configuration overrides
    ///
    /// # Arguments
    ///
    /// * `config` - Base configuration for the Pinax API client
    /// * `chain_overrides` - Chain-specific configuration overrides
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created or configuration is invalid
    pub fn with_chain_overrides(
        config: PinaxConfig,
        chain_overrides: HashMap<ChainId, PerChainPinaxConfig>,
    ) -> Result<Self, PinaxError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("nft-api/0.1.0")
            .build()
            .map_err(PinaxError::Http)?;

        Ok(Self {
            client,
            config,
            chain_overrides,
        })
    }

    /// Get effective configuration for a specific chain, applying overrides
    fn get_chain_config(&self, chain_id: ChainId) -> ChainPinaxEffectiveConfig {
        let override_config = self.chain_overrides.get(&chain_id);

        ChainPinaxEffectiveConfig {
            db_name: override_config
                .and_then(|o| o.db_name.as_ref())
                .map_or_else(|| self.config.db_name.as_str().to_string(), String::clone),
            timeout_seconds: override_config
                .and_then(|o| o.timeout_seconds)
                .unwrap_or(self.config.timeout_seconds),
            max_retries: override_config
                .and_then(|o| o.max_retries)
                .unwrap_or(self.config.max_retries),
        }
    }

    /// Validate that a chain is supported for Pinax operations
    fn validate_chain_support(&self, chain_id: ChainId) -> Result<(), PinaxError> {
        // Check if chain is fully implemented
        if !chain_id.is_fully_implemented() {
            return Err(PinaxError::UnsupportedChain {
                chain_id: chain_id.chain_id(),
                chain_name: chain_id.name().to_string(),
            });
        }
        Ok(())
    }

    /// Get NFT contract metadata from Pinax using SQL query with chain-specific configuration
    async fn get_nft_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
    ) -> Result<Option<ContractMetadata>, PinaxError> {
        if address == Address::ZERO {
            return Err(PinaxError::Config("Invalid address provided".to_string()));
        }

        // Validate chain support
        self.validate_chain_support(chain_id)?;

        // Get chain-specific configuration
        let chain_config = self.get_chain_config(chain_id);

        let address_lower = format!("{address:#x}").to_lowercase();

        let query = format!(
            r"
            WITH contract_metadata AS (
                SELECT symbol, name, contract FROM `{}`.erc1155_metadata_by_contract
                WHERE contract = '{}'

                UNION ALL

                SELECT symbol, name, contract FROM `{}`.erc721_metadata_by_contract
                WHERE contract = '{}'
            )
            SELECT
                cm.symbol,
                cm.name,
                nm.description
            FROM contract_metadata cm
            LEFT JOIN `{}`.nft_metadata nm
            ON cm.contract = nm.contract
            LIMIT 1
            FORMAT JSON
            ",
            chain_config.db_name,
            address_lower,
            chain_config.db_name,
            address_lower,
            chain_config.db_name
        );

        debug!(
            query,
            chain_id = %chain_id,
            db_name = %chain_config.db_name,
            "executing Pinax SQL query with chain-specific database"
        );

        let request = self
            .client
            .post(self.config.endpoint.as_str())
            .body(query)
            .basic_auth(
                self.config.api_user.as_str(),
                Some(self.config.api_auth.as_str()),
            )
            .header("Content-Type", "text/plain");

        let response = timeout(
            Duration::from_secs(chain_config.timeout_seconds),
            request.send(),
        )
        .await
        .map_err(|_| PinaxError::Timeout {
            seconds: chain_config.timeout_seconds,
        })?
        .map_err(PinaxError::Http)?;

        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await.map_err(PinaxError::Http)?;
                debug!(response = %response_text, "received Pinax response");

                let pinax_response: PinaxResponse =
                    serde_json::from_str(&response_text).map_err(PinaxError::Json)?;

                if let Some(error) = pinax_response.error {
                    return Err(PinaxError::SqlError(error));
                }

                if let Some(data) = pinax_response.data {
                    if let Some(metadata) = data.into_iter().next() {
                        Ok(Some(self.convert_metadata(address, metadata)))
                    } else {
                        debug!("No NFT metadata found for address: {}", address);
                        Ok(None)
                    }
                } else {
                    debug!("No data returned for address: {}", address);
                    Ok(None)
                }
            }
            StatusCode::UNAUTHORIZED => Err(PinaxError::Unauthorized),
            status => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                warn!(
                    status = status.as_u16(),
                    error = error_text,
                    "Pinax API error"
                );
                Err(PinaxError::ApiError {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    /// Convert Pinax metadata to our standard format
    fn convert_metadata(&self, address: Address, metadata: PinaxNftMetadata) -> ContractMetadata {
        let mut additional_data = HashMap::new();

        if let Some(ref description) = metadata.description {
            additional_data.insert(
                "description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        ContractMetadata {
            address,
            name: metadata.name,
            symbol: metadata.symbol,
            total_supply: None,
            holder_count: None,
            transaction_count: None,
            creation_block: None,
            creation_timestamp: None,
            creator_address: None,
            is_verified: None,
            contract_type: Some(ContractType::Unknown), // Pinax doesn't specify exact type in this query
            additional_data,
        }
    }
}

impl ApiClient for PinaxClient {
    async fn health_check(&self) -> Result<HealthStatus, ApiError> {
        let simple_query = "SELECT 1 FORMAT JSON";

        debug!(query = simple_query, "performing health check on Pinax API");

        let request = self
            .client
            .post(self.config.endpoint.as_str())
            .body(simple_query)
            .basic_auth(
                self.config.api_user.as_str(),
                Some(self.config.api_auth.as_str()),
            )
            .header("Content-Type", "text/plain");

        let start_time = std::time::Instant::now();
        let response = timeout(
            Duration::from_secs(self.config.health_check_timeout_seconds),
            request.send(),
        )
        .await
        .map_err(|_| PinaxError::Timeout {
            seconds: start_time.elapsed().as_secs(),
        })?
        .map_err(PinaxError::Http)?;

        let response_time = start_time.elapsed();

        match response.status() {
            StatusCode::OK => {
                info!("Pinax API health check passed in {:?}", response_time);
                Ok(HealthStatus::Up)
            }
            StatusCode::UNAUTHORIZED => {
                warn!("Pinax API health check failed: unauthorized");
                Ok(HealthStatus::Down {
                    reason: "Authentication failed".to_string(),
                })
            }
            status => {
                warn!("Pinax API health check failed with status: {}", status);
                Ok(HealthStatus::Degraded {
                    reason: format!("API returned status {}", status.as_u16()),
                })
            }
        }
    }

    async fn get_contract_metadata(
        &self,
        address: Address,
        chain_id: ChainId,
    ) -> Result<Option<ContractMetadata>, ApiError> {
        info!(
            "Fetching contract metadata from Pinax for address: {} on chain: {}",
            address,
            chain_id.name()
        );
        match self.get_nft_metadata(address, chain_id).await {
            Ok(metadata) => Ok(metadata),
            Err(e) => {
                error!(
                    "Failed to fetch NFT metadata from Pinax for address {}: {}",
                    address, e
                );
                Err(e.into())
            }
        }
    }

    fn name(&self) -> &'static str {
        "pinax"
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{basic_auth, body_string_contains, header, method},
    };

    use super::*;

    fn test_address() -> Address {
        "0xED5AF388653567Af2F388E6224dC7C4b3241C544"
            .parse()
            .unwrap()
    }

    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }

    fn create_test_config(server_url: &str) -> PinaxConfig {
        PinaxConfig::new(
            server_url,
            "test-user",
            "test-auth",
            "mainnet:evm-nft-tokens@v0.6.2",
            1, // timeout_seconds
            DEFAULT_PINAX_HEALTH_CHECK_TIMEOUT_SECONDS,
            DEFAULT_PINAX_MAX_RETRIES,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn pinax_client_creation_success() {
        let config = PinaxConfig::new(
            "https://api.pinax.network/sql",
            "valid-user",
            "valid-auth",
            "mainnet:evm-nft-tokens@v0.6.2",
            DEFAULT_PINAX_TIMEOUT_SECONDS,
            DEFAULT_PINAX_HEALTH_CHECK_TIMEOUT_SECONDS,
            DEFAULT_PINAX_MAX_RETRIES,
        )
        .unwrap();

        let client = PinaxClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn pinax_client_creation_invalid_config() {
        // Try to create config with empty user - should fail at config creation time
        let config_result = PinaxConfig::new(
            "https://api.pinax.network/sql",
            "", // Empty user should cause config creation to fail
            "valid-auth",
            "mainnet:evm-nft-tokens@v0.6.2",
            DEFAULT_PINAX_TIMEOUT_SECONDS,
            DEFAULT_PINAX_HEALTH_CHECK_TIMEOUT_SECONDS,
            DEFAULT_PINAX_MAX_RETRIES,
        );

        assert!(config_result.is_err());
        assert!(
            config_result
                .unwrap_err()
                .contains("String cannot be empty")
        );
    }

    #[tokio::test]
    async fn convert_metadata() {
        let config = PinaxConfig::default_test();
        let client = PinaxClient::new(config).unwrap();
        let address = test_address();

        let pinax_metadata = PinaxNftMetadata {
            symbol: Some("PNFT".to_string()),
            name: Some("Test Pinax NFT".to_string()),
            description: Some("A test NFT from Pinax".to_string()),
        };

        let metadata = client.convert_metadata(address, pinax_metadata);
        assert_eq!(metadata.name, Some("Test Pinax NFT".to_string()));
        assert_eq!(metadata.symbol, Some("PNFT".to_string()));
        assert_eq!(metadata.address, address);
        assert!(metadata.additional_data.contains_key("description"));
    }

    #[test]
    fn pinax_error_conversion() {
        let error = PinaxError::Unauthorized;
        let api_error: ApiError = error.into();
        assert!(matches!(api_error, ApiError::Authentication { .. }));

        let error = PinaxError::Config("test".to_string());
        let api_error: ApiError = error.into();
        assert!(matches!(api_error, ApiError::Configuration { .. }));
    }

    #[tokio::test]
    async fn get_contract_metadata_success() {
        let mock_server = setup_mock_server().await;
        let config = create_test_config(&mock_server.uri());
        let client = PinaxClient::new(config).unwrap();

        let expected_response = serde_json::json!({
            "data": [{"name": "TestNFT"}]
        });

        Mock::given(method("POST"))
            .and(basic_auth("test-user", "test-auth"))
            .and(header("Content-Type", "text/plain"))
            .and(body_string_contains(
                "0xed5af388653567af2f388e6224dc7c4b3241c544",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected_response))
            .mount(&mock_server)
            .await;

        let result = client
            .get_contract_metadata(test_address(), ChainId::Ethereum)
            .await;

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.name, Some("TestNFT".to_string()));
        assert_eq!(metadata.address, test_address());
    }

    #[tokio::test]
    async fn get_contract_metadata_invalid_address() {
        let config = PinaxConfig::default_test();
        let client = PinaxClient::new(config).unwrap();

        let result = client
            .get_contract_metadata(Address::ZERO, ChainId::Ethereum)
            .await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ApiError::Configuration { .. }));
        if let ApiError::Configuration { message } = error {
            assert!(message.contains("Invalid address provided"));
        }
    }

    #[tokio::test]
    async fn get_contract_metadata_http_error() {
        let mock_server = setup_mock_server().await;
        let config = create_test_config(&mock_server.uri());
        let client = PinaxClient::new(config).unwrap();

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Timeout"))
            .mount(&mock_server)
            .await;

        let result = client
            .get_contract_metadata(test_address(), ChainId::Ethereum)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_contract_metadata_invalid_json() {
        let mock_server = setup_mock_server().await;
        let config = create_test_config(&mock_server.uri());
        let client = PinaxClient::new(config).unwrap();

        Mock::given(method("POST"))
            .and(basic_auth("test-user", "test-auth"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Invalid JSON"))
            .mount(&mock_server)
            .await;

        let result = client
            .get_contract_metadata(test_address(), ChainId::Ethereum)
            .await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ApiError::InvalidResponse { .. }));
    }

    #[tokio::test]
    async fn api_client_name() {
        let config = PinaxConfig::default_test();
        let client = PinaxClient::new(config).unwrap();
        assert_eq!(client.name(), "pinax");
    }
}
