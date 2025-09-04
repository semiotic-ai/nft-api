// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Middleware module for HTTP request processing
//!
//! This module provides middleware for rate limiting, request logging,
//! and other cross-cutting concerns for the NFT API server.

use std::{
    net::IpAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use shared_types::{ChainCapability, ChainId, ChainStatus};
use tokio::time::timeout;
use tracing::{Instrument, Level, debug, info, span, warn};

use crate::{config::RateLimitingConfig, error::ChainValidationError};

// Rate limiting constants
const RATE_LIMIT_WINDOW_SECONDS: u64 = 60;
const MAX_RATE_LIMIT_ENTRIES: usize = 10_000;

/// Rate limiting middleware state
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitingConfig,
    // Lock-free concurrent rate limiting using DashMap
    requests: Arc<DashMap<IpAddr, RequestCounter>>,
}

#[derive(Debug, Clone)]
struct RequestCounter {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitingConfig) -> Self {
        Self {
            config,
            requests: Arc::new(DashMap::new()),
        }
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a request from the given IP should be rate limited
    pub fn is_rate_limited(&self, ip: IpAddr) -> bool {
        if !self.config.enabled {
            return false;
        }

        let now = Instant::now();
        let window_duration = Duration::from_secs(RATE_LIMIT_WINDOW_SECONDS);

        // Periodically clean up expired entries to prevent memory leaks
        if self.requests.len() > MAX_RATE_LIMIT_ENTRIES {
            self.cleanup_expired_entries(now, window_duration);
        }

        // Lock-free atomic operation to check/update rate limit
        let is_limited = self
            .requests
            .entry(ip)
            .and_modify(|counter| {
                if now.duration_since(counter.window_start) > window_duration {
                    // Reset window
                    counter.count = 1;
                    counter.window_start = now;
                } else {
                    // Increment in current window
                    counter.count += 1;
                }
            })
            .or_insert_with(|| RequestCounter {
                count: 1,
                window_start: now,
            });

        let current_count = is_limited.count;

        if current_count > self.config.requests_per_minute {
            debug!("rate limiting IP: {} ({} requests)", ip, current_count);
            true
        } else {
            false
        }
    }

    /// Clean up expired entries using efficient retain operation
    fn cleanup_expired_entries(&self, now: Instant, window_duration: Duration) {
        let entries_before = self.requests.len();

        // Use DashMap's retain for efficient concurrent cleanup
        self.requests
            .retain(|_, counter| now.duration_since(counter.window_start) <= window_duration);

        let entries_after = self.requests.len();
        let cleaned_up = entries_before.saturating_sub(entries_after);

        if cleaned_up > 0 {
            debug!("cleaned up {} expired rate limiter entries", cleaned_up);
        }

        // If still too many entries, remove oldest ones
        if entries_after > MAX_RATE_LIMIT_ENTRIES {
            warn!(
                "rate limiter still has {} entries after cleanup, removing oldest",
                entries_after
            );

            // Collect oldest entries for removal
            let mut oldest_entries: Vec<_> = self
                .requests
                .iter()
                .map(|entry| (*entry.key(), entry.value().window_start))
                .collect();

            oldest_entries.sort_by_key(|(_, window_start)| *window_start);

            let entries_to_remove = entries_after - MAX_RATE_LIMIT_ENTRIES / 2;
            for (ip, _) in oldest_entries.into_iter().take(entries_to_remove) {
                self.requests.remove(&ip);
            }
        }
    }
}

