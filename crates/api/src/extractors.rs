// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Custom extractors for improved error handling
//!
//! This module provides custom extractors that offer better error messages
//! than the default Axum extractors, particularly for JSON parsing failures.

use axum::{
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use crate::error::ServerError;

mod error_hints {
    pub const ADDRESS_FORMAT: &str = "addresses must be valid hexadecimal strings";
    pub const MISSING_COMMA: &str =
        "check for missing or extra commas between object properties or array elements";
    pub const MISSING_BRACE: &str = "check for missing closing brace '}' for JSON object";
    pub const MISSING_BRACKET: &str = "check for missing closing bracket ']' for JSON array";
    pub const MISSING_QUOTES: &str =
        "check for missing or improperly escaped quotes around string values";
    pub const CONTROL_CHARS: &str = "JSON contains invalid control characters that must be escaped";
    pub const EXPECTED_VALUE: &str =
        "expected a valid JSON value (string, number, boolean, null, object, or array)";
    pub const DEFAULT_SYNTAX: &str = "check JSON formatting and structure";
    pub const EMPTY_BODY: &str = "request body is empty, expected valid JSON";
    pub const TRUNCATED_JSON: &str =
        "unexpected end of JSON input, request appears to be truncated";
}

const MAX_JSON_PAYLOAD_SIZE: usize = 1024 * 1024; // 1MB limit

/// Checks if the error message indicates an address validation error
fn is_address_validation_error(err_msg: &str) -> bool {
    (err_msg.contains("invalid string length")
        || err_msg.contains("odd number of digits")
        || err_msg.contains("invalid character")
        || (err_msg.contains("expected") && err_msg.contains("hex")))
        && (err_msg.contains("line") || err_msg.contains("column"))
}

/// Custom JSON extractor that provides detailed error messages for parsing failures
#[derive(Debug)]
pub struct JsonExtractor<T>(pub T);

impl<T, S> FromRequest<S> for JsonExtractor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Self::extract_json(req, state).await
    }
}

impl<T> JsonExtractor<T>
where
    T: DeserializeOwned,
{
    async fn extract_json<S>(req: Request, state: &S) -> Result<Self, ServerError>
    where
        S: Send + Sync,
    {
        // Validate content-type if present
        if let Some(content_type) = req.headers().get("content-type")
            && let Ok(content_type_str) = content_type.to_str()
            && !content_type_str.starts_with("application/json")
        {
            return Err(ServerError::JsonError {
                message: format!(
                    "invalid content-type: expected 'application/json', got '{content_type_str}'"
                ),
            });
        }

        let bytes = match axum::body::Bytes::from_request(req, state).await {
            Ok(bytes) => bytes,
            Err(rejection) => {
                return Err(ServerError::JsonError {
                    message: format!("failed to read request body: {rejection}"),
                });
            }
        };

        // Check payload size limit
        if bytes.len() > MAX_JSON_PAYLOAD_SIZE {
            return Err(ServerError::JsonError {
                message: format!(
                    "request body too large: {} bytes (max: {} bytes)",
                    bytes.len(),
                    MAX_JSON_PAYLOAD_SIZE
                ),
            });
        }

        // Check for empty body
        if bytes.is_empty() {
            return Err(ServerError::JsonError {
                message: error_hints::EMPTY_BODY.to_string(),
            });
        }

        // Attempt to parse as JSON with detailed error reporting
        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Ok(JsonExtractor(value)),
            Err(err) => {
                let error_message = if err.is_syntax() {
                    let line = err.line();
                    format!(
                        "invalid JSON syntax at line {}, column {}: {}",
                        line,
                        err.column(),
                        get_json_syntax_hint(&err)
                    )
                } else if err.is_data() {
                    format!(
                        "JSON data validation failed: {}",
                        get_data_validation_hint_with_context(&err, &bytes)
                    )
                } else if err.is_eof() {
                    error_hints::TRUNCATED_JSON.to_string()
                } else {
                    format!("JSON parsing error: {err}")
                };

                Err(ServerError::JsonError {
                    message: error_message,
                })
            }
        }
    }
}

impl<T> IntoResponse for JsonExtractor<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

