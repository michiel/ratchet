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

/// General errors that can occur in Ratchet
#[derive(Error, Debug)]
pub enum RatchetError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("JavaScript execution error: {0}")]
    JsExecution(#[from] JsExecutionError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Watcher error: {0}")]
    WatcherError(String),

    #[error("Load error: {0}")]
    LoadError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for Ratchet operations
pub type Result<T> = std::result::Result<T, RatchetError>;