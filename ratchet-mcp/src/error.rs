//! Error types for MCP operations

use std::time::Duration;
use thiserror::Error;

/// Result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;

/// Comprehensive error type for MCP operations
#[derive(Error, Debug)]
pub enum McpError {
    /// Transport-level errors
    #[error("Transport error: {message}")]
    Transport { message: String },

    /// Connection-related errors
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },

    /// Connection timeout error
    #[error("Connection timeout after {timeout:?}")]
    ConnectionTimeout { timeout: Duration },

    /// Protocol-level errors
    #[error("Protocol error: {message}")]
    Protocol { message: String },

    /// Invalid JSON-RPC message
    #[error("Invalid JSON-RPC message: {details}")]
    InvalidJsonRpc { details: String },

    /// MCP method not found
    #[error("Method not found: {method}")]
    MethodNotFound { method: String },

    /// Invalid method parameters
    #[error("Invalid parameters for method {method}: {details}")]
    InvalidParams { method: String, details: String },

    /// Tool not found
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    /// Tool execution error
    #[error("Tool execution failed: {tool_name}: {reason}")]
    ToolExecutionFailed { tool_name: String, reason: String },

    /// Authentication errors
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    /// Authorization/permission errors
    #[error("Authorization denied: {reason}")]
    AuthorizationDenied { reason: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}, retry after {retry_after:?}")]
    RateLimitExceeded {
        message: String,
        retry_after: Option<Duration>,
    },

    /// Resource quota exceeded
    #[error("Resource quota exceeded: {resource}: {message}")]
    QuotaExceeded { resource: String, message: String },

    /// Server timeout
    #[error("Server timeout after {timeout:?}")]
    ServerTimeout { timeout: Duration },

    /// Server unavailable
    #[error("Server unavailable: {reason}")]
    ServerUnavailable { reason: String },

    /// Server error
    #[error("Server error: {message}")]
    ServerError { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {details}")]
    Serialization { details: String },

    /// Internal server error
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// Network-related errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// Security-related errors
    #[error("Security error: {message}")]
    Security { message: String },

    /// Validation errors
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    /// Resource not found
    #[error("Resource not found: {resource_type}: {resource_id}")]
    ResourceNotFound {
        resource_type: String,
        resource_id: String,
    },

    /// Operation cancelled
    #[error("Operation cancelled: {reason}")]
    Cancelled { reason: String },

    /// Generic error with context
    #[error("MCP error: {message}")]
    Generic { message: String },
}

impl McpError {
    /// Create a transport error
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }

    /// Create a connection failed error
    pub fn connection_failed(reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            reason: reason.into(),
        }
    }

    /// Create a protocol error
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol {
            message: message.into(),
        }
    }

    /// Create a tool not found error
    pub fn tool_not_found(tool_name: impl Into<String>) -> Self {
        Self::ToolNotFound {
            tool_name: tool_name.into(),
        }
    }

    /// Create an authentication failed error
    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
        }
    }

    /// Create an authorization denied error
    pub fn authorization_denied(reason: impl Into<String>) -> Self {
        Self::AuthorizationDenied {
            reason: reason.into(),
        }
    }

    /// Create a rate limit exceeded error
    pub fn rate_limit_exceeded(message: impl Into<String>, retry_after: Option<Duration>) -> Self {
        Self::RateLimitExceeded {
            message: message.into(),
            retry_after,
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            McpError::ConnectionTimeout { .. }
            | McpError::ServerTimeout { .. }
            | McpError::ServerUnavailable { .. }
            | McpError::Network { .. }
            | McpError::RateLimitExceeded { .. } => true,

            McpError::AuthenticationFailed { .. }
            | McpError::AuthorizationDenied { .. }
            | McpError::MethodNotFound { .. }
            | McpError::ToolNotFound { .. }
            | McpError::InvalidParams { .. }
            | McpError::InvalidJsonRpc { .. }
            | McpError::Configuration { .. }
            | McpError::Validation { .. } => false,

            _ => false,
        }
    }

    /// Get suggested retry delay for retryable errors
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            McpError::RateLimitExceeded { retry_after, .. } => *retry_after,
            McpError::ConnectionTimeout { .. } => Some(Duration::from_secs(1)),
            McpError::ServerTimeout { .. } => Some(Duration::from_secs(2)),
            McpError::ServerUnavailable { .. } => Some(Duration::from_secs(5)),
            McpError::Network { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }
}

// Implement conversions from common error types
impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::Serialization {
            details: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for McpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            McpError::ServerTimeout {
                timeout: Duration::from_secs(30), // Default timeout
            }
        } else if err.is_connect() {
            McpError::ConnectionFailed {
                reason: err.to_string(),
            }
        } else {
            McpError::Network {
                message: err.to_string(),
            }
        }
    }
}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::TimedOut => McpError::ConnectionTimeout {
                timeout: Duration::from_secs(30),
            },
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected => McpError::ConnectionFailed {
                reason: err.to_string(),
            },
            _ => McpError::Transport {
                message: err.to_string(),
            },
        }
    }
}

impl From<ratchet_core::error::RatchetError> for McpError {
    fn from(err: ratchet_core::error::RatchetError) -> Self {
        McpError::Internal {
            message: format!("Ratchet core error: {}", err),
        }
    }
}

impl From<ratchet_ipc::error::IpcError> for McpError {
    fn from(err: ratchet_ipc::error::IpcError) -> Self {
        McpError::Transport {
            message: format!("IPC error: {}", err),
        }
    }
}
