//! Standardized error handling framework for all Ratchet modules
//!
//! This module provides traits and implementations for consistent error handling
//! across all Ratchet components.

use std::collections::HashMap;
use std::time::Duration;

/// Standard error metadata that all Ratchet errors should provide
#[derive(Debug, Clone)]
pub struct ErrorMetadata {
    /// Unique error code for programmatic handling
    pub code: String,
    /// HTTP status code for web APIs
    pub http_status: u16,
    /// Whether this error is retryable
    pub retryable: bool,
    /// Suggested retry delay if retryable
    pub retry_delay: Option<Duration>,
    /// Error category for grouping
    pub category: ErrorCategory,
    /// Additional context fields
    pub context: HashMap<String, String>,
}

/// Standard error categories
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Client-side errors (user input, validation, etc.)
    Client,
    /// Server-side errors (internal failures, database issues, etc.)
    Server,
    /// Network/transport errors (connectivity, timeouts)
    Network,
    /// Authentication/authorization errors
    Security,
    /// Configuration errors
    Configuration,
    /// Resource not found errors
    NotFound,
    /// Rate limiting errors
    RateLimit,
    /// Validation errors
    Validation,
}

impl ErrorCategory {
    /// Get the default HTTP status code for this category
    pub fn default_http_status(&self) -> u16 {
        match self {
            ErrorCategory::Client => 400,
            ErrorCategory::Server => 500,
            ErrorCategory::Network => 503,
            ErrorCategory::Security => 401,
            ErrorCategory::Configuration => 500,
            ErrorCategory::NotFound => 404,
            ErrorCategory::RateLimit => 429,
            ErrorCategory::Validation => 400,
        }
    }

    /// Check if errors in this category are typically retryable
    pub fn is_typically_retryable(&self) -> bool {
        matches!(self, ErrorCategory::Network | ErrorCategory::Server | ErrorCategory::RateLimit)
    }
}

/// Trait that all Ratchet errors should implement for consistent behavior
pub trait StandardizedError: std::error::Error + Send + Sync {
    /// Get error metadata
    fn metadata(&self) -> ErrorMetadata;

    /// Get error code for programmatic handling
    fn error_code(&self) -> String {
        self.metadata().code
    }

    /// Get HTTP status code for web responses
    fn http_status(&self) -> u16 {
        self.metadata().http_status
    }

    /// Check if this error is retryable
    fn is_retryable(&self) -> bool {
        self.metadata().retryable
    }

    /// Get suggested retry delay if retryable
    fn retry_delay(&self) -> Option<Duration> {
        self.metadata().retry_delay
    }

    /// Get error category
    fn category(&self) -> ErrorCategory {
        self.metadata().category
    }

    /// Get additional context
    fn context(&self) -> HashMap<String, String> {
        self.metadata().context
    }

    /// Check if this error should be sanitized in user-facing messages
    fn should_sanitize(&self) -> bool {
        matches!(self.category(), ErrorCategory::Server | ErrorCategory::Configuration)
    }

    /// Get a user-friendly error message (potentially sanitized)
    fn user_message(&self) -> String {
        if self.should_sanitize() {
            match self.category() {
                ErrorCategory::Server => "An internal server error occurred".to_string(),
                ErrorCategory::Configuration => "A configuration error occurred".to_string(),
                _ => self.to_string(),
            }
        } else {
            self.to_string()
        }
    }

    /// Get suggestions for fixing this error
    fn suggestions(&self) -> Vec<String> {
        match self.category() {
            ErrorCategory::Client => vec![
                "Check your input parameters".to_string(),
                "Refer to the API documentation".to_string(),
            ],
            ErrorCategory::Server => vec![
                "Try again later".to_string(),
                "Contact support if the problem persists".to_string(),
            ],
            ErrorCategory::Network => vec![
                "Check your network connection".to_string(),
                "Retry the operation".to_string(),
            ],
            ErrorCategory::Security => vec![
                "Check your authentication credentials".to_string(),
                "Verify you have the required permissions".to_string(),
            ],
            ErrorCategory::Configuration => vec![
                "Check the configuration settings".to_string(),
                "Verify all required values are provided".to_string(),
            ],
            ErrorCategory::NotFound => vec![
                "Verify the resource ID is correct".to_string(),
                "Check if the resource still exists".to_string(),
            ],
            ErrorCategory::RateLimit => vec![
                "Reduce the frequency of requests".to_string(),
                "Wait before retrying".to_string(),
            ],
            ErrorCategory::Validation => vec![
                "Check the format of your input".to_string(),
                "Ensure all required fields are provided".to_string(),
            ],
        }
    }
}

/// Helper trait for converting errors to the unified API error format
/// Note: This would be implemented when integrating with ratchet-api-types
pub trait ToApiError {
    /// Convert this error to a string representation for API responses
    fn to_api_message(&self) -> String;
}

impl<T: StandardizedError> ToApiError for T {
    fn to_api_message(&self) -> String {
        format!("{}: {}", self.error_code(), self.user_message())
    }
}

/// Helper macro for easily implementing StandardizedError (simplified version)
/// Usage example in tests below

/// Standard error result type
pub type StandardResult<T, E> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use thiserror::Error;

    #[derive(Debug, Error)]
    enum TestError {
        #[error("Not found: {resource}")]
        NotFound { resource: String },
        #[error("Network timeout")]
        NetworkTimeout,
        #[error("Internal error: {message}")]
        Internal { message: String },
    }

    impl StandardizedError for TestError {
        fn metadata(&self) -> ErrorMetadata {
            let (code, category, retryable, retry_delay, http_status) = match self {
                TestError::NotFound { .. } => (
                    "NOT_FOUND", ErrorCategory::NotFound, false, None, 404
                ),
                TestError::NetworkTimeout => (
                    "NETWORK_TIMEOUT", ErrorCategory::Network, true, Some(Duration::from_secs(1)), 503
                ),
                TestError::Internal { .. } => (
                    "INTERNAL_ERROR", ErrorCategory::Server, false, None, 500
                ),
            };

            ErrorMetadata {
                code: code.to_string(),
                http_status,
                retryable,
                retry_delay,
                category,
                context: HashMap::new(),
            }
        }
    }

    #[test]
    fn test_error_metadata() {
        let error = TestError::NotFound {
            resource: "user".to_string(),
        };
        
        assert_eq!(error.error_code(), "NOT_FOUND");
        assert_eq!(error.http_status(), 404);
        assert!(!error.is_retryable());
        assert_eq!(error.category(), ErrorCategory::NotFound);
    }

    #[test]
    fn test_retryable_error() {
        let error = TestError::NetworkTimeout;
        
        assert_eq!(error.error_code(), "NETWORK_TIMEOUT");
        assert!(error.is_retryable());
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(1)));
        assert_eq!(error.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_error_sanitization() {
        let error = TestError::Internal {
            message: "Database connection string: postgres://user:pass@host/db".to_string(),
        };
        
        assert!(error.should_sanitize());
        assert_eq!(error.user_message(), "An internal server error occurred");
    }

    #[test]
    fn test_error_suggestions() {
        let error = TestError::NotFound {
            resource: "user".to_string(),
        };
        
        let suggestions = error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("resource ID")));
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(ErrorCategory::Client.default_http_status(), 400);
        assert_eq!(ErrorCategory::Server.default_http_status(), 500);
        assert_eq!(ErrorCategory::NotFound.default_http_status(), 404);
        assert!(ErrorCategory::Network.is_typically_retryable());
        assert!(!ErrorCategory::Client.is_typically_retryable());
    }
}