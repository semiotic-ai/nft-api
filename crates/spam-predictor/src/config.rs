// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Configuration management for spam prediction
//!
//! This module handles loading and caching of model registries and prompt
//! configurations from YAML and JSON files, providing versioned access
//! to models and prompts with intelligent caching.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info, warn};
use url::Url;

use crate::{
    cache::SpamCache,
    error::{SpamPredictorError, SpamPredictorResult},
    types::{ModelSpec, ModelType, ModelVersion},
};

/// Model registry configuration loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    /// Registry of models by type and version
    pub model_registry: HashMap<String, HashMap<String, String>>,
}

impl ModelRegistry {
    /// Load model registry from a YAML file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> SpamPredictorResult<Self> {
        let path = path.as_ref();
        debug!("Loading model registry from: {}", path.display());

        let content = fs::read_to_string(path).await.map_err(|e| {
            SpamPredictorError::io(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let registry: ModelRegistry = serde_yaml::from_str(&content).map_err(|e| {
            SpamPredictorError::yaml(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        info!(
            "Loaded model registry with {} model types from {}",
            registry.model_registry.len(),
            path.display()
        );

        Ok(registry)
    }

    /// Validate that all model IDs are properly formatted
    pub fn validate(&self) -> SpamPredictorResult<()> {
        for (model_type, versions) in &self.model_registry {
            if versions.is_empty() {
                return Err(SpamPredictorError::model_registry(format!(
                    "Model type '{}' has no versions configured",
                    model_type
                )));
            }

            for (version, model_id) in versions {
                if model_id.is_empty() {
                    return Err(SpamPredictorError::model_registry(format!(
                        "Empty model ID for {}:{}",
                        model_type, version
                    )));
                }

                // Basic validation for OpenAI fine-tuned model format
                if !model_id.starts_with("ft:") && !model_id.starts_with("gpt-") {
                    warn!(
                        "Model ID '{}' for {}:{} doesn't match expected OpenAI format",
                        model_id, model_type, version
                    );
                }
            }
        }

        Ok(())
    }

    /// Get a model ID by model specification
    pub fn get_model(&self, spec: &ModelSpec) -> SpamPredictorResult<String> {
        let model_type_registry = self
            .model_registry
            .get(spec.model_type().as_str())
            .ok_or_else(|| {
                SpamPredictorError::model_registry(format!(
                    "Model type '{}' not found",
                    spec.model_type()
                ))
            })?;

        let model_id = model_type_registry
            .get(spec.version().as_str())
            .ok_or_else(|| {
                SpamPredictorError::model_registry(format!(
                    "Version '{}' not found for model type '{}'",
                    spec.version(),
                    spec.model_type()
                ))
            })?;

        Ok(model_id.clone())
    }

    /// Get all available model types
    pub fn get_model_types(&self) -> SpamPredictorResult<Vec<ModelType>> {
        self.model_registry
            .keys()
            .map(|s| ModelType::new(s.clone()))
            .collect()
    }

    /// Get all available versions for a model type
    pub fn get_versions(&self, model_type: &ModelType) -> SpamPredictorResult<Vec<ModelVersion>> {
        let versions = self
            .model_registry
            .get(model_type.as_str())
            .ok_or_else(|| {
                SpamPredictorError::model_registry(format!("Model type '{}' not found", model_type))
            })?;

        versions
            .keys()
            .map(|s| ModelVersion::new(s.clone()))
            .collect()
    }

    /// Check if a model spec exists in the registry
    pub fn has_model_spec(&self, spec: &ModelSpec) -> bool {
        self.model_registry
            .get(spec.model_type().as_str())
            .map(|versions| versions.contains_key(spec.version().as_str()))
            .unwrap_or(false)
    }
}

/// Prompt version configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    /// Version identifier (e.g., "1.0.0")
    pub version: String,
    /// Creation date of this version
    pub date: String,
    /// Description of this prompt version
    pub description: String,
    /// The actual system message/prompt
    pub system_message: String,
}

/// Prompt registry configuration loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRegistry {
    /// All available prompt versions
    pub versions: Vec<PromptVersion>,
    /// Current/default version to use
    pub current_version: String,
}

impl PromptRegistry {
    /// Load prompt registry from a JSON file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> SpamPredictorResult<Self> {
        let path = path.as_ref();
        debug!("Loading prompt registry from: {}", path.display());

        let content = fs::read_to_string(path).await.map_err(|e| {
            SpamPredictorError::io(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let registry: PromptRegistry = serde_json::from_str(&content).map_err(|e| {
            SpamPredictorError::json(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        info!(
            "Loaded prompt registry with {} versions from {}",
            registry.versions.len(),
            path.display()
        );

        Ok(registry)
    }

    /// Get a prompt by version
    pub fn get_prompt(&self, version: &str) -> SpamPredictorResult<String> {
        let prompt_version = self
            .versions
            .iter()
            .find(|v| v.version == version)
            .ok_or_else(|| {
                SpamPredictorError::prompt_registry(format!(
                    "Prompt version '{}' not found",
                    version
                ))
            })?;

        Ok(prompt_version.system_message.clone())
    }

    /// Get the current/default prompt
    pub fn get_current_prompt(&self) -> SpamPredictorResult<String> {
        self.get_prompt(&self.current_version)
    }

    /// Get all available prompt versions
    pub fn get_versions(&self) -> Vec<String> {
        self.versions.iter().map(|v| v.version.clone()).collect()
    }

    /// Get prompt version metadata (without the full system message)
    pub fn get_version_info(&self, version: &str) -> Option<PromptVersionInfo> {
        self.versions
            .iter()
            .find(|v| v.version == version)
            .map(|v| PromptVersionInfo {
                version: v.version.clone(),
                date: v.date.clone(),
                description: v.description.clone(),
                message_length: v.system_message.len(),
            })
    }

    /// Validate the prompt registry
    pub fn validate(&self) -> SpamPredictorResult<()> {
        if self.versions.is_empty() {
            return Err(SpamPredictorError::prompt_registry(
                "No prompt versions configured".to_string(),
            ));
        }

        // Validate that current version exists
        if !self
            .versions
            .iter()
            .any(|v| v.version == self.current_version)
        {
            return Err(SpamPredictorError::prompt_registry(format!(
                "Current version '{}' not found in available versions",
                self.current_version
            )));
        }

        // Validate each version
        for version in &self.versions {
            if version.version.is_empty() {
                return Err(SpamPredictorError::prompt_registry(
                    "Empty version identifier found".to_string(),
                ));
            }

            if version.system_message.is_empty() {
                return Err(SpamPredictorError::prompt_registry(format!(
                    "Empty system message for version '{}'",
                    version.version
                )));
            }
        }

        // Check for duplicate versions
        let mut seen_versions = std::collections::HashSet::new();
        for version in &self.versions {
            if !seen_versions.insert(&version.version) {
                return Err(SpamPredictorError::prompt_registry(format!(
                    "Duplicate version '{}' found",
                    version.version
                )));
            }
        }

        Ok(())
    }
}

/// Metadata about a prompt version (without the full message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersionInfo {
    /// Version identifier
    pub version: String,
    /// Creation date
    pub date: String,
    /// Description
    pub description: String,
    /// Length of the system message in characters
    pub message_length: usize,
}

/// OpenAI API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// OpenAI API key
    pub api_key: String,
    /// Base URL for OpenAI API (defaults to official API)
    pub base_url: Option<Url>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum number of tokens in the response
    pub max_tokens: Option<u32>,
    /// Temperature for response generation
    pub temperature: Option<f32>,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            timeout_seconds: 30,
            max_tokens: Some(10),
            temperature: Some(0.0),
            organization_id: None,
        }
    }
}

impl OpenAiConfig {
    /// Create a new OpenAI configuration
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            ..Default::default()
        }
    }

    /// Set the base URL for the OpenAI API
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set the maximum tokens for responses
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature for response generation
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the organization ID
    pub fn with_organization(mut self, organization_id: String) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    /// Validate the OpenAI configuration
    pub fn validate(&self) -> SpamPredictorResult<()> {
        if self.api_key.is_empty() {
            return Err(SpamPredictorError::config(
                "OpenAI API key cannot be empty".to_string(),
            ));
        }

        if !self.api_key.starts_with("sk-") && !self.api_key.starts_with("test-") {
            warn!("OpenAI API key doesn't match expected format (should start with 'sk-')");
        }

        if self.timeout_seconds == 0 || self.timeout_seconds > 300 {
            return Err(SpamPredictorError::config(format!(
                "Invalid timeout: {} seconds (must be 1-300)",
                self.timeout_seconds
            )));
        }

        if let Some(max_tokens) = self.max_tokens
            && (max_tokens == 0 || max_tokens > 4096)
        {
            return Err(SpamPredictorError::config(format!(
                "Invalid max_tokens: {} (must be 1-4096)",
                max_tokens
            )));
        }

        if let Some(temperature) = self.temperature
            && !(0.0..=2.0).contains(&temperature)
        {
            return Err(SpamPredictorError::config(format!(
                "Invalid temperature: {} (must be 0.0-2.0)",
                temperature
            )));
        }

        Ok(())
    }
}

/// Complete spam predictor configuration
#[derive(Debug, Clone)]
pub struct SpamPredictorConfig {
    /// Model registry
    pub model_registry: Arc<ModelRegistry>,
    /// Prompt registry
    pub prompt_registry: Arc<PromptRegistry>,
    /// OpenAI API configuration
    pub openai_config: OpenAiConfig,
    /// Cache instance
    pub cache: Arc<SpamCache>,
    /// Configuration file paths for hot reloading
    pub model_registry_path: PathBuf,
    pub prompt_registry_path: PathBuf,
}

impl SpamPredictorConfig {
    /// Create configuration from file paths
    pub async fn from_files<P1, P2>(
        model_registry_path: P1,
        prompt_registry_path: P2,
        openai_config: OpenAiConfig,
    ) -> SpamPredictorResult<Self>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        // Validate file existence and canonicalize paths
        let model_registry_path =
            Self::validate_and_canonicalize_path(model_registry_path.as_ref(), "model registry")?;
        let prompt_registry_path =
            Self::validate_and_canonicalize_path(prompt_registry_path.as_ref(), "prompt registry")?;

        let model_registry = ModelRegistry::from_file(&model_registry_path).await?;
        model_registry.validate()?;

        let prompt_registry = PromptRegistry::from_file(&prompt_registry_path).await?;
        prompt_registry.validate()?;

        openai_config.validate()?;

        let cache = Arc::new(SpamCache::new());

        // Pre-cache all models and prompts
        Self::populate_cache(&cache, &model_registry, &prompt_registry);

        Ok(Self {
            model_registry: Arc::new(model_registry),
            prompt_registry: Arc::new(prompt_registry),
            openai_config,
            cache,
            model_registry_path,
            prompt_registry_path,
        })
    }

    /// Populate the cache with all models and prompts
    fn populate_cache(
        cache: &SpamCache,
        model_registry: &ModelRegistry,
        prompt_registry: &PromptRegistry,
    ) {
        // Cache all models
        for (model_type, versions) in &model_registry.model_registry {
            for (version, model_id) in versions {
                cache.store_model(model_type, version, model_id.clone());
            }
        }

        // Cache all prompts
        for version in &prompt_registry.versions {
            cache.store_prompt(&version.version, version.system_message.clone());
        }

        debug!(
            "Populated cache with {} model entries and {} prompt entries",
            model_registry
                .model_registry
                .values()
                .map(|v| v.len())
                .sum::<usize>(),
            prompt_registry.versions.len()
        );
    }

    /// Reload configurations from files (hot reload)
    pub async fn reload(&mut self) -> SpamPredictorResult<()> {
        debug!("Reloading spam predictor configurations");

        let model_registry = ModelRegistry::from_file(&self.model_registry_path).await?;
        model_registry.validate()?;

        let prompt_registry = PromptRegistry::from_file(&self.prompt_registry_path).await?;
        prompt_registry.validate()?;

        // Clear old cached configurations
        self.cache.clear_configurations();

        // Update configurations
        self.model_registry = Arc::new(model_registry);
        self.prompt_registry = Arc::new(prompt_registry);

        // Re-populate cache
        Self::populate_cache(&self.cache, &self.model_registry, &self.prompt_registry);

        info!("Successfully reloaded spam predictor configurations");
        Ok(())
    }

    /// Get a model ID with caching
    pub fn get_model(&self, spec: &ModelSpec) -> SpamPredictorResult<String> {
        let model_type = spec.model_type().as_str();
        let version = spec.version().as_str();

        // Try cache first
        if let Some(model_id) = self.cache.get_model(model_type, version) {
            return Ok(model_id);
        }

        // Fallback to registry
        let model_id = self.model_registry.get_model(spec)?;

        // Cache the result
        self.cache
            .store_model(model_type, version, model_id.clone());

        Ok(model_id)
    }

    /// Get a prompt with caching
    pub fn get_prompt(&self, version: &str) -> SpamPredictorResult<String> {
        // Try cache first
        if let Some(prompt) = self.cache.get_prompt(version) {
            return Ok(prompt);
        }

        // Fallback to registry
        let prompt = self.prompt_registry.get_prompt(version)?;

        // Cache the result
        self.cache.store_prompt(version, prompt.clone());

        Ok(prompt)
    }

    /// Get configuration summary
    pub fn get_summary(&self) -> ConfigSummary {
        ConfigSummary {
            model_types: self
                .model_registry
                .get_model_types()
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.as_str().to_string())
                .collect(),
            prompt_versions: self.prompt_registry.get_versions(),
            current_prompt_version: self.prompt_registry.current_version.clone(),
            cache_stats: self.cache.get_stats(),
            openai_configured: !self.openai_config.api_key.is_empty(),
        }
    }

