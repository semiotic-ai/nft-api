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
use serde_with::{DisplayFromStr, serde_as};
use shared_types::ChainId;
use tracing::warn;
use url::Url;
use utoipa::ToSchema;

use crate::error::{ServerError, ServerResult};

// Configuration constants
const DEFAULT_SERVER_PORT: u16 = 3000;
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const TESTING_TIMEOUT_SECONDS: u64 = 5;
const MAX_TIMEOUT_SECONDS: u64 = 300;
const DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE: u32 = 60;
const DEFAULT_METRICS_ENDPOINT_PATH: &str = "/metrics";
const DEFAULT_METRICS_PORT: u16 = 9102;
const DEFAULT_MAX_RETRIES: u32 = 3;

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

    /// Get the port number
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Create a safe default port for development
    pub const fn default_development() -> Self {
        Self {
            port: DEFAULT_SERVER_PORT,
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
    /// Returns an error if timeout is 0 or greater than 300 seconds (5 minutes).
    /// This limit prevents excessively long-running requests that could exhaust
    /// server resources, while still allowing sufficient time for blockchain API calls.
    pub fn new(seconds: u64) -> Result<Self> {
        ensure!(seconds != 0, "timeout must be greater than 0");
        ensure!(
            seconds <= MAX_TIMEOUT_SECONDS,
            "timeout cannot exceed {} seconds for production safety",
            MAX_TIMEOUT_SECONDS
        );
        Ok(Self(Duration::from_secs(seconds)))
    }

    /// Create a safe default timeout (30 seconds)
    pub const fn default_value() -> Self {
        Self(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
    }

    /// Create a safe testing timeout (5 seconds)
    pub const fn testing() -> Self {
        Self(Duration::from_secs(TESTING_TIMEOUT_SECONDS))
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

/// A validated API key
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiKey(String);

impl ApiKey {
    /// Create a new `ApiKey`, ensuring it's not empty
    ///
    /// # Errors
    ///
    /// Returns an error if the key is empty
    pub fn new(key: String) -> Result<Self> {
        ensure!(!key.trim().is_empty(), "API key cannot be empty");
        Ok(Self(key))
    }

    /// Create a placeholder API key for testing
    pub fn testing() -> Self {
        Self("test-api-key".to_string())
    }

    /// Get the API key value
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ApiKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let key = String::deserialize(deserializer)?;
        Self::new(key).map_err(|e| de::Error::custom(e.to_string()))
    }
}

/// External API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalApiConfig {
    /// Moralis API configuration
    pub moralis: MoralisConfig,
    /// Pinax API configuration
    pub pinax: PinaxConfig,
}

/// Moralis API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralisConfig {
    /// Base URL for Moralis API
    pub base_url: Url,
    /// API key for authentication
    pub api_key: ApiKey,
    /// Request timeout in seconds
    pub timeout_seconds: TimeoutSeconds,
    /// Health check timeout in seconds
    pub health_check_timeout_seconds: TimeoutSeconds,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Enable/disable the Moralis client
    pub enabled: bool,
}

impl Default for MoralisConfig {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://deep-index.moralis.io/api/v2")
                .expect("valid default Moralis URL"),
            api_key: ApiKey::testing(), // Will be overridden in production
            timeout_seconds: TimeoutSeconds::default(),
            health_check_timeout_seconds: TimeoutSeconds::new(DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS)
                .expect("default health check timeout is valid"),
            max_retries: DEFAULT_MAX_RETRIES,
            enabled: false,
        }
    }
}

/// Pinax API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinaxConfig {
    /// Base URL for Pinax API endpoint
    pub endpoint: Url,
    /// Username for Pinax API authentication
    pub api_user: ApiKey,
    /// Password/token for Pinax API authentication
    pub api_auth: ApiKey,
    /// Database name in the Pinax environment
    pub db_name: String,
    /// Request timeout in seconds
    pub timeout_seconds: TimeoutSeconds,
    /// Health check timeout in seconds
    pub health_check_timeout_seconds: TimeoutSeconds,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Enable/disable the Pinax client
    pub enabled: bool,
}

