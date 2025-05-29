use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

use crate::rest::models::common::ApiError;

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
    
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
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
            RestError::Database(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::internal_error(format!("Database error: {}", err)),
            ),
            RestError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::internal_error(msg),
            ),
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