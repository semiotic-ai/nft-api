// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Main spam prediction orchestrator
//!
//! This module provides the core `SpamPredictor` struct that orchestrates
//! all components to provide high-level spam prediction functionality with
//! caching, error handling, and observability.

use std::{sync::Arc, time::Instant};

use api_client::{ContractMetadata, SpamAnalysis};
use chrono::Utc;
use serde_json;
use tracing::{debug, info, instrument, warn};

use crate::{
    cache::PredictionCacheKey,
    config::SpamPredictorConfig,
    error::{SpamPredictorError, SpamPredictorResult},
    openai::OpenAiClient,
    types::{ModelSpec, ModelType, ModelVersion, SpamPredictionRequest, SpamPredictionResult},
};

/// Main spam prediction orchestrator
///
/// The `SpamPredictor` coordinates between configuration management, caching,
/// and OpenAI API calls to provide high-performance spam prediction for NFT contracts.
#[derive(Debug, Clone)]
pub struct SpamPredictor {
    /// Configuration for models, prompts, and API settings
    config: Arc<SpamPredictorConfig>,
    /// OpenAI API client
    openai_client: Arc<OpenAiClient>,
}

impl SpamPredictor {
    /// Create a new spam predictor with the given configuration
    #[instrument(skip(config))]
    pub async fn new(config: SpamPredictorConfig) -> SpamPredictorResult<Self> {
        info!("Initializing SpamPredictor");

        // Create OpenAI client
        let openai_client = Arc::new(
            OpenAiClient::new(
                config.openai_config.api_key.clone(),
                config.openai_config.base_url.clone(),
                config.openai_config.timeout_seconds,
                config.openai_config.organization_id.clone(),
            )?
            .with_max_tokens(config.openai_config.max_tokens.unwrap_or(10))
            .with_temperature(config.openai_config.temperature.unwrap_or(0.0)),
        );

        // Test OpenAI connection
        match openai_client.health_check().await {
            Ok(true) => info!("OpenAI API connection verified"),
            Ok(false) => warn!("OpenAI API health check failed, but proceeding"),
            Err(e) => warn!("OpenAI API health check error: {}, but proceeding", e),
        }

        let predictor = Self {
            config: Arc::new(config),
            openai_client,
        };

        // Log configuration summary
        let summary = predictor.config.get_summary();
        info!(
            "SpamPredictor initialized with {} model types and {} prompt versions",
            summary.model_types.len(),
            summary.prompt_versions.len()
        );

        Ok(predictor)
    }

