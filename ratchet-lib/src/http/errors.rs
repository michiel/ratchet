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

    #[error("JavaScript error: {0}")]
    JsError(String),
}

/// Convert a JS error to an HttpError
#[allow(dead_code)]
pub fn js_error_to_http_error(err: boa_engine::JsError) -> HttpError {
    HttpError::JsError(err.to_string())
}
