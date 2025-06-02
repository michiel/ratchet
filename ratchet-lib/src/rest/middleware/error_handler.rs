use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::{
    api::errors::ApiError as UnifiedApiError,
    rest::models::common::ApiError,
    database::{SafeDatabaseError, ErrorCode},
};

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
        // Convert to unified error first, then to legacy format for backward compatibility
        let unified_error = self.to_unified_error();
        let status = StatusCode::from_u16(unified_error.http_status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        
        // For now, convert back to legacy format for compatibility
        let legacy_error = self.to_legacy_error();
        
        (status, Json(legacy_error)).into_response()
    }
}

impl RestError {
    /// Convert to unified API error
    pub fn to_unified_error(&self) -> UnifiedApiError {
        match self {
            RestError::NotFound(msg) => UnifiedApiError::not_found("Resource", msg),
            RestError::BadRequest(msg) => UnifiedApiError::bad_request(msg),
            RestError::InternalError(msg) => UnifiedApiError::internal_error(msg),
            RestError::MethodNotAllowed(msg) => UnifiedApiError::bad_request(format!("Method not allowed: {}", msg)),
            RestError::ServiceUnavailable(msg) => UnifiedApiError::service_unavailable(Some(msg)),
            RestError::Conflict(msg) => UnifiedApiError::conflict("Resource", msg),
            RestError::Timeout(msg) => UnifiedApiError::timeout("Request"),
            RestError::SafeDatabase(safe_err) => {
                match safe_err.code {
                    ErrorCode::NotFound => UnifiedApiError::not_found("Resource", &safe_err.message),
                    ErrorCode::Conflict => UnifiedApiError::conflict("Resource", &safe_err.message),
                    ErrorCode::ValidationError => UnifiedApiError::validation_error("field", &safe_err.message),
                    ErrorCode::Timeout => UnifiedApiError::timeout("Database operation"),
                    _ => UnifiedApiError::internal_error(&safe_err.message),
                }
            },
            RestError::Database(err) => UnifiedApiError::internal_error(format!("Database error: {}", err)),
            RestError::Task(err) => UnifiedApiError::validation_error("task", &err.to_string()),
            RestError::Ratchet(err) => UnifiedApiError::internal_error(format!("System error: {}", err)),
        }
    }
    
    /// Convert to legacy API error for backward compatibility
    pub fn to_legacy_error(&self) -> ApiError {
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
        
        error_response
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