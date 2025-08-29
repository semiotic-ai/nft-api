// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! AI-powered NFT contract spam prediction
//!
//! This crate provides sophisticated spam prediction capabilities for NFT contracts
//! using fine-tuned OpenAI models. It features async operations, intelligent caching,
//! comprehensive error handling, and seamless integration with blockchain data providers.
//!
//! # Key Features
//!
//! - **Fine-tuned AI Models**: Leverages OpenAI GPT-4 models specifically trained for NFT spam detection
//! - **Async Operations**: Full tokio async support for high-performance concurrent predictions
//! - **Intelligent Caching**: In-memory caching for configurations, models, and frequent predictions
//! - **Robust Error Handling**: Comprehensive error types with graceful degradation
//! - **Configuration Management**: Versioned model and prompt management with hot reloading
//! - **Observability**: Structured logging and tracing for monitoring and debugging
//!
//! # Architecture
//!
//! The crate is organized into several key modules:
//!
//! - [`predictor`]: Core spam prediction logic and orchestration
//! - [`config`]: Configuration management for models, prompts, and API settings
//! - [`openai`]: OpenAI API client with fine-tuned model support
//! - [`cache`]: In-memory caching layer for performance optimization
//! - [`error`]: Comprehensive error types and handling
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use spam_predictor::{SpamPredictor, SpamPredictorConfig, SpamPredictionRequest, config::OpenAiConfig};
//! use api_client::ContractMetadata;
//! use alloy_primitives::Address;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize OpenAI configuration
//! let openai_config = OpenAiConfig::new("sk-your-api-key".to_string());
//!
//! // Initialize the predictor with configuration
//! let config = SpamPredictorConfig::from_files(
//!     "assets/configs/models.yaml",
//!     "assets/prompts/ft_prompt.json",
//!     openai_config
//! ).await?;
//!
//! let predictor = SpamPredictor::new(config).await?;
//!
//! // Create sample contract metadata
//! let metadata = ContractMetadata::minimal(Address::ZERO);
//!
//! // Create a typed prediction request
//! let request = SpamPredictionRequest::spam_classification(metadata);
//!
//! // Predict spam status with typed API
//! let result = predictor.predict_spam_typed(request).await?;
//!
//! println!("Contract is spam: {}", result.classification().is_spam());
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod config;
pub mod error;
pub mod openai;
pub mod predictor;
pub mod types;

// Re-export main types for convenience
pub use cache::SpamCache;
pub use config::{ModelRegistry, PromptRegistry, SpamPredictorConfig};
pub use error::{SpamPredictorError, SpamPredictorResult};
pub use openai::OpenAiClient;
pub use predictor::SpamPredictor;
pub use types::{
    ConfidenceScore, ModelSpec, ModelType, ModelVersion, PromptVersion, SpamClassification,
    SpamPredictionRequest, SpamPredictionResult,
};