impl Default for PinaxConfig {
    fn default() -> Self {
        Self {
            endpoint: Url::parse("https://api.pinax.network/sql").expect("valid default Pinax URL"),
            api_user: ApiKey::testing(),
            api_auth: ApiKey::testing(),
            db_name: "mainnet:evm-nft-tokens@v0.6.2".to_string(),
            timeout_seconds: TimeoutSeconds::default(),
            health_check_timeout_seconds: TimeoutSeconds::new(DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS)
                .expect("default health check timeout is valid"),
            max_retries: DEFAULT_MAX_RETRIES,
            enabled: false,
        }
    }
}

/// Chain-specific configuration for multi-chain support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Enable/disable this chain
    pub enabled: bool,
    /// Chain-specific Moralis configuration (optional override)
    pub moralis: Option<ChainMoralisConfig>,
    /// Chain-specific Pinax configuration (optional override)
    pub pinax: Option<ChainPinaxConfig>,
}

/// Chain-specific Moralis API configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainMoralisConfig {
    /// Chain-specific base URL (optional)
    pub base_url: Option<Url>,
    /// Chain-specific timeout (optional)
    pub timeout_seconds: Option<TimeoutSeconds>,
    /// Chain-specific max retries (optional)
    pub max_retries: Option<u32>,
}

/// Chain-specific Pinax API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainPinaxConfig {
    /// Chain-specific database name (required for each chain)
    pub db_name: String,
    /// Chain-specific timeout (optional)
    pub timeout_seconds: Option<TimeoutSeconds>,
    /// Chain-specific max retries (optional)
    pub max_retries: Option<u32>,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            moralis: None,
            pinax: None,
        }
    }
}

impl Default for ChainPinaxConfig {
    fn default() -> Self {
        Self {
            db_name: "mainnet:evm-nft-tokens@v0.6.2".to_string(),
            timeout_seconds: None,
            max_retries: None,
        }
    }
}

/// Spam predictor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamPredictorConfig {
    /// `OpenAI` API key for GPT model access
    pub openai_api_key: ApiKey,
    /// `OpenAI` API base URL (optional, defaults to official API)
    pub openai_base_url: Option<Url>,
    /// `OpenAI` organization ID (optional)
    pub openai_organization_id: Option<String>,
    /// Path to model registry YAML file
    pub model_registry_path: String,
    /// Path to prompt registry JSON file
    pub prompt_registry_path: String,
    /// Request timeout in seconds
    pub timeout_seconds: TimeoutSeconds,
    /// Maximum tokens for model responses
    pub max_tokens: Option<u32>,
    /// Temperature for model responses (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Cache TTL for predictions in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum number of cached predictions
    pub max_cache_size: usize,
}

impl Default for SpamPredictorConfig {
    fn default() -> Self {
        Self {
            openai_api_key: ApiKey::testing(),
            openai_base_url: None,
            openai_organization_id: None,
            model_registry_path: "assets/configs/models.yaml".to_string(),
            prompt_registry_path: "assets/prompts/ft_prompt.json".to_string(),
            timeout_seconds: TimeoutSeconds::default(),
            max_tokens: Some(10),
            temperature: Some(0.0),
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_size: 10000,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable/disable rate limiting
    pub enabled: bool,
    /// Maximum requests per minute per IP address
    pub requests_per_minute: u32,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE,
        }
    }
}

