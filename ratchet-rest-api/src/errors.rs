//! REST API specific error types and conversions with sanitization

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::errors::ApiError;
use ratchet_core::validation::{error_sanitization::ErrorSanitizer, InputValidationError};
use ratchet_interfaces::DatabaseError;
use ratchet_web::WebError;
use serde_json::json;
use thiserror::Error;

/// REST API specific error type
#[derive(Error, Debug)]
pub enum RestError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Database error")]
    Database(#[from] DatabaseError),

    #[error("Web error")]
    Web(#[from] WebError),

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Input validation error")]
    InputValidation(#[from] InputValidationError),
}

/// Result type for REST operations
pub type RestResult<T> = Result<T, RestError>;

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        // Convert to unified error first, then to HTTP response
        let unified_error = self.to_unified_error();
        let status = StatusCode::from_u16(unified_error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let error_response = json!({
            "error": {
                "code": unified_error.code,
                "message": unified_error.message,
                "status": status.as_u16()
            }
        });
        (status, Json(error_response)).into_response()
    }
}

impl RestError {
    /// Convert to unified API error with sanitization
    pub fn to_unified_error(&self) -> ApiError {
        // Apply error sanitization to prevent sensitive data leakage
        let sanitizer = ErrorSanitizer::default();
        let sanitized = sanitizer.sanitize_error(self);
        
        // Use sanitized error message and determine appropriate error code
        let error_code = sanitized.error_code.unwrap_or_else(|| {
            match self {
                RestError::NotFound(_) => "NOT_FOUND".to_string(),
                RestError::BadRequest(_) => "BAD_REQUEST".to_string(),
                RestError::Unauthorized(_) => "UNAUTHORIZED".to_string(),
                RestError::Forbidden(_) => "FORBIDDEN".to_string(),
                RestError::InternalError(_) => "INTERNAL_ERROR".to_string(),
                RestError::MethodNotAllowed(_) => "METHOD_NOT_ALLOWED".to_string(),
                RestError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE".to_string(),
                RestError::Conflict(_) => "CONFLICT".to_string(),
                RestError::Timeout(_) => "TIMEOUT".to_string(),
                RestError::Database(_) => "DATABASE_ERROR".to_string(),
                RestError::Web(_) => "WEB_ERROR".to_string(),
                RestError::Validation { .. } => "VALIDATION_ERROR".to_string(),
                RestError::InputValidation(_) => "BAD_REQUEST".to_string(),
            }
        });
        
        ApiError::new(error_code, sanitized.message)
    }

    // Common error constructors
    pub fn not_found(resource: &str, id: &str) -> Self {
        RestError::NotFound(format!("{} with ID '{}' not found", resource, id))
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        RestError::BadRequest(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        RestError::Unauthorized(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        RestError::Forbidden(message.into())
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        RestError::InternalError(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        RestError::Conflict(message.into())
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        RestError::Validation {
            message: message.into(),
        }
    }
}

/// Convert any error that implements Display into a RestError
pub fn internal_error<E: std::fmt::Display>(err: E) -> RestError {
    RestError::InternalError(err.to_string())
}

/// Convert database errors to RestError
pub fn db_error(err: DatabaseError) -> RestError {
    RestError::Database(err)
}

/// Convert web errors to RestError
pub fn web_error(err: WebError) -> RestError {
    RestError::Web(err)
}