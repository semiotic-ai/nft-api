// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the contract status endpoint

use api::{Server, ServerConfig, ShutdownConfig};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn contract_status_valid_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn contract_status_empty_addresses() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn contract_status_chain_id_string() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
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
async fn contract_status_partial_implementation_chain() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let partial_chain_request = json!({
        "chain_id": 8453, // Base - partially implemented
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&partial_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: serde_json::Value = response.json().await.expect("Failed to parse response");
    let result = &response_body["0x1234567890123456789012345678901234567890"];
    assert_eq!(result["chain_id"], 8453);
    assert!(
        result["message"]
            .as_str()
            .unwrap()
            .contains("partially supported")
    );
}

#[tokio::test]
async fn contract_status_planned_implementation_chain() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    let planned_chain_request = json!({
        "chain_id": 43114, // Avalanche - planned
        "addresses": ["0x1234567890123456789012345678901234567890"]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&planned_chain_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: serde_json::Value = response.json().await.expect("Failed to parse response");
    let result = &response_body["0x1234567890123456789012345678901234567890"];
    assert_eq!(result["chain_id"], 43114);
    assert!(
        result["message"]
            .as_str()
            .unwrap()
            .contains("not yet implemented")
    );
}

#[tokio::test]
async fn contract_status_missing_chain_id() {
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
