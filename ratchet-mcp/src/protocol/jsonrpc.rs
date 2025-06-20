//! JSON-RPC 2.0 implementation for MCP

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// JSON-RPC 2.0 version string
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 request message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Method name to call
    pub method: String,

    /// Method parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// Request ID for correlation (optional for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id,
        }
    }

    /// Create a new JSON-RPC request with string ID
    pub fn with_id(method: impl Into<String>, params: Option<Value>, id: impl Into<String>) -> Self {
        Self::new(method, params, Some(Value::String(id.into())))
    }

    /// Create a new JSON-RPC notification (no ID, no response expected)
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self::new(method, params, None)
    }

    /// Check if this is a notification (has no ID)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Get the request ID as a string if present
    pub fn id_as_string(&self) -> Option<String> {
        match &self.id {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Number(n)) => Some(n.to_string()),
            Some(value) => Some(value.to_string()),
            None => None,
        }
    }
}

/// JSON-RPC 2.0 response message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Successful result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error information (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request ID for correlation
    pub id: Option<Value>,
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: JsonRpcError, id: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Check if this response contains an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Check if this response is successful
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }
}

/// JSON-RPC 2.0 error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error
    pub fn new(code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    /// Create a parse error
    pub fn parse_error(data: Option<Value>) -> Self {
        Self::new(JsonRpcErrorCode::ParseError as i32, "Parse error", data)
    }

    /// Create an invalid request error
    pub fn invalid_request(data: Option<Value>) -> Self {
        Self::new(JsonRpcErrorCode::InvalidRequest as i32, "Invalid Request", data)
    }

    /// Create a method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::MethodNotFound as i32,
            "Method not found",
            Some(Value::String(format!("Method '{}' not found", method))),
        )
    }

    /// Create an invalid params error
    pub fn invalid_params(details: impl Into<String>) -> Self {
        Self::new(
            JsonRpcErrorCode::InvalidParams as i32,
            "Invalid params",
            Some(Value::String(details.into())),
        )
    }

    /// Create an internal error
    pub fn internal_error(details: impl Into<String>) -> Self {
        Self::new(
            JsonRpcErrorCode::InternalError as i32,
            "Internal error",
            Some(Value::String(details.into())),
        )
    }

    /// Create a server error
    pub fn server_error(code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self::new(code, message, data)
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC Error {}: {}", self.code, self.message)?;
        if let Some(data) = &self.data {
            write!(f, " (data: {})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonRpcError {}

/// Standard JSON-RPC 2.0 error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum JsonRpcErrorCode {
    /// Invalid JSON was received by the server
    ParseError = -32700,

    /// The JSON sent is not a valid Request object
    InvalidRequest = -32600,

    /// The method does not exist / is not available
    MethodNotFound = -32601,

    /// Invalid method parameter(s)
    InvalidParams = -32602,

    /// Internal JSON-RPC error
    InternalError = -32603,

    // Server error range: -32000 to -32099
    /// Server is not initialized
    ServerNotInitialized = -32002,

    /// Server is shutting down
    ServerShuttingDown = -32001,

    /// Request was cancelled
    RequestCancelled = -32800,

    /// Content was modified
    ContentModified = -32801,
}

impl JsonRpcErrorCode {
    /// Check if this is a server error (in the -32000 to -32099 range)
    pub fn is_server_error(code: i32) -> bool {
        (-32099..=-32000).contains(&code)
    }

    /// Check if this is a reserved error (predefined by JSON-RPC spec)
    pub fn is_reserved_error(code: i32) -> bool {
        (-32768..=-32000).contains(&code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let request = JsonRpcRequest::with_id("test_method", Some(json!({"param": "value"})), "123");

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request, deserialized);
        assert_eq!(request.method, "test_method");
        assert_eq!(request.id_as_string(), Some("123".to_string()));
        assert!(!request.is_notification());
    }

    #[test]
    fn test_jsonrpc_notification() {
        let notification = JsonRpcRequest::notification("notify_method", Some(json!({"data": "test"})));

        assert!(notification.is_notification());
        assert_eq!(notification.id, None);
        assert_eq!(notification.id_as_string(), None);
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(json!({"result": "success"}), Some(json!("123")));

        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.result, Some(json!({"result": "success"})));
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let error = JsonRpcError::method_not_found("unknown_method");
        let response = JsonRpcResponse::error(error, Some(json!("123")));

        assert!(!response.is_success());
        assert!(response.is_error());
        assert_eq!(
            response.error.as_ref().unwrap().code,
            JsonRpcErrorCode::MethodNotFound as i32
        );
    }

    #[test]
    fn test_error_codes() {
        assert!(JsonRpcErrorCode::is_server_error(-32001));
        assert!(JsonRpcErrorCode::is_server_error(-32099));
        assert!(!JsonRpcErrorCode::is_server_error(-32100));
        assert!(!JsonRpcErrorCode::is_server_error(-31999));

        assert!(JsonRpcErrorCode::is_reserved_error(-32700));
        assert!(JsonRpcErrorCode::is_reserved_error(-32000));
        assert!(!JsonRpcErrorCode::is_reserved_error(-31999));
    }
}
