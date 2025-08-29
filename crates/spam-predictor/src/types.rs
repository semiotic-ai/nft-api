// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Type-safe domain models for spam prediction
//!
//! This module provides strongly-typed wrappers that encode business invariants
//! in the type system, making invalid states irrepresentable.

use std::{sync::LazyLock, time::Duration};

use api_client::ContractMetadata;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::{SpamPredictorError, SpamPredictorResult};

// Compile regex once at startup - safe because pattern is static
static VERSION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^v?\d+(\.\d+)*$").expect("version regex is valid"));

/// Model type with compile-time known variants and runtime validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelType(String);

impl ModelType {
    /// Known model types as associated constants
    pub const SPAM_CLASSIFICATION: &'static str = "spam_classification";

    /// Create a new model type with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the model type is empty or contains invalid characters
    pub fn new(value: impl Into<String>) -> SpamPredictorResult<Self> {
        let s = value.into();
        if s.is_empty() {
            return Err(SpamPredictorError::config(
                "Model type cannot be empty".to_string(),
            ));
        }

        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(SpamPredictorError::config(
                "Model type must contain only alphanumeric characters and underscores".to_string(),
            ));
        }

        Ok(Self(s))
    }

    /// Create spam classification model type (infallible)
    pub fn spam_classification() -> Self {
        Self(Self::SPAM_CLASSIFICATION.to_string())
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Model version with semantic versioning validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelVersion(String);

impl ModelVersion {
    /// Constant for latest version
    pub const LATEST: &'static str = "latest";

    /// Create a new model version with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the version format is invalid
    pub fn new(value: impl Into<String>) -> SpamPredictorResult<Self> {
        let s = value.into();
        if s.is_empty() {
            return Err(SpamPredictorError::config(
                "Model version cannot be empty".to_string(),
            ));
        }

        // Accept "latest" or semantic version pattern
        if s == Self::LATEST {
            return Ok(Self(s));
        }

        // Validate semantic version pattern (v0, v1.0, v1.2.3, etc.)
        if !VERSION_REGEX.is_match(&s) {
            return Err(SpamPredictorError::config(
                "Model version must be 'latest' or semantic version (e.g., 'v1', 'v1.0', '1.2.3')"
                    .to_string(),
            ));
        }

        Ok(Self(s))
    }

    /// Create latest version (infallible)
    pub fn latest() -> Self {
        Self(Self::LATEST.to_string())
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Prompt version with semantic versioning validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromptVersion(Version);

impl PromptVersion {
    /// Create a new prompt version with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is not valid semantic versioning
    pub fn new(value: impl AsRef<str>) -> SpamPredictorResult<Self> {
        let version = Version::parse(value.as_ref())
            .map_err(|e| SpamPredictorError::config(format!("Invalid prompt version: {}", e)))?;
        Ok(Self(version))
    }

    /// Create version 1.0.0 (infallible)
    pub fn v1_0_0() -> Self {
        Self(Version::new(1, 0, 0))
    }

    /// Get the version as a string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Get the semantic version
    pub fn version(&self) -> &Version {
        &self.0
    }
}

impl std::fmt::Display for PromptVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Model specification combining type and version with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelSpec {
    model_type: ModelType,
    version: ModelVersion,
}

impl ModelSpec {
    /// Create a new model specification
    pub fn new(model_type: ModelType, version: ModelVersion) -> Self {
        Self {
            model_type,
            version,
        }
    }

    /// Create spam classification model spec with latest version
    pub fn spam_classification_latest() -> Self {
        Self {
            model_type: ModelType::spam_classification(),
            version: ModelVersion::latest(),
        }
    }

    /// Get the model type
    pub fn model_type(&self) -> &ModelType {
        &self.model_type
    }

    /// Get the model version
    pub fn version(&self) -> &ModelVersion {
        &self.version
    }
}

impl std::fmt::Display for ModelSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.model_type, self.version)
    }
}

/// Spam prediction request with validated components
#[derive(Debug, Clone)]
pub struct SpamPredictionRequest {
    metadata: ContractMetadata,
    model_spec: ModelSpec,
    prompt_version: PromptVersion,
}

impl SpamPredictionRequest {
    /// Create a new spam prediction request
    pub fn new(
        metadata: ContractMetadata,
        model_spec: ModelSpec,
        prompt_version: PromptVersion,
    ) -> Self {
        Self {
            metadata,
            model_spec,
            prompt_version,
        }
    }

    /// Create request with default spam classification settings
    pub fn spam_classification(metadata: ContractMetadata) -> Self {
        Self {
            metadata,
            model_spec: ModelSpec::spam_classification_latest(),
            prompt_version: PromptVersion::v1_0_0(),
        }
    }

    /// Get the contract metadata
    pub fn metadata(&self) -> &ContractMetadata {
        &self.metadata
    }

