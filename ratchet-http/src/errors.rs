//! HTTP error types

use crate::types::HttpMethodError;

/// Error type for HTTP operations
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(#[from] HttpMethodError),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid header name: {0}")]
    InvalidHeaderName(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Recording error: {0}")]
    RecordingError(String),
}