    /// Create a comprehensive spam analysis
    #[instrument(skip(self, request), fields(
        contract_address = %request.metadata().address,
        model_spec = %request.model_spec()
    ))]
    pub async fn analyze_contract(
        &self,
        request: SpamPredictionRequest,
    ) -> SpamPredictorResult<SpamAnalysis> {
        let start_time = Instant::now();

        let result = self.predict_spam_typed(request.clone()).await?;
        let metadata = request.metadata();
        let is_spam = result.classification().is_spam();

        let mut reasons = Vec::new();

        // Add basic analysis reasons based on metadata
        if let Some(ref name) = metadata.name {
            if name.is_empty() {
                reasons.push("Empty contract name".to_string());
            } else if name.len() > 100 {
                reasons.push("Unusually long contract name".to_string());
            }
        } else {
            reasons.push("No contract name available".to_string());
        }

        if let Some(ref symbol) = metadata.symbol {
            if symbol.is_empty() {
                reasons.push("Empty contract symbol".to_string());
            }
        } else {
            reasons.push("No contract symbol available".to_string());
        }

        // Add AI model reasoning with typed information
        let model_spec = request.model_spec();
        if is_spam {
            reasons.push(format!("AI model {} classified as spam", model_spec));
        } else {
            reasons.push(format!("AI model {} classified as legitimate", model_spec));
        }

        let duration = start_time.elapsed();
        debug!(
            "Completed typed contract analysis for {} in {:?}",
            metadata.address, duration
        );

        Ok(SpamAnalysis {
            is_spam,
            reasons,
            source: format!("spam-predictor-typed ({})", model_spec),
            analyzed_at: Utc::now(),
        })
    }

    /// Prepare contract metadata for AI model input
    fn prepare_contract_data(&self, metadata: &ContractMetadata) -> SpamPredictorResult<String> {
        // Create a structured representation of the contract data
        let contract_data = serde_json::json!({
            "address": format!("{:?}", metadata.address),
            "name": metadata.name,
            "symbol": metadata.symbol,
            "contract_type": metadata.contract_type,
            "is_verified": metadata.is_verified,
            "total_supply": metadata.total_supply,
            "holder_count": metadata.holder_count,
            "transaction_count": metadata.transaction_count,
            "creation_block": metadata.creation_block,
            "additional_data": metadata.additional_data
        });

        serde_json::to_string_pretty(&contract_data).map_err(|e| {
            SpamPredictorError::json(format!("Failed to serialize contract data: {}", e))
        })
    }

    /// Get the current configuration summary
    pub fn get_config_summary(&self) -> crate::config::ConfigSummary {
        self.config.get_summary()
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> crate::cache::CacheStats {
        self.config.cache.get_stats()
    }

    /// Reload configuration from files (hot reload)
    pub async fn reload_config(&self) -> SpamPredictorResult<()> {
        info!("Reloading spam predictor configuration");

        // Note: We can't modify the Arc<SpamPredictorConfig>, so this would need
        // to be implemented at a higher level (e.g., recreating the SpamPredictor)
        warn!("Hot reload not implemented - requires SpamPredictor recreation");

        Ok(())
    }

    /// Perform health check on all components
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> SpamPredictorResult<PredictorHealthStatus> {
        debug!("Performing comprehensive health check");

        let start_time = Instant::now();

        // Check OpenAI API
        let openai_healthy = match self.openai_client.health_check().await {
            Ok(healthy) => healthy,
            Err(e) => {
                warn!("OpenAI health check failed: {}", e);
                false
            }
        };

        // Check configuration availability
        let config_healthy = self.check_config_health().await;

        // Check cache functionality
        let cache_healthy = self.check_cache_health().await;

        let duration = start_time.elapsed();
        let overall_healthy = openai_healthy && config_healthy && cache_healthy;

        let status = PredictorHealthStatus {
            overall_healthy,
            openai_healthy,
            config_healthy,
            cache_healthy,
            check_duration_ms: duration.as_millis() as u64,
            cache_stats: self.get_cache_stats(),
        };

        if overall_healthy {
            debug!("Health check passed in {:?}", duration);
        } else {
            warn!("Health check failed in {:?}: {:?}", duration, status);
        }

        Ok(status)
    }

    /// Check configuration health
    async fn check_config_health(&self) -> bool {
        // Try to access a model and prompt
        let spam_classification_spec = match (
            ModelType::new("spam_classification".to_string()),
            ModelVersion::new("latest".to_string()),
        ) {
            (Ok(model_type), Ok(version)) => ModelSpec::new(model_type, version),
            (Err(e), _) => {
                warn!("Config health check failed - invalid model type: {}", e);
                return false;
            }
            (_, Err(e)) => {
                warn!("Config health check failed - invalid version: {}", e);
                return false;
            }
        };

        match (
            self.config.get_model(&spam_classification_spec),
            self.config
                .get_prompt(&self.config.prompt_registry.current_version),
        ) {
            (Ok(_), Ok(_)) => true,
            (Err(e), _) => {
                warn!("Config health check failed - model access: {}", e);
                false
            }
            (_, Err(e)) => {
                warn!("Config health check failed - prompt access: {}", e);
                false
            }
        }
    }

    /// Check cache functionality
    async fn check_cache_health(&self) -> bool {
        // Test cache read/write operations
        let test_key = "health_check";
        let test_value = "test_value".to_string();

        self.config.cache.store_prompt(test_key, test_value.clone());

        match self.config.cache.get_prompt(test_key) {
            Some(value) if value == test_value => {
                debug!("Cache health check passed");
                true
            }
            Some(value) => {
                warn!(
                    "Cache health check failed - value mismatch: expected '{}', got '{}'",
                    test_value, value
                );
                false
            }
            None => {
                warn!("Cache health check failed - could not retrieve test value");
                false
            }
        }
    }

    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) -> SpamPredictorResult<usize> {
        self.config.cache.cleanup_expired()
    }

    /// Get OpenAI client information
    pub fn get_openai_info(&self) -> crate::openai::ClientInfo {
        self.openai_client.get_info()
    }

    /// Type-safe spam prediction with comprehensive result
    #[instrument(skip(self, request), fields(
        contract_address = %request.metadata().address,
        model_spec = %request.model_spec(),
        prompt_version = %request.prompt_version()
    ))]
    pub async fn predict_spam_typed(
        &self,
        request: SpamPredictionRequest,
    ) -> SpamPredictorResult<SpamPredictionResult> {
        let start_time = Instant::now();

        debug!(
            "Starting type-safe spam prediction for contract {} using {} and prompt {}",
            request.metadata().address,
            request.model_spec(),
            request.prompt_version()
        );

        // Check cache first
        let cache_key = PredictionCacheKey::from_metadata(
            request.metadata(),
            request.model_spec().model_type().as_str(),
            request.model_spec().version().as_str(),
            &request.prompt_version().as_str(),
        );

        if let Some(cached_result) = self.config.cache.get_prediction(&cache_key) {
            debug!("Cache hit for prediction key: {:?}", cache_key);
            return Ok(SpamPredictionResult::new(
                match cached_result {
                    Some(true) => crate::types::SpamClassification::Spam,
                    Some(false) => crate::types::SpamClassification::Legitimate,
                    None => crate::types::SpamClassification::Inconclusive,
                },
                crate::types::ConfidenceScore::high(), // Assuming high confidence for cached results
                Some("Cached prediction result".to_string()),
                request.model_spec().clone(),
                start_time.elapsed(),
                true,
            ));
        }

        // Get model ID from configuration
        let model_id = match self.config.get_model(request.model_spec()) {
            Ok(id) => id,
            Err(e) => {
                warn!("Model lookup failed: {}", e);
                return Ok(SpamPredictionResult::error_fallback(
                    request.model_spec().clone(),
                    start_time.elapsed(),
                ));
            }
        };

        // Get prompt from configuration
        let prompt = match self.config.get_prompt(&request.prompt_version().as_str()) {
            Ok(p) => p,
            Err(e) => {
                warn!("Prompt lookup failed: {}", e);
                return Ok(SpamPredictionResult::error_fallback(
                    request.model_spec().clone(),
                    start_time.elapsed(),
                ));
            }
        };

        // Prepare contract data for analysis
        let contract_data = match self.prepare_contract_data(request.metadata()) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to prepare contract data: {}", e);
                return Ok(SpamPredictionResult::error_fallback(
                    request.model_spec().clone(),
                    start_time.elapsed(),
                ));
            }
        };

        // Make prediction via OpenAI
        let prediction_result: SpamPredictorResult<crate::openai::PredictionResult> = self
            .openai_client
            .predict_spam(&model_id, &prompt, &contract_data)
            .await;

        let result = match prediction_result {
            Ok(openai_result) => {
                match openai_result.is_spam {
                    Some(true) => {
                        // Cache positive result
                        self.config.cache.store_prediction(cache_key, Some(true));
                        SpamPredictionResult::spam(
                            request.model_spec().clone(),
                            start_time.elapsed(),
                        )
                    }
                    Some(false) => {
                        // Cache negative result
                        self.config.cache.store_prediction(cache_key, Some(false));
                        SpamPredictionResult::legitimate(
                            request.model_spec().clone(),
                            start_time.elapsed(),
                        )
                    }
                    None => {
                        // Cache inconclusive result
                        self.config.cache.store_prediction(cache_key, None);
                        SpamPredictionResult::inconclusive(
                            request.model_spec().clone(),
                            start_time.elapsed(),
                        )
                    }
                }
            }
            Err(e) => {
                warn!("OpenAI prediction failed: {}", e);
                SpamPredictionResult::error_fallback(
                    request.model_spec().clone(),
                    start_time.elapsed(),
                )
            }
        };

        info!(
            contract_address = %request.metadata().address,
            is_spam = result.is_spam(),
            confidence = result.confidence().as_f64(),
            cached = result.is_cached(),
            duration_ms = result.processing_time().as_millis(),
            "Type-safe spam prediction completed"
        );

        Ok(result)
    }

    /// Convenience method for spam classification with default settings
    pub async fn classify_spam(
        &self,
        metadata: &ContractMetadata,
    ) -> SpamPredictorResult<SpamPredictionResult> {
        let request = SpamPredictionRequest::spam_classification(metadata.clone());
        self.predict_spam_typed(request).await
    }
}

