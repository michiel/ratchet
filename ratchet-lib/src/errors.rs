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

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailableError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Data error: {0}")]
    DataError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

/// Errors that can occur during JavaScript execution
#[derive(Error, Debug)]
pub enum JsExecutionError {
    #[error("Failed to read JavaScript file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to compile JavaScript: {0}")]
    CompileError(String),

    #[error("Failed to execute JavaScript: {0}")]
    ExecutionError(String),

    #[error("JavaScript threw typed error: {0}")]
    TypedJsError(#[from] JsErrorType),

    #[error("Schema validation error: {0}")]
    SchemaValidationError(String),

    #[error("Invalid input schema: {0}")]
    InvalidInputSchema(String),

    #[error("Invalid output schema: {0}")]
    InvalidOutputSchema(String),

    #[error("Invalid output format: {0}")]
    InvalidOutputFormat(String),
}