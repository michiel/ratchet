//! Logging interface definitions
//!
//! Provides structured logging interfaces that can be implemented
//! by different logging backends.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Log level enumeration
///
/// Defines the severity levels for log messages, following standard
/// logging conventions from most verbose (Trace) to least verbose (Error).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Detailed tracing information for debugging
    Trace,
    /// Debug information useful during development
    Debug,
    /// Informational messages about normal operation
    Info,
    /// Warning messages about potential issues
    Warn,
    /// Error messages about failures
    Error,
}

impl LogLevel {
    /// Convert log level to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    /// Parse log level from string
    pub fn from_str(s: &str) -> Result<Self, LogLevelParseError> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(LogLevelParseError(s.to_string())),
        }
    }

    /// Check if this level should be logged given a minimum level
    pub fn should_log(&self, min_level: LogLevel) -> bool {
        self >= &min_level
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error when parsing log level from string
#[derive(Debug, thiserror::Error)]
#[error("Invalid log level: '{0}'")]
pub struct LogLevelParseError(String);

/// Structured log event
///
/// Represents a single log entry with structured metadata.
/// This provides a common format for log events across the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// Log level/severity
    pub level: LogLevel,
    /// Primary log message
    pub message: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Source component that generated the log
    pub source: Option<String>,
    /// Structured context data (legacy: used to be called 'context')
    pub context: JsonValue,
    /// Optional target for routing logs
    pub target: Option<String>,
    /// Optional correlation ID for tracing (legacy: used to be called 'correlation_id')
    pub correlation_id: Option<String>,

    // Legacy compatibility fields for ratchet-mcp
    /// Trace ID for distributed tracing
    pub trace_id: Option<String>,
    /// Span ID for distributed tracing
    pub span_id: Option<String>,
    /// Additional structured fields
    pub fields: HashMap<String, JsonValue>,
    /// Logger name/component
    pub logger: Option<String>,
    /// Error information if this is an error log
    pub error: Option<JsonValue>,
}

