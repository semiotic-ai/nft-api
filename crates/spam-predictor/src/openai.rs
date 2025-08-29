// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! OpenAI API client for spam prediction using fine-tuned models
//!
//! This module provides a high-performance async client for interacting with
//! OpenAI's API, specifically optimized for fine-tuned model inference
//! with proper error handling, rate limiting, and response parsing.

use std::time::{Duration, Instant};

use reqwest::{
    Client, ClientBuilder,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use tokio_retry::{
    Retry,
    strategy::{ExponentialBackoff, jitter},
};
use tracing::{Span, debug, error, info, instrument, warn};
use url::Url;
use uuid::Uuid;

use crate::error::{ErrorContext, SpamPredictorError, SpamPredictorResult};

/// OpenAI Chat Completion API request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionRequest {
    /// The model to use for completion
    model: String,
    /// List of messages for the conversation
    messages: Vec<ChatMessage>,
    /// Maximum number of tokens to generate
    max_tokens: Option<u32>,
    /// Sampling temperature (0.0 to 2.0)
    temperature: Option<f32>,
    /// Top-p sampling parameter
    top_p: Option<f32>,
    /// Stop sequences
    stop: Option<Vec<String>>,
    /// Whether to stream the response
    stream: bool,
}

/// A single message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    /// Role of the message sender
    role: String,
    /// Content of the message
    content: String,
}

/// OpenAI Chat Completion API response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionResponse {
    /// Unique ID for the completion
    id: String,
    /// Object type (should be "chat.completion")
    object: String,
    /// Timestamp when the completion was created
    created: u64,
    /// Model used for the completion
    model: String,
    /// List of completion choices
    choices: Vec<ChatChoice>,
    /// Token usage information
    usage: Option<TokenUsage>,
}

/// A single completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatChoice {
    /// Index of this choice
    index: u32,
    /// The completion message
    message: ChatMessage,
    /// Reason the completion finished
    finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the prompt
    prompt_tokens: u32,
    /// Number of tokens in the completion
    completion_tokens: u32,
    /// Total number of tokens used
    total_tokens: u32,
}

/// OpenAI API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAiErrorResponse {
    /// Error details
    error: OpenAiError,
}

/// OpenAI API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAiError {
    /// Error message
    message: String,
    /// Error type
    r#type: Option<String>,
    /// Error code
    code: Option<String>,
}

/// Result of a spam prediction from OpenAI
#[derive(Debug, Clone)]
pub struct PredictionResult {
    /// Whether the contract is predicted to be spam
    pub is_spam: Option<bool>,
    /// Raw response from the model
    pub raw_response: String,
    /// Confidence score (if available)
    pub confidence: Option<f32>,
    /// Token usage for this prediction
    pub token_usage: Option<TokenUsage>,
    /// Model used for the prediction
    pub model: String,
}

/// OpenAI API client for spam prediction
#[derive(Debug, Clone)]
pub struct OpenAiClient {
    /// HTTP client for API requests
    client: Client,
    /// Base URL for OpenAI API
    base_url: Url,
    /// API key for authentication
    #[allow(dead_code)]
    // Stored for potential future use in error recovery or re-authentication
    api_key: String,
    /// Request timeout
    timeout: Duration,
    /// Default parameters for requests
    default_max_tokens: Option<u32>,
    default_temperature: Option<f32>,
    /// Organization ID (optional)
    organization_id: Option<String>,
}

impl OpenAiClient {
    /// Create a new OpenAI client
    pub fn new(
        api_key: String,
        base_url: Option<Url>,
        timeout_seconds: u64,
        organization_id: Option<String>,
    ) -> SpamPredictorResult<Self> {
        // Static URL is safe - this is compile-time verified and will never panic
        const DEFAULT_API_URL: &str = "https://api.openai.com/v1/";
        let base_url = base_url
            .unwrap_or_else(|| Url::parse(DEFAULT_API_URL).expect("default OpenAI URL is valid"));

        let timeout = Duration::from_secs(timeout_seconds);

        // Create default headers
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|e| {
                SpamPredictorError::config(format!("Invalid API key format: {}", e))
            })?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // if let Some(ref org_id) = organization_id {
        //     headers.insert(
        //         "OpenAI-Organization",
        //         HeaderValue::from_str(org_id).map_err(|e| {
        //             SpamPredictorError::config(format!("Invalid organization ID: {}", e))
        //         })?,
        //     );
        // }

