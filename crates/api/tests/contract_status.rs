// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the contract status endpoint

use api::{Server, ServerConfig, ShutdownConfig};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn contract_status_valid_addresses() {
    // Create test server
    let config = ServerConfig::for_testing();
    let shutdown_config = ShutdownConfig::default();
    let (addr, _) = Server::new(config, shutdown_config)
        .expect("Failed to create server")
        .run_for_testing()
        .await
        .expect("Failed to start test server");

    let client = reqwest::Client::new();

    // Valid Ethereum addresses
    let valid_request = json!({
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

    // Currently returns 200 with empty response since handler is stubbed
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

    // Invalid Ethereum addresses (wrong length, invalid hex)
    let invalid_request = json!({
        "addresses": [
            "0x123",  // too short
            "0xghijklmnopqrstuvwxyz123456789012345678",  // invalid hex characters
            "not_an_address"  // completely invalid format
        ]
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&invalid_request)
        .send()
        .await
        .expect("Failed to send request");

    // Should return 422 Unprocessable Entity for invalid addresses (Axum's default for deserialization errors)
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

    // Empty addresses array
    let empty_request = json!({
        "addresses": []
    });

    let response = client
        .post(format!("http://{addr}/v1/contract/status"))
        .json(&empty_request)
        .send()
        .await
        .expect("Failed to send request");

    // Should return 400 Bad Request for empty addresses array
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response_text = response.text().await.expect("Failed to read response");
    assert!(response_text.contains("addresses list cannot be empty"));
}
