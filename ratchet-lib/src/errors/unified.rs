use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error context for structured error handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub trace_id: String,
    pub span_id: String,
    pub timestamp: DateTime<Utc>,
    pub component: String,
    pub operation: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub user_session: Option<String>,
    pub request_path: Option<String>,
}

impl ErrorContext {
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            component: component.to_string(),
            operation: operation.to_string(),
            metadata: HashMap::new(),
            user_session: None,
            request_path: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = trace_id;
        self
    }

    pub fn with_span_id(mut self, span_id: String) -> Self {
        self.span_id = span_id;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    pub fn with_user_session(mut self, session: String) -> Self {
        self.user_session = Some(session);
        self
    }

    pub fn with_request_path(mut self, path: String) -> Self {
        self.request_path = Some(path);
        self
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    pub fn should_alert(&self) -> bool {
        matches!(self, ErrorSeverity::High | ErrorSeverity::Critical)
    }

    pub fn should_retry(&self) -> bool {
        !matches!(self, ErrorSeverity::Critical)
    }
}

/// Retry information for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryInfo {
    pub should_retry: bool,
    pub retry_after: Option<Duration>,
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub current_attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay: Duration },
    Linear { base: Duration, increment: Duration },
    Exponential { base: Duration, factor: f64, max: Duration },
}

impl Default for RetryInfo {
    fn default() -> Self {
        Self {
            should_retry: false,
            retry_after: None,
            max_attempts: 0,
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(100),
            },
            current_attempt: 0,
        }
    }
}

/// Unified error trait for all Ratchet errors
pub trait RatchetErrorExt: std::error::Error {
    fn error_code(&self) -> &'static str;
    fn is_transient(&self) -> bool;
    fn severity(&self) -> ErrorSeverity;
    fn user_message(&self) -> String;
    fn retry_info(&self) -> RetryInfo;
    fn context(&self) -> Option<&ErrorContext>;
    fn should_log(&self) -> bool { true }
    fn should_notify(&self) -> bool { self.severity().should_alert() }
}

/// Contextual error wrapper
#[derive(Debug, Error)]
pub struct ContextualError<E: std::error::Error + Send + Sync + 'static> {
    #[source]
    pub source: E,
    pub context: ErrorContext,
    pub severity: ErrorSeverity,
    pub retry_info: RetryInfo,
}

impl<E: std::error::Error + Send + Sync + 'static> std::fmt::Display for ContextualError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.context.component, self.context.operation, self.source)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> ContextualError<E> {
    pub fn new(source: E, context: ErrorContext) -> Self {
        Self {
            source,
            context,
            severity: ErrorSeverity::Medium,
            retry_info: RetryInfo::default(),
        }
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_retry_info(mut self, retry_info: RetryInfo) -> Self {
        self.retry_info = retry_info;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.context.metadata.insert(key.to_string(), value.into());
        self
    }
}

/// Categorized error types
#[derive(Debug, Error)]
pub enum TransientError {
    #[error("Database connection lost: {message}")]
    DatabaseConnection { message: String, retry_after: Duration },

    #[error("Network timeout: {message}")]
    NetworkTimeout { message: String, retry_after: Duration },

    #[error("Resource temporarily unavailable: {message}")]
    ResourceBusy { message: String, retry_after: Duration },

    #[error("Rate limit exceeded")]
    RateLimited { retry_after: Duration },

    #[error("Service temporarily unavailable: {message}")]
    ServiceUnavailable { message: String, retry_after: Duration },
}

