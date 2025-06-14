//! REST API specific error types and conversions with sanitization

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::errors::ApiError;
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
    /// Convert to unified API error
    pub fn to_unified_error(&self) -> ApiError {
        match self {
            RestError::NotFound(msg) => {
                ApiError::not_found("resource", msg)
            },
            RestError::BadRequest(msg) => {
                ApiError::bad_request(msg.clone())
            },
            RestError::InternalError(msg) => {
                ApiError::internal_error(msg.clone())
            },
            RestError::MethodNotAllowed(msg) => {
                ApiError::bad_request(format!("Method not allowed: {}", msg))
            },
            RestError::ServiceUnavailable(msg) => {
                ApiError::service_unavailable(Some(msg))
            },
            RestError::Conflict(msg) => {
                ApiError::conflict("resource", msg)
            },
            RestError::Timeout(_msg) => ApiError::timeout("Request"),
            RestError::Database(db_err) => {
                ApiError::internal_error(format!("Database error: {}", db_err))
            },
            RestError::Web(web_err) => {
                ApiError::internal_error(web_err.to_string())
            },
            RestError::Validation { message } => {
                ApiError::validation_error("input", message)
            }
        }
    }

    // Common error constructors
    pub fn not_found(resource: &str, id: &str) -> Self {
        RestError::NotFound(format!("{} with ID '{}' not found", resource, id))
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        RestError::BadRequest(message.into())
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