//! Web-specific error types and conversions
//!
//! This module provides error types that integrate well with HTTP APIs
//! and can be converted to appropriate HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::errors::ApiError;
use ratchet_core::validation::error_sanitization::ErrorSanitizer;
use serde_json::json;
use thiserror::Error;

/// Web-specific error type for HTTP API operations
#[derive(Debug, Error)]
pub enum WebError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Too many requests: {message}")]
    TooManyRequests { message: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Validation error: {errors:?}")]
    Validation { errors: Vec<ValidationError> },

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Request timeout")]
    Timeout,
}

/// Validation error details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

/// Result type for web operations
pub type WebResult<T> = Result<T, WebError>;

impl WebError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            WebError::BadRequest { .. } | WebError::Validation { .. } => StatusCode::BAD_REQUEST,
            WebError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            WebError::Forbidden { .. } => StatusCode::FORBIDDEN,
            WebError::NotFound { .. } => StatusCode::NOT_FOUND,
            WebError::Conflict { .. } => StatusCode::CONFLICT,
            WebError::TooManyRequests { .. } | WebError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            WebError::Timeout => StatusCode::REQUEST_TIMEOUT,
            WebError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            WebError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            WebError::BadRequest { .. } => "BAD_REQUEST",
            WebError::Unauthorized { .. } => "UNAUTHORIZED",
            WebError::Forbidden { .. } => "FORBIDDEN",
            WebError::NotFound { .. } => "NOT_FOUND",
            WebError::Conflict { .. } => "CONFLICT",
            WebError::TooManyRequests { .. } | WebError::RateLimit => "RATE_LIMITED",
            WebError::Timeout => "TIMEOUT",
            WebError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            WebError::Internal { .. } => "INTERNAL_ERROR",
            WebError::Validation { .. } => "VALIDATION_ERROR",
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        
        // Apply sanitization for sensitive errors before creating response
        let (error_code, safe_message) = match &self {
            // For internal errors, use sanitization
            WebError::Internal { .. } => {
                let sanitizer = ErrorSanitizer::default();
                let sanitized = sanitizer.sanitize_error(&self);
                let error_code = sanitized.error_code.unwrap_or_else(|| self.error_code().to_string());
                (error_code, sanitized.message)
            }
            // For other errors, use the original error code and message
            _ => (self.error_code().to_string(), self.to_string())
        };
        
        let error_response = match &self {
            WebError::Validation { errors } => {
                json!({
                    "error": {
                        "code": error_code,
                        "message": safe_message,
                        "details": errors
                    }
                })
            }
            _ => {
                json!({
                    "error": {
                        "code": error_code,
                        "message": safe_message
                    }
                })
            }
        };

        (status, Json(error_response)).into_response()
    }
}

// Conversion from WebError to ApiError with sanitization
impl From<WebError> for ApiError {
    fn from(error: WebError) -> Self {
        // Apply selective sanitization - some errors are safe and shouldn't be sanitized
        let (error_code, message) = match &error {
            // These error types are safe user-facing errors that don't need sanitization
            WebError::BadRequest { message } => ("BAD_REQUEST".to_string(), message.clone()),
            WebError::Unauthorized { message } => ("UNAUTHORIZED".to_string(), message.clone()),
            WebError::Forbidden { message } => ("FORBIDDEN".to_string(), message.clone()),
            WebError::NotFound { message } => ("NOT_FOUND".to_string(), message.clone()),
            WebError::Conflict { message } => ("CONFLICT".to_string(), message.clone()),
            WebError::TooManyRequests { message } => ("RATE_LIMITED".to_string(), message.clone()),
            WebError::ServiceUnavailable { message } => ("SERVICE_UNAVAILABLE".to_string(), message.clone()),
            WebError::RateLimit => ("RATE_LIMITED".to_string(), "Rate limit exceeded".to_string()),
            WebError::Timeout => ("TIMEOUT".to_string(), "Request timeout".to_string()),
            WebError::Validation { errors } => {
                // Validation errors are generally safe, but sanitize details
                let safe_errors: Vec<_> = errors.iter().map(|e| format!("{}: {}", e.field.as_deref().unwrap_or("field"), e.message)).collect();
                ("VALIDATION_ERROR".to_string(), safe_errors.join(", "))
            },

            // These error types may contain sensitive data and need sanitization
            WebError::Internal { .. } => {
                let sanitizer = ErrorSanitizer::default();
                let sanitized = sanitizer.sanitize_error(&error);
                
                let error_code = sanitized.error_code.unwrap_or_else(|| "INTERNAL_ERROR".to_string());
                (error_code, sanitized.message)
            }
        };

        ApiError::new(error_code, message)
    }
}

// Conversion from ApiError to WebError
impl From<ApiError> for WebError {
    fn from(api_error: ApiError) -> Self {
        match api_error.code.as_str() {
            "BAD_REQUEST" => WebError::BadRequest {
                message: api_error.message,
            },
            "UNAUTHORIZED" => WebError::Unauthorized {
                message: api_error.message,
            },
            "FORBIDDEN" => WebError::Forbidden {
                message: api_error.message,
            },
            "NOT_FOUND" => WebError::NotFound {
                message: api_error.message,
            },
            "CONFLICT" => WebError::Conflict {
                message: api_error.message,
            },
            "RATE_LIMITED" => WebError::TooManyRequests {
                message: api_error.message,
            },
            "TIMEOUT" => WebError::Timeout,
            "SERVICE_UNAVAILABLE" => WebError::ServiceUnavailable {
                message: api_error.message,
            },
            "VALIDATION_ERROR" => {
                // Try to parse details as validation errors
                let errors = api_error
                    .details
                    .and_then(|details| serde_json::from_value(details).ok())
                    .unwrap_or_else(|| {
                        vec![ValidationError {
                            field: None,
                            message: api_error.message.clone(),
                            code: "VALIDATION_FAILED".to_string(),
                        }]
                    });
                WebError::Validation { errors }
            }
            _ => WebError::Internal {
                message: api_error.message,
            },
        }
    }
}

// Common error constructors
impl WebError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        WebError::BadRequest {
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        WebError::Unauthorized {
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        WebError::Forbidden {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        WebError::NotFound {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        WebError::Conflict {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        WebError::Internal {
            message: message.into(),
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        WebError::ServiceUnavailable {
            message: message.into(),
        }
    }

    pub fn validation(errors: Vec<ValidationError>) -> Self {
        WebError::Validation { errors }
    }

    pub fn validation_single(field: Option<String>, message: String, code: String) -> Self {
        WebError::Validation {
            errors: vec![ValidationError { field, message, code }],
        }
    }
}
