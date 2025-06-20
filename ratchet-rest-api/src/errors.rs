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
        // Apply selective sanitization - some errors are safe and shouldn't be sanitized
        let (error_code, message) = match self {
            // These error types are safe user-facing errors that don't need sanitization
            RestError::NotFound(msg) => ("NOT_FOUND".to_string(), msg.clone()),
            RestError::BadRequest(msg) => ("BAD_REQUEST".to_string(), msg.clone()),
            RestError::Unauthorized(msg) => ("UNAUTHORIZED".to_string(), msg.clone()),
            RestError::Forbidden(msg) => ("FORBIDDEN".to_string(), msg.clone()),
            RestError::MethodNotAllowed(msg) => ("METHOD_NOT_ALLOWED".to_string(), msg.clone()),
            RestError::Conflict(msg) => ("CONFLICT".to_string(), msg.clone()),
            RestError::Timeout(msg) => ("TIMEOUT".to_string(), msg.clone()),
            RestError::ServiceUnavailable(msg) => ("SERVICE_UNAVAILABLE".to_string(), msg.clone()),
            RestError::Validation { message } => ("VALIDATION_ERROR".to_string(), message.clone()),
            
            // These error types may contain sensitive data and need sanitization
            RestError::InternalError(_) | RestError::Database(_) | RestError::Web(_) | RestError::InputValidation(_) => {
                let sanitizer = ErrorSanitizer::default();
                let sanitized = sanitizer.sanitize_error(self);
                
                let error_code = sanitized.error_code.unwrap_or_else(|| {
                    match self {
                        RestError::InternalError(_) => "INTERNAL_ERROR".to_string(),
                        RestError::Database(_) => "DATABASE_ERROR".to_string(),
                        RestError::Web(_) => "WEB_ERROR".to_string(),
                        RestError::InputValidation(_) => "BAD_REQUEST".to_string(),
                        _ => "INTERNAL_ERROR".to_string(),
                    }
                });
                
                (error_code, sanitized.message)
            }
        };
        
        ApiError::new(error_code, message)
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