impl RatchetErrorExt for TransientError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::DatabaseConnection { .. } => "DB_CONN_ERROR",
            Self::NetworkTimeout { .. } => "NETWORK_TIMEOUT",
            Self::ResourceBusy { .. } => "RESOURCE_BUSY",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
        }
    }

    fn is_transient(&self) -> bool {
        true
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::DatabaseConnection { .. } => ErrorSeverity::High,
            Self::NetworkTimeout { .. } => ErrorSeverity::Medium,
            Self::ResourceBusy { .. } => ErrorSeverity::Low,
            Self::RateLimited { .. } => ErrorSeverity::Low,
            Self::ServiceUnavailable { .. } => ErrorSeverity::High,
        }
    }

    fn user_message(&self) -> String {
        match self {
            Self::DatabaseConnection { .. } => "Service temporarily unavailable".to_string(),
            Self::NetworkTimeout { .. } => "Request timed out, please try again".to_string(),
            Self::ResourceBusy { .. } => "Resource is busy, please try again later".to_string(),
            Self::RateLimited { .. } => "Too many requests, please slow down".to_string(),
            Self::ServiceUnavailable { .. } => "Service temporarily unavailable".to_string(),
        }
    }

    fn retry_info(&self) -> RetryInfo {
        let (should_retry, retry_after, max_attempts) = match self {
            Self::DatabaseConnection { retry_after, .. } => (true, Some(*retry_after), 3),
            Self::NetworkTimeout { retry_after, .. } => (true, Some(*retry_after), 5),
            Self::ResourceBusy { retry_after, .. } => (true, Some(*retry_after), 10),
            Self::RateLimited { retry_after } => (true, Some(*retry_after), 1),
            Self::ServiceUnavailable { retry_after, .. } => (true, Some(*retry_after), 3),
        };

        RetryInfo {
            should_retry,
            retry_after,
            max_attempts,
            backoff_strategy: BackoffStrategy::Exponential {
                base: Duration::from_millis(100),
                factor: 2.0,
                max: Duration::from_secs(30),
            },
            current_attempt: 0,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        None
    }
}

#[derive(Debug, Error)]
pub enum PermanentError {
    #[error("Resource not found: {resource_type} with id {resource_id}")]
    NotFound { resource_type: String, resource_id: String },

    #[error("Invalid configuration: {field} = {value}")]
    Configuration { field: String, value: String },

    #[error("Unsupported operation: {operation} on {resource}")]
    Unsupported { operation: String, resource: String },

    #[error("Permission denied: {action} on {resource}")]
    PermissionDenied { action: String, resource: String },

    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation { rule: String },
}

impl RatchetErrorExt for PermanentError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Configuration { .. } => "CONFIG_ERROR",
            Self::Unsupported { .. } => "UNSUPPORTED",
            Self::PermissionDenied { .. } => "PERMISSION_DENIED",
            Self::InvalidInput { .. } => "INVALID_INPUT",
            Self::BusinessRuleViolation { .. } => "BUSINESS_RULE_VIOLATION",
        }
    }

    fn is_transient(&self) -> bool {
        false
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::NotFound { .. } => ErrorSeverity::Low,
            Self::Configuration { .. } => ErrorSeverity::High,
            Self::Unsupported { .. } => ErrorSeverity::Medium,
            Self::PermissionDenied { .. } => ErrorSeverity::Medium,
            Self::InvalidInput { .. } => ErrorSeverity::Low,
            Self::BusinessRuleViolation { .. } => ErrorSeverity::Medium,
        }
    }

    fn user_message(&self) -> String {
        match self {
            Self::NotFound { resource_type, .. } => format!("{} not found", resource_type),
            Self::Configuration { .. } => "System configuration error".to_string(),
            Self::Unsupported { operation, .. } => format!("Operation '{}' is not supported", operation),
            Self::PermissionDenied { .. } => "Permission denied".to_string(),
            Self::InvalidInput { field, reason } => format!("Invalid {}: {}", field, reason),
            Self::BusinessRuleViolation { rule } => format!("Business rule violation: {}", rule),
        }
    }

    fn retry_info(&self) -> RetryInfo {
        RetryInfo {
            should_retry: false,
            ..Default::default()
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        None
    }
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Authorization failed: user {user_id} cannot {action} {resource}")]
    AuthorizationFailed { 
        user_id: String, 
        action: String, 
        resource: String 
    },

    #[error("Security policy violation: {policy}")]
    PolicyViolation { policy: String },

    #[error("Suspicious activity detected: {activity}")]
    SuspiciousActivity { activity: String },

    #[error("Rate limit exceeded for user {user_id}")]
    RateLimitExceeded { user_id: String },
}

