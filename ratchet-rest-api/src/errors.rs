//! REST API specific error types and conversions

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::errors::ApiError;
use ratchet_interfaces::DatabaseError;
use ratchet_web::{WebError, WebResult};
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
                "timestamp": unified_error.timestamp
            }
        });

        (status, Json(error_response)).into_response()
    }
}

impl RestError {
    /// Convert to unified API error
    pub fn to_unified_error(&self) -> ApiError {
        match self {
            RestError::NotFound(msg) => ApiError::not_found("Resource", msg),
            RestError::BadRequest(msg) => ApiError::bad_request(msg),
            RestError::InternalError(msg) => ApiError::internal_error(msg),
            RestError::MethodNotAllowed(msg) => {
                ApiError::bad_request(format!("Method not allowed: {}", msg))
            }
            RestError::ServiceUnavailable(msg) => ApiError::service_unavailable(Some(msg)),
            RestError::Conflict(msg) => ApiError::conflict("Resource", msg),
            RestError::Timeout(_msg) => ApiError::timeout("Request"),
            RestError::Database(db_err) => match db_err {
                DatabaseError::NotFound { entity, id } => {
                    ApiError::not_found(entity, id)
                }
                DatabaseError::Constraint { message } => {
                    ApiError::conflict("Database", message)
                }
                DatabaseError::Validation { message } => {
                    ApiError::validation_error("field", message)
                }
                _ => ApiError::internal_error(&db_err.to_string()),
            },
            RestError::Web(web_err) => {
                // Convert WebError to ApiError
                match web_err {
                    WebError::NotFound { message } => ApiError::not_found("Resource", message),
                    WebError::BadRequest { message } => ApiError::bad_request(message),
                    WebError::Unauthorized { message } => ApiError::unauthorized(Some(message)),
                    WebError::Forbidden { message } => ApiError::forbidden(Some(message)),
                    WebError::Conflict { message } => ApiError::conflict("Resource", message),
                    WebError::Internal { message } => ApiError::internal_error(message),
                    WebError::ServiceUnavailable { message } => {
                        ApiError::service_unavailable(Some(message))
                    }
                    WebError::RateLimit => ApiError::rate_limited(None),
                    WebError::Timeout => ApiError::timeout("Request"),
                    WebError::Validation { errors } => {
                        let first_error = errors.first();
                        if let Some(error) = first_error {
                            ApiError::validation_error(
                                error.field.as_deref().unwrap_or("field"),
                                &error.message,
                            )
                        } else {
                            ApiError::validation_error("field", "Validation failed")
                        }
                    }
                    _ => ApiError::internal_error(&web_err.to_string()),
                }
            }
            RestError::Validation { message } => {
                ApiError::validation_error("request", message)
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