/// Environment types for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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
///
/// ## Required Environment Variables for Production
///
/// To enable external APIs in production, set these environment variables:
/// - `SERVER__EXTERNAL_APIS__MORALIS__API_KEY`: Your Moralis API key
/// - `SERVER__EXTERNAL_APIS__MORALIS__ENABLED`: Set to "true" to enable
/// - `SERVER__EXTERNAL_APIS__MORALIS__HEALTH_CHECK_TIMEOUT_SECONDS`: Health check timeout (default: 5)
/// - `SERVER__EXTERNAL_APIS__PINAX__API_USER`: Your Pinax username
/// - `SERVER__EXTERNAL_APIS__PINAX__API_AUTH`: Your Pinax auth token
/// - `SERVER__EXTERNAL_APIS__PINAX__ENABLED`: Set to "true" to enable
/// - `SERVER__EXTERNAL_APIS__PINAX__HEALTH_CHECK_TIMEOUT_SECONDS`: Health check timeout (default: 5)
/// - `SERVER__SPAM_PREDICTOR__OPENAI_API_KEY`: Your `OpenAI` API key for spam prediction (required)
/// - `SERVER__RATE_LIMITING__ENABLED`: Enable rate limiting (default: true)
/// - `SERVER__RATE_LIMITING__REQUESTS_PER_MINUTE`: Requests per minute limit (default: 60)
///
/// External APIs with placeholder credentials are automatically disabled by default for security.
/// Spam predictor is always required and must have valid configuration.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: IpAddr,
    /// Server port (validated for environment compatibility)
    pub port: ServerPort,
    /// Request timeout in seconds (validated range: 1-60)
    pub timeout_seconds: TimeoutSeconds,
    /// Environment type
    pub environment: Environment,
    /// External API configurations
    pub external_apis: ExternalApiConfig,
    /// Spam predictor configuration
    pub spam_predictor: SpamPredictorConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    /// Prometheus metrics configuration
    pub metrics: MetricsConfig,
    /// Chain-specific configurations
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub chains: HashMap<ChainId, ChainConfig>,
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
            external_apis: ExternalApiConfig::default(),
            spam_predictor: SpamPredictorConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            metrics: MetricsConfig::default(),
            chains: Self::default_chains(),
            extensions: HashMap::new(),
        }
    }
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// HTTP path for metrics exposition (e.g. "/metrics")
    pub endpoint_path: String,
    /// Port for the metrics HTTP server (default 9102)
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            endpoint_path: DEFAULT_METRICS_ENDPOINT_PATH.to_string(),
            port: DEFAULT_METRICS_PORT,
        }
    }
}

impl ServerConfig {
    /// Create default chain configurations based on the test config mappings
    fn default_chains() -> HashMap<ChainId, ChainConfig> {
        let mut chains = HashMap::new();

        // Add default configurations for all supported chains
        // Based on the test config from assets/configs/test_multi_chain.yaml

        // Ethereum (mainnet)
        chains.insert(
            ChainId::Ethereum,
            ChainConfig {
                enabled: true,
                moralis: None, // Use global config
                pinax: Some(ChainPinaxConfig {
                    db_name: "mainnet:evm-nft-tokens@v0.6.2".to_string(),
                    timeout_seconds: None,
                    max_retries: None,
                }),
            },
        );

        // Polygon (matic)
        chains.insert(
            ChainId::Polygon,
            ChainConfig {
                enabled: true,
                moralis: None, // Use global config
                pinax: Some(ChainPinaxConfig {
                    db_name: "matic:evm-nft-tokens@v0.5.1".to_string(),
                    timeout_seconds: None,
                    max_retries: None,
                }),
            },
        );

        // Base
        chains.insert(
            ChainId::Base,
            ChainConfig {
                enabled: false, // Partial implementation
                moralis: None,
                pinax: Some(ChainPinaxConfig {
                    db_name: "base:evm-nft-tokens@v0.5.1".to_string(),
                    timeout_seconds: None,
                    max_retries: None,
                }),
            },
        );

        // Avalanche
        chains.insert(
            ChainId::Avalanche,
            ChainConfig {
                enabled: false, // Planned implementation
                moralis: None,
                pinax: Some(ChainPinaxConfig {
                    db_name: "avalanche:evm-nft-tokens@v0.5.1".to_string(),
                    timeout_seconds: None,
                    max_retries: None,
                }),
            },
        );

        // Arbitrum
        chains.insert(
            ChainId::Arbitrum,
            ChainConfig {
                enabled: false, // Planned implementation
                moralis: None,
                pinax: Some(ChainPinaxConfig {
                    db_name: "arbitrum-one:evm-nft-tokens@v0.5.1".to_string(),
                    timeout_seconds: None,
                    max_retries: None,
                }),
            },
        );

        chains
    }

    /// Create configuration from environment variables and optional configuration files
    ///
    /// # Errors
    ///
    /// Returns `ServerError::Config` if configuration is invalid or cannot be loaded.
    pub fn from_env() -> ServerResult<Self> {
        let config = Self::load().map_err(|e| ServerError::Config {
            message: format!("failed to load configuration: {e}"),
        })?;

        config.validate().map_err(|e| ServerError::Config {
            message: format!("configuration validation failed: {e}"),
        })?;

        Ok(config)
    }

