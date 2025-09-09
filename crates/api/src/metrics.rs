// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Prometheus metrics module
//!
//! Provides global metrics using the default Prometheus registry via macros and
//! an Axum-compatible metrics handler.

use std::sync::LazyLock;

use axum::{
    http::{StatusCode, header},
    response::Response,
};
use prometheus::{
    Encoder, HistogramVec, IntCounterVec, TextEncoder, register_histogram_vec,
    register_int_counter_vec,
};
use shared_types::ChainId;

/// Total number of API requests received, labeled by `chain_id`.
pub static REQUESTS_BY_CHAIN: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "nft_api_requests_total",
        "Total number of API requests, labeled by chain_id",
        &["chain_id"]
    )
    .expect("Failed to create nft_api_requests_total counter vec")
});

/// Histogram for external API request durations in seconds.
pub static METADATA_API_REQUEST_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "nft_api_metadata_api_request_duration",
        "Metadata API request durations in seconds",
        &["api_name", "result"],
        vec![0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("Failed to create metadata API request duration histogram")
});

/// Histogram for spam predictor request durations in seconds.
pub static SPAM_PREDICTOR_REQUEST_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "nft_api_spam_predictor_request_duration",
        "Spam predictor request durations in seconds",
        &["result"],
        vec![0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("Failed to create spam predictor request duration histogram")
});

/// Histogram for concurrent batch processing durations in seconds.
pub static CONCURRENT_BATCH_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "nft_api_concurrent_batch_duration",
        "Concurrent batch processing durations in seconds",
        &["chain_id", "batch_size_category"],
        vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]
    )
    .expect("Failed to create concurrent batch duration histogram")
});

/// Increment the requests counter with `chain_id` label
///
/// # Arguments
/// * `chain_id` - The chain ID of the request
pub fn inc_requests_by_chain(chain_id: ChainId) {
    REQUESTS_BY_CHAIN
        .with_label_values(&[&chain_id.to_string()])
        .inc();
}

/// Observe the duration of a metadata API request
///
/// # Arguments
/// * `api_name` - The name of the metadata API
/// * `result` - The result of the metadata API request
/// * `duration_secs` - The duration of the request in seconds
pub fn observe_metadata_api_duration(api_name: &str, result: &str, duration_secs: f64) {
    METADATA_API_REQUEST_DURATION
        .with_label_values(&[api_name, result])
        .observe(duration_secs);
}

/// Observe the duration of a spam predictor request
///
/// # Arguments
/// * `result` - The result of the spam prediction
/// * `duration_secs` - The duration of the request in seconds
pub fn observe_spam_predictor_duration(result: &str, duration_secs: f64) {
    SPAM_PREDICTOR_REQUEST_DURATION
        .with_label_values(&[result])
        .observe(duration_secs);
}

/// Observe the duration of concurrent batch processing
///
/// # Arguments
/// * `chain_id_str` - The chain ID string being processed
/// * `batch_size_category` - The category of batch size
/// * `duration_secs` - The duration of the batch processing in seconds
pub fn observe_concurrent_batch_duration(
    chain_id_str: &str,
    batch_size_category: &str,
    duration_secs: f64,
) {
    CONCURRENT_BATCH_DURATION
        .with_label_values(&[chain_id_str, batch_size_category])
        .observe(duration_secs);
}

/// Axum handler that exports metrics in Prometheus text format
///
/// # Panics
///
/// This function will panic if:
/// - The metrics encoder fails to encode the metrics data
/// - The UTF-8 conversion of the encoded buffer fails
/// - The HTTP response builder fails to create the response
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
        .body(String::from_utf8(buffer).expect("metrics buffer should be valid UTF-8"))
        .expect("Failed to create metrics response")
}
