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
        let error_response = match &self {
            WebError::Validation { errors } => {
                json!({
                    "error": {
                        "code": self.error_code(),
                        "message": self.to_string(),
                        "details": errors
                    }
                })
            }
            _ => {
                json!({
                    "error": {
                        "code": self.error_code(),
                        "message": self.to_string()
                    }
                })
            }
        };

        (status, Json(error_response)).into_response()
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