// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for `MoralisClient`
//!
//! These tests use wiremock to mock HTTP responses and test the client behavior
//! in various scenarios, similar to the Python test patterns.

use alloy_primitives::Address;
use api_client::{ApiClient, ApiError, ContractType, HealthStatus};
use external_apis::{MoralisClient, MoralisConfig, MoralisError};
use serde_json::json;
use shared_types::ChainId;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, path_regex, query_param},
};

mod fixtures;
use fixtures::*;

const TEST_TIMEOUT_SECONDS: u64 = 10;
const TEST_HEALTH_CHECK_TIMEOUT_SECONDS: u64 = 5;
const TEST_MAX_RETRIES: u32 = 1;

/// Create a test `MoralisConfig` with the mock server URL
fn create_test_config(base_url: String) -> MoralisConfig {
    MoralisConfig {
        base_url,
        api_key: "test-api-key".to_string(),
        timeout_seconds: TEST_TIMEOUT_SECONDS,
        health_check_timeout_seconds: TEST_HEALTH_CHECK_TIMEOUT_SECONDS,
        max_retries: TEST_MAX_RETRIES,
    }
}

/// Test successful NFT metadata retrieval
#[tokio::test]
async fn get_nft_metadata_success() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0x12; 20]);
    let mock_response = json!({
        "result": [{
            "token_address": test_address.to_string(),
            "token_id": "1",
            "contract_type": "ERC721",
            "token_hash": "abc123",
            "metadata": {"name": "Cool NFT Item"},
            "name": "CoolNFT",
            "symbol": "CNFT"
        }]
    });

    Mock::given(method("GET"))
        .and(path(format!("/nft/{test_address}")))
        .and(header("X-API-Key", "test-api-key"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
        .mount(&mock_server)
        .await;

    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await
        .unwrap();

    assert!(result.is_some());
    let metadata = result.unwrap();
    assert_eq!(metadata.name, Some("CoolNFT".to_string()));
    assert_eq!(metadata.symbol, Some("CNFT".to_string()));
    assert_eq!(metadata.contract_type, Some(ContractType::Erc721));
    assert_eq!(metadata.address, test_address);
}

/// Test contract not found
#[tokio::test]
async fn get_contract_metadata_not_found() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0x56; 20]);

    let empty_response = json!({
        "result": []
    });

    Mock::given(method("GET"))
        .and(path(format!("/nft/{test_address}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response))
        .mount(&mock_server)
        .await;

    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await
        .unwrap();
    assert!(result.is_none());
}

/// Test API authentication failure
#[tokio::test]
async fn get_contract_metadata_unauthorized() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0x78; 20]);

    Mock::given(method("GET"))
        .and(path(format!("/nft/{test_address}")))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Authentication { .. } => {}
        other => panic!("Expected Authentication error, got: {other:?}"),
    }
}

/// Test rate limiting
#[tokio::test]
async fn get_contract_metadata_rate_limited() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0x9a; 20]);

    Mock::given(method("GET"))
        .and(path(format!("/nft/{test_address}")))
        .respond_with(ResponseTemplate::new(429))
        .mount(&mock_server)
        .await;

    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::RateLimitExceeded { .. } => {}
        other => panic!("Expected RateLimitExceeded error, got: {other:?}"),
    }
}

/// Test API server error
#[tokio::test]
async fn get_contract_metadata_server_error() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0xbc; 20]);

    Mock::given(method("GET"))
        .and(path(format!("/nft/{test_address}")))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Custom { .. } => {}
        other => panic!("Expected Custom error, got: {other:?}"),
    }
}

