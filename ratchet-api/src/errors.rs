//! Unified error handling for both REST and GraphQL APIs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Result type for API operations
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// Unified API error type
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Optional detailed error information
    pub details: Option<serde_json::Value>,
    
    /// Suggestions for resolving the error
    pub suggestions: Vec<String>,
    
    /// HTTP status code for REST API
    #[serde(skip)]
    pub status_code: u16,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestions: Vec::new(),
            status_code: 500,
            metadata: HashMap::new(),
        }
    }
    
    /// Set the HTTP status code
    pub fn with_status(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }
    
    /// Add detailed information
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    /// Add suggestions for resolving the error
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
    
    /// Add a single suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self.status_code, 429 | 500 | 502 | 503 | 504)
    }
    
    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self.status_code {
            400..=499 => ErrorCategory::Client,
            500..=599 => ErrorCategory::Server,
            _ => ErrorCategory::Unknown,
        }
    }
}

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Client,
    Server,
    Unknown,
}

// Predefined error constructors
impl ApiError {
    /// Bad request error (400)
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message).with_status(400)
    }
    
    /// Unauthorized error (401)
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new("UNAUTHORIZED", message)
            .with_status(401)
            .with_suggestion("Check your authentication credentials")
    }
    
    /// Forbidden error (403)
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new("FORBIDDEN", message)
            .with_status(403)
            .with_suggestion("Check your permissions for this resource")
    }
    
    /// Not found error (404)
    pub fn not_found(resource: impl Into<String>) -> Self {
        let resource = resource.into();
        Self::new("NOT_FOUND", format!("{} not found", resource))
            .with_status(404)
            .with_suggestion(format!("Verify the {} identifier is correct", resource))
    }
    
    /// Conflict error (409)
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("CONFLICT", message)
            .with_status(409)
            .with_suggestion("Resolve the conflicting state and try again")
    }
    
    /// Validation error (422)
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
            .with_status(422)
            .with_suggestion("Check the request data format and required fields")
    }
    
    /// Rate limit exceeded error (429)
    pub fn rate_limit_exceeded() -> Self {
        Self::new("RATE_LIMIT_EXCEEDED", "Too many requests")
            .with_status(429)
            .with_suggestion("Wait before making another request")
    }
    
    /// Internal server error (500)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
            .with_status(500)
            .with_suggestion("Try again later or contact support if the problem persists")
    }
    
    /// Service unavailable error (503)
    pub fn service_unavailable(service: impl Into<String>) -> Self {
        Self::new("SERVICE_UNAVAILABLE", format!("{} service is temporarily unavailable", service.into()))
            .with_status(503)
            .with_suggestion("Try again in a few moments")
    }
    
    /// Timeout error (504)
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::new("TIMEOUT", format!("{} timed out", operation.into()))
            .with_status(504)
            .with_suggestion("Try again with a smaller request or contact support")
    }
}

// Convert from storage errors
impl From<ratchet_storage::StorageError> for ApiError {
    fn from(err: ratchet_storage::StorageError) -> Self {
        match err {
            ratchet_storage::StorageError::NotFound => {
                ApiError::not_found("Resource")
            }
            ratchet_storage::StorageError::DuplicateKey(msg) => {
                ApiError::conflict(format!("Duplicate entry: {}", msg))
            }
            ratchet_storage::StorageError::ValidationFailed(msg) => {
                ApiError::validation_error(msg)
            }
            ratchet_storage::StorageError::ConnectionFailed(msg) => {
                ApiError::service_unavailable("Database")
                    .with_details(serde_json::json!({"connection_error": msg}))
            }
            ratchet_storage::StorageError::QueryFailed(msg) => {
                ApiError::internal_error("Database query failed")
                    .with_details(serde_json::json!({"query_error": msg}))
            }
            ratchet_storage::StorageError::TransactionFailed(msg) => {
                ApiError::internal_error("Database transaction failed")
                    .with_details(serde_json::json!({"transaction_error": msg}))
            }
            ratchet_storage::StorageError::ConcurrencyError(msg) => {
                ApiError::conflict(format!("Concurrency conflict: {}", msg))
                    .with_suggestion("Try the operation again")
            }
            _ => ApiError::internal_error(err.to_string()),
        }
    }
}

