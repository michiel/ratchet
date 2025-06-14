//! GraphQL error handling using unified error types with sanitization

use ratchet_api_types::ApiError;
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
        match error {
            GraphQLError::Repository(e) => {
                ApiError::internal_error(format!("Database error: {}", e))
            },
            GraphQLError::Registry(e) => {
                ApiError::internal_error(format!("Registry error: {}", e))
            },
        }
    }
}

/// Result type for GraphQL operations using unified error types
pub type Result<T> = std::result::Result<T, ApiError>;