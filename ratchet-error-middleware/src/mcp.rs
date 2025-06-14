//! MCP-specific error handling utilities

use ratchet_api_types::errors::ApiError;
use serde::{Deserialize, Serialize};
use crate::traits::ToSanitizedApiError;

/// MCP JSON-RPC error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP error response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    pub error: McpJsonRpcError,
}

impl McpErrorResponse {
    pub fn new(id: Option<serde_json::Value>, error: McpJsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error,
        }
    }
}

/// Convert ApiError to MCP JSON-RPC error
impl From<ApiError> for McpJsonRpcError {
    fn from(api_error: ApiError) -> Self {
        let code = match api_error.code.as_str() {
            "PARSE_ERROR" => -32700,
            "INVALID_REQUEST" => -32600,
            "METHOD_NOT_FOUND" => -32601,
            "INVALID_PARAMS" => -32602,
            "INTERNAL_ERROR" => -32603,
            "AUTHENTICATION_FAILED" => -32000,
            "AUTHORIZATION_DENIED" => -32001,
            "RATE_LIMITED" => -32002,
            "VALIDATION_ERROR" => -32003,
            "NOT_FOUND" => -32004,
            "CONFLICT" => -32005,
            "TIMEOUT" => -32006,
            "SERVICE_UNAVAILABLE" => -32007,
            _ => -32000, // Server error
        };
        
        let mut data = serde_json::Map::new();
        
        if let Some(request_id) = api_error.request_id {
            data.insert("requestId".to_string(), serde_json::Value::String(request_id));
        }
        
        if let Some(path) = api_error.path {
            data.insert("path".to_string(), serde_json::Value::String(path));
        }
        
        if let Some(details) = api_error.details {
            data.insert("details".to_string(), details);
        }
        
        if let Some(suggestions) = api_error.suggestions {
            data.insert("suggestions".to_string(), serde_json::Value::Array(
                suggestions.into_iter().map(serde_json::Value::String).collect()
            ));
        }
        
        data.insert("timestamp".to_string(), serde_json::Value::String(
            api_error.timestamp.to_rfc3339()
        ));
        
        Self {
            code,
            message: api_error.message,
            data: if data.is_empty() { None } else { Some(serde_json::Value::Object(data)) },
        }
    }
}

/// Trait for converting errors to MCP responses
pub trait ToMcpResponse {
    fn to_mcp_error(&self, id: Option<serde_json::Value>) -> McpErrorResponse;
    fn to_mcp_json_rpc_error(&self) -> McpJsonRpcError;
}

impl<E: std::error::Error> ToMcpResponse for E {
    fn to_mcp_error(&self, id: Option<serde_json::Value>) -> McpErrorResponse {
        let api_error = self.to_sanitized_api_error();
        let json_rpc_error = McpJsonRpcError::from(api_error);
        McpErrorResponse::new(id, json_rpc_error)
    }
    
    fn to_mcp_json_rpc_error(&self) -> McpJsonRpcError {
        let api_error = self.to_sanitized_api_error();
        McpJsonRpcError::from(api_error)
    }
}

/// MCP error categories with specific codes
pub enum McpErrorCode {
    ParseError,
    InvalidRequest, 
    MethodNotFound,
    InvalidParams,
    InternalError,
    AuthenticationFailed,
    AuthorizationDenied,
    RateLimited,
    ValidationError,
    ResourceNotFound,
    Conflict,
    Timeout,
    ServiceUnavailable,
    ToolNotFound,
    ToolExecutionFailed,
    Custom(i32),
}

impl McpErrorCode {
    pub fn code(&self) -> i32 {
        match self {
            McpErrorCode::ParseError => -32700,
            McpErrorCode::InvalidRequest => -32600,
            McpErrorCode::MethodNotFound => -32601,
            McpErrorCode::InvalidParams => -32602,
            McpErrorCode::InternalError => -32603,
            McpErrorCode::AuthenticationFailed => -32000,
            McpErrorCode::AuthorizationDenied => -32001,
            McpErrorCode::RateLimited => -32002,
            McpErrorCode::ValidationError => -32003,
            McpErrorCode::ResourceNotFound => -32004,
            McpErrorCode::Conflict => -32005,
            McpErrorCode::Timeout => -32006,
            McpErrorCode::ServiceUnavailable => -32007,
            McpErrorCode::ToolNotFound => -32008,
            McpErrorCode::ToolExecutionFailed => -32009,
            McpErrorCode::Custom(code) => *code,
        }
    }
    
    pub fn message(&self) -> &'static str {
        match self {
            McpErrorCode::ParseError => "Parse error",
            McpErrorCode::InvalidRequest => "Invalid request",
            McpErrorCode::MethodNotFound => "Method not found",
            McpErrorCode::InvalidParams => "Invalid parameters",
            McpErrorCode::InternalError => "Internal error",
            McpErrorCode::AuthenticationFailed => "Authentication failed",
            McpErrorCode::AuthorizationDenied => "Authorization denied",
            McpErrorCode::RateLimited => "Rate limited",
            McpErrorCode::ValidationError => "Validation error",
            McpErrorCode::ResourceNotFound => "Resource not found",
            McpErrorCode::Conflict => "Conflict",
            McpErrorCode::Timeout => "Timeout",
            McpErrorCode::ServiceUnavailable => "Service unavailable",
            McpErrorCode::ToolNotFound => "Tool not found",
            McpErrorCode::ToolExecutionFailed => "Tool execution failed",
            McpErrorCode::Custom(_) => "Custom error",
        }
    }
}

/// Builder for creating MCP error responses
pub struct McpErrorBuilder;