/// Provides helpful hints for JSON syntax errors
fn get_json_syntax_hint(err: &serde_json::Error) -> &'static str {
    let err_msg = err.to_string();

    if err_msg.contains("expected ','") || err_msg.contains("trailing comma") {
        error_hints::MISSING_COMMA
    } else if err_msg.contains("expected '}'") {
        error_hints::MISSING_BRACE
    } else if err_msg.contains("expected ']'") {
        error_hints::MISSING_BRACKET
    } else if err_msg.contains("expected '\"'") {
        error_hints::MISSING_QUOTES
    } else if err_msg.contains("control character") {
        error_hints::CONTROL_CHARS
    } else if err_msg.contains("expected value") {
        error_hints::EXPECTED_VALUE
    } else {
        error_hints::DEFAULT_SYNTAX
    }
}

/// Provides helpful hints for data validation errors with JSON context
fn get_data_validation_hint_with_context(err: &serde_json::Error, raw_json: &[u8]) -> String {
    let err_msg = err.to_string();

    // Check for address validation errors first
    if is_address_validation_error(&err_msg) {
        // Try to extract invalid addresses from the raw JSON
        if let Some(invalid_addresses) = extract_invalid_addresses(raw_json)
            && !invalid_addresses.is_empty()
        {
            return format!(
                "invalid address format - the following addresses are invalid: [{}]. {}",
                invalid_addresses.join(", "),
                error_hints::ADDRESS_FORMAT
            );
        }

        return format!("invalid address format - {}", error_hints::ADDRESS_FORMAT);
    }

    // Fall back to the original function for other errors
    get_data_validation_hint(err)
}

/// Provides helpful hints for data validation errors
fn get_data_validation_hint(err: &serde_json::Error) -> String {
    let err_msg = err.to_string();

    // Check for address validation errors first
    if is_address_validation_error(&err_msg) {
        return format!("invalid address format - {}", error_hints::ADDRESS_FORMAT);
    }

    if err_msg.contains("invalid type") {
        if err_msg.contains("expected string") {
            "expected a string value, but received a different data type".to_string()
        } else if err_msg.contains("expected integer") || err_msg.contains("expected number") {
            "expected a numeric value, but received a different data type".to_string()
        } else if err_msg.contains("expected boolean") {
            "expected a boolean value (true or false), but received a different data type"
                .to_string()
        } else if err_msg.contains("expected array") {
            "expected an array, but received a different data type".to_string()
        } else if err_msg.contains("expected object") {
            "expected a JSON object, but received a different data type".to_string()
        } else {
            format!("data type mismatch: {err_msg}")
        }
    } else if err_msg.contains("missing field") {
        format!("required field is missing: {err_msg}")
    } else if err_msg.contains("unknown field") {
        format!("unrecognized field found: {err_msg}")
    } else {
        err_msg
    }
}

