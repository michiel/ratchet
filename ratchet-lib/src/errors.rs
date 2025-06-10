use thiserror::Error;

pub mod unified;

pub use unified::{
    BackoffStrategy, ContextualError, ErrorContext, ErrorSeverity, PermanentError, RatchetErrorExt,
    RetryInfo, SecurityError, TransientError,
};

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

/// Convert from ratchet-core's validation errors to JS execution errors
impl From<ratchet_core::error::RatchetError> for JsExecutionError {
    fn from(err: ratchet_core::error::RatchetError) -> Self {
        match err {
            ratchet_core::error::RatchetError::Validation(validation_err) => match validation_err {
                ratchet_core::error::ValidationError::SchemaValidation(msg) => {
                    JsExecutionError::SchemaValidationError(msg)
                }
                ratchet_core::error::ValidationError::InvalidFormat(msg) => {
                    JsExecutionError::InvalidInputSchema(msg)
                }
                ratchet_core::error::ValidationError::InputValidation(msg) => {
                    JsExecutionError::InvalidInputSchema(msg)
                }
                ratchet_core::error::ValidationError::OutputValidation(msg) => {
                    JsExecutionError::InvalidOutputSchema(msg)
                }
                ratchet_core::error::ValidationError::RequiredFieldMissing(msg) => {
                    JsExecutionError::SchemaValidationError(format!(
                        "Required field missing: {}",
                        msg
                    ))
                }
            },
            ratchet_core::error::RatchetError::Io(io_err) => {
                JsExecutionError::FileReadError(io_err)
            }
            other => JsExecutionError::ExecutionError(format!("Core error: {}", other)),
        }
    }
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