    /// Get the model specification
    pub fn model_spec(&self) -> &ModelSpec {
        &self.model_spec
    }

    /// Get the prompt version
    pub fn prompt_version(&self) -> &PromptVersion {
        &self.prompt_version
    }
}

/// Spam classification with explicit states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpamClassification {
    /// Contract is classified as spam
    Spam,
    /// Contract is classified as legitimate
    Legitimate,
    /// Analysis was inconclusive
    Inconclusive,
}

impl SpamClassification {
    /// Convert to boolean (spam = true, others = false)
    pub fn is_spam(&self) -> bool {
        matches!(self, SpamClassification::Spam)
    }

    /// Get human-readable message
    pub fn message(&self) -> &'static str {
        match self {
            SpamClassification::Spam => "AI analysis classified as spam",
            SpamClassification::Legitimate => "AI analysis classified as legitimate",
            SpamClassification::Inconclusive => {
                "AI analysis was inconclusive, defaulting to not spam"
            }
        }
    }
}

/// Confidence score with validation (0.0 to 1.0)
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// Create a new confidence score with validation
    ///
    /// # Errors
    ///
    /// Returns an error if the score is not between 0.0 and 1.0 or is NaN
    pub fn new(score: f64) -> SpamPredictorResult<Self> {
        if !(0.0..=1.0).contains(&score) {
            return Err(SpamPredictorError::config(
                "Confidence score must be between 0.0 and 1.0".to_string(),
            ));
        }

        if score.is_nan() {
            return Err(SpamPredictorError::config(
                "Confidence score cannot be NaN".to_string(),
            ));
        }

        Ok(Self(score))
    }

    /// High confidence (0.9)
    pub fn high() -> Self {
        Self(0.9)
    }

    /// Medium confidence (0.6)
    pub fn medium() -> Self {
        Self(0.6)
    }

    /// Low confidence (0.3)
    pub fn low() -> Self {
        Self(0.3)
    }

    /// Get the score as f64
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

/// Comprehensive spam prediction result with confidence and reasoning
#[derive(Debug, Clone)]
pub struct SpamPredictionResult {
    classification: SpamClassification,
    confidence: ConfidenceScore,
    reasoning: Option<String>,
    model_used: ModelSpec,
    processing_time: Duration,
    cached: bool,
}

impl SpamPredictionResult {
    /// Create a new prediction result
    pub fn new(
        classification: SpamClassification,
        confidence: ConfidenceScore,
        reasoning: Option<String>,
        model_used: ModelSpec,
        processing_time: Duration,
        cached: bool,
    ) -> Self {
        Self {
            classification,
            confidence,
            reasoning,
            model_used,
            processing_time,
            cached,
        }
    }

    /// Create spam result
    pub fn spam(model_used: ModelSpec, processing_time: Duration) -> Self {
        Self {
            classification: SpamClassification::Spam,
            confidence: ConfidenceScore::high(),
            reasoning: Some("AI analysis classified as spam".to_string()),
            model_used,
            processing_time,
            cached: false,
        }
    }

    /// Create legitimate result
    pub fn legitimate(model_used: ModelSpec, processing_time: Duration) -> Self {
        Self {
            classification: SpamClassification::Legitimate,
            confidence: ConfidenceScore::high(),
            reasoning: Some("AI analysis classified as legitimate".to_string()),
            model_used,
            processing_time,
            cached: false,
        }
    }

    /// Create inconclusive result (fallback case)
    pub fn inconclusive(model_used: ModelSpec, processing_time: Duration) -> Self {
        Self {
            classification: SpamClassification::Inconclusive,
            confidence: ConfidenceScore::low(),
            reasoning: Some("AI analysis was inconclusive".to_string()),
            model_used,
            processing_time,
            cached: false,
        }
    }

    /// Create error fallback result (safe default)
    pub fn error_fallback(model_used: ModelSpec, processing_time: Duration) -> Self {
        Self {
            classification: SpamClassification::Legitimate, // Safe default
            confidence: ConfidenceScore::low(),
            reasoning: Some("Prediction failed, defaulting to legitimate".to_string()),
            model_used,
            processing_time,
            cached: false,
        }
    }

    /// Mark result as cached
    pub fn with_cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    /// Get the classification
    pub fn classification(&self) -> &SpamClassification {
        &self.classification
    }

    /// Check if result indicates spam
    pub fn is_spam(&self) -> bool {
        self.classification.is_spam()
    }

