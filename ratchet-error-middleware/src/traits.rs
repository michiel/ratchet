//! Shared traits for error handling across APIs

use ratchet_api_types::errors::ApiError;
use ratchet_core::validation::error_sanitization::ErrorSanitizer;
use std::sync::OnceLock;

/// Global error sanitizer instance
static GLOBAL_SANITIZER: OnceLock<ErrorSanitizer> = OnceLock::new();

/// Trait for providing error sanitization
pub trait ErrorSanitizationProvider {
    fn get_sanitizer() -> &'static ErrorSanitizer {
        GLOBAL_SANITIZER.get_or_init(|| ErrorSanitizer::default())
    }
    
    /// Initialize the sanitizer with custom configuration
    fn init_sanitizer(sanitizer: ErrorSanitizer) -> Result<(), ErrorSanitizer> {
        GLOBAL_SANITIZER.set(sanitizer)
    }
}

/// Unified trait for converting any error to a sanitized ApiError
pub trait ToSanitizedApiError {
    fn to_sanitized_api_error(&self) -> ApiError;
    fn to_sanitized_api_error_with_context(&self, context: &str) -> ApiError;
}

impl<E: std::error::Error> ToSanitizedApiError for E {
    fn to_sanitized_api_error(&self) -> ApiError {
        let sanitizer = GLOBAL_SANITIZER.get_or_init(|| ErrorSanitizer::default());
        let sanitized = sanitizer.sanitize_error(self);
        
        ApiError::new(
            sanitized.error_code.unwrap_or_else(|| "INTERNAL_ERROR".to_string()),
            sanitized.message
        )
        .with_details(
            sanitized.context
                .map(|ctx| serde_json::to_value(ctx).unwrap_or_default())
                .unwrap_or_default()
        )
    }
    
    fn to_sanitized_api_error_with_context(&self, context: &str) -> ApiError {
        let mut api_error = self.to_sanitized_api_error();
        api_error = api_error.with_path(context.to_string());
        api_error
    }
}

/// Trait for errors that should be retried
pub trait RetryableError {
    fn is_retryable(&self) -> bool;
    fn retry_delay(&self) -> Option<std::time::Duration>;
}

/// Trait for categorizing errors by domain
pub trait ErrorCategory {
    fn category(&self) -> ErrorDomain;
}

/// Error domain categorization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorDomain {
    Authentication,
    Authorization,
    Validation,
    Database,
    Network,
    Configuration,
    Business,
    System,
    Unknown,
}

impl ErrorDomain {
    /// Get default error code for this domain
    pub fn default_code(&self) -> &'static str {
        match self {
            ErrorDomain::Authentication => "AUTHENTICATION_ERROR",
            ErrorDomain::Authorization => "AUTHORIZATION_ERROR", 
            ErrorDomain::Validation => "VALIDATION_ERROR",
            ErrorDomain::Database => "DATABASE_ERROR",
            ErrorDomain::Network => "NETWORK_ERROR",
            ErrorDomain::Configuration => "CONFIG_ERROR",
            ErrorDomain::Business => "BUSINESS_ERROR",
            ErrorDomain::System => "SYSTEM_ERROR",
            ErrorDomain::Unknown => "INTERNAL_ERROR",
        }
    }
    
    /// Get user-friendly suggestions for this domain
    pub fn default_suggestions(&self) -> Vec<String> {
        match self {
            ErrorDomain::Authentication => vec![
                "Check your authentication credentials".to_string(),
                "Verify your API key or token is valid".to_string(),
            ],
            ErrorDomain::Authorization => vec![
                "Verify you have permission for this operation".to_string(),
                "Contact an administrator if needed".to_string(),
            ],
            ErrorDomain::Validation => vec![
                "Check your input format and values".to_string(),
                "Refer to the API documentation".to_string(),
            ],
            ErrorDomain::Database => vec![
                "This appears to be a temporary issue".to_string(),
                "Please try again in a moment".to_string(),
            ],
            ErrorDomain::Network => vec![
                "Check your network connection".to_string(),
                "Retry the operation".to_string(),
            ],
            ErrorDomain::Configuration => vec![
                "Check your configuration settings".to_string(),
                "Contact support if the issue persists".to_string(),
            ],
            ErrorDomain::Business => vec![
                "Check the business rules for this operation".to_string(),
                "Verify the operation is allowed in the current context".to_string(),
            ],
            ErrorDomain::System => vec![
                "This is likely a temporary system issue".to_string(),
                "Please try again later".to_string(),
            ],
            ErrorDomain::Unknown => vec![
                "An unexpected error occurred".to_string(),
                "Contact support if the issue persists".to_string(),
            ],
        }
    }
}

/// Helper trait for enriching ApiErrors with domain-specific information
pub trait EnrichApiError {
    fn with_domain(self, domain: ErrorDomain) -> Self;
    fn with_retry_info(self, retryable: bool, delay: Option<std::time::Duration>) -> Self;
}

impl EnrichApiError for ApiError {
    fn with_domain(mut self, domain: ErrorDomain) -> Self {
        if self.code == "INTERNAL_ERROR" {
            self.code = domain.default_code().to_string();
        }
        
        if self.suggestions.is_none() {
            self = self.with_suggestions(domain.default_suggestions());
        }
        
        self
    }
    
    fn with_retry_info(mut self, retryable: bool, delay: Option<std::time::Duration>) -> Self {
        let mut details = self.details.unwrap_or_default();
        if let Ok(mut obj) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(details) {
            obj.insert("retryable".to_string(), serde_json::Value::Bool(retryable));
            if let Some(delay) = delay {
                obj.insert("retryDelayMs".to_string(), serde_json::Value::Number(
                    serde_json::Number::from(delay.as_millis() as u64)
                ));
            }
            details = serde_json::Value::Object(obj);
        }
        
        self.with_details(details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    
    #[test]
    fn test_to_sanitized_api_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found: /secret/path/config.yaml");
        let api_error = io_error.to_sanitized_api_error();
        
        // Should not contain the sensitive path
        assert!(!api_error.message.contains("/secret/path"));
        assert!(!api_error.code.is_empty());
    }
    
    #[test]
    fn test_error_domain_suggestions() {
        let auth_suggestions = ErrorDomain::Authentication.default_suggestions();
        assert!(!auth_suggestions.is_empty());
        assert!(auth_suggestions.iter().any(|s| s.contains("credentials")));
    }
    
    #[test]
    fn test_enrich_api_error() {
        let api_error = ApiError::new("INTERNAL_ERROR", "Something went wrong");
        let enriched = api_error.with_domain(ErrorDomain::Database);
        
        assert_eq!(enriched.code, "DATABASE_ERROR");
        assert!(enriched.suggestions.is_some());
    }
}