/// Test health check success
#[tokio::test]
async fn health_check_success() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    Mock::given(method("GET"))
        .and(path("/info/endpointWeights"))
        .and(header("X-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    let result = client.health_check().await.unwrap();

    match result {
        HealthStatus::Up => {}
        other => panic!("Expected Up status, got: {other:?}"),
    }
}

/// Test health check unauthorized
#[tokio::test]
async fn health_check_unauthorized() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    Mock::given(method("GET"))
        .and(path("/info/endpointWeights"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let result = client.health_check().await.unwrap();

    match result {
        HealthStatus::Down { reason } => {
            assert_eq!(reason, "Authentication failed");
        }
        other => panic!("Expected Down status, got: {other:?}"),
    }
}

/// Test health check rate limited
#[tokio::test]
async fn health_check_rate_limited() {
    let mock_server = MockServer::start().await;
    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    Mock::given(method("GET"))
        .and(path("/info/endpointWeights"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&mock_server)
        .await;

    let result = client.health_check().await.unwrap();

    match result {
        HealthStatus::Degraded { reason } => {
            assert_eq!(reason, "Rate limited");
        }
        other => panic!("Expected Degraded status, got: {other:?}"),
    }
}

/// Test client configuration validation
#[test]
fn client_creation_invalid_api_key() {
    let config = MoralisConfig {
        api_key: String::new(),
        base_url: "https://api.example.com".to_string(),
        timeout_seconds: 30,
        health_check_timeout_seconds: 5,
        max_retries: 3,
    };

    let result = MoralisClient::new(config);

    assert!(result.is_err());
    match result.unwrap_err() {
        MoralisError::Config(msg) => {
            assert!(msg.contains("API key cannot be empty"));
        }
        other => panic!("Expected Config error, got: {other:?}"),
    }
}

/// Test client configuration validation for empty base URL
#[test]
fn client_creation_invalid_base_url() {
    let config = MoralisConfig {
        api_key: "valid-key".to_string(),
        base_url: String::new(),
        timeout_seconds: 30,
        health_check_timeout_seconds: 5,
        max_retries: 3,
    };

    let result = MoralisClient::new(config);

    assert!(result.is_err());
    match result.unwrap_err() {
        MoralisError::Config(msg) => {
            assert!(msg.contains("Base URL cannot be empty"));
        }
        other => panic!("Expected Config error, got: {other:?}"),
    }
}

/// Test client name
#[tokio::test]
async fn client_name() {
    let config = MoralisConfig::default();
    let client = MoralisClient::new(config).unwrap();

    assert_eq!(client.name(), "moralis");
}

// Multi-chain comprehensive tests

#[tokio::test]
async fn get_nft_metadata_all_chains() {
    let mock_server = MockServer::start().await;
    MoralisChainFixture::setup_all_chains_mocks(&mock_server).await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    // Test each supported chain
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.valid, chain)
            .await
            .unwrap();

        assert!(
            result.is_some(),
            "should return metadata for chain {}",
            chain.name()
        );

        let metadata = result.unwrap();
        assert_eq!(metadata.address, test_addresses.valid);
        assert!(
            metadata.name.is_some(),
            "should have name for chain {}",
            chain.name()
        );
        assert!(
            metadata.symbol.is_some(),
            "should have symbol for chain {}",
            chain.name()
        );

        // Verify contract type matches expected type for this chain
        let expected_type = ChainTestUtils::expected_contract_type(chain);
        assert_eq!(
            metadata.contract_type,
            Some(expected_type),
            "contract type should match for chain {}",
            chain.name()
        );
    }
}

#[tokio::test]
async fn get_nft_metadata_not_found_all_chains() {
    let mock_server = MockServer::start().await;
    MoralisChainFixture::setup_all_chains_mocks(&mock_server).await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    // Test not found scenario for each chain
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.not_found, chain)
            .await
            .unwrap();

        assert!(
            result.is_none(),
            "should return None for not found address on chain {}",
            chain.name()
        );
    }
}

#[tokio::test]
async fn get_nft_metadata_rate_limited_all_chains() {
    let mock_server = MockServer::start().await;
    MoralisChainFixture::setup_all_chains_mocks(&mock_server).await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    // Test rate limiting for each chain
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.rate_limited, chain)
            .await;

        assert!(
            result.is_err(),
            "should return error for rate limited on chain {}",
            chain.name()
        );

        match result.unwrap_err() {
            ApiError::RateLimitExceeded { .. } => {}
            other => panic!(
                "Expected RateLimitExceeded error for chain {}, got: {:?}",
                chain.name(),
                other
            ),
        }
    }
}