    /// Validate and canonicalize a file path
    fn validate_and_canonicalize_path(
        path: &Path,
        file_type: &str,
    ) -> SpamPredictorResult<PathBuf> {
        // Check if file exists
        if !path.exists() {
            return Err(SpamPredictorError::config(format!(
                "{} file not found: {} (current working directory: {})",
                file_type,
                path.display(),
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            )));
        }

        // Check if it's actually a file (not a directory)
        if !path.is_file() {
            return Err(SpamPredictorError::config(format!(
                "{} path exists but is not a file: {}",
                file_type,
                path.display()
            )));
        }

        // Canonicalize the path to get absolute path and resolve symlinks
        path.canonicalize().map_err(|e| {
            SpamPredictorError::config(format!(
                "Failed to canonicalize {} path {}: {}",
                file_type,
                path.display(),
                e
            ))
        })
    }

    /// Get all available model types
    pub fn get_model_types(&self) -> SpamPredictorResult<Vec<ModelType>> {
        self.model_registry.get_model_types()
    }

    /// Get all available versions for a model type
    pub fn get_versions(&self, model_type: &ModelType) -> SpamPredictorResult<Vec<ModelVersion>> {
        self.model_registry.get_versions(model_type)
    }

    /// Check if a model spec exists in the registry
    pub fn has_model_spec(&self, spec: &ModelSpec) -> bool {
        self.model_registry.has_model_spec(spec)
    }
}

