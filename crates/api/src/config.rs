// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Server configuration module
//!
//! This module provides configuration structures and logic for the NFT API server,
//! supporting different environments and validation of configuration parameters.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{Result, anyhow, ensure};
use config::{Config, ConfigError, Environment as ConfigEnv, File};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::error::{ServerError, ServerResult};

/// A validated server port that ensures the value is appropriate for the environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ServerPort {
    port: u16,
    environment: Environment,
}

impl ServerPort {
    /// Create a new `ServerPort`, ensuring it's valid for the given environment
    ///
    /// # Errors
    ///
    /// Returns an error if the port is 0 in non-testing environments
    pub fn new(port: u16, environment: Environment) -> Result<Self> {
        if port == 0 && environment != Environment::Testing {
            return Err(anyhow!("port cannot be 0 in non-testing environments"));
        }
        Ok(Self { port, environment })
    }

    /// Create a safe default port for development
    pub const fn default_development() -> Self {
        Self {
            port: 3000,
            environment: Environment::Development,
        }
    }

    /// Create a safe testing port (port 0)
    pub const fn testing() -> Self {
        Self {
            port: 0,
            environment: Environment::Testing,
        }
    }

    /// Get the port value
    pub fn value(&self) -> u16 {
        self.port
    }
}

impl<'de> Deserialize<'de> for ServerPort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let port = u16::deserialize(deserializer)?;
        // We'll validate this during configuration loading when we know the environment
        Ok(Self {
            port,
            environment: Environment::Development, // temporary, will be fixed during load
        })
    }
}

/// A validated timeout duration in seconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TimeoutSeconds(Duration);

impl TimeoutSeconds {
    /// Create a new `TimeoutSeconds`, ensuring the value is within valid bounds
    ///
    /// # Errors
    ///
    /// Returns an error if timeout is 0 or greater than 300 seconds
    pub fn new(seconds: u64) -> Result<Self> {
        ensure!(seconds != 0, "timeout must be greater than 0");
        ensure!(seconds <= 300, "timeout cannot exceed 300");
        Ok(Self(Duration::from_secs(seconds)))
    }

    /// Create a safe default timeout (30 seconds)
    pub const fn default_value() -> Self {
        Self(Duration::from_secs(30))
    }

    /// Create a safe testing timeout (5 seconds)
    pub const fn testing() -> Self {
        Self(Duration::from_secs(5))
    }

    /// Get the timeout value in seconds
    pub fn value(&self) -> Duration {
        self.0
    }
}

impl<'de> Deserialize<'de> for TimeoutSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Self::new(seconds).map_err(|e| de::Error::custom(e.to_string()))
    }
}

impl Default for TimeoutSeconds {
    fn default() -> Self {
        Self::default_value()
    }
}

/// Environment types for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Production environment
    Production,
    /// Development environment
    Development,
    /// Testing environment
    Testing,
}

/// Server configuration for different environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: IpAddr,
    /// Server port (validated for environment compatibility)
    pub port: ServerPort,
    /// Request timeout in seconds (validated range: 1-300)
    pub timeout_seconds: TimeoutSeconds,
    /// Environment type
    pub environment: Environment,
    /// Additional configuration parameters
    pub extensions: HashMap<String, String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: ServerPort::default_development(),
            timeout_seconds: TimeoutSeconds::default(),
            environment: Environment::Development,
            extensions: HashMap::new(),
        }
    }
}

impl ServerConfig {
    /// Create configuration from environment variables and optional configuration files
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Config` if configuration is invalid or cannot be loaded.
    pub fn from_env() -> ServerResult<Self> {
        Self::load().map_err(|e| ServerError::Config {
            message: format!("failed to load configuration: {e}"),
        })
    }

    /// Load configuration using the config crate with hierarchical sources
    ///
    /// Configuration is loaded in the following order (later sources override earlier ones):
    /// 1. Default values
    /// 2. Configuration file (config.json)
    /// 3. Environment-specific files (config.{env}.json)
    /// 4. Environment variables with SERVER_ prefix
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if configuration cannot be loaded or is invalid.
    pub fn load() -> Result<Self, ConfigError> {
        let env_var = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let mut config_builder = Config::builder()
            // Start with default values
            .set_default("host", "127.0.0.1")?
            .set_default("port", 3000)?
            .set_default("timeout_seconds", 30)?
            .set_default("environment", "development")?
            // Add optional configuration files
            .add_source(File::with_name("config.json").required(false))
            // Add environment-specific config file
            .add_source(
                File::with_name(&format!("config.{}.json", env_var.to_lowercase())).required(false),
            )
            // Add environment variables with SERVER_ prefix
            .add_source(
                ConfigEnv::with_prefix("SERVER")
                    .separator("_")
                    .try_parsing(true),
            );

        if std::env::var("ENVIRONMENT").is_ok() {
            config_builder = config_builder.set_override("environment", env_var.to_lowercase())?;
        }

        let config = config_builder.build()?;
        let mut server_config: Self = config.try_deserialize()?;

        // Fix the ServerPort to have the correct environment context
        server_config.port = ServerPort::new(server_config.port.value(), server_config.environment)
            .map_err(|e| ConfigError::Message(format!("invalid port configuration: {e}")))?;

        Ok(server_config)
    }

    /// Create configuration optimized for testing
    pub fn for_testing() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: ServerPort::testing(), // let OS choose available port
            timeout_seconds: TimeoutSeconds::testing(),
            environment: Environment::Testing,
            extensions: HashMap::new(),
        }
    }

    /// Get socket address for binding
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port.value())
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Production => write!(f, "production"),
            Environment::Development => write!(f, "development"),
            Environment::Testing => write!(f, "testing"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_validation() {
        // Invalid timeout values should fail to construct
        assert!(TimeoutSeconds::new(0).is_err());
        assert!(TimeoutSeconds::new(400).is_err());

        // Valid timeout values should construct successfully
        assert!(TimeoutSeconds::new(30).is_ok());
        assert!(TimeoutSeconds::new(1).is_ok());
        assert!(TimeoutSeconds::new(300).is_ok());
    }

    #[test]
    fn server_port_validation() {
        // Port 0 should only be valid in testing environment
        assert!(ServerPort::new(0, Environment::Testing).is_ok());
        assert!(ServerPort::new(0, Environment::Development).is_err());
        assert!(ServerPort::new(0, Environment::Production).is_err());

        // Non-zero ports should be valid in all environments
        assert!(ServerPort::new(3000, Environment::Development).is_ok());
        assert!(ServerPort::new(443, Environment::Production).is_ok());
    }

    #[test]
    fn environment_display() {
        assert_eq!(Environment::Production.to_string(), "production");
        assert_eq!(Environment::Development.to_string(), "development");
        assert_eq!(Environment::Testing.to_string(), "testing");
    }
}