#[tokio::test]
async fn get_nft_metadata_authentication_error_all_chains() {
    let mock_server = MockServer::start().await;

    // Setup authentication error mocks for all chains
    for &chain in ChainId::all() {
        let chain_name = match chain {
            ChainId::Ethereum => "eth",
            ChainId::Polygon => "polygon",
            ChainId::Base => "base",
            ChainId::Avalanche => "avalanche",
            ChainId::Arbitrum => "arbitrum",
        };

        Mock::given(method("GET"))
            .and(path_regex(r"^/nft/0x[a-fA-F0-9]{40}$"))
            .and(query_param("chain", chain_name))
            .and(header("X-API-Key", "invalid-key"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;
    }

    let mut config = create_test_config(mock_server.uri());
    config.api_key = "invalid-key".to_string();
    let client = MoralisClient::new(config).unwrap();

    // Test authentication error for each chain
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.valid, chain)
            .await;

        assert!(
            result.is_err(),
            "should return error for invalid auth on chain {}",
            chain.name()
        );

        match result.unwrap_err() {
            ApiError::Authentication { .. } => {}
            other => panic!(
                "Expected Authentication error for chain {}, got: {:?}",
                chain.name(),
                other
            ),
        }
    }
}

#[tokio::test]
async fn get_nft_metadata_server_error_all_chains() {
    let mock_server = MockServer::start().await;
    MoralisChainFixture::setup_all_chains_mocks(&mock_server).await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    // Test server error for each chain
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.server_error, chain)
            .await;

        assert!(
            result.is_err(),
            "should return error for server error on chain {}",
            chain.name()
        );

        match result.unwrap_err() {
            ApiError::Custom { .. } => {}
            other => panic!(
                "Expected Custom error for chain {}, got: {:?}",
                chain.name(),
                other
            ),
        }
    }
}

#[tokio::test]
async fn chain_specific_response_validation() {
    let mock_server = MockServer::start().await;
    MoralisChainFixture::setup_all_chains_mocks(&mock_server).await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    // Test that responses are chain-specific
    for &chain in ChainId::all() {
        let test_addresses = MoralisChainFixture::test_addresses(chain);

        let result = client
            .get_contract_metadata(test_addresses.valid, chain)
            .await
            .unwrap()
            .unwrap();

        // Verify the response contains expected chain-specific data
        let expected_name_prefix = match chain {
            ChainId::Ethereum => "CryptoPunks",
            ChainId::Polygon => "PolygonNFT",
            ChainId::Base => "BaseArt",
            ChainId::Avalanche => "SnowNFT",
            ChainId::Arbitrum => "ArbitrumArt",
        };

        assert!(
            result.name.as_ref().unwrap().contains(expected_name_prefix),
            "response should contain chain-specific name for {}: got {}",
            chain.name(),
            result.name.as_ref().unwrap()
        );

        // Verify the response uses the correct test address
        assert_eq!(
            result.address,
            test_addresses.valid,
            "response should use correct address for chain {}",
            chain.name()
        );
    }
}

#[tokio::test]
async fn moralis_chain_parameter_validation() {
    let mock_server = MockServer::start().await;

    // Setup mock that expects specific chain parameter
    Mock::given(method("GET"))
        .and(path("/nft/0x1212121212121212121212121212121212121212"))
        .and(query_param("chain", "eth"))
        .and(header("X-API-Key", "test-api-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(MoralisChainFixture::success_response(ChainId::Ethereum)),
        )
        .mount(&mock_server)
        .await;

    let config = create_test_config(mock_server.uri());
    let client = MoralisClient::new(config).unwrap();

    let test_address = Address::from([0x12; 20]);

    // This should work because we're calling with Ethereum
    let result = client
        .get_contract_metadata(test_address, ChainId::Ethereum)
        .await
        .unwrap();

    assert!(
        result.is_some(),
        "should get result for correct chain parameter"
    );

    // This should fail because the mock expects "eth" but Polygon sends "polygon"
    let result = client
        .get_contract_metadata(test_address, ChainId::Polygon)
        .await;

    // The request should fail because the chain parameter doesn't match the mock
    assert!(
        result.is_err() || result.unwrap().is_none(),
        "should fail or return None for mismatched chain parameter"
    );
}
