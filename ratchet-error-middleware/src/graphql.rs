//! GraphQL-specific error handling utilities

#[cfg(feature = "graphql")]
use async_graphql::{Error as GraphQLError, ErrorExtensions};
use ratchet_api_types::errors::ApiError;
use crate::traits::ToSanitizedApiError;

/// Extension trait for enhancing GraphQL errors with sanitization
#[cfg(feature = "graphql")]
pub trait GraphQLErrorExtensions {
    fn to_sanitized_graphql_error(&self) -> GraphQLError;
    fn with_graphql_extensions(self, api_error: &ApiError) -> Self;
}

#[cfg(feature = "graphql")]
impl<E: std::error::Error> GraphQLErrorExtensions for E {
    fn to_sanitized_graphql_error(&self) -> GraphQLError {
        let api_error = self.to_sanitized_api_error();
        let mut error = GraphQLError::new(api_error.message.clone());
        
        error = error.extend_with(|_, e| {
            e.set("code", api_error.code.clone());
            
            if let Some(request_id) = &api_error.request_id {
                e.set("requestId", request_id.clone());
            }
            
            if let Some(path) = &api_error.path {
                e.set("path", path.clone());
            }
            
            if let Some(details) = &api_error.details {
                e.set("details", details.to_string());
            }
            
            if let Some(suggestions) = &api_error.suggestions {
                e.set("suggestions", suggestions.clone());
            }
            
            e.set("timestamp", api_error.timestamp.to_rfc3339());
        });
        
        error
    }
    
    fn with_graphql_extensions(self, api_error: &ApiError) -> Self {
        // This is a marker method - the actual implementation
        // would depend on the specific error type
        self
    }
}

#[cfg(feature = "graphql")]
impl From<ApiError> for GraphQLError {
    fn from(api_error: ApiError) -> Self {
        let mut error = GraphQLError::new(api_error.message.clone());
        
        error = error.extend_with(|_, e| {
            e.set("code", api_error.code.clone());
            
            if let Some(request_id) = &api_error.request_id {
                e.set("requestId", request_id.clone());
            }
            
            if let Some(path) = &api_error.path {
                e.set("path", path.clone());
            }
            
            if let Some(details) = &api_error.details {
                // Convert details to a string representation for GraphQL
                match details {
                    serde_json::Value::Object(obj) => {
                        for (key, value) in obj {
                            e.set(key.clone(), value.to_string());
                        }
                    }
                    _ => {
                        e.set("details", details.to_string());
                    }
                }
            }
            
            if let Some(suggestions) = &api_error.suggestions {
                e.set("suggestions", suggestions.clone());
            }
            
            e.set("timestamp", api_error.timestamp.to_rfc3339());
        });
        
        error
    }
}

/// GraphQL error formatter that applies sanitization
#[cfg(feature = "graphql")]
pub fn sanitized_error_formatter(err: GraphQLError) -> GraphQLError {
    // If the error doesn't have our sanitization markers,
    // apply sanitization to the message
    if !err.extensions.as_ref().map(|ext| ext.contains_key("code")).unwrap_or(false) {
        let error_message = err.message.clone();
        let sanitizer = crate::traits::ErrorSanitizationProvider::get_sanitizer();
        let sanitized = sanitizer.sanitize_message(&error_message);
        
        let mut new_error = GraphQLError::new(sanitized.message);
        new_error = new_error.extend_with(|_, e| {
            if let Some(code) = sanitized.error_code {
                e.set("code", code);
            }
            
            if let Some(context) = sanitized.context {
                for (key, value) in context {
                    e.set(key, value);
                }
            }
        });
        
        // Preserve original extensions
        if let Some(extensions) = err.extensions {
            new_error = new_error.extend_with(|_, e| {
                for (key, value) in extensions {
                    e.set(key, value);
                }
            });
        }
        
        new_error
    } else {
        err
    }
}

/// Helper for creating GraphQL field errors with sanitization
#[cfg(feature = "graphql")]
pub fn create_field_error(
    field_name: &str,
    error: impl std::error::Error,
    path: Option<&str>,
) -> GraphQLError {
    let api_error = error.to_sanitized_api_error()
        .with_path(path.unwrap_or(field_name).to_string());
    
    api_error.into()
}

/// Middleware for GraphQL request context
#[cfg(feature = "graphql")]
pub struct GraphQLErrorContext {
    pub request_id: String,
    pub operation_name: Option<String>,
    pub user_id: Option<String>,
}

#[cfg(feature = "graphql")]
impl GraphQLErrorContext {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            operation_name: None,
            user_id: None,
        }
    }
    
    pub fn with_operation(mut self, operation_name: String) -> Self {
        self.operation_name = Some(operation_name);
        self
    }
    
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

#[cfg(test)]
#[cfg(feature = "graphql")]
mod tests {
    use super::*;
    use std::io;
    
    #[test]
    fn test_graphql_error_conversion() {
        let io_error = io::Error::new(
            io::ErrorKind::NotFound, 
            "file not found: /secret/path/config.yaml"
        );
        
        let graphql_error = io_error.to_sanitized_graphql_error();
        
        // Should not contain sensitive path
        assert!(!graphql_error.message.contains("/secret/path"));
        
        // Should have error code extension
        assert!(graphql_error.extensions.as_ref()
            .map(|ext| ext.contains_key("code"))
            .unwrap_or(false));
    }
    
    #[test]
    fn test_api_error_to_graphql() {
        let api_error = ApiError::new("TEST_ERROR", "Test message")
            .with_request_id("req-123")
            .with_path("/test/path");
        
        let graphql_error: GraphQLError = api_error.into();
        
        assert_eq!(graphql_error.message, "Test message");
        
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap(), "TEST_ERROR");
        assert_eq!(extensions.get("requestId").unwrap(), "req-123");
        assert_eq!(extensions.get("path").unwrap(), "/test/path");
    }
}