    /// Get human-readable message
    pub fn message(&self) -> &'static str {
        self.classification.message()
    }

    /// Get confidence score
    pub fn confidence(&self) -> &ConfidenceScore {
        &self.confidence
    }

    /// Get reasoning text
    pub fn reasoning(&self) -> Option<&str> {
        self.reasoning.as_deref()
    }

    /// Get the model used
    pub fn model_used(&self) -> &ModelSpec {
        &self.model_used
    }

    /// Get processing time
    pub fn processing_time(&self) -> Duration {
        self.processing_time
    }

    /// Check if result was cached
    pub fn is_cached(&self) -> bool {
        self.cached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_type_validation() {
        assert!(ModelType::new("spam_classification").is_ok());
        assert!(ModelType::new("test_model_123").is_ok());

        assert!(ModelType::new("").is_err());
        assert!(ModelType::new("invalid-model").is_err());
        assert!(ModelType::new("invalid model").is_err());

        let model_type = ModelType::spam_classification();
        assert_eq!(model_type.as_str(), "spam_classification");
    }

    #[test]
    fn model_version_validation() {
        assert!(ModelVersion::new("latest").is_ok());
        assert!(ModelVersion::new("v1").is_ok());
        assert!(ModelVersion::new("v1.0").is_ok());
        assert!(ModelVersion::new("1.2.3").is_ok());

        assert!(ModelVersion::new("").is_err());
        assert!(ModelVersion::new("invalid").is_err());
        assert!(ModelVersion::new("v").is_err());

        let version = ModelVersion::latest();
        assert_eq!(version.as_str(), "latest");
    }

    #[test]
    fn prompt_version_validation() {
        assert!(PromptVersion::new("1.0.0").is_ok());
        assert!(PromptVersion::new("2.1.3").is_ok());

        assert!(PromptVersion::new("").is_err());
        assert!(PromptVersion::new("invalid").is_err());
        assert!(PromptVersion::new("1.0").is_err()); // semver requires patch version

        let version = PromptVersion::v1_0_0();
        assert_eq!(version.as_str(), "1.0.0");
    }

    #[test]
    fn confidence_score_validation() {
        assert!(ConfidenceScore::new(0.0).is_ok());
        assert!(ConfidenceScore::new(0.5).is_ok());
        assert!(ConfidenceScore::new(1.0).is_ok());

        assert!(ConfidenceScore::new(-0.1).is_err());
        assert!(ConfidenceScore::new(1.1).is_err());
        assert!(ConfidenceScore::new(f64::NAN).is_err());

        assert_eq!(ConfidenceScore::high().as_f64(), 0.9);
        assert_eq!(ConfidenceScore::medium().as_f64(), 0.6);
        assert_eq!(ConfidenceScore::low().as_f64(), 0.3);
    }

    #[test]
    fn spam_classification_behavior() {
        assert!(SpamClassification::Spam.is_spam());
        assert!(!SpamClassification::Legitimate.is_spam());
        assert!(!SpamClassification::Inconclusive.is_spam());

        assert_eq!(
            SpamClassification::Spam.message(),
            "AI analysis classified as spam"
        );
        assert_eq!(
            SpamClassification::Legitimate.message(),
            "AI analysis classified as legitimate"
        );
        assert_eq!(
            SpamClassification::Inconclusive.message(),
            "AI analysis was inconclusive, defaulting to not spam"
        );
    }

    #[test]
    fn model_spec_creation() {
        let spec = ModelSpec::spam_classification_latest();
        assert_eq!(spec.model_type().as_str(), "spam_classification");
        assert_eq!(spec.version().as_str(), "latest");

        let custom_spec = ModelSpec::new(
            ModelType::new("custom_model").unwrap(),
            ModelVersion::new("v1.0").unwrap(),
        );
        assert_eq!(custom_spec.to_string(), "custom_model:v1.0");
    }

    #[test]
    fn spam_prediction_request_creation() {
        use alloy_primitives::Address;

        let metadata = ContractMetadata::minimal(Address::ZERO);

        let request = SpamPredictionRequest::spam_classification(metadata.clone());
        assert_eq!(
            request.model_spec().model_type().as_str(),
            "spam_classification"
        );
        assert_eq!(request.model_spec().version().as_str(), "latest");
        assert_eq!(request.prompt_version().as_str(), "1.0.0");
        assert_eq!(request.metadata().address, Address::ZERO);
    }

    #[test]
    fn spam_prediction_result_factory_methods() {
        let spec = ModelSpec::spam_classification_latest();
        let duration = Duration::from_millis(100);

        let spam_result = SpamPredictionResult::spam(spec.clone(), duration);
        assert!(spam_result.is_spam());
        assert_eq!(spam_result.message(), "AI analysis classified as spam");

        let legit_result = SpamPredictionResult::legitimate(spec.clone(), duration);
        assert!(!legit_result.is_spam());
        assert_eq!(
            legit_result.message(),
            "AI analysis classified as legitimate"
        );

        let inconclusive_result = SpamPredictionResult::inconclusive(spec.clone(), duration);
        assert!(!inconclusive_result.is_spam());
        assert_eq!(
            inconclusive_result.message(),
            "AI analysis was inconclusive, defaulting to not spam"
        );

        let error_result = SpamPredictionResult::error_fallback(spec, duration);
        assert!(!error_result.is_spam()); // Safe default
    }
}