/// Rate limiting middleware function
pub async fn rate_limiting_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    State(rate_limiter): State<RateLimiter>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = addr.ip();

    if rate_limiter.is_rate_limited(client_ip) {
        warn!("Rate limit exceeded for IP: {}", client_ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}

/// Chain validation result with context information
#[derive(Debug, Clone)]
pub struct ChainValidationResult {
    /// The validated chain ID
    pub chain_id: ChainId,
    /// Status of the chain
    pub status: ChainStatus,
    /// Available capabilities
    pub capabilities: Vec<ChainCapability>,
    /// Limitations if any
    pub limitations: Vec<String>,
    /// Whether warnings should be added to response
    pub add_warnings: bool,
}

impl ChainValidationResult {
    /// Create validation result for a chain
    pub fn from_chain_id(chain_id: ChainId) -> Result<Self, ChainValidationError> {
        let status = chain_id.support_status();

        match status {
            ChainStatus::FullySupported => Ok(Self {
                chain_id,
                status,
                capabilities: chain_id.capabilities(),
                limitations: vec![],
                add_warnings: false,
            }),
            ChainStatus::PartiallySupported => Ok(Self {
                chain_id,
                status,
                capabilities: chain_id.capabilities(),
                limitations: chain_id
                    .limitations()
                    .into_iter()
                    .map(ToString::to_string)
                    .collect(),
                add_warnings: true,
            }),
            ChainStatus::Planned => Err(ChainValidationError::planned_chain(chain_id)),
            ChainStatus::Deprecated => Err(ChainValidationError::unsupported_chain(chain_id)),
        }
    }

    /// Check if chain supports a specific capability
    pub fn supports_capability(&self, capability: ChainCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

/// Result type for chain ID parsing
enum ChainIdParseResult {
    Found(ChainId),
    NotFound,
    Invalid(String),
}

/// Parse and validate chain ID from URL path
fn parse_chain_id_from_path(path: &str) -> ChainIdParseResult {
    let Some(chain_id_str) = extract_chain_id_from_path(path) else {
        return ChainIdParseResult::NotFound;
    };

    let Ok(chain_id) = ChainId::from_str(&chain_id_str) else {
        return ChainIdParseResult::Invalid(chain_id_str);
    };

    ChainIdParseResult::Found(chain_id)
}

/// Handle chain validation errors with structured logging and responses
fn handle_chain_validation_error(chain_id: ChainId, error: &ChainValidationError) -> Response {
    match error {
        ChainValidationError::PlannedChain {
            chain_name,
            estimated_availability,
            ..
        } => {
            info!(
                chain_id = %chain_id,
                chain_name = %chain_name,
                estimated_availability = ?estimated_availability,
                "rejecting request for planned chain"
            );
        }
        ChainValidationError::UnsupportedChain { chain_name, .. } => {
            info!(
                chain_id = %chain_id,
                chain_name = %chain_name,
                "rejecting request for unsupported chain"
            );
        }
        _ => {
            warn!(
                chain_id = %chain_id,
                error = %error,
                "chain validation failed"
            );
        }
    }

    let error_response = axum::Json(error.to_json_response());
    let mut response = error_response.into_response();
    *response.status_mut() = error.status_code();
    response
}

/// Add chain status and warning headers to response for partially supported chains
fn add_chain_warning_headers(response: &mut Response, validation_result: &ChainValidationResult) {
    if !validation_result.add_warnings {
        return;
    }

    let headers = response.headers_mut();

    add_chain_status_header(headers, validation_result);
    add_capabilities_header(headers, validation_result);
    add_limitations_header(headers, validation_result);
    add_warning_header(headers, validation_result);
}

/// Add X-Chain-Status header
fn add_chain_status_header(
    headers: &mut axum::http::HeaderMap,
    validation_result: &ChainValidationResult,
) {
    if let Ok(status_value) = HeaderValue::from_str(&validation_result.status.to_string()) {
        headers.insert("X-Chain-Status", status_value);
    } else {
        warn!(
            "failed to create X-Chain-Status header value for status: {}",
            validation_result.status
        );
    }
}

/// Add X-Chain-Capabilities header
fn add_capabilities_header(
    headers: &mut axum::http::HeaderMap,
    validation_result: &ChainValidationResult,
) {
    let capabilities_str = validation_result
        .capabilities
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");

    if let Ok(capabilities_value) = HeaderValue::from_str(&capabilities_str) {
        headers.insert("X-Chain-Capabilities", capabilities_value);
    } else {
        warn!(
            "failed to create X-Chain-Capabilities header value: {}",
            capabilities_str
        );
    }
}

/// Add X-Chain-Limitations header if limitations exist
fn add_limitations_header(
    headers: &mut axum::http::HeaderMap,
    validation_result: &ChainValidationResult,
) {
    if validation_result.limitations.is_empty() {
        return;
    }

    let limitations_str = validation_result.limitations.join("; ");
    if let Ok(limitations_value) = HeaderValue::from_str(&limitations_str) {
        headers.insert("X-Chain-Limitations", limitations_value);
    } else {
        warn!(
            "failed to create X-Chain-Limitations header value: {}",
            limitations_str
        );
    }
}

/// Add RFC 7234 compatible Warning header
fn add_warning_header(
    headers: &mut axum::http::HeaderMap,
    validation_result: &ChainValidationResult,
) {
    let warning_msg = format!(
        "199 - \"Chain {} has limited functionality: {}\"",
        validation_result.chain_id.name(),
        validation_result.limitations.join(", ")
    );

    if let Ok(warning_value) = HeaderValue::from_str(&warning_msg) {
        headers.insert("Warning", warning_value);
    } else {
        warn!("failed to create Warning header value: {}", warning_msg);
    }
}

/// Chain validation middleware that validates `chain_id` from URL path
///
/// This middleware intercepts requests early to validate chain support before
/// routing to handlers. It adds chain context to tracing spans and returns
/// structured error responses for unsupported chains.
///
/// # Behavior
///
/// The middleware performs the following operations:
/// 1. Extracts chain ID from URL path patterns like `/chains/{chain_id}/...`
/// 2. Validates that the chain ID corresponds to a supported blockchain
/// 3. Creates a tracing span with chain context for observability
/// 4. Returns structured JSON error responses for validation failures
/// 5. Adds warning headers for chains with limited functionality
///
/// # URL Patterns
///
/// The middleware recognizes these URL patterns for chain ID extraction:
/// - `/api/v1/chains/{chain_id}/contracts/{address}/status`
/// - `/v1/chains/{chain_id}/contract/status`
/// - `/chains/{chain_id}/...`
/// - `/{chain_id}/...` (if numeric and corresponds to a valid chain)
///
/// # Error Responses
///
/// For invalid or unsupported chains, the middleware returns structured JSON responses:
///
/// ```json
/// {
///   "error": "chain_not_supported",
///   "message": "Chain Ethereum (ID: 1) is not supported",
///   "details": {
///     "chain_id": 1,
///     "chain_name": "Ethereum",
///     "status": "unsupported"
///   }
/// }
/// ```
///
/// # Chain Status Headers
///
/// For partially supported chains, the middleware adds informational headers:
/// - `X-Chain-Status`: The current support status
/// - `X-Chain-Capabilities`: Comma-separated list of supported capabilities
/// - `X-Chain-Limitations`: Semicolon-separated list of current limitations
/// - `Warning`: RFC 7234 compatible warning for limited functionality
pub async fn chain_validation_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let chain_id = match parse_chain_id_from_path(req.uri().path()) {
        ChainIdParseResult::Found(chain_id) => chain_id,
        ChainIdParseResult::NotFound => {
            // No chain_id in path, proceed normally
            return next.run(req).await;
        }
        ChainIdParseResult::Invalid(chain_id_str) => {
            warn!("invalid chain ID in request path: {}", chain_id_str);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    // Cooperative yielding after CPU-intensive parsing
    tokio::task::yield_now().await;

    let validation_result = match ChainValidationResult::from_chain_id(chain_id) {
        Ok(result) => result,
        Err(chain_err) => return handle_chain_validation_error(chain_id, &chain_err),
    };

    let span = span!(
        Level::INFO,
        "chain_request",
        chain_id = %validation_result.chain_id,
        chain_name = validation_result.chain_id.name(),
        chain_status = %validation_result.status,
        capabilities = ?validation_result.capabilities.iter().map(ToString::to_string).collect::<Vec<_>>()
    );

    let Ok(mut response) = timeout(Duration::from_secs(30), next.run(req).instrument(span)).await
    else {
        warn!(
            "request timed out for chain: {}",
            validation_result.chain_id.name()
        );
        return StatusCode::REQUEST_TIMEOUT.into_response();
    };

    add_chain_warning_headers(&mut response, &validation_result);
    response
}

/// Extract `chain_id` parameter from URL path
///
/// Supports various path patterns:
/// - `/api/v1/chains/{chain_id}/contracts/{address}/status`
/// - `/chains/{chain_id}/...`
/// - `/{chain_id}/...` (if numeric and valid)
fn extract_chain_id_from_path(path: &str) -> Option<String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    for (i, segment) in segments.iter().enumerate() {
        // Look for "chains" segment followed by chain_id
        if *segment == "chains" && i + 1 < segments.len() {
            let potential_chain_id = segments[i + 1];
            // Only return if it's a valid chain ID (either numeric or name)
            if ChainId::from_str(potential_chain_id).is_ok() {
                return Some(potential_chain_id.to_string());
            }
        }
    }

    // Alternative: Look for numeric segments that could be chain IDs
    for segment in &segments {
        if segment.parse::<u64>().is_ok() {
            // Validate it's a known chain ID
            if ChainId::from_str(segment).is_ok() {
                return Some((*segment).to_string());
            }
        }
    }

    None
}

/// Validate that a chain supports a specific capability
///
/// This can be used by handlers to check capability support before
/// making API calls to external services.
pub fn validate_chain_capability(
    chain_id: ChainId,
    capability: ChainCapability,
) -> Result<(), ChainValidationError> {
    if chain_id.supports_capability(capability) {
        Ok(())
    } else {
        Err(ChainValidationError::unsupported_capability(
            chain_id, capability,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_creation() {
        let config = RateLimitingConfig {
            enabled: true,
            requests_per_minute: 10,
        };
        let limiter = RateLimiter::new(config);
        assert!(limiter.config.enabled);
        assert_eq!(limiter.config.requests_per_minute, 10);
    }

    #[test]
    fn rate_limiter_disabled() {
        let config = RateLimitingConfig {
            enabled: false,
            requests_per_minute: 1,
        };
        let limiter = RateLimiter::new(config);

        let ip = "127.0.0.1".parse().unwrap();
        // Should never be rate limited when disabled
        for _ in 0..10 {
            assert!(!limiter.is_rate_limited(ip));
        }
    }

    #[test]
    fn rate_limiter_within_limits() {
        let config = RateLimitingConfig {
            enabled: true,
            requests_per_minute: 5,
        };
        let limiter = RateLimiter::new(config);

        let ip = "127.0.0.1".parse().unwrap();

        // First 5 requests should not be rate limited
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited(ip));
        }
    }

    #[test]
    fn rate_limiter_exceeds_limits() {
        let config = RateLimitingConfig {
            enabled: true,
            requests_per_minute: 3,
        };
        let limiter = RateLimiter::new(config);

        let ip = "127.0.0.1".parse().unwrap();

        // First 3 requests should not be rate limited
        for _ in 0..3 {
            assert!(!limiter.is_rate_limited(ip));
        }

        // 4th request should be rate limited
        assert!(limiter.is_rate_limited(ip));

        // Subsequent requests should also be rate limited
        assert!(limiter.is_rate_limited(ip));
    }

    #[test]
    fn rate_limiter_different_ips() {
        let config = RateLimitingConfig {
            enabled: true,
            requests_per_minute: 2,
        };
        let limiter = RateLimiter::new(config);

        let ip1 = "127.0.0.1".parse().unwrap();
        let ip2 = "192.168.1.1".parse().unwrap();

        // Each IP should have its own limit
        assert!(!limiter.is_rate_limited(ip1));
        assert!(!limiter.is_rate_limited(ip2));
        assert!(!limiter.is_rate_limited(ip1));
        assert!(!limiter.is_rate_limited(ip2));

        // Now both should be at their limits
        assert!(limiter.is_rate_limited(ip1));
        assert!(limiter.is_rate_limited(ip2));
    }

    #[test]
    fn extract_chain_id_from_path() {
        // Standard chains path pattern
        assert_eq!(
            super::extract_chain_id_from_path("/api/v1/chains/137/contracts/0x123/status"),
            Some("137".to_string())
        );

        // Simplified chains path
        assert_eq!(
            super::extract_chain_id_from_path("/chains/1/status"),
            Some("1".to_string())
        );

        // Chain name instead of ID
        assert_eq!(
            super::extract_chain_id_from_path("/chains/polygon/status"),
            Some("polygon".to_string())
        );

        // Numeric chain ID as direct path segment
        assert_eq!(
            super::extract_chain_id_from_path("/137/contracts/status"),
            Some("137".to_string())
        );

        // No chain ID in path
        assert_eq!(super::extract_chain_id_from_path("/api/v1/health"), None);

        // Invalid chain ID should not be extracted
        assert_eq!(
            super::extract_chain_id_from_path("/chains/999/status"),
            None
        );
    }

    #[test]
    fn chain_validation_result_fully_supported() {
        let result = ChainValidationResult::from_chain_id(ChainId::Ethereum).unwrap();

        assert_eq!(result.chain_id, ChainId::Ethereum);
        assert_eq!(result.status, ChainStatus::FullySupported);
        assert!(!result.add_warnings);
        assert!(result.limitations.is_empty());
        assert!(result.supports_capability(ChainCapability::MoralisMetadata));
        assert!(result.supports_capability(ChainCapability::PinaxAnalytics));
        assert!(result.supports_capability(ChainCapability::SpamPrediction));
    }

    #[test]
    fn chain_validation_result_all_chains_fully_supported() {
        // Test that Base is now fully supported (was previously partial)
        let result = ChainValidationResult::from_chain_id(ChainId::Base).unwrap();

        assert_eq!(result.chain_id, ChainId::Base);
        assert_eq!(result.status, ChainStatus::FullySupported);
        assert!(!result.add_warnings);
        assert!(result.limitations.is_empty());
        assert!(result.supports_capability(ChainCapability::MoralisMetadata));
        assert!(result.supports_capability(ChainCapability::PinaxAnalytics));
        assert!(result.supports_capability(ChainCapability::SpamPrediction));
    }

    #[test]
    fn chain_validation_result_avalanche_now_supported() {
        // Test that Avalanche is now fully supported (was previously planned)
        let result = ChainValidationResult::from_chain_id(ChainId::Avalanche).unwrap();

        assert_eq!(result.chain_id, ChainId::Avalanche);
        assert_eq!(result.status, ChainStatus::FullySupported);
        assert!(!result.add_warnings);
        assert!(result.limitations.is_empty());
        assert!(result.supports_capability(ChainCapability::MoralisMetadata));
        assert!(result.supports_capability(ChainCapability::PinaxAnalytics));
        assert!(result.supports_capability(ChainCapability::SpamPrediction));
    }

    #[test]
    fn validate_chain_capability_supported() {
        let result = validate_chain_capability(ChainId::Polygon, ChainCapability::MoralisMetadata);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_chain_capability_all_now_supported() {
        // Test that all capabilities are now supported on all chains
        let result = validate_chain_capability(ChainId::Base, ChainCapability::PinaxAnalytics);
        assert!(result.is_ok());

        // Test Arbitrum with all capabilities
        assert!(
            validate_chain_capability(ChainId::Arbitrum, ChainCapability::MoralisMetadata).is_ok()
        );
        assert!(
            validate_chain_capability(ChainId::Arbitrum, ChainCapability::PinaxAnalytics).is_ok()
        );
        assert!(
            validate_chain_capability(ChainId::Arbitrum, ChainCapability::SpamPrediction).is_ok()
        );
    }

    #[test]
    fn comprehensive_chain_validation_support() {
        // Test that all supported chains are now fully supported
        let chains = [
            ChainId::Ethereum,
            ChainId::Polygon,
            ChainId::Base,
            ChainId::Avalanche,
            ChainId::Arbitrum,
        ];

        for chain in chains {
            let result = ChainValidationResult::from_chain_id(chain).unwrap();

            assert_eq!(result.chain_id, chain);
            assert_eq!(result.status, ChainStatus::FullySupported);
            assert!(
                !result.add_warnings,
                "Chain {chain:?} should not add warnings"
            );
            assert!(
                result.limitations.is_empty(),
                "Chain {chain:?} should have no limitations"
            );

            // All chains should support all capabilities
            assert!(result.supports_capability(ChainCapability::MoralisMetadata));
            assert!(result.supports_capability(ChainCapability::PinaxAnalytics));
            assert!(result.supports_capability(ChainCapability::SpamPrediction));
        }
    }

    #[test]
    fn extract_chain_id_comprehensive_patterns() {
        // Test various URL patterns that should extract chain IDs
        let test_cases = vec![
            // Standard API patterns
            ("/api/v1/chains/1/contracts/0x123/status", Some("1")),
            ("/api/v1/chains/137/contracts/0x456/metadata", Some("137")),
            ("/v1/chains/8453/contract/status", Some("8453")),
            // Simplified patterns
            ("/chains/1/status", Some("1")),
            ("/chains/137", Some("137")),
            // Direct chain ID patterns (if numeric and valid)
            ("/1/status", Some("1")),
            ("/137/contracts", Some("137")),
            ("/8453", Some("8453")),
            // Patterns that should NOT extract chain IDs
            ("/api/v1/health", None),
            ("/api/v1/status", None),
            ("/chains/invalid/status", None),
            ("/999/status", None),               // Invalid chain ID
            ("/api/v1/chains/999/status", None), // Invalid chain ID
            ("", None),
            ("/", None),
        ];

        for (path, expected) in test_cases {
            let result = super::extract_chain_id_from_path(path);
            assert_eq!(
                result,
                expected.map(String::from),
                "Failed for path: {path}"
            );
        }
    }

    #[test]
    fn chain_validation_result_capabilities() {
        let result = ChainValidationResult::from_chain_id(ChainId::Ethereum).unwrap();

        // Test capability checking
        assert!(result.supports_capability(ChainCapability::MoralisMetadata));
        assert!(result.supports_capability(ChainCapability::PinaxAnalytics));
        assert!(result.supports_capability(ChainCapability::SpamPrediction));

        // Verify all expected capabilities are present
        assert_eq!(result.capabilities.len(), 3);
        assert!(
            result
                .capabilities
                .contains(&ChainCapability::MoralisMetadata)
        );
        assert!(
            result
                .capabilities
                .contains(&ChainCapability::PinaxAnalytics)
        );
        assert!(
            result
                .capabilities
                .contains(&ChainCapability::SpamPrediction)
        );
    }

    #[test]
    fn validation_error_types() {
        use crate::error::ChainValidationError;

        // Test error creation for different scenarios
        let unsupported_error = ChainValidationError::unsupported_chain(ChainId::Ethereum);
        match unsupported_error {
            ChainValidationError::UnsupportedChain {
                chain_id,
                chain_name,
                ..
            } => {
                assert_eq!(chain_id, 1);
                assert_eq!(chain_name, "Ethereum");
            }
            _ => panic!("Expected UnsupportedChain error"),
        }

        let planned_error = ChainValidationError::planned_chain(ChainId::Polygon);
        match planned_error {
            ChainValidationError::PlannedChain {
                chain_id,
                chain_name,
                ..
            } => {
                assert_eq!(chain_id, 137);
                assert_eq!(chain_name, "Polygon");
            }
            _ => panic!("Expected PlannedChain error"),
        }
    }

    #[test]
    fn error_status_codes() {
        use crate::error::ChainValidationError;

        let unsupported = ChainValidationError::unsupported_chain(ChainId::Ethereum);
        assert_eq!(unsupported.status_code(), StatusCode::BAD_REQUEST);

        let planned = ChainValidationError::planned_chain(ChainId::Ethereum);
        assert_eq!(planned.status_code(), StatusCode::NOT_IMPLEMENTED);

        let capability = ChainValidationError::unsupported_capability(
            ChainId::Ethereum,
            ChainCapability::MoralisMetadata,
        );
        assert_eq!(capability.status_code(), StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn error_json_responses() {
        use crate::error::ChainValidationError;

        let error = ChainValidationError::unsupported_chain(ChainId::Ethereum);
        let json = error.to_json_response();

        // Verify JSON structure
        assert_eq!(json["error"], "chain_not_supported");
        assert!(json["message"].is_string());
        assert_eq!(json["details"]["chain_id"], 1);
        assert_eq!(json["details"]["chain_name"], "Ethereum");
        assert_eq!(json["details"]["status"], "unsupported");
    }

    #[test]
    fn path_extraction_edge_cases() {
        // Test edge cases for path extraction
        let edge_cases = vec![
            // Empty and minimal paths
            ("", None),
            ("/", None),
            ("//", None),
            // Multiple slashes
            ("///chains//1//status", Some("1")),
            ("/api//v1//chains//137//status", Some("137")),
            // Case sensitivity (chain ID is still extracted if it's valid)
            ("/Chains/1/Status", Some("1")), // Chain ID extraction works regardless of case
            ("/CHAINS/1/STATUS", Some("1")),
            // Numbers that look like chain IDs but aren't valid
            ("/12345/status", None),
            ("/0/status", None),
            // Valid chain IDs in different positions
            ("/prefix/1/suffix", Some("1")),
            ("/long/path/with/137/in/middle", Some("137")),
        ];

        for (path, expected) in edge_cases {
            let result = super::extract_chain_id_from_path(path);
            assert_eq!(
                result,
                expected.map(String::from),
                "Failed for edge case path: {path}"
            );
        }
    }
}
