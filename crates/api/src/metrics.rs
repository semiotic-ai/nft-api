// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Prometheus metrics module
//!
//! Provides global metrics using the default Prometheus registry via macros and
//! an Axum-compatible metrics handler.

use axum::http::{StatusCode, header};
use axum::response::Response;
use lazy_static::lazy_static;
use prometheus::{Encoder, IntCounterVec, TextEncoder, register_int_counter_vec};
use shared_types::ChainId;

lazy_static! {
    /// Total number of API requests received, labeled by `chain_id`.
    pub static ref REQUESTS_BY_CHAIN: IntCounterVec = register_int_counter_vec!(
        "nft_api_requests_total",
        "Total number of API requests, labeled by chain_id",
        &["chain_id"]
    )
    .expect("Failed to create nft_api_requests_total counter vec");
}

/// Increment the requests counter with `chain_id` label
pub fn inc_requests_by_chain(chain_id: ChainId) {
    REQUESTS_BY_CHAIN
        .with_label_values(&[&chain_id.to_string()])
        .inc();
}

/// Axum handler that exports metrics in Prometheus text format
pub async fn metrics_handler() -> Response<String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, encoder.format_type())
        .body(String::from_utf8(buffer).unwrap())
        .expect("Failed to create metrics response")
}
