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
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

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

    let result = client.get_contract_metadata(test_address).await.unwrap();

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

    let result = client.get_contract_metadata(test_address).await.unwrap();
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

    let result = client.get_contract_metadata(test_address).await;

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

    let result = client.get_contract_metadata(test_address).await;

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

    let result = client.get_contract_metadata(test_address).await;

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
