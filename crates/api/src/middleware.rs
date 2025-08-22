// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Middleware module for HTTP request processing
//!
//! This module provides middleware for rate limiting, request logging,
//! and other cross-cutting concerns for the NFT API server.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{debug, error, warn};

use crate::config::RateLimitingConfig;

// Rate limiting constants
const RATE_LIMIT_WINDOW_SECONDS: u64 = 60;
const MAX_RATE_LIMIT_ENTRIES: usize = 10_000;

/// Rate limiting middleware state
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitingConfig,
    // Simple in-memory rate limiting using IP addresses
    // In production, consider using Redis or similar distributed cache
    requests: Arc<Mutex<HashMap<IpAddr, RequestCounter>>>,
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
            requests: Arc::new(Mutex::new(HashMap::new())),
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

        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => {
                error!(
                    "Rate limiter mutex poisoned - potential data corruption detected. \
                     This indicates a panic occurred while holding the mutex. Recovering with potentially invalid state."
                );
                poisoned.into_inner()
            }
        };

        let now = Instant::now();
        let window_duration = Duration::from_secs(RATE_LIMIT_WINDOW_SECONDS);

        // Clean up expired entries periodically
        let expired_keys: Vec<IpAddr> = requests
            .iter()
            .filter_map(|(&ip, counter)| {
                if now.duration_since(counter.window_start) > window_duration {
                    Some(ip)
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            requests.remove(&key);
        }

        // If we still have too many entries, clean up the oldest ones to prevent memory exhaustion
        if requests.len() > MAX_RATE_LIMIT_ENTRIES {
            warn!(
                "Rate limiter has {} entries, cleaning up oldest to prevent memory leak",
                requests.len()
            );

            let mut entries: Vec<_> = requests.iter().collect();
            entries.sort_by_key(|(_, counter)| counter.window_start);

            let entries_to_remove = requests.len() - MAX_RATE_LIMIT_ENTRIES / 2; // Remove down to half capacity
            let oldest_keys: Vec<IpAddr> = entries
                .into_iter()
                .take(entries_to_remove)
                .map(|(&ip, _)| ip)
                .collect();

            for key in oldest_keys {
                requests.remove(&key);
            }
        }

        // Check current IP
        if let Some(counter) = requests.get_mut(&ip) {
            if now.duration_since(counter.window_start) > window_duration {
                // Start new window
                counter.count = 1;
                counter.window_start = now;
                false
            } else {
                // Within current window
                counter.count += 1;
                if counter.count > self.config.requests_per_minute {
                    debug!("Rate limiting IP: {} ({} requests)", ip, counter.count);
                    true
                } else {
                    false
                }
            }
        } else {
            // First request from this IP
            requests.insert(
                ip,
                RequestCounter {
                    count: 1,
                    window_start: now,
                },
            );
            false
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
}