    #[error("Registry error: {0}")]
    Registry(#[from] ratchet_registry::RegistryError),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for Ratchet operations
pub type Result<T> = std::result::Result<T, RatchetError>;

impl RatchetError {
    /// Convert to a log event for structured logging
    pub fn to_log_event(&self, context: &ratchet_logging::LogContext) -> ratchet_logging::LogEvent {
        use ratchet_logging::{ErrorInfo, LogEvent, LogLevel};

        let event = LogEvent::new(LogLevel::Error, self.to_string())
            .with_logger("ratchet.error")
            .with_trace_id(context.trace_id.clone())
            .with_span_id(context.span_id.clone())
            .with_fields(context.fields.clone());

        let error_info = ErrorInfo {
            error_type: self.error_type(),
            error_code: self.error_code(),
            message: self.to_string(),
            severity: self.severity().into(),
            is_retryable: self.is_retryable(),
            stack_trace: None, // Backtrace capture can be expensive, enable only in debug mode
            context: self.get_error_context(),
            suggestions: self.get_suggestions(),
            related_errors: Vec::new(),
        };

        event.with_error(error_info)
    }

    fn error_type(&self) -> String {
        match self {
            Self::Io(_) => "IoError",
            Self::TaskNotFound(_) => "TaskNotFound",
            Self::NotImplemented(_) => "NotImplemented",
            Self::JsExecution(_) => "JsExecutionError",
            Self::Database(_) => "DatabaseError",
            Self::Configuration(_) => "ConfigurationError",
            Self::Validation(_) => "ValidationError",
            Self::WatcherError(_) => "WatcherError",
            Self::LoadError(_) => "LoadError",
            Self::Registry(_) => "RegistryError",
            Self::Other(_) => "Other",
        }
        .to_string()
    }

    fn error_code(&self) -> String {
        match self {
            Self::Io(_) => "IO_ERROR",
            Self::TaskNotFound(_) => "TASK_NOT_FOUND",
            Self::NotImplemented(_) => "NOT_IMPLEMENTED",
            Self::JsExecution(e) => match e {
                JsExecutionError::FileReadError(_) => "JS_FILE_READ_ERROR",
                JsExecutionError::CompileError(_) => "JS_COMPILE_ERROR",
                JsExecutionError::ExecutionError(_) => "JS_EXECUTION_ERROR",
                JsExecutionError::TypedJsError(_) => "JS_TYPED_ERROR",
                JsExecutionError::SchemaValidationError(_) => "JS_SCHEMA_VALIDATION_ERROR",
                JsExecutionError::InvalidInputSchema(_) => "JS_INVALID_INPUT_SCHEMA",
                JsExecutionError::InvalidOutputSchema(_) => "JS_INVALID_OUTPUT_SCHEMA",
                JsExecutionError::InvalidOutputFormat(_) => "JS_INVALID_OUTPUT_FORMAT",
            },
            Self::Database(_) => "DATABASE_ERROR",
            Self::Configuration(_) => "CONFIG_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::WatcherError(_) => "WATCHER_ERROR",
            Self::LoadError(_) => "LOAD_ERROR",
            Self::Registry(_) => "REGISTRY_ERROR",
            Self::Other(_) => "OTHER_ERROR",
        }
        .to_string()
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Io(_) => ErrorSeverity::High,
            Self::TaskNotFound(_) => ErrorSeverity::Medium,
            Self::NotImplemented(_) => ErrorSeverity::Low,
            Self::JsExecution(_) => ErrorSeverity::High,
            Self::Database(_) => ErrorSeverity::High,
            Self::Configuration(_) => ErrorSeverity::Critical,
            Self::Validation(_) => ErrorSeverity::Medium,
            Self::WatcherError(_) => ErrorSeverity::Medium,
            Self::LoadError(_) => ErrorSeverity::High,
            Self::Registry(_) => ErrorSeverity::Medium,
            Self::Other(_) => ErrorSeverity::Medium,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Io(_) => true,
            Self::Database(_) => true,
            Self::Registry(_) => true,
            Self::JsExecution(e) => matches!(
                e,
                JsExecutionError::TypedJsError(JsErrorType::NetworkError(_))
                    | JsExecutionError::TypedJsError(JsErrorType::TimeoutError(_))
                    | JsExecutionError::TypedJsError(JsErrorType::ServiceUnavailableError(_))
            ),
            _ => false,
        }
    }

    fn get_error_context(&self) -> std::collections::HashMap<String, serde_json::Value> {
        use serde_json::json;
        let mut context = std::collections::HashMap::new();

        match self {
            Self::TaskNotFound(task) => {
                context.insert("task_name".to_string(), json!(task));
            }
            Self::JsExecution(e) => {
                context.insert("js_error_type".to_string(), json!(e.to_string()));
            }
            _ => {}
        }

        context
    }

    fn get_suggestions(&self) -> ratchet_logging::ErrorSuggestions {
        let mut suggestions = ratchet_logging::ErrorSuggestions::default();

        match self {
            Self::TaskNotFound(task) => {
                suggestions
                    .immediate
                    .push(format!("Check if task '{}' exists in the registry", task));
                suggestions
                    .immediate
                    .push("Run 'ratchet list' to see available tasks".to_string());
            }
            Self::Configuration(msg) => {
                suggestions
                    .immediate
                    .push("Check your configuration file for errors".to_string());
                suggestions
                    .immediate
                    .push(format!("Configuration issue: {}", msg));
                suggestions
                    .preventive
                    .push("Validate configuration on startup".to_string());
            }
            Self::Database(msg) => {
                suggestions
                    .immediate
                    .push("Check database connectivity".to_string());
                suggestions
                    .immediate
                    .push(format!("Database error: {}", msg));
                suggestions
                    .preventive
                    .push("Implement connection pooling and retries".to_string());
            }
            _ => {}
        }

        suggestions
    }
}

/// Convert from ratchet_lib ErrorSeverity to ratchet_logging ErrorSeverity
impl From<ErrorSeverity> for ratchet_logging::ErrorSeverity {
    fn from(severity: ErrorSeverity) -> Self {
        match severity {
            ErrorSeverity::Info => ratchet_logging::ErrorSeverity::Info,
            ErrorSeverity::Low => ratchet_logging::ErrorSeverity::Low,
            ErrorSeverity::Medium => ratchet_logging::ErrorSeverity::Medium,
            ErrorSeverity::High => ratchet_logging::ErrorSeverity::High,
            ErrorSeverity::Critical => ratchet_logging::ErrorSeverity::Critical,
        }
    }
}