/// Configuration summary for monitoring and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Available model types
    pub model_types: Vec<String>,
    /// Available prompt versions
    pub prompt_versions: Vec<String>,
    /// Current prompt version
    pub current_prompt_version: String,
    /// Cache statistics
    pub cache_stats: crate::cache::CacheStats,
    /// Whether OpenAI is configured
    pub openai_configured: bool,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use tokio::fs::write;

    use super::*;

    async fn create_test_model_registry() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("models.yaml");

        let content = r#"
model_registry:
  spam_classification:
    latest: ft:gpt-4o-2024-08-06:semiotic-labs::TEST123
    v1: ft:gpt-4o-2024-08-06:semiotic-labs::TEST123
    v0: ft:gpt-4o-2024-08-06:semiotic-labs::OLD456
  sentiment_analysis:
    latest: ft:gpt-4o-2024-08-06:semiotic-labs::SENT789
"#;

        write(&file_path, content).await.unwrap();
        (temp_dir, file_path)
    }

    async fn create_test_prompt_registry() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("prompts.json");

        let content = r#"{
    "versions": [
        {
            "version": "1.0.0",
            "date": "2025-04-29",
            "description": "Initial version",
            "system_message": "You are an expert in NFTs. Analyze and classify as spam or legitimate."
        },
        {
            "version": "1.1.0",
            "date": "2025-05-01",
            "description": "Enhanced version",
            "system_message": "You are an expert in NFTs and blockchain technology. Analyze the provided metadata and determine if it's spam."
        }
    ],
    "current_version": "1.1.0"
}"#;

        write(&file_path, content).await.unwrap();
        (temp_dir, file_path)
    }

    #[tokio::test]
    async fn load_model_registry() {
        let (_temp_dir, file_path) = create_test_model_registry().await;
        let registry = ModelRegistry::from_file(&file_path).await.unwrap();

        assert_eq!(registry.model_registry.len(), 2);
        assert!(registry.model_registry.contains_key("spam_classification"));
        assert!(registry.model_registry.contains_key("sentiment_analysis"));

        let spam_spec = ModelSpec::new(
            ModelType::new("spam_classification".to_string()).expect("valid model type"),
            ModelVersion::new("latest".to_string()).expect("valid version"),
        );
        let model_id = registry.get_model(&spam_spec).unwrap();
        assert_eq!(model_id, "ft:gpt-4o-2024-08-06:semiotic-labs::TEST123");
    }

    #[tokio::test]
    async fn load_prompt_registry() {
        let (_temp_dir, file_path) = create_test_prompt_registry().await;
        let registry = PromptRegistry::from_file(&file_path).await.unwrap();

        assert_eq!(registry.versions.len(), 2);
        assert_eq!(registry.current_version, "1.1.0");

        let prompt = registry.get_prompt("1.0.0").unwrap();
        assert!(prompt.contains("You are an expert in NFTs"));

        let current_prompt = registry.get_current_prompt().unwrap();
        assert!(current_prompt.contains("blockchain technology"));
    }

    #[tokio::test]
    async fn full_configuration() {
        let (_temp_dir1, model_path) = create_test_model_registry().await;
        let (_temp_dir2, prompt_path) = create_test_prompt_registry().await;

        let openai_config = OpenAiConfig::new("test-api-key".to_string())
            .with_timeout(60)
            .with_max_tokens(50);

        let config = SpamPredictorConfig::from_files(model_path, prompt_path, openai_config)
            .await
            .unwrap();

        // Test model access
        let spam_spec = ModelSpec::new(
            ModelType::new("spam_classification".to_string()).expect("valid model type"),
            ModelVersion::new("latest".to_string()).expect("valid version"),
        );
        let model_id = config.get_model(&spam_spec).unwrap();
        assert_eq!(model_id, "ft:gpt-4o-2024-08-06:semiotic-labs::TEST123");

        // Test prompt access
        let prompt = config.get_prompt("1.0.0").unwrap();
        assert!(prompt.contains("You are an expert in NFTs"));

        // Test summary
        let summary = config.get_summary();
        assert_eq!(summary.model_types.len(), 2);
        assert_eq!(summary.prompt_versions.len(), 2);
        assert!(summary.openai_configured);
    }

    #[test]
    fn openai_config_validation() {
        let config = OpenAiConfig::new("sk-test-key".to_string());
        assert!(config.validate().is_ok());

        let invalid_config = OpenAiConfig::new("".to_string());
        assert!(invalid_config.validate().is_err());

        let invalid_timeout = OpenAiConfig::new("sk-test".to_string()).with_timeout(500);
        assert!(invalid_timeout.validate().is_err());
    }

    #[test]
    fn file_validation() {
        // Test with non-existent file
        let result = SpamPredictorConfig::validate_and_canonicalize_path(
            Path::new("/non/existent/file.yaml"),
            "test",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("test file not found")
        );

        // Test with existing file - create a temp file for testing
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        let result = SpamPredictorConfig::validate_and_canonicalize_path(temp_path, "test");
        assert!(result.is_ok());

        // Should return absolute path
        let canonical_path = result.unwrap();
        assert!(canonical_path.is_absolute());
        assert!(canonical_path.exists());
    }
}