    /// Validate the configuration for production readiness
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration contains placeholder values or is invalid for production use.
    pub fn validate(&self) -> Result<()> {
        self.validate_basic_configuration()?;
        self.validate_api_credentials()?;
        self.validate_production_safety()?;
        Ok(())
    }

    /// Validate basic configuration parameters
    fn validate_basic_configuration(&self) -> Result<()> {
        // Port validation is handled by the u16 type - no need to check upper bound
        let port = self.port.port();
        ensure!(
            port > 0,
            "Port {} is invalid - must be greater than 0",
            port
        );

        // Validate rate limiting bounds
        if self.rate_limiting.enabled {
            ensure!(
                self.rate_limiting.requests_per_minute > 0,
                "Rate limiting requests_per_minute must be greater than 0 when enabled"
            );
            ensure!(
                self.rate_limiting.requests_per_minute <= 10_000,
                "Rate limiting requests_per_minute of {} is too high - maximum is 10,000",
                self.rate_limiting.requests_per_minute
            );
        }

        // Validate timeout values
        ensure!(
            self.timeout_seconds.0.as_secs() >= 1,
            "Timeout must be at least 1 second"
        );

        // Validate external API URLs are properly formatted
        if self.external_apis.moralis.enabled {
            let base_url = self.external_apis.moralis.base_url.as_str();
            ensure!(
                base_url.starts_with("http://") || base_url.starts_with("https://"),
                "Moralis base_url must be a valid HTTP(S) URL"
            );
        }

        if self.external_apis.pinax.enabled {
            let endpoint = self.external_apis.pinax.endpoint.as_str();
            ensure!(
                endpoint.starts_with("http://") || endpoint.starts_with("https://"),
                "Pinax endpoint must be a valid HTTP(S) URL"
            );
        }

        // Validate chain configurations
        self.validate_chain_configurations()?;

        Ok(())
    }

    /// Validate chain-specific configurations
    fn validate_chain_configurations(&self) -> Result<()> {
        // Ensure at least one chain is enabled
        let enabled_chains: Vec<_> = self
            .chains
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect();

        if enabled_chains.is_empty() {
            return Err(anyhow!(
                "At least one chain must be enabled. Current enabled chains: none"
            ));
        }

        // Validate each enabled chain's configuration
        for (chain_id, chain_config) in &self.chains {
            if !chain_config.enabled {
                continue;
            }

            // warn chain implementation status
            if matches!(
                chain_id.implementation_status(),
                shared_types::ChainImplementationStatus::Planned
                    | shared_types::ChainImplementationStatus::Partial
            ) {
                warn!(
                    "Chain {} is enabled but has '{}' implementation status. \
                         This may result in limited functionality.",
                    chain_id.name(),
                    chain_id.implementation_status()
                );
            }

            // Validate Pinax configuration for enabled chains
            if let Some(pinax_config) = &chain_config.pinax
                && pinax_config.db_name.trim().is_empty()
            {
                return Err(anyhow!(
                    "Chain {} has enabled Pinax configuration but empty database name",
                    chain_id.name()
                ));
            }

            // Validate Moralis configuration overrides if present
            if let Some(moralis_config) = &chain_config.moralis
                && let Some(base_url) = &moralis_config.base_url
            {
                let url = base_url.as_str();
                ensure!(
                    url.starts_with("http://") || url.starts_with("https://"),
                    "Chain {} Moralis base_url must be a valid HTTP(S) URL: {}",
                    chain_id.name(),
                    url
                );
            }
        }

        Ok(())
    }

