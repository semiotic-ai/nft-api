// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0
#![allow(missing_docs, dead_code)]

//! External API test fixtures for multi-chain testing
//!
//! Provides chain-specific mock responses and test data for external API clients.

use alloy_primitives::Address;
use api_client::ContractType;
use serde_json::{Value, json};
use shared_types::ChainId;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, path_regex, query_param},
};

/// Create chain-specific Moralis API mock responses
#[derive(Debug)]
pub struct MoralisChainFixture;

impl MoralisChainFixture {
    /// Setup Moralis mocks for all supported chains
    pub async fn setup_all_chains_mocks(mock_server: &MockServer) {
        for &chain in ChainId::all() {
            Self::setup_chain_mocks(mock_server, chain).await;
        }
    }

    /// Setup Moralis mocks for a specific chain
    pub async fn setup_chain_mocks(mock_server: &MockServer, chain_id: ChainId) {
        let chain_name = Self::moralis_chain_name(chain_id);

        // Empty response (no data found)
        Mock::given(method("GET"))
            .and(path(format!("/nft/{}", Address::from([0xff; 20]))))
            .and(query_param("chain", chain_name))
            .respond_with(ResponseTemplate::new(200).set_body_json(Self::empty_response()))
            .mount(mock_server)
            .await;

        // Rate limited response
        Mock::given(method("GET"))
            .and(path(format!("/nft/{}", Address::from([0xaa; 20]))))
            .and(query_param("chain", chain_name))
            .respond_with(ResponseTemplate::new(429))
            .mount(mock_server)
            .await;

        // Authentication error
        Mock::given(method("GET"))
            .and(path(format!("/nft/{}", Address::from([0xbb; 20]))))
            .and(query_param("chain", chain_name))
            .and(header("X-API-Key", "invalid-key"))
            .respond_with(ResponseTemplate::new(401))
            .mount(mock_server)
            .await;

        // Server error
        Mock::given(method("GET"))
            .and(path(format!("/nft/{}", Address::from([0xcc; 20]))))
            .and(query_param("chain", chain_name))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(mock_server)
            .await;

        // Successful metadata response - General catch-all must be last
        Mock::given(method("GET"))
            .and(path_regex(r"^/nft/0x[a-fA-F0-9]{40}$"))
            .and(query_param("chain", chain_name))
            .and(header("X-API-Key", "test-api-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(Self::success_response(chain_id)),
            )
            .mount(mock_server)
            .await;

        // Health check endpoint
        Mock::given(method("GET"))
            .and(path("/info/endpointWeights"))
            .and(header("X-API-Key", "test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(mock_server)
            .await;
    }

    /// Get Moralis chain name for API calls
    fn moralis_chain_name(chain_id: ChainId) -> &'static str {
        match chain_id {
            ChainId::Ethereum => "eth",
            ChainId::Polygon => "polygon",
            ChainId::Base => "base",
            ChainId::Avalanche => "avalanche",
            ChainId::Arbitrum => "arbitrum",
        }
    }

    /// Create a successful metadata response for a chain
    pub fn success_response(chain_id: ChainId) -> Value {
        let (name, symbol, contract_type) = match chain_id {
            ChainId::Ethereum => ("CryptoPunks", "PUNKS", "ERC721"),
            ChainId::Polygon => ("PolygonNFT", "PNFT", "ERC1155"),
            ChainId::Base => ("BaseArt", "BART", "ERC721"),
            ChainId::Avalanche => ("SnowNFT", "SNOW", "ERC721"),
            ChainId::Arbitrum => ("ArbitrumArt", "ARBT", "ERC1155"),
        };

        json!({
            "result": [{
                "token_address": Address::from([0x12; 20]).to_string(),
                "token_id": "1",
                "contract_type": contract_type,
                "token_hash": "abc123",
                "metadata": {
                    "name": format!("{} Item #1", name),
                    "description": format!("Test NFT on {}", chain_id.name()),
                    "image": format!("https://{}.example.com/nft1.png", chain_id.name().to_lowercase())
                },
                "name": name,
                "symbol": symbol
            }]
        })
    }

    /// Create an empty response (no NFTs found)
    pub fn empty_response() -> Value {
        json!({
            "result": []
        })
    }

    /// Create spam metadata response
    pub fn spam_response(_chain_id: ChainId) -> Value {
        json!({
            "result": [{
                "token_address": Address::from([0x4d; 20]).to_string(),
                "token_id": "1",
                "contract_type": "ERC721",
                "token_hash": "spam123",
                "metadata": {
                    "name": "DEFINITELY NOT SCAM",
                    "description": "FREE MONEY CLICK HERE!!!",
                    "image": "https://suspicious-domain.xyz/fake.png"
                },
                "name": "FreeMoneyNFT",
                "symbol": "SCAM"
            }]
        })
    }

    /// Get test addresses for a specific chain
    pub fn test_addresses(_chain_id: ChainId) -> TestAddresses {
        TestAddresses {
            valid: Address::from([0x12; 20]),
            not_found: Address::from([0xff; 20]),
            rate_limited: Address::from([0xaa; 20]),
            auth_error: Address::from([0xbb; 20]),
            server_error: Address::from([0xcc; 20]),
            spam: Address::from([0x4d; 20]),
        }
    }
}

/// Test addresses for different scenarios
#[derive(Debug)]
pub struct TestAddresses {
    pub valid: Address,
    pub not_found: Address,
    pub rate_limited: Address,
    pub auth_error: Address,
    pub server_error: Address,
    pub spam: Address,
}

/// Create chain-specific Pinax API mock responses
#[derive(Debug)]
pub struct PinaxChainFixture;

impl PinaxChainFixture {
    /// Setup Pinax mocks for all supported chains
    pub async fn setup_all_chains_mocks(mock_server: &MockServer) {
        for &chain in ChainId::all() {
            Self::setup_chain_mocks(mock_server, chain).await;
        }
    }

    /// Setup Pinax mocks for a specific chain
    pub async fn setup_chain_mocks(mock_server: &MockServer, chain_id: ChainId) {
        // Success response with chain-specific data
        Mock::given(method("POST"))
            .and(path("/v1/sql"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(Self::success_response(chain_id)),
            )
            .mount(mock_server)
            .await;

        // Health check
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "healthy"})))
            .mount(mock_server)
            .await;
    }

    /// Create successful Pinax response for a chain
    pub fn success_response(chain_id: ChainId) -> Value {
        json!({
            "data": [{
                "chain_id": chain_id.chain_id(),
                "contract_address": Address::from([0x12; 20]).to_string(),
                "transaction_count": 1000,
                "unique_holders": 500,
                "last_activity": "2024-01-01T00:00:00Z"
            }],
            "meta": {
                "row_count": 1
            }
        })
    }

    /// Create empty Pinax response
    pub fn empty_response() -> Value {
        json!({
            "data": [],
            "meta": {
                "row_count": 0
            }
        })
    }
}

/// Utility functions for multi-chain testing
#[derive(Debug)]
pub struct ChainTestUtils;

impl ChainTestUtils {
    /// Create a test request for a specific chain
    pub fn create_request(chain_id: ChainId, addresses: &[Address]) -> Value {
        json!({
            "chain_id": chain_id.chain_id(),
            "addresses": addresses.iter().map(ToString::to_string).collect::<Vec<_>>()
        })
    }

    /// Verify response contains expected chain data
    pub fn verify_chain_response(response: &Value, chain_id: ChainId, address: Address) -> bool {
        if let Some(result) = response.get(address.to_string())
            && let Some(response_chain_id) = result.get("chain_id")
        {
            return response_chain_id == &json!(chain_id.chain_id());
        }
        false
    }

    /// Get expected contract type for chain
    pub fn expected_contract_type(chain_id: ChainId) -> ContractType {
        match chain_id {
            ChainId::Polygon | ChainId::Arbitrum => ContractType::Erc1155,
            ChainId::Ethereum | ChainId::Base | ChainId::Avalanche => ContractType::Erc721,
        }
    }
}
