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