/// Health status of the spam predictor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictorHealthStatus {
    /// Overall health status
    pub overall_healthy: bool,
    /// OpenAI API connectivity
    pub openai_healthy: bool,
    /// Configuration accessibility
    pub config_healthy: bool,
    /// Cache functionality
    pub cache_healthy: bool,
    /// Time taken for health check in milliseconds
    pub check_duration_ms: u64,
    /// Current cache statistics
    pub cache_stats: crate::cache::CacheStats,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use alloy_primitives::Address;
    use tempfile::TempDir;
    use tokio::fs::write;

    use super::*;

    async fn create_test_config() -> (SpamPredictorConfig, TempDir, TempDir) {
        let temp_dir1 = TempDir::new().unwrap();
        let model_path = temp_dir1.path().join("models.yaml");
        write(
            &model_path,
            r#"
model_registry:
  spam_classification:
    latest: ft:gpt-4o-2024-08-06:test::TEST123
    v1: ft:gpt-4o-2024-08-06:test::TEST123
"#,
        )
        .await
        .unwrap();

        let temp_dir2 = TempDir::new().unwrap();
        let prompt_path = temp_dir2.path().join("prompts.json");
        write(&prompt_path, r#"{
    "versions": [
        {
            "version": "1.0.0",
            "date": "2025-04-29",
            "description": "Test version",
            "system_message": "Classify NFT contracts as spam or legitimate. Respond with 'true' for spam, 'false' for legitimate."
        }
    ],
    "current_version": "1.0.0"
}"#).await.unwrap();

        let openai_config =
            crate::config::OpenAiConfig::new("sk-test-key".to_string()).with_timeout(30);

        let config = SpamPredictorConfig::from_files(model_path, prompt_path, openai_config)
            .await
            .unwrap();

        (config, temp_dir1, temp_dir2)
    }

    fn create_test_metadata() -> ContractMetadata {
        ContractMetadata {
            address: Address::ZERO,
            name: Some("Test NFT Collection".to_string()),
            symbol: Some("TEST".to_string()),
            total_supply: Some("10000".to_string()),
            holder_count: Some(100),
            transaction_count: Some(500),
            creation_block: Some(12345678),
            creation_timestamp: None,
            creator_address: Some(Address::ZERO),
            is_verified: Some(true),
            contract_type: Some(api_client::ContractType::Erc721),
            additional_data: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn predictor_creation() {
        let (config, _temp1, _temp2) = create_test_config().await;

        // This will fail with real OpenAI API, but we're testing the creation logic
        let result = SpamPredictor::new(config).await;

        // Should create successfully even if OpenAI health check fails
        assert!(result.is_ok() || result.unwrap_err().is_auth_error());
    }

    #[tokio::test]
    async fn contract_data_preparation() {
        let (config, _temp1, _temp2) = create_test_config().await;
        let predictor = SpamPredictor::new(config).await;

        // Skip if OpenAI auth fails
        if predictor.is_err() {
            return;
        }
        let predictor = predictor.unwrap();

        let metadata = create_test_metadata();
        let contract_data = predictor.prepare_contract_data(&metadata).unwrap();

        // Verify the JSON structure
        let parsed: serde_json::Value = serde_json::from_str(&contract_data).unwrap();
        assert!(parsed["name"].as_str().unwrap().contains("Test NFT"));
        assert!(parsed["symbol"].as_str().unwrap().contains("TEST"));
        assert_eq!(parsed["total_supply"].as_str().unwrap(), "10000");
    }

    #[tokio::test]
    async fn health_check() {
        let (config, _temp1, _temp2) = create_test_config().await;
        let predictor = SpamPredictor::new(config).await;

        // Skip if OpenAI auth fails
        if predictor.is_err() {
            return;
        }
        let predictor = predictor.unwrap();

        let health = predictor.health_check().await.unwrap();

        // Config and cache should be healthy even if OpenAI isn't
        assert!(health.config_healthy);
        assert!(health.cache_healthy);
        assert!(health.check_duration_ms > 0);
    }

    #[tokio::test]
    async fn cache_functionality() {
        let (config, _temp1, _temp2) = create_test_config().await;
        let predictor = SpamPredictor::new(config).await;

        // Skip if OpenAI auth fails
        if predictor.is_err() {
            return;
        }
        let predictor = predictor.unwrap();

        // Test cache operations
        let cache_stats = predictor.get_cache_stats();
        assert_eq!(cache_stats.prediction_count, 0);

        // Test cleanup
        let cleaned = predictor.cleanup_cache().await.unwrap();
        assert_eq!(cleaned, 0); // No expired entries initially
    }

    #[test]
    fn config_summary() {
        // This is a unit test that doesn't require async or external dependencies
        use crate::{cache::SpamCache, config::ConfigSummary};

        let summary = ConfigSummary {
            model_types: vec!["spam_classification".to_string()],
            prompt_versions: vec!["1.0.0".to_string()],
            current_prompt_version: "1.0.0".to_string(),
            cache_stats: SpamCache::new().get_stats(),
            openai_configured: true,
        };

        assert_eq!(summary.model_types.len(), 1);
        assert_eq!(summary.prompt_versions.len(), 1);
        assert!(summary.openai_configured);
    }
}
