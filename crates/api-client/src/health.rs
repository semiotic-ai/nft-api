// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Health check types and utilities for API clients

use std::time::Duration;

// Health check constants
const DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_HEALTH_CHECK_INTERVAL_SECONDS: u64 = 30;
const DEFAULT_FAILURE_THRESHOLD: u8 = 3;
const DEFAULT_SUCCESS_THRESHOLD: u8 = 1;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Health status of an API client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum HealthStatus {
    /// Service is healthy and operational
    Up,
    /// Service is degraded but still functional
    Degraded { reason: String },
    /// Service is down and not functional
    Down { reason: String },
}

/// Detailed health check result with timing and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// The health status
    pub status: HealthStatus,
    /// Response time for the health check
    pub response_time: Duration,
    /// When the health check was performed
    pub timestamp: DateTime<Utc>,
    /// Optional additional details
    pub details: Option<String>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Timeout for health check requests
    pub timeout: Duration,
    /// Interval between health checks
    pub interval: Duration,
    /// Number of consecutive failures before marking as down
    pub failure_threshold: u8,
    /// Number of consecutive successes before marking as up
    pub success_threshold: u8,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS),
            interval: Duration::from_secs(DEFAULT_HEALTH_CHECK_INTERVAL_SECONDS),
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            success_threshold: DEFAULT_SUCCESS_THRESHOLD,
        }
    }
}

impl HealthStatus {
    /// Check if this health status indicates the service is available
    pub fn is_available(&self) -> bool {
        matches!(self, HealthStatus::Up | HealthStatus::Degraded { .. })
    }

    /// Check if this health status indicates the service is completely down
    pub fn is_down(&self) -> bool {
        matches!(self, HealthStatus::Down { .. })
    }

    /// Get a human-readable description of the status
    pub fn description(&self) -> &str {
        match self {
            HealthStatus::Up => "Service is healthy",
            HealthStatus::Degraded { reason } | HealthStatus::Down { reason } => reason,
        }
    }
}

impl HealthCheckResult {
    /// Create a new successful health check result
    pub fn healthy(response_time: Duration) -> Self {
        Self {
            status: HealthStatus::Up,
            response_time,
            timestamp: Utc::now(),
            details: None,
        }
    }

    /// Create a new degraded health check result
    pub fn degraded(response_time: Duration, reason: String) -> Self {
        Self {
            status: HealthStatus::Degraded { reason },
            response_time,
            timestamp: Utc::now(),
            details: None,
        }
    }

    /// Create a new unhealthy health check result
    pub fn unhealthy(response_time: Duration, reason: String) -> Self {
        Self {
            status: HealthStatus::Down { reason },
            response_time,
            timestamp: Utc::now(),
            details: None,
        }
    }

    /// Add additional details to the health check result
    #[must_use]
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_availability() {
        assert!(HealthStatus::Up.is_available());
        assert!(
            HealthStatus::Degraded {
                reason: "slow".to_string()
            }
            .is_available()
        );
        assert!(
            !HealthStatus::Down {
                reason: "offline".to_string()
            }
            .is_available()
        );
    }

    #[test]
    fn health_status_down_check() {
        assert!(!HealthStatus::Up.is_down());
        assert!(
            !HealthStatus::Degraded {
                reason: "slow".to_string()
            }
            .is_down()
        );
        assert!(
            HealthStatus::Down {
                reason: "offline".to_string()
            }
            .is_down()
        );
    }

    #[test]
    fn health_check_result_creation() {
        let duration = Duration::from_millis(100);

        let healthy = HealthCheckResult::healthy(duration);
        assert!(healthy.status.is_available());
        assert_eq!(healthy.response_time, duration);

        let degraded = HealthCheckResult::degraded(duration, "slow response".to_string());
        assert!(degraded.status.is_available());

        let unhealthy = HealthCheckResult::unhealthy(duration, "connection failed".to_string());
        assert!(unhealthy.status.is_down());
    }

    #[test]
    fn health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        assert_eq!(
            config.timeout,
            Duration::from_secs(DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS)
        );
        assert_eq!(
            config.interval,
            Duration::from_secs(DEFAULT_HEALTH_CHECK_INTERVAL_SECONDS)
        );
        assert_eq!(config.failure_threshold, DEFAULT_FAILURE_THRESHOLD);
        assert_eq!(config.success_threshold, DEFAULT_SUCCESS_THRESHOLD);
    }
}