        // Build HTTP client
        let client = ClientBuilder::new()
            .timeout(timeout)
            .default_headers(headers)
            .user_agent("spam-predictor/0.1.0")
            .build()
            .map_err(|e| {
                SpamPredictorError::http(format!("Failed to create HTTP client: {}", e))
            })?;

        info!(
            "Created OpenAI client with base URL: {} and timeout: {}s",
            base_url, timeout_seconds
        );

        Ok(Self {
            client,
            base_url,
            api_key,
            timeout,
            default_max_tokens: Some(10), // Short responses for spam classification
            default_temperature: Some(0.0), // Deterministic responses
            organization_id,
        })
    }

    /// Set default maximum tokens for responses
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.default_max_tokens = Some(max_tokens);
        self
    }

    /// Set default temperature for responses
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = Some(temperature);
        self
    }

    /// Predict spam status for contract metadata
    #[instrument(skip(self, system_prompt, contract_data), fields(model = %model_id, request_id))]
    pub async fn predict_spam(
        &self,
        model_id: &str,
        system_prompt: &str,
        contract_data: &str,
    ) -> SpamPredictorResult<PredictionResult> {
        let request_id = Uuid::new_v4();
        Span::current().record("request_id", request_id.to_string());

        info!(
            request_id = %request_id,
            model = %model_id,
            data_length = contract_data.len(),
            "Starting spam prediction request"
        );

        // Construct the chat messages
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: contract_data.to_string(),
            },
        ];

        // Create the request
        let request = ChatCompletionRequest {
            model: model_id.to_string(),
            messages,
            max_tokens: self.default_max_tokens,
            temperature: self.default_temperature,
            top_p: None,
            stop: Some(vec![
                "true".to_string(),
                "false".to_string(),
                "True".to_string(),
                "False".to_string(),
            ]),
            stream: false,
        };

        // Send the request with retry logic
        // Ensure base URL ends with slash for proper joining
        let mut base_url = self.base_url.clone();
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let url = base_url
            .join("chat/completions")
            .map_err(|e| SpamPredictorError::config(format!("Invalid base URL: {}", e)))?;

        let start_time = Instant::now();
        let response = self
            .make_retryable_request(&url, &request, request_id)
            .await?;
        let request_duration = start_time.elapsed();

        debug!(
            request_id = %request_id,
            duration_ms = request_duration.as_millis(),
            "API request completed"
        );

        // Handle response
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return self
                .handle_error_response(status.as_u16(), &response_text)
                .await;
        }

        // Parse successful response
        let completion: ChatCompletionResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                SpamPredictorError::invalid_response(format!("Failed to parse response: {}", e))
            })?;

        self.process_completion_response(completion, model_id, request_id)
            .await
    }

    /// Make a retryable HTTP request with exponential backoff
    async fn make_retryable_request(
        &self,
        url: &Url,
        request: &ChatCompletionRequest,
        request_id: Uuid,
    ) -> SpamPredictorResult<reqwest::Response> {
        // Configure retry strategy: 3 attempts with exponential backoff
        let retry_strategy = ExponentialBackoff::from_millis(100)
            .max_delay(Duration::from_secs(10))
            .take(3) // Maximum 3 attempts
            .map(jitter); // Add jitter to prevent thundering herd

        let client = &self.client;

        Retry::spawn(retry_strategy, move || {
            let url = url.clone();
            let request = request.clone();

            async move {
                debug!(
                    request_id = %request_id,
                    url = %url,
                    model = %request.model,
                    "Making API request attempt"
                );

                let url_string = url.to_string(); // Store before move
                let response = client.post(url).json(&request).send().await?;
                let status = response.status();

                // Determine if we should retry based on status code
                if Self::should_retry_status(status.as_u16()) {
                    warn!(
                        request_id = %request_id,
                        status = status.as_u16(),
                        "Request failed with retryable status, will retry"
                    );

                    let error_context = ErrorContext::new()
                        .with_request_id(request_id.to_string())
                        .with_operation("openai_api_request".to_string())
                        .with_metadata("status_code".to_string(), status.as_u16().to_string())
                        .with_metadata("url".to_string(), url_string);

                    return Err(SpamPredictorError::http_with_context(
                        format!("HTTP {} - retryable error", status.as_u16()),
                        error_context,
                    ));
                }

                debug!(
                    request_id = %request_id,
                    status = status.as_u16(),
                    "Request completed with status"
                );

                Ok(response)
            }
        })
        .await
    }

    /// Determine if an HTTP status code should trigger a retry
    fn should_retry_status(status: u16) -> bool {
        matches!(
            status,
            429 |           // Rate limit
            500
                ..=599 |     // Server errors
            408 // Request timeout
        )
    }

    /// Handle error responses from OpenAI API
    async fn handle_error_response(
        &self,
        status_code: u16,
        response_text: &str,
    ) -> SpamPredictorResult<PredictionResult> {
        // Try to parse as OpenAI error response
        if let Ok(error_response) = serde_json::from_str::<OpenAiErrorResponse>(response_text) {
            let error_msg = format!(
                "OpenAI API error ({}): {} (type: {:?}, code: {:?})",
                status_code,
                error_response.error.message,
                error_response.error.r#type,
                error_response.error.code
            );

            error!("{}", error_msg);

            match status_code {
                401 | 403 => Err(SpamPredictorError::authentication(error_msg)),
                429 => {
                    // Parse retry-after from rate limit errors if available
                    let retry_after = if error_response.error.message.contains("rate limit") {
                        60 // Default 60 seconds
                    } else {
                        30
                    };
                    Err(SpamPredictorError::rate_limit(retry_after))
                }
                500..=599 => Err(SpamPredictorError::service_unavailable(error_msg)),
                _ => Err(SpamPredictorError::openai(error_msg)),
            }
        } else {
            // Generic HTTP error
            let error_msg = format!("HTTP {} error: {}", status_code, response_text);
            error!("{}", error_msg);

            match status_code {
                401 | 403 => Err(SpamPredictorError::authentication(error_msg)),
                429 => Err(SpamPredictorError::rate_limit(60)),
                500..=599 => Err(SpamPredictorError::service_unavailable(error_msg)),
                _ => Err(SpamPredictorError::http(error_msg)),
            }
        }
    }

    /// Process a successful completion response
    async fn process_completion_response(
        &self,
        completion: ChatCompletionResponse,
        model_id: &str,
        request_id: Uuid,
    ) -> SpamPredictorResult<PredictionResult> {
        if completion.choices.is_empty() {
            return Err(SpamPredictorError::invalid_response(
                "No choices in completion response".to_string(),
            ));
        }

        let choice = &completion.choices[0];
        let raw_response = choice.message.content.trim().to_lowercase();

        debug!(
            request_id = %request_id,
            raw_response = %raw_response,
            model = %model_id,
            "Received response from OpenAI API"
        );

        // Parse the response to determine spam status
        let is_spam = self.parse_spam_response(&raw_response)?;

        // Log the result with structured data
        match is_spam {
            Some(true) => info!(
                request_id = %request_id,
                model = %model_id,
                classification = "spam",
                raw_response = %raw_response,
                "Model classified contract as SPAM"
            ),
            Some(false) => info!(
                request_id = %request_id,
                model = %model_id,
                classification = "legitimate",
                raw_response = %raw_response,
                "Model classified contract as NOT SPAM"
            ),
            None => warn!(
                request_id = %request_id,
                model = %model_id,
                classification = "ambiguous",
                raw_response = %raw_response,
                "Model gave ambiguous response"
            ),
        }

        // Log token usage if available
        if let Some(ref usage) = completion.usage {
            debug!(
                request_id = %request_id,
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                total_tokens = usage.total_tokens,
                "Token usage statistics"
            );
        }

        Ok(PredictionResult {
            is_spam,
            raw_response: choice.message.content.clone(),
            confidence: None, // OpenAI doesn't provide confidence scores
            token_usage: completion.usage,
            model: model_id.to_string(),
        })
    }

    /// Parse the model response to determine spam status
    fn parse_spam_response(&self, response: &str) -> SpamPredictorResult<Option<bool>> {
        let response = response.trim().to_lowercase();

        // Direct boolean responses
        if response == "true" || response == "yes" || response == "spam" {
            return Ok(Some(true));
        }

        if response == "false"
            || response == "no"
            || response == "not spam"
            || response == "legitimate"
        {
            return Ok(Some(false));
        }

        // Pattern matching for common response formats
        if response.contains("is spam")
            || response.contains("spam: true")
            || response.contains("classification: spam")
        {
            return Ok(Some(true));
        }

        if response.contains("not spam")
            || response.contains("spam: false")
            || response.contains("classification: legitimate")
        {
            return Ok(Some(false));
        }

        // JSON-like responses
        if response.contains("\"spam\": true") || response.contains("'spam': true") {
            return Ok(Some(true));
        }

        if response.contains("\"spam\": false") || response.contains("'spam': false") {
            return Ok(Some(false));
        }

        // If we can't parse the response, log a warning and return None
        warn!(
            "Could not parse spam prediction from response: '{}'",
            response
        );
        Ok(None)
    }

    /// Test the connection to OpenAI API
    pub async fn health_check(&self) -> SpamPredictorResult<bool> {
        debug!("Performing OpenAI API health check");

        // Use a minimal request to test connectivity
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        }];

        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(), // Use base model for health check
            messages,
            max_tokens: Some(1),
            temperature: Some(0.0),
            top_p: None,
            stop: None,
            stream: false,
        };

        // Ensure base URL ends with slash for proper joining
        let mut base_url = self.base_url.clone();
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let url = base_url
            .join("chat/completions")
            .map_err(|e| SpamPredictorError::config(format!("Invalid base URL: {}", e)))?;

        match self.client.post(url).json(&request).send().await {
            Ok(response) => {
                let is_healthy = response.status().is_success() || response.status() == 400;
                // 400 is acceptable for health check as it means API is responding

                if is_healthy {
                    debug!("OpenAI API health check passed");
                } else {
                    warn!(
                        "OpenAI API health check failed with status: {}",
                        response.status()
                    );
                }

                Ok(is_healthy)
            }
            Err(e) => {
                warn!("OpenAI API health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get client information for debugging
    pub fn get_info(&self) -> ClientInfo {
        ClientInfo {
            base_url: self.base_url.clone(),
            timeout_seconds: self.timeout.as_secs(),
            has_organization_id: self.organization_id.is_some(),
            default_max_tokens: self.default_max_tokens,
            default_temperature: self.default_temperature,
        }
    }
}

/// Information about the OpenAI client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Base URL for API requests
    pub base_url: Url,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Whether an organization ID is configured
    pub has_organization_id: bool,
    /// Default maximum tokens
    pub default_max_tokens: Option<u32>,
    /// Default temperature
    pub default_temperature: Option<f32>,
}

#[cfg(test)]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    use super::*;

    #[tokio::test]
    async fn client_creation() {
        let client = OpenAiClient::new("sk-test-key".to_string(), None, 30, None).unwrap();

        let info = client.get_info();
        assert_eq!(info.timeout_seconds, 30);
        assert_eq!(info.default_max_tokens, Some(10));
        assert_eq!(info.default_temperature, Some(0.0));
    }

    #[tokio::test]
    async fn parse_spam_responses() {
        let client = OpenAiClient::new("sk-test".to_string(), None, 30, None).unwrap();

        // Test various response formats
        assert_eq!(client.parse_spam_response("true").unwrap(), Some(true));
        assert_eq!(client.parse_spam_response("false").unwrap(), Some(false));
        assert_eq!(client.parse_spam_response("spam").unwrap(), Some(true));
        assert_eq!(client.parse_spam_response("not spam").unwrap(), Some(false));
        assert_eq!(
            client.parse_spam_response("legitimate").unwrap(),
            Some(false)
        );
        assert_eq!(client.parse_spam_response("yes").unwrap(), Some(true));
        assert_eq!(client.parse_spam_response("no").unwrap(), Some(false));

        // Test JSON-like responses
        assert_eq!(
            client.parse_spam_response("\"spam\": true").unwrap(),
            Some(true)
        );
        assert_eq!(
            client.parse_spam_response("\"spam\": false").unwrap(),
            Some(false)
        );

        // Test ambiguous response
        assert_eq!(client.parse_spam_response("maybe").unwrap(), None);
        assert_eq!(client.parse_spam_response("unclear").unwrap(), None);
    }

    #[tokio::test]
    async fn mock_successful_prediction() {
        let mock_server = MockServer::start().await;
        let base_url = Url::parse(&mock_server.uri()).unwrap();

        // Mock successful response
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer sk-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "ft:gpt-4o-2024-08-06:test",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "false"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 50,
                    "completion_tokens": 1,
                    "total_tokens": 51
                }
            })))
            .mount(&mock_server)
            .await;

        let client =
            OpenAiClient::new("sk-test-key".to_string(), Some(base_url), 30, None).unwrap();

        let result = client
            .predict_spam(
                "ft:gpt-4o-2024-08-06:test",
                "Classify as spam or not",
                "Contract data here",
            )
            .await
            .unwrap();

        assert_eq!(result.is_spam, Some(false));
        assert_eq!(result.raw_response, "false");
        assert_eq!(result.model, "ft:gpt-4o-2024-08-06:test");
        assert!(result.token_usage.is_some());
    }

    #[tokio::test]
    async fn mock_error_response() {
        let mock_server = MockServer::start().await;
        let base_url = Url::parse(&mock_server.uri()).unwrap();

        // Mock error response
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "message": "Invalid API key",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            })))
            .mount(&mock_server)
            .await;

        let client =
            OpenAiClient::new("sk-invalid-key".to_string(), Some(base_url), 30, None).unwrap();

        let result = client
            .predict_spam(
                "ft:gpt-4o-2024-08-06:test",
                "Classify as spam or not",
                "Contract data here",
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_auth_error());
    }

    #[tokio::test]
    async fn retry_logic_with_transient_failure() {
        let mock_server = MockServer::start().await;
        let base_url = Url::parse(&mock_server.uri()).unwrap();

        // First request fails with 500, second succeeds
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .up_to_n_times(2) // First two attempts fail
            .expect(2)
            .named("transient_failures")
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "ft:gpt-4o-2024-08-06:test",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "false"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 50,
                    "completion_tokens": 1,
                    "total_tokens": 51
                }
            })))
            .expect(1)
            .named("successful_request")
            .mount(&mock_server)
            .await;

        let client =
            OpenAiClient::new("sk-test-key".to_string(), Some(base_url), 30, None).unwrap();

        // This should succeed after retries
        let result = client
            .predict_spam(
                "ft:gpt-4o-2024-08-06:test",
                "Classify as spam or not",
                "Contract data here",
            )
            .await;

        assert!(result.is_ok());
        let prediction = result.unwrap();
        assert_eq!(prediction.is_spam, Some(false));
    }

    #[tokio::test]
    async fn should_retry_status_classification() {
        // Should retry server errors
        assert!(OpenAiClient::should_retry_status(500));
        assert!(OpenAiClient::should_retry_status(502));
        assert!(OpenAiClient::should_retry_status(503));

        // Should retry rate limits
        assert!(OpenAiClient::should_retry_status(429));

        // Should retry timeouts
        assert!(OpenAiClient::should_retry_status(408));

        // Should not retry client errors
        assert!(!OpenAiClient::should_retry_status(400));
        assert!(!OpenAiClient::should_retry_status(401));
        assert!(!OpenAiClient::should_retry_status(404));

        // Should not retry success
        assert!(!OpenAiClient::should_retry_status(200));
    }
}
