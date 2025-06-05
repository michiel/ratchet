//! Runtime error types

use thiserror::Error;

/// JavaScript execution errors
#[derive(Error, Debug)]
pub enum JsExecutionError {
    #[error("File read error: {0}")]
    FileReadError(#[from] std::io::Error),
    
    #[error("Compile error: {0}")]
    CompileError(String),
    
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("Typed JavaScript error: {0:?}")]
    TypedJsError(JsErrorType),
    
    #[error("Schema validation error: {0}")]
    SchemaValidationError(String),
    
    #[error("HTTP error: {0}")]
    HttpError(String),
    
    #[error("Invalid output format: {0}")]
    InvalidOutputFormat(String),
}

/// JavaScript error types that can be thrown from within JavaScript code
#[derive(Debug, Clone)]
pub enum JsErrorType {
    AuthenticationError(String),
    AuthorizationError(String),
    NetworkError(String),
    HttpError { status: u16, message: String },
    ValidationError(String),
    ConfigurationError(String),
    RateLimitError(String),
    ServiceUnavailableError(String),
    TimeoutError(String),
    DataError(String),
    UnknownError(String),
}

impl std::fmt::Display for JsErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsErrorType::AuthenticationError(msg) => write!(f, "AuthenticationError: {}", msg),
            JsErrorType::AuthorizationError(msg) => write!(f, "AuthorizationError: {}", msg),
            JsErrorType::NetworkError(msg) => write!(f, "NetworkError: {}", msg),
            JsErrorType::HttpError { status, message } => write!(f, "HttpError: {} - {}", status, message),
            JsErrorType::ValidationError(msg) => write!(f, "ValidationError: {}", msg),
            JsErrorType::ConfigurationError(msg) => write!(f, "ConfigurationError: {}", msg),
            JsErrorType::RateLimitError(msg) => write!(f, "RateLimitError: {}", msg),
            JsErrorType::ServiceUnavailableError(msg) => write!(f, "ServiceUnavailableError: {}", msg),
            JsErrorType::TimeoutError(msg) => write!(f, "TimeoutError: {}", msg),
            JsErrorType::DataError(msg) => write!(f, "DataError: {}", msg),
            JsErrorType::UnknownError(msg) => write!(f, "UnknownError: {}", msg),
        }
    }
}