//! Error types for MCP operations

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// MCP error types
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum McpError {
    /// Transport-related errors
    #[error("Transport error: {message}")]
    Transport { message: String },

    /// Protocol errors (invalid JSON-RPC, etc.)
    #[error("Protocol error: {message}")]
    Protocol { message: String },

    /// Authentication/authorization errors
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Authorization errors
    #[error("Authorization failed: {message}")]
    Authorization { message: String },

    /// Tool not found
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },

    /// Tool execution errors
    #[error("Tool execution failed: {tool}: {message}")]
    ToolExecution { tool: String, message: String },

    /// Server timeout
    #[error("Server timeout after {timeout:?}")]
    ServerTimeout { timeout: Duration },

    /// Client timeout
    #[error("Client timeout after {timeout:?}")]
    ClientTimeout { timeout: Duration },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Session errors
    #[error("Session error: {message}")]
    Session { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Rate limit exceeded with details
    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { 
        message: String,
        retry_after: Option<u64>,
    },

    /// Network errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// I/O errors
    #[error("I/O error: {message}")]
    Io { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Connection errors
    #[error("Connection error: {message}")]
    Connection { message: String },

    /// Connection failed
    #[error("Connection failed: {message}")]
    ConnectionFailed { message: String },

    /// Connection timeout
    #[error("Connection timeout: {message}")]
    ConnectionTimeout { message: String },

    /// Internal server errors
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl McpError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            McpError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            McpError::Authorization { .. } => StatusCode::FORBIDDEN,
            McpError::ToolNotFound { .. } => StatusCode::NOT_FOUND,
            McpError::Validation { .. } => StatusCode::BAD_REQUEST,
            McpError::Protocol { .. } => StatusCode::BAD_REQUEST,
            McpError::Configuration { .. } => StatusCode::BAD_REQUEST,
            McpError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            McpError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            McpError::Network { .. } => StatusCode::SERVICE_UNAVAILABLE,
            McpError::ServerTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            McpError::ClientTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            McpError::Connection { .. } => StatusCode::SERVICE_UNAVAILABLE,
            McpError::ConnectionFailed { .. } => StatusCode::SERVICE_UNAVAILABLE,
            McpError::ConnectionTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            McpError::Transport { .. } => StatusCode::SERVICE_UNAVAILABLE,
            McpError::ToolExecution { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            McpError::Session { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            McpError::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            McpError::Serialization { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            McpError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code for JSON-RPC responses
    pub fn error_code(&self) -> i32 {
        match self {
            McpError::Protocol { .. } => -32600, // Invalid Request
            McpError::ToolNotFound { .. } => -32601, // Method not found
            McpError::Validation { .. } => -32602, // Invalid params
            McpError::Authentication { .. } => -32000, // Server error (auth)
            McpError::Authorization { .. } => -32000, // Server error (authz)
            McpError::RateLimit { .. } => -32000, // Server error (rate limit)
            McpError::RateLimitExceeded { .. } => -32000, // Server error (rate limit)
            McpError::ToolExecution { .. } => -32000, // Server error (execution)
            _ => -32603, // Internal error
        }
    }

    /// Create a sanitized error message for external clients
    pub fn client_message(&self) -> String {
        match self {
            McpError::Authentication { .. } => "Authentication required".to_string(),
            McpError::Authorization { .. } => "Access denied".to_string(),
            McpError::ToolNotFound { name } => format!("Tool '{}' not found", name),
            McpError::Validation { message } => message.clone(),
            McpError::Protocol { message } => message.clone(),
            McpError::RateLimit { .. } => "Rate limit exceeded".to_string(),
            McpError::RateLimitExceeded { .. } => "Rate limit exceeded".to_string(),
            McpError::ServerTimeout { .. } => "Request timeout".to_string(),
            McpError::ClientTimeout { .. } => "Request timeout".to_string(),
            _ => "Internal server error".to_string(),
        }
    }
}

/// Error response for HTTP endpoints
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: Option<i32>,
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for McpError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: self.client_message(),
            code: Some(self.error_code()),
            details: None,
        };

        (status, Json(error_response)).into_response()
    }
}

// Standard error conversions
impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        McpError::Io {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<url::ParseError> for McpError {
    fn from(err: url::ParseError) -> Self {
        McpError::Configuration {
            message: format!("Invalid URL: {}", err),
        }
    }
}

impl From<tokio::time::error::Elapsed> for McpError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        McpError::ServerTimeout {
            timeout: Duration::from_secs(30),
        }
    }
}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::Internal {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            McpError::Authentication {
                message: "test".to_string()
            }
            .status_code(),
            StatusCode::UNAUTHORIZED
        );

        assert_eq!(
            McpError::Authorization {
                message: "test".to_string()
            }
            .status_code(),
            StatusCode::FORBIDDEN
        );

        assert_eq!(
            McpError::ToolNotFound {
                name: "test".to_string()
            }
            .status_code(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            McpError::Protocol {
                message: "test".to_string()
            }
            .error_code(),
            -32600
        );

        assert_eq!(
            McpError::ToolNotFound {
                name: "test".to_string()
            }
            .error_code(),
            -32601
        );
    }

    #[test]
    fn test_client_messages() {
        let auth_error = McpError::Authentication {
            message: "Invalid token".to_string(),
        };
        assert_eq!(auth_error.client_message(), "Authentication required");

        let tool_error = McpError::ToolNotFound {
            name: "test_tool".to_string(),
        };
        assert_eq!(tool_error.client_message(), "Tool 'test_tool' not found");
    }
}