    /// Validate API credentials are not placeholders
    fn validate_api_credentials(&self) -> Result<()> {
        // Validate Moralis configuration if enabled
        if self.external_apis.moralis.enabled {
            let api_key = self.external_apis.moralis.api_key.value();
            if api_key == "test-api-key" || api_key.starts_with("REPLACE_WITH_") {
                return Err(anyhow!(
                    "Moralis API is enabled but still has placeholder API key. Set SERVER_EXTERNAL_APIS_MORALIS_API_KEY or update config file."
                ));
            }
        }

        // Validate Pinax configuration if enabled
        if self.external_apis.pinax.enabled {
            let api_user = self.external_apis.pinax.api_user.value();
            let api_auth = self.external_apis.pinax.api_auth.value();

            if api_user == "test-api-key"
                || api_user == "test-user"
                || api_user.starts_with("REPLACE_WITH_")
            {
                return Err(anyhow!(
                    "Pinax API is enabled but still has placeholder API user. Set SERVER_EXTERNAL_APIS_PINAX_API_USER or update config file."
                ));
            }

            if api_auth == "test-api-key"
                || api_auth == "test-auth"
                || api_auth.starts_with("REPLACE_WITH_")
            {
                return Err(anyhow!(
                    "Pinax API is enabled but still has placeholder API auth. Set SERVER_EXTERNAL_APIS_PINAX_API_AUTH or update config file."
                ));
            }
        }

        // Validate Spam Predictor configuration (always required)
        {
            let api_key = self.spam_predictor.openai_api_key.value();

            if api_key == "test-openai-key"
                || api_key == "test-api-key"
                || api_key.starts_with("REPLACE_WITH_")
            {
                return Err(anyhow!(
                    "Spam Predictor has placeholder OpenAI API key. Set SERVER_SPAM_PREDICTOR_OPENAI_API_KEY or update config file."
                ));
            }

            // Basic validation for OpenAI API key format
            if !api_key.starts_with("sk-") && !api_key.starts_with("test-") {
                warn!("OpenAI API key doesn't match expected format (should start with 'sk-')");
            }

            // Validate temperature range
            if let Some(temperature) = self.spam_predictor.temperature
                && !(0.0..=2.0).contains(&temperature)
            {
                return Err(anyhow!(
                    "Spam Predictor temperature {} is invalid (must be 0.0-2.0)",
                    temperature
                ));
            }

            // Validate max_tokens
            if let Some(max_tokens) = self.spam_predictor.max_tokens
                && (max_tokens == 0 || max_tokens > 4096)
            {
                return Err(anyhow!(
                    "Spam Predictor max_tokens {} is invalid (must be 1-4096)",
                    max_tokens
                ));
            }

            // Validate cache settings
            if self.spam_predictor.cache_ttl_seconds == 0 {
                return Err(anyhow!("Spam Predictor cache TTL cannot be 0"));
            }

            if self.spam_predictor.max_cache_size == 0 {
                return Err(anyhow!("Spam Predictor max cache size cannot be 0"));
            }

            // Validate that configuration files exist
            if !std::path::Path::new(&self.spam_predictor.model_registry_path).exists() {
                return Err(anyhow!(
                    "Spam Predictor model registry file not found: {} (current directory: {})",
                    self.spam_predictor.model_registry_path,
                    std::env::current_dir()
                        .map_or_else(|_| "unknown".to_string(), |p| p.display().to_string())
                ));
            }

            if !std::path::Path::new(&self.spam_predictor.prompt_registry_path).exists() {
                return Err(anyhow!(
                    "Spam Predictor prompt registry file not found: {} (current directory: {})",
                    self.spam_predictor.prompt_registry_path,
                    std::env::current_dir()
                        .map_or_else(|_| "unknown".to_string(), |p| p.display().to_string())
                ));
            }
        }

        Ok(())
    }

