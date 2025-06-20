//! GraphQL error handling using unified error types with sanitization

use ratchet_api_types::ApiError;
use ratchet_core::validation::error_sanitization::ErrorSanitizer;
use thiserror::Error;

// Re-export the unified error types for consistency
pub use ratchet_api_types::ApiError as UnifiedApiError;

/// GraphQL-specific error wrapper for external errors
#[derive(Error, Debug)]
pub enum GraphQLError {
    #[error("Repository error: {0}")]
    Repository(#[from] ratchet_interfaces::DatabaseError),

    #[error("Registry error: {0}")]
    Registry(#[from] ratchet_interfaces::RegistryError),
}

impl From<GraphQLError> for ApiError {
    fn from(error: GraphQLError) -> Self {
        // Apply error sanitization to prevent sensitive data leakage
        let sanitizer = ErrorSanitizer::default();
        let sanitized = sanitizer.sanitize_error(&error);

        // Use sanitized error message and code
        ApiError::new(
            sanitized.error_code.unwrap_or_else(|| match error {
                GraphQLError::Repository(_) => "DATABASE_ERROR".to_string(),
                GraphQLError::Registry(_) => "REGISTRY_ERROR".to_string(),
            }),
            sanitized.message,
        )
    }
}

/// Result type for GraphQL operations using unified error types
pub type Result<T> = std::result::Result<T, ApiError>;