impl LogEvent {
    /// Create a new log event with minimal information
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp: Utc::now(),
            source: None,
            context: JsonValue::Null,
            target: None,
            correlation_id: None,
            trace_id: None,
            span_id: None,
            fields: HashMap::new(),
            logger: None,
            error: None,
        }
    }

    /// Set the source component
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add context data
    pub fn with_context(mut self, context: JsonValue) -> Self {
        self.context = context;
        self
    }

    /// Set the target for log routing
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set correlation ID for request tracing
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set trace ID for distributed tracing
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Set span ID for distributed tracing
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    /// Set logger name
    pub fn with_logger(mut self, logger: impl Into<String>) -> Self {
        self.logger = Some(logger.into());
        self
    }

    /// Set error information
    pub fn with_error(mut self, error: JsonValue) -> Self {
        self.error = Some(error);
        self
    }

    /// Add a single key-value pair to the context
    pub fn with_field(mut self, key: &str, value: impl Into<JsonValue>) -> Self {
        if self.context.is_null() {
            self.context = JsonValue::Object(serde_json::Map::new());
        }

        if let JsonValue::Object(ref mut map) = self.context {
            map.insert(key.to_string(), value.into());
        }

        self
    }

    /// Add a field to the fields HashMap (for legacy compatibility)
    pub fn with_structured_field(mut self, key: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

/// Structured logger trait
///
/// Defines the interface for structured logging backends.
/// Implementations can route logs to different destinations (console, files, remote systems).
pub trait StructuredLogger: Send + Sync {
    /// Log a structured event
    fn log(&self, event: LogEvent);

    /// Log with level and message (convenience method)
    fn log_simple(&self, level: LogLevel, message: String) {
        self.log(LogEvent::new(level, message));
    }

    /// Log with level, message, and source
    fn log_with_source(&self, level: LogLevel, message: String, source: String) {
        self.log(LogEvent::new(level, message).with_source(source));
    }

    /// Log with full context
    fn log_with_context(&self, level: LogLevel, message: String, context: JsonValue) {
        self.log(LogEvent::new(level, message).with_context(context));
    }

    /// Trace level logging
    fn trace(&self, message: String) {
        self.log_simple(LogLevel::Trace, message);
    }

    /// Debug level logging
    fn debug(&self, message: String) {
        self.log_simple(LogLevel::Debug, message);
    }

    /// Info level logging
    fn info(&self, message: String) {
        self.log_simple(LogLevel::Info, message);
    }

    /// Warn level logging
    fn warn(&self, message: String) {
        self.log_simple(LogLevel::Warn, message);
    }

    /// Error level logging
    fn error(&self, message: String) {
        self.log_simple(LogLevel::Error, message);
    }

    /// Check if a log level should be logged
    fn should_log(&self, _level: LogLevel) -> bool {
        // Default implementation logs everything
        // Implementations should override based on configuration
        true
    }

    /// Flush any buffered log entries
    fn flush(&self) {
        // Default implementation does nothing
    }
}

/// Log context builder for structured logging
///
/// Helps build complex log contexts incrementally.
#[derive(Debug, Default)]
pub struct LogContext {
    fields: HashMap<String, JsonValue>,
}

impl LogContext {
    /// Create a new empty log context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field to the context
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<JsonValue>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Add multiple fields from a HashMap
    pub fn with_fields(mut self, fields: HashMap<String, JsonValue>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Convert to JsonValue for use in LogEvent
    pub fn to_json(self) -> JsonValue {
        JsonValue::Object(self.fields.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("INFO").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warn);

        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_level_should_log() {
        assert!(LogLevel::Error.should_log(LogLevel::Info));
        assert!(LogLevel::Warn.should_log(LogLevel::Info));
        assert!(LogLevel::Info.should_log(LogLevel::Info));
        assert!(!LogLevel::Debug.should_log(LogLevel::Info));
        assert!(!LogLevel::Trace.should_log(LogLevel::Info));
    }

    #[test]
    fn test_log_event_builder() {
        let event = LogEvent::new(LogLevel::Info, "Test message")
            .with_source("test-module")
            .with_field("user_id", "123")
            .with_field("action", "login")
            .with_correlation_id("req-456");

        assert_eq!(event.level, LogLevel::Info);
        assert_eq!(event.message, "Test message");
        assert_eq!(event.source, Some("test-module".to_string()));
        assert_eq!(event.correlation_id, Some("req-456".to_string()));

        if let JsonValue::Object(context) = &event.context {
            assert_eq!(context["user_id"], "123");
            assert_eq!(context["action"], "login");
        } else {
            panic!("Context should be an object");
        }
    }

    #[test]
    fn test_log_context_builder() {
        let context = LogContext::new()
            .with_field("service", "auth")
            .with_field("version", "1.2.3")
            .with_field("environment", "production");

        let json = context.to_json();

        if let JsonValue::Object(obj) = json {
            assert_eq!(obj["service"], "auth");
            assert_eq!(obj["version"], "1.2.3");
            assert_eq!(obj["environment"], "production");
        } else {
            panic!("Should be a JSON object");
        }
    }

    // Mock logger for testing
    struct MockLogger {
        logged_events: std::sync::Mutex<Vec<LogEvent>>,
    }

    impl MockLogger {
        fn new() -> Self {
            Self {
                logged_events: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn get_events(&self) -> Vec<LogEvent> {
            self.logged_events.lock().unwrap().clone()
        }
    }

    impl StructuredLogger for MockLogger {
        fn log(&self, event: LogEvent) {
            self.logged_events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn test_structured_logger_convenience_methods() {
        let logger = MockLogger::new();

        logger.info("Test info message".to_string());
        logger.warn("Test warning".to_string());
        logger.error("Test error".to_string());

        let events = logger.get_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].level, LogLevel::Info);
        assert_eq!(events[1].level, LogLevel::Warn);
        assert_eq!(events[2].level, LogLevel::Error);
    }
}