    /// Validate production deployment safety
    fn validate_production_safety(&self) -> Result<()> {
        // In production environment, ensure rate limiting is enabled
        if self.environment == Environment::Production {
            if !self.rate_limiting.enabled {
                return Err(anyhow!(
                    "Rate limiting must be enabled in production environment for security"
                ));
            }

            if self.rate_limiting.requests_per_minute == 0 {
                return Err(anyhow!(
                    "Rate limiting requests per minute cannot be 0 in production"
                ));
            }

            // Warn about binding to all interfaces in production (but allow for container deployments)
            if self.host.is_unspecified() {
                warn!(
                    "Binding to all interfaces (0.0.0.0 or ::) in production. Ensure proper firewall/proxy configuration is in place for security."
                );
            }

            // Warn about development-specific settings
            if let Some(jwt_secret) = self.extensions.get("jwt_secret")
                && jwt_secret == "dev-secret-key-not-for-production"
            {
                return Err(anyhow!(
                    "Production environment detected with development JWT secret. Set a secure JWT secret via SERVER_EXTENSIONS_JWT_SECRET"
                ));
            }
        }

        Ok(())
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
            .set_default("port", DEFAULT_SERVER_PORT)?
            .set_default("timeout_seconds", DEFAULT_TIMEOUT_SECONDS)?
            .set_default("environment", "development")?
            // External API defaults
            .set_default(
                "external_apis.moralis.base_url",
                "https://deep-index.moralis.io/api/v2",
            )?
            .set_default("external_apis.moralis.api_key", "test-api-key")?
            .set_default(
                "external_apis.moralis.timeout_seconds",
                DEFAULT_TIMEOUT_SECONDS,
            )?
            .set_default(
                "external_apis.moralis.health_check_timeout_seconds",
                DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS,
            )?
            .set_default("external_apis.moralis.max_retries", DEFAULT_MAX_RETRIES)?
            .set_default("external_apis.moralis.enabled", false)?
            // Pinax API defaults
            .set_default(
                "external_apis.pinax.endpoint",
                "https://api.pinax.network/sql",
            )?
            .set_default("external_apis.pinax.api_user", "test-user")?
            .set_default("external_apis.pinax.api_auth", "test-auth")?
            .set_default(
                "external_apis.pinax.db_name",
                "mainnet:evm-nft-tokens@v0.6.2",
            )?
            .set_default(
                "external_apis.pinax.timeout_seconds",
                DEFAULT_TIMEOUT_SECONDS,
            )?
            .set_default(
                "external_apis.pinax.health_check_timeout_seconds",
                DEFAULT_HEALTH_CHECK_TIMEOUT_SECONDS,
            )?
            .set_default("external_apis.pinax.max_retries", DEFAULT_MAX_RETRIES)?
            .set_default("external_apis.pinax.enabled", false)?
            // Spam predictor defaults
            .set_default("spam_predictor.openai_api_key", "test-openai-key")?
            .set_default("spam_predictor.openai_base_url", None::<String>)?
            .set_default("spam_predictor.openai_organization_id", None::<String>)?
            .set_default(
                "spam_predictor.model_registry_path",
                "assets/configs/models.yaml",
            )?
            .set_default(
                "spam_predictor.prompt_registry_path",
                "assets/prompts/ft_prompt.json",
            )?
            .set_default("spam_predictor.timeout_seconds", DEFAULT_TIMEOUT_SECONDS)?
            .set_default("spam_predictor.max_tokens", 10u32)?
            .set_default("spam_predictor.temperature", 0.0f64)?
            .set_default("spam_predictor.cache_ttl_seconds", 3600i64)?
            .set_default("spam_predictor.max_cache_size", 10000i64)?
            // Rate limiting defaults
            .set_default("rate_limiting.enabled", true)?
            .set_default(
                "rate_limiting.requests_per_minute",
                DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE,
            )?
            // Metrics defaults
            .set_default("metrics.endpoint_path", DEFAULT_METRICS_ENDPOINT_PATH)?
            .set_default("metrics.port", i64::from(DEFAULT_METRICS_PORT))?
            // Add optional configuration files
            .add_source(File::with_name("config.json").required(false))
            // Add environment-specific config file
            .add_source(
                File::with_name(&format!("config.{}.json", env_var.to_lowercase())).required(false),
            )
            // Add environment variables with SERVER__ prefix
            .add_source(
                ConfigEnv::with_prefix("SERVER")
                    .separator("__")
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
    ///
    /// # Panics
    ///
    /// Panics if the test API key "sk-test-valid-key" is invalid, which should never happen
    /// in normal circumstances.
    pub fn for_testing() -> Self {
        let spam_predictor_config = SpamPredictorConfig {
            model_registry_path: "../../assets/configs/models.yaml".to_string(),
            prompt_registry_path: "../../assets/prompts/ft_prompt.json".to_string(),
            openai_api_key: ApiKey::new("sk-test-valid-key".to_string())
                .expect("test api key should be valid"),
            ..Default::default()
        };

        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: ServerPort::testing(), // let OS choose available port
            timeout_seconds: TimeoutSeconds::testing(),
            environment: Environment::Testing,
            external_apis: ExternalApiConfig::default(),
            spam_predictor: spam_predictor_config,
            rate_limiting: RateLimitingConfig {
                enabled: false,
                requests_per_minute: 0,
            },
            metrics: MetricsConfig::default(),
            chains: Self::default_chains(),
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
        assert!(TimeoutSeconds::new(DEFAULT_TIMEOUT_SECONDS).is_ok());
        assert!(TimeoutSeconds::new(1).is_ok());
        assert!(TimeoutSeconds::new(MAX_TIMEOUT_SECONDS).is_ok());
    }

    #[test]
    fn server_port_validation() {
        // Port 0 should only be valid in testing environment
        assert!(ServerPort::new(0, Environment::Testing).is_ok());
        assert!(ServerPort::new(0, Environment::Development).is_err());
        assert!(ServerPort::new(0, Environment::Production).is_err());

        // Non-zero ports should be valid in all environments
        assert!(ServerPort::new(DEFAULT_SERVER_PORT, Environment::Development).is_ok());
        assert!(ServerPort::new(443, Environment::Production).is_ok());
    }

    #[test]
    fn environment_display() {
        assert_eq!(Environment::Production.to_string(), "production");
        assert_eq!(Environment::Development.to_string(), "development");
        assert_eq!(Environment::Testing.to_string(), "testing");
    }

    #[test]
    fn api_key_validation() {
        // Valid API keys should construct successfully
        assert!(ApiKey::new("valid-api-key-123".to_string()).is_ok());
        assert!(ApiKey::new("abc123".to_string()).is_ok());

        // Invalid API keys should fail
        assert!(ApiKey::new(String::new()).is_err());
        assert!(ApiKey::new("   ".to_string()).is_err()); // whitespace only
    }

    #[test]
    fn validate_placeholder_api_keys() {
        let mut config = ServerConfig::default();

        // Disable external APIs
        config.external_apis.moralis.enabled = false;
        config.external_apis.pinax.enabled = false;

        // Set valid spam predictor API key and valid file paths (spam predictor is always required)
        config.spam_predictor.openai_api_key =
            ApiKey::new("sk-test-valid-key".to_string()).expect("test key should be valid");
        // Use paths relative to the workspace root for testing
        config.spam_predictor.model_registry_path = "../../assets/configs/models.yaml".to_string();
        config.spam_predictor.prompt_registry_path =
            "../../assets/prompts/ft_prompt.json".to_string();

        // Should be valid when external APIs are disabled but spam predictor has valid key
        assert!(config.validate().is_ok());

        // Should fail when Moralis is enabled with placeholder key
        config.external_apis.moralis.enabled = true;
        let validation_result = config.validate();
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("placeholder API key")
        );

        // Should fail when Pinax is enabled with placeholder credentials
        config.external_apis.moralis.enabled = false;
        config.external_apis.pinax.enabled = true;
        let validation_result = config.validate();
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("placeholder API")
        );
    }