impl McpErrorBuilder {
    pub fn parse_error(id: Option<serde_json::Value>, message: Option<&str>) -> McpErrorResponse {
        let error = McpJsonRpcError {
            code: McpErrorCode::ParseError.code(),
            message: message.unwrap_or(McpErrorCode::ParseError.message()).to_string(),
            data: None,
        };
        McpErrorResponse::new(id, error)
    }
    
    pub fn invalid_request(id: Option<serde_json::Value>, message: Option<&str>) -> McpErrorResponse {
        let error = McpJsonRpcError {
            code: McpErrorCode::InvalidRequest.code(),
            message: message.unwrap_or(McpErrorCode::InvalidRequest.message()).to_string(),
            data: None,
        };
        McpErrorResponse::new(id, error)
    }
    
    pub fn method_not_found(id: Option<serde_json::Value>, method: &str) -> McpErrorResponse {
        let error = McpJsonRpcError {
            code: McpErrorCode::MethodNotFound.code(),
            message: format!("Method '{}' not found", method),
            data: Some(serde_json::json!({ "method": method })),
        };
        McpErrorResponse::new(id, error)
    }
    
    pub fn invalid_params(id: Option<serde_json::Value>, details: Option<&str>) -> McpErrorResponse {
        let mut error = McpJsonRpcError {
            code: McpErrorCode::InvalidParams.code(),
            message: "Invalid parameters".to_string(),
            data: None,
        };
        
        if let Some(details) = details {
            error.data = Some(serde_json::json!({ "details": details }));
        }
        
        McpErrorResponse::new(id, error)
    }
    
    pub fn tool_not_found(id: Option<serde_json::Value>, tool_name: &str) -> McpErrorResponse {
        let error = McpJsonRpcError {
            code: McpErrorCode::ToolNotFound.code(),
            message: format!("Tool '{}' not found", tool_name),
            data: Some(serde_json::json!({ "toolName": tool_name })),
        };
        McpErrorResponse::new(id, error)
    }
    
    pub fn authentication_failed(id: Option<serde_json::Value>, reason: Option<&str>) -> McpErrorResponse {
        let error = McpJsonRpcError {
            code: McpErrorCode::AuthenticationFailed.code(),
            message: reason.unwrap_or("Authentication failed").to_string(),
            data: None,
        };
        McpErrorResponse::new(id, error)
    }
    
    pub fn internal_error(id: Option<serde_json::Value>, error: &dyn std::error::Error) -> McpErrorResponse {
        let api_error = error.to_sanitized_api_error();
        let json_rpc_error = McpJsonRpcError::from(api_error);
        McpErrorResponse::new(id, json_rpc_error)
    }
}

/// Sanitize MCP error messages to prevent information leakage
pub fn sanitize_mcp_error(error: &mut McpJsonRpcError) {
    let sanitizer = crate::traits::ErrorSanitizationProvider::get_sanitizer();
    let sanitized = sanitizer.sanitize_message(&error.message);
    error.message = sanitized.message;
    
    // Also sanitize any string data in the data field
    if let Some(data) = &mut error.data {
        sanitize_json_value(data);
    }
}

/// Recursively sanitize JSON values
fn sanitize_json_value(value: &mut serde_json::Value) {
    let sanitizer = crate::traits::ErrorSanitizationProvider::get_sanitizer();
    
    match value {
        serde_json::Value::String(s) => {
            let sanitized = sanitizer.sanitize_message(s);
            *s = sanitized.message;
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj.iter_mut() {
                sanitize_json_value(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                sanitize_json_value(v);
            }
        }
        _ => {} // Numbers, booleans, null don't need sanitization
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    
    #[test]
    fn test_api_error_to_mcp_json_rpc() {
        let api_error = ApiError::new("VALIDATION_ERROR", "Invalid input")
            .with_request_id("req-123");
        
        let mcp_error = McpJsonRpcError::from(api_error);
        
        assert_eq!(mcp_error.code, -32003); // VALIDATION_ERROR code
        assert_eq!(mcp_error.message, "Invalid input");
        assert!(mcp_error.data.is_some());
        
        let data = mcp_error.data.unwrap();
        assert_eq!(data["requestId"], "req-123");
    }
    
    #[test]
    fn test_mcp_error_builder() {
        let error = McpErrorBuilder::method_not_found(
            Some(serde_json::Value::String("test-123".to_string())),
            "unknown_method"
        );
        
        assert_eq!(error.jsonrpc, "2.0");
        assert_eq!(error.id, Some(serde_json::Value::String("test-123".to_string())));
        assert_eq!(error.error.code, -32601);
        assert!(error.error.message.contains("unknown_method"));
    }
    
    #[test]
    fn test_error_conversion() {
        let io_error = io::Error::new(
            io::ErrorKind::NotFound,
            "file not found: /secret/path/config.yaml"
        );
        
        let mcp_error = io_error.to_mcp_json_rpc_error();
        
        // Should not contain sensitive path
        assert!(!mcp_error.message.contains("/secret/path"));
    }
    
    #[test]
    fn test_sanitize_mcp_error() {
        let mut error = McpJsonRpcError {
            code: -32603,
            message: "Database error: postgresql://user:pass@host/db".to_string(),
            data: Some(serde_json::json!({
                "details": "Connection failed: postgresql://user:pass@host/db"
            })),
        };
        
        sanitize_mcp_error(&mut error);
        
        // Should not contain sensitive database connection info
        assert!(!error.message.contains("postgresql://"));
        assert!(!error.message.contains("user:pass"));
        
        if let Some(data) = &error.data {
            let details = data["details"].as_str().unwrap();
            assert!(!details.contains("postgresql://"));
            assert!(!details.contains("user:pass"));
        }
    }
}