impl RatchetErrorExt for SecurityError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::AuthenticationFailed { .. } => "AUTH_FAILED",
            Self::AuthorizationFailed { .. } => "AUTHZ_FAILED",
            Self::PolicyViolation { .. } => "POLICY_VIOLATION",
            Self::SuspiciousActivity { .. } => "SUSPICIOUS_ACTIVITY",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
        }
    }

    fn is_transient(&self) -> bool {
        matches!(self, Self::RateLimitExceeded { .. })
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AuthenticationFailed { .. } => ErrorSeverity::Medium,
            Self::AuthorizationFailed { .. } => ErrorSeverity::Medium,
            Self::PolicyViolation { .. } => ErrorSeverity::High,
            Self::SuspiciousActivity { .. } => ErrorSeverity::Critical,
            Self::RateLimitExceeded { .. } => ErrorSeverity::Low,
        }
    }

    fn user_message(&self) -> String {
        match self {
            Self::AuthenticationFailed { .. } => "Authentication failed".to_string(),
            Self::AuthorizationFailed { .. } => "Access denied".to_string(),
            Self::PolicyViolation { .. } => "Security policy violation".to_string(),
            Self::SuspiciousActivity { .. } => "Access temporarily restricted".to_string(),
            Self::RateLimitExceeded { .. } => "Too many requests, please try again later".to_string(),
        }
    }

    fn retry_info(&self) -> RetryInfo {
        match self {
            Self::RateLimitExceeded { .. } => RetryInfo {
                should_retry: true,
                retry_after: Some(Duration::from_secs(60)),
                max_attempts: 1,
                backoff_strategy: BackoffStrategy::Fixed {
                    delay: Duration::from_secs(60),
                },
                current_attempt: 0,
            },
            _ => RetryInfo::default(),
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        None
    }

    fn should_notify(&self) -> bool {
        true // All security errors should notify
    }
}

/// Macro for creating contextual errors with automatic context
#[macro_export]
macro_rules! contextual_error {
    ($error:expr, $component:expr, $operation:expr) => {{
        let context = $crate::errors::unified::ErrorContext::new($component, $operation);
        $crate::errors::unified::ContextualError::new($error, context)
    }};
    
    ($error:expr, $component:expr, $operation:expr, $severity:expr) => {{
        let context = $crate::errors::unified::ErrorContext::new($component, $operation);
        $crate::errors::unified::ContextualError::new($error, context)
            .with_severity($severity)
    }};
    
    ($error:expr, $component:expr, $operation:expr, $($key:expr => $value:expr),*) => {{
        let mut context = $crate::errors::unified::ErrorContext::new($component, $operation);
        $(
            context = context.with_metadata($key, $value);
        )*
        $crate::errors::unified::ContextualError::new($error, context)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("database", "query")
            .with_metadata("table", "users")
            .with_metadata("query_id", 12345);

        assert_eq!(context.component, "database");
        assert_eq!(context.operation, "query");
        assert_eq!(context.metadata.len(), 2);
        assert_eq!(context.metadata["table"], "users");
    }

    #[test]
    fn test_transient_error() {
        let error = TransientError::DatabaseConnection {
            message: "Connection lost".to_string(),
            retry_after: Duration::from_secs(5),
        };

        assert_eq!(error.error_code(), "DB_CONN_ERROR");
        assert!(error.is_transient());
        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(error.retry_info().should_retry);
    }

    #[test]
    fn test_permanent_error() {
        let error = PermanentError::NotFound {
            resource_type: "Task".to_string(),
            resource_id: "123".to_string(),
        };

        assert_eq!(error.error_code(), "NOT_FOUND");
        assert!(!error.is_transient());
        assert_eq!(error.severity(), ErrorSeverity::Low);
        assert!(!error.retry_info().should_retry);
    }

    #[test]
    fn test_security_error() {
        let error = SecurityError::SuspiciousActivity {
            activity: "Multiple failed login attempts".to_string(),
        };

        assert_eq!(error.error_code(), "SUSPICIOUS_ACTIVITY");
        assert!(!error.is_transient());
        assert_eq!(error.severity(), ErrorSeverity::Critical);
        assert!(error.should_notify());
    }

    #[test]
    fn test_contextual_error() {
        let base_error = PermanentError::NotFound {
            resource_type: "Task".to_string(),
            resource_id: "123".to_string(),
        };

        let contextual = ContextualError::new(
            base_error,
            ErrorContext::new("task_service", "get_task"),
        )
        .with_severity(ErrorSeverity::High)
        .with_metadata("user_id", "user123");

        assert_eq!(contextual.severity, ErrorSeverity::High);
        assert_eq!(contextual.context.component, "task_service");
        assert_eq!(contextual.context.metadata["user_id"], "user123");
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::High);
        assert!(ErrorSeverity::High > ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
        assert!(ErrorSeverity::Low > ErrorSeverity::Info);
    }
}