    #[test]
    fn validate_production_safety() {
        let mut config = ServerConfig {
            environment: Environment::Production,
            ..Default::default()
        };

        // Set valid spam predictor API key and file paths first (spam predictor is always required)
        config.spam_predictor.openai_api_key =
            ApiKey::new("sk-test-valid-key".to_string()).expect("test key should be valid");
        config.spam_predictor.model_registry_path = "../../assets/configs/models.yaml".to_string();
        config.spam_predictor.prompt_registry_path =
            "../../assets/prompts/ft_prompt.json".to_string();

        // Production should require rate limiting
        config.rate_limiting.enabled = false;
        let validation_result = config.validate();
        assert!(validation_result.is_err());
        assert!(
            validation_result
                .unwrap_err()
                .to_string()
                .contains("Rate limiting must be enabled in production")
        );

        // Fix rate limiting
        config.rate_limiting.enabled = true;
        config.rate_limiting.requests_per_minute = DEFAULT_RATE_LIMIT_REQUESTS_PER_MINUTE;

        // Production allows binding to all interfaces (with warning) for container deployments
        config.host = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let validation_result = config.validate();
        assert!(
            validation_result.is_ok(),
            "Production should allow binding to 0.0.0.0 with warning"
        );

        // Also works with specific host binding
        assert!(config.validate().is_ok());
    }

    // Note: Environment variable support is provided via the config crate
    // Environment variables can override configuration using the SERVER_ prefix:
    // - SERVER_EXTERNAL_APIS_MORALIS_API_KEY
    // - SERVER_EXTERNAL_APIS_MORALIS_ENABLED
    // - SERVER_EXTERNAL_APIS_PINAX_API_USER
    // - SERVER_EXTERNAL_APIS_PINAX_API_AUTH
    // etc.
}
