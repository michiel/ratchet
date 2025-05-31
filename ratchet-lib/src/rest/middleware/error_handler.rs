use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::rest::models::common::ApiError;
use crate::database::{SafeDatabaseError, ErrorCode};

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
    
    #[error("Safe database error")]
    SafeDatabase(#[from] SafeDatabaseError),
    
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("Task error: {0}")]
    Task(#[from] crate::task::TaskError),
    
    #[error("Ratchet error: {0}")]
    Ratchet(#[from] crate::errors::RatchetError),
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            RestError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ApiError::not_found(msg),
            ),
            RestError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ApiError::bad_request(msg),
            ),
            RestError::MethodNotAllowed(msg) => (
                StatusCode::METHOD_NOT_ALLOWED,
                ApiError::method_not_allowed(msg),
            ),
            RestError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::internal_error(msg),
            ),
            RestError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ApiError::service_unavailable(msg),
            ),
            RestError::Conflict(msg) => (
                StatusCode::CONFLICT,
                ApiError::conflict(msg),
            ),
            RestError::Timeout(msg) => (
                StatusCode::REQUEST_TIMEOUT,
                ApiError::timeout(msg),
            ),
            RestError::SafeDatabase(safe_err) => {
                let status_code = match safe_err.code {
                    ErrorCode::NotFound => StatusCode::NOT_FOUND,
                    ErrorCode::Conflict => StatusCode::CONFLICT,
                    ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
                    ErrorCode::Timeout => StatusCode::REQUEST_TIMEOUT,
                };

                let api_error = ApiError {
                    error: safe_err.message.clone(),
                    error_code: Some(format!("{:?}", safe_err.code)),
                    request_id: Some(safe_err.request_id.clone()),
                    timestamp: safe_err.timestamp,
                    path: None, // Will be set by the calling handler if needed
                    #[cfg(debug_assertions)]
                    debug_info: safe_err.debug_info,
                };

                (status_code, api_error)
            },
            RestError::Database(err) => {
                // Convert raw database errors to safe errors
                let safe_err = SafeDatabaseError::from(err);
                let status_code = match safe_err.code {
                    ErrorCode::NotFound => StatusCode::NOT_FOUND,
                    ErrorCode::Conflict => StatusCode::CONFLICT,
                    ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
                    ErrorCode::Timeout => StatusCode::REQUEST_TIMEOUT,
                };

                let api_error = ApiError {
                    error: safe_err.message.clone(),
                    error_code: Some(format!("{:?}", safe_err.code)),
                    request_id: Some(safe_err.request_id.clone()),
                    timestamp: safe_err.timestamp,
                    path: None, // Will be set by the calling handler if needed
                    #[cfg(debug_assertions)]
                    debug_info: safe_err.debug_info,
                };

                (status_code, api_error)
            },
            RestError::Task(err) => (
                StatusCode::BAD_REQUEST,
                ApiError::bad_request(format!("Task error: {}", err)),
            ),
            RestError::Ratchet(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::internal_error(format!("System error: {}", err)),
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

/// Generic error handler function
pub async fn handle_error(err: RestError) -> impl IntoResponse {
    tracing::error!("REST API error: {}", err);
    err.into_response()
}

/// Convert any error that implements Display into a RestError
pub fn internal_error<E: std::fmt::Display>(err: E) -> RestError {
    RestError::InternalError(err.to_string())
}

/// Convert database errors to RestError
pub fn db_error(err: sea_orm::DbErr) -> RestError {
    RestError::Database(err)
}