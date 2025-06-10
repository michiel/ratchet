//! GraphQL-specific error handling

use async_graphql::{Error as GraphQLError, ErrorExtensions, Result as GraphQLResult};
use thiserror::Error;

/// GraphQL API errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Repository error: {0}")]
    Repository(#[from] ratchet_interfaces::DatabaseError),
    
    #[error("Registry error: {0}")]  
    Registry(#[from] ratchet_interfaces::RegistryError),
    
    #[error("Validation error: {0}")]
    Validation(String),
}

impl ErrorExtensions for ApiError {
    fn extend(&self) -> GraphQLError {
        GraphQLError::new(format!("{}", self)).extend_with(|_, e| {
            match self {
                ApiError::NotFound(_) => {
                    e.set("code", "NOT_FOUND");
                    e.set("type", "CLIENT_ERROR");
                }
                ApiError::InvalidInput(_) => {
                    e.set("code", "INVALID_INPUT");
                    e.set("type", "CLIENT_ERROR");
                }
                ApiError::PermissionDenied(_) => {
                    e.set("code", "PERMISSION_DENIED");
                    e.set("type", "CLIENT_ERROR");
                }
                ApiError::InternalError(_) => {
                    e.set("code", "INTERNAL_ERROR");
                    e.set("type", "SERVER_ERROR");
                }
                ApiError::Repository(_) => {
                    e.set("code", "REPOSITORY_ERROR");
                    e.set("type", "SERVER_ERROR");
                }
                ApiError::Registry(_) => {
                    e.set("code", "REGISTRY_ERROR");
                    e.set("type", "SERVER_ERROR");
                }
                ApiError::Validation(_) => {
                    e.set("code", "VALIDATION_ERROR");
                    e.set("type", "CLIENT_ERROR");
                }
            }
        })
    }
}

/// Result type for GraphQL operations
pub type ApiResult<T> = GraphQLResult<T, ApiError>;

// Note: Direct From implementations for external types are not allowed due to orphan rules.
// Error conversion is handled through the ApiError enum instead.