// Convert from core errors
impl From<ratchet_core::RatchetError> for ApiError {
    fn from(err: ratchet_core::RatchetError) -> Self {
        match err {
            ratchet_core::RatchetError::Task(task_err) => {
                match task_err {
                    ratchet_core::error::TaskError::NotFound(id) => {
                        ApiError::not_found(format!("Task {}", id))
                    }
                    ratchet_core::error::TaskError::Disabled(id) => {
                        ApiError::bad_request(format!("Task {} is disabled", id))
                    }
                    ratchet_core::error::TaskError::Deprecated(msg) => {
                        ApiError::bad_request(format!("Task is deprecated: {}", msg))
                    }
                    ratchet_core::error::TaskError::ValidationFailed(msg) => {
                        ApiError::validation_error(msg)
                    }
                    _ => ApiError::internal_error(task_err.to_string()),
                }
            }
            ratchet_core::RatchetError::Execution(exec_err) => {
                match exec_err {
                    ratchet_core::error::ExecutionError::NotFound(id) => {
                        ApiError::not_found(format!("Execution {}", id))
                    }
                    ratchet_core::error::ExecutionError::Timeout(seconds) => {
                        ApiError::timeout("Task execution")
                            .with_details(serde_json::json!({"timeout_seconds": seconds}))
                    }
                    ratchet_core::error::ExecutionError::Cancelled => {
                        ApiError::bad_request("Execution was cancelled")
                    }
                    _ => ApiError::internal_error(exec_err.to_string()),
                }
            }
            ratchet_core::RatchetError::Validation(val_err) => {
                ApiError::validation_error(val_err.to_string())
            }
            ratchet_core::RatchetError::Config(config_err) => {
                ApiError::internal_error(format!("Configuration error: {}", config_err))
            }
            ratchet_core::RatchetError::Storage(storage_err) => {
                // Convert storage error to string and create internal error
                ApiError::internal_error(format!("Storage error: {}", storage_err))
            }
            _ => ApiError::internal_error(err.to_string()),
        }
    }
}

/// Error response for REST API
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ApiError,
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: ApiError, request_id: Option<String>) -> Self {
        Self {
            error,
            request_id,
            timestamp: chrono::Utc::now(),
        }
    }
}

// GraphQL integration (disabled for now)
// #[cfg(feature = "graphql")]
// impl From<ApiError> for async_graphql::Error {
//     fn from(err: ApiError) -> Self {
//         let mut error = async_graphql::Error::new(err.message.clone());
//         error = error.extend_with(|_, e| {
//             e.set("code", err.code.clone());
//             if let Some(details) = &err.details {
//                 e.set("details", details.clone());
//             }
//             if !err.suggestions.is_empty() {
//                 e.set("suggestions", err.suggestions.clone());
//             }
//             for (key, value) in &err.metadata {
//                 e.set(key, value.clone());
//             }
//         });
//         error
//     }
// }
// 
// #[cfg(feature = "graphql")]
// impl From<async_graphql::Error> for ApiError {
//     fn from(err: async_graphql::Error) -> Self {
//         ApiError::internal_error(err.message)
//             .with_details(serde_json::json!({
//                 "graphql_error": true,
//                 "path": err.path,
//                 "locations": err.locations
//             }))
//     }
// }

// Axum integration for REST API (disabled for now)
// #[cfg(feature = "rest")]
// impl axum::response::IntoResponse for ApiError {
//     fn into_response(self) -> axum::response::Response {
//         use axum::response::Json;
//         use axum::http::StatusCode;
//         
//         let status = StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
//         let response = ErrorResponse::new(self, None);
//         
//         (status, Json(response)).into_response()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_error_creation() {
        let error = ApiError::bad_request("Invalid input")
            .with_suggestion("Check the input format");
        
        assert_eq!(error.code, "BAD_REQUEST");
        assert_eq!(error.message, "Invalid input");
        assert_eq!(error.status_code, 400);
        assert_eq!(error.suggestions.len(), 1);
        assert_eq!(error.category(), ErrorCategory::Client);
    }
    
    #[test]
    fn test_error_metadata() {
        let error = ApiError::internal_error("Test error")
            .with_metadata("custom_field", serde_json::json!("custom_value"));
        
        assert!(error.metadata.contains_key("custom_field"));
    }
    
    #[test]
    fn test_error_retryable() {
        assert!(ApiError::rate_limit_exceeded().is_retryable());
        assert!(ApiError::internal_error("Server error").is_retryable());
        assert!(!ApiError::bad_request("Invalid request").is_retryable());
        assert!(!ApiError::not_found("Resource").is_retryable());
    }
    
    #[test]
    fn test_storage_error_conversion() {
        let storage_error = ratchet_storage::StorageError::NotFound;
        let api_error = ApiError::from(storage_error);
        
        assert_eq!(api_error.code, "NOT_FOUND");
        assert_eq!(api_error.status_code, 404);
    }
    
    #[test]
    fn test_error_response_serialization() {
        let error = ApiError::validation_error("Missing required field");
        let response = ErrorResponse::new(error, Some("req-123".to_string()));
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("VALIDATION_ERROR"));
        assert!(json.contains("req-123"));
    }
}