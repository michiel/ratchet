//! JavaScript execution engine for Ratchet
//!
//! This crate provides JavaScript execution capabilities using the Boa engine,
//! including HTTP fetch API integration, error handling, and schema validation.

pub mod conversion;
pub mod error_handling;
pub mod execution;
pub mod http_integration;
pub mod js_task;
pub mod task_loader;
pub mod types;

#[cfg(feature = "http")]
pub mod fetch;

// Re-export main types for convenience
pub use conversion::{convert_js_result_to_json, prepare_input_argument};
pub use error_handling::{parse_js_error, register_error_types};
pub use execution::{execute_js_file, execute_js_with_content};
pub use js_task::JsTaskRunner;
pub use task_loader::{load_and_execute_task, FileSystemTask, TaskLoadError};
pub use types::{ExecutionContext, JsTask};

#[cfg(feature = "http")]
pub use fetch::register_fetch;

// JavaScript error types
use thiserror::Error;

/// JavaScript error types that can be thrown from JS code
#[derive(Error, Debug, Clone)]
pub enum JsErrorType {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Authorization failed: {0}")]
    AuthorizationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Resource not found: {0}")]
    NotFoundError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailableError(String),

    #[error("Data error: {0}")]
    DataError(String),

    #[error("HTTP error (status {status}): {message}")]
    HttpError { status: u16, message: String },

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

/// JavaScript execution errors
#[derive(Error, Debug)]
pub enum JsExecutionError {
    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Compile error: {0}")]
    CompileError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Schema error: {0}")]
    SchemaError(String),

    #[error("Input preparation error: {0}")]
    InputError(String),

    #[error("Output conversion error: {0}")]
    OutputError(String),

    #[error("Invalid output format: {0}")]
    InvalidOutputFormat(String),

    #[error("HTTP integration error: {0}")]
    HttpError(String),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("File read error: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Typed JavaScript error: {0:?}")]
    TypedJsError(JsErrorType),

    #[error("JavaScript error: {error_type} - {message}")]
    JsError { error_type: JsErrorType, message: String },

    #[error("Ratchet error: {0}")]
    RatchetError(#[from] ratchet_core::error::RatchetError),
}