/// Attempts to extract invalid addresses from raw JSON by parsing it loosely
fn extract_invalid_addresses(raw_json: &[u8]) -> Option<Vec<String>> {
    use alloy_primitives::Address;

    // Try to parse as a generic JSON value first
    let json_value: serde_json::Value = serde_json::from_slice(raw_json).ok()?;

    // Look for an "addresses" field
    let addresses = json_value.get("addresses")?.as_array()?;

    let mut invalid_addresses = Vec::new();

    for addr_value in addresses {
        if let Some(addr_str) = addr_value.as_str() {
            // Try to parse as Address - if it fails, it's invalid
            if addr_str.parse::<Address>().is_err() {
                invalid_addresses.push(format!("\"{addr_str}\""));
            }
        }
    }

    Some(invalid_addresses)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderValue, Method},
    };
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        age: u32,
    }

    fn create_request(body: &str) -> Request {
        let mut req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::from(body.to_string()))
            .unwrap();

        req.headers_mut()
            .insert("content-type", HeaderValue::from_static("application/json"));

        req
    }

    #[tokio::test]
    async fn valid_json_parsing() {
        let req = create_request(r#"{"name": "Alice", "age": 30}"#);
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_ok());
        let JsonExtractor(data) = result.unwrap();
        assert_eq!(data.name, "Alice");
        assert_eq!(data.age, 30);
    }

    #[tokio::test]
    async fn empty_body_error() {
        let req = create_request("");
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("request body is empty"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn syntax_error_handling() {
        let req = create_request(r#"{"name": "Alice", "age": 30"#); // Missing closing brace
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                // This specific JSON error is detected as EOF (truncated), not syntax error
                assert!(
                    message.contains("unexpected end of JSON input")
                        || message.contains("invalid JSON syntax")
                        || message.contains("JSON parsing error")
                );
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn syntax_error_with_comma() {
        let req = create_request(r#"{"name": "Alice",, "age": 30}"#); // Extra comma
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(
                    message.contains("invalid JSON syntax")
                        || message.contains("JSON parsing error")
                );
                assert!(message.contains("line"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn data_validation_error() {
        let req = create_request(r#"{"name": "Alice", "age": "thirty"}"#); // Age should be number
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("JSON data validation failed"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn missing_field_error() {
        let req = create_request(r#"{"name": "Alice"}"#); // Missing age field
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("JSON data validation failed"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn invalid_address_error() {
        use alloy_primitives::Address;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct AddressRequest {
            #[allow(dead_code)]
            addresses: Vec<Address>,
        }

        let req = create_request(r#"{"addresses": ["0x123", "invalid_address"]}"#); // Invalid addresses
        let result = JsonExtractor::<AddressRequest>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("invalid address format"));
                assert!(message.contains("the following addresses are invalid"));
                assert!(message.contains("\"0x123\""));
                assert!(message.contains("\"invalid_address\""));
                assert!(message.contains("hexadecimal strings"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn mixed_valid_invalid_addresses() {
        use alloy_primitives::Address;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct AddressRequest {
            #[allow(dead_code)]
            addresses: Vec<Address>,
        }

        // Mix of valid and invalid addresses
        let req = create_request(
            r#"{"addresses": ["0x1234567890123456789012345678901234567890", "0x123", "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd", "invalid"]}"#,
        );
        let result = JsonExtractor::<AddressRequest>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("invalid address format"));
                assert!(message.contains("the following addresses are invalid"));
                assert!(message.contains("\"0x123\""));
                assert!(message.contains("\"invalid\""));
                // Should NOT contain the valid addresses
                assert!(!message.contains("0x1234567890123456789012345678901234567890"));
                assert!(!message.contains("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn large_payload_rejection() {
        let large_body = format!(r#"{{"data": "{}"}}"#, "x".repeat(1024 * 1024)); // >1MB
        let req = create_request(&large_body);
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("request body too large"));
                assert!(message.contains("bytes"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn invalid_content_type() {
        let mut req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::from(r#"{"name": "Alice", "age": 30}"#))
            .unwrap();

        req.headers_mut()
            .insert("content-type", HeaderValue::from_static("text/plain"));

        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                assert!(message.contains("invalid content-type"));
                assert!(message.contains("expected 'application/json'"));
                assert!(message.contains("text/plain"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn unicode_handling() {
        let req = create_request(r#"{"name": "Alice 🦀", "age": 30}"#);
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_ok());
        let JsonExtractor(data) = result.unwrap();
        assert_eq!(data.name, "Alice 🦀");
        assert_eq!(data.age, 30);
    }

    #[tokio::test]
    async fn deeply_nested_json() {
        #[derive(Debug, Deserialize)]
        struct NestedStruct {
            #[allow(dead_code)]
            level1: Level1,
        }

        #[derive(Debug, Deserialize)]
        struct Level1 {
            #[allow(dead_code)]
            level2: Level2,
        }

        #[derive(Debug, Deserialize)]
        struct Level2 {
            #[allow(dead_code)]
            level3: Level3,
        }

        #[derive(Debug, Deserialize)]
        struct Level3 {
            #[allow(dead_code)]
            value: String,
        }

        let nested_json = r#"{"level1": {"level2": {"level3": {"value": "deep"}}}}"#;
        let req = create_request(nested_json);
        let result = JsonExtractor::<NestedStruct>::from_request(req, &()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn malformed_json_with_addresses() {
        use alloy_primitives::Address;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct AddressRequest {
            #[allow(dead_code)]
            addresses: Vec<Address>,
        }

        // JSON with syntax error but containing address-like strings
        let req = create_request(r#"{"addresses": ["0x123" "invalid_addr"]}"#); // Missing comma
        let result = JsonExtractor::<AddressRequest>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                // This JSON is parseable but has invalid address data
                // So it should be detected as a data validation error, not syntax error
                assert!(message.contains("JSON data validation failed"));
                assert!(message.contains("invalid address format"));
            }
            _ => panic!("expected JsonError"),
        }
    }

    #[tokio::test]
    async fn truly_malformed_json() {
        // Test with completely invalid JSON
        let req = create_request("{invalid json");
        let result = JsonExtractor::<TestStruct>::from_request(req, &()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerError::JsonError { message } => {
                // Should detect as syntax error
                assert!(
                    message.contains("invalid JSON syntax")
                        || message.contains("unexpected end of JSON input")
                        || message.contains("JSON parsing error")
                );
            }
            _ => panic!("expected JsonError"),
        }
    }
}
