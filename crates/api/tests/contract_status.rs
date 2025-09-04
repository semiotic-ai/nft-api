// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the contract status endpoint

use api::{Server, ServerConfig, ShutdownConfig};
use axum::http::StatusCode;
use serde_json::json;
use shared_types::ChainId;

mod fixtures;
use fixtures::*;

#[tokio::test]
async fn contract_status_valid_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let valid_request = json!({
        "chain_id": 137,
        "addresses": [
            "0x1234567890123456789012345678901234567890",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        ]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&valid_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn contract_status_invalid_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let invalid_request = json!({
        "chain_id": 137,
        "addresses": [
            "0x123",
            "0xghijklmnopqrstuvwxyz123456789012345678",
            "not_an_address"
        ]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&invalid_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn contract_status_empty_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let empty_request = json!({
        "chain_id": 137,
        "addresses": []
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&empty_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_text = response.text().await.expect("Failed to read response");
    assert!(response_text.contains("addresses list cannot be empty"));
}

#[tokio::test]
async fn contract_status_invalid_chain_id() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let invalid_chain_request = json!({
        "chain_id": 999,
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&invalid_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn contract_status_chain_id_string() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let string_chain_request = json!({
        "chain_id": "MATIC",
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&string_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn contract_status_full_implementation_chain() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let full_chain_request = json!({
        "chain_id": 1, // Ethereum mainnet - fully implemented
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&full_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: serde_json::Value = response.json().await.expect("Failed to parse response");
    let result = &response_body["0x1234567890123456789012345678901234567890"];
    assert_eq!(result["chain_id"], 1);
}

#[tokio::test]
async fn contract_status_base_now_fully_supported() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let base_chain_request = json!({
        "chain_id": 8453,
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&base_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: serde_json::Value = response.json().await.expect("Failed to parse response");
    let result = &response_body["0x1234567890123456789012345678901234567890"];
    assert_eq!(result["chain_id"], 8453);
}

#[tokio::test]
async fn contract_status_avalanche_now_supported() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let avalanche_chain_request = json!({
        "chain_id": 43114,
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&avalanche_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: serde_json::Value = response.json().await.expect("Failed to parse response");
    let result = &response_body["0x1234567890123456789012345678901234567890"];
    assert_eq!(result["chain_id"], 43114);
}

#[tokio::test]
async fn contract_status_missing_chain_id() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let missing_chain_request = json!({
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&missing_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Multi-chain comprehensive integration tests

#[tokio::test]
async fn contract_status_all_chains_valid_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test each supported chain
    for fixture in ChainTestFixture::all_chains() {
        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": [fixture.get_valid_address().to_string()]
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "chain {} should return OK",
            fixture.chain_id.name()
        );

        let response_body: serde_json::Value =
            response.json().await.expect("failed to parse response");

        let result = &response_body[fixture.get_valid_address().to_string()];
        assert_eq!(
            result["chain_id"],
            fixture.chain_id.chain_id(),
            "response should contain correct chain_id for {}",
            fixture.chain_id.name()
        );

        // Verify message contains chain name
        let message = result["message"].as_str().unwrap();
        assert!(
            message.contains(fixture.chain_id.name()),
            "message should contain chain name for {}: {}",
            fixture.chain_id.name(),
            message
        );
    }
}

#[tokio::test]
async fn contract_status_all_chains_invalid_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test invalid addresses for each supported chain
    for fixture in ChainTestFixture::all_chains() {
        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": [fixture.get_invalid_address()]
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "invalid address should return BAD_REQUEST for chain {}",
            fixture.chain_id.name()
        );
    }
}

#[tokio::test]
async fn contract_status_mixed_chains_batch() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test multiple chains in sequence to simulate batch processing
    let test_fixtures = vec![
        ChainTestFixture::for_chain(ChainId::Ethereum),
        ChainTestFixture::for_chain(ChainId::Polygon),
        ChainTestFixture::for_chain(ChainId::Base),
    ];

    for fixture in test_fixtures {
        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": [
                fixture.get_valid_address().to_string(),
                fixture.get_legitimate_address().to_string()
            ]
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        assert_eq!(response.status(), StatusCode::OK);

        let response_body: serde_json::Value =
            response.json().await.expect("failed to parse response");

        // Verify both addresses were processed
        assert!(
            response_body
                .get(fixture.get_valid_address().to_string())
                .is_some()
        );
        assert!(
            response_body
                .get(fixture.get_legitimate_address().to_string())
                .is_some()
        );

        // Verify chain_id consistency in all responses
        for (_, result) in response_body.as_object().unwrap() {
            assert_eq!(
                result["chain_id"],
                fixture.chain_id.chain_id(),
                "all results should have correct chain_id"
            );
        }
    }
}

#[tokio::test]
async fn contract_status_chain_capabilities_validation() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test that all chains support the expected capabilities
    for fixture in ChainTestFixture::all_chains() {
        // Verify the chain supports all expected capabilities
        for capability in &fixture.expected_capabilities {
            assert!(
                fixture.chain_id.supports_capability(*capability),
                "chain {} should support capability {:?}",
                fixture.chain_id.name(),
                capability
            );
        }

        // Test actual API call to verify capability works
        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": [fixture.get_valid_address().to_string()]
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "chain {} should successfully process requests",
            fixture.chain_id.name()
        );
    }
}

#[tokio::test]
async fn contract_status_chain_specific_error_messages() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test that error messages are chain-specific
    for fixture in ChainTestFixture::all_chains() {
        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": []  // Empty addresses should trigger validation error
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let error_text = response.text().await.expect("failed to read response");
        assert!(
            error_text.contains("addresses list cannot be empty"),
            "error message should be descriptive for chain {}",
            fixture.chain_id.name()
        );
    }
}

#[tokio::test]
async fn contract_status_all_chains_implementation_status() {
    // Verify all chains are fully implemented for contract status endpoint
    for &chain in ChainId::all() {
        assert!(
            chain.is_fully_implemented(),
            "chain {} should be fully implemented",
            chain.name()
        );

        assert_eq!(
            chain.implementation_status(),
            shared_types::ChainImplementationStatus::Full,
            "chain {} should have full implementation status",
            chain.name()
        );

        // Verify chain supports required capabilities for contract status
        assert!(
            chain.supports_capability(shared_types::ChainCapability::MoralisMetadata),
            "chain {} should support Moralis metadata",
            chain.name()
        );

        assert!(
            chain.supports_capability(shared_types::ChainCapability::SpamPrediction),
            "chain {} should support spam prediction",
            chain.name()
        );
    }
}

#[tokio::test]
async fn contract_status_performance_all_chains() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .await
        .expect("failed to create server")
        .run_for_testing()
        .await
        .expect("failed to start test server");

    let client = reqwest::Client::new();

    // Test performance across all chains
    for fixture in ChainTestFixture::all_chains() {
        let start_time = std::time::Instant::now();

        let request = json!({
            "chain_id": fixture.chain_id.chain_id(),
            "addresses": [fixture.get_valid_address().to_string()]
        });

        let response = client
            .post(format!("http://{addr}/v1/contract/status"))
            .json(&request)
            .send()
            .await
            .expect("failed to send request");

        let duration = start_time.elapsed();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            duration.as_secs() < 30, // Should complete within 30 seconds
            "chain {} took too long: {:?}",
            fixture.chain_id.name(),
            duration
        );
    }
}
