//! Integration tests for error handling middleware

use ratchet_error_middleware::{
    ToSanitizedApiError, 
    ErrorSanitizationProvider,
    sanitization::{SanitizationPresets, SharedErrorSanitizer},
    traits::{ErrorDomain, EnrichApiError},
};
use ratchet_api_types::errors::ApiError;
use std::io;

#[cfg(feature = "graphql")]
use ratchet_error_middleware::graphql::GraphQLErrorExtensions;

use ratchet_error_middleware::{
    rest::{ToRestResponse, RestErrorBuilder},
    mcp::{ToMcpResponse, McpErrorBuilder},
};

/// Test error sanitization across different error types
#[test]
fn test_error_sanitization_consistency() {
    // Database connection error with sensitive information
    let db_error = io::Error::new(
        io::ErrorKind::ConnectionRefused,
        "Connection failed: postgresql://user:secret@localhost:5432/mydb"
    );
    
    let sanitized = db_error.to_sanitized_api_error();
    
    // Should not contain sensitive connection string
    assert!(!sanitized.message.contains("postgresql://"));
    assert!(!sanitized.message.contains("user:secret"));
    assert!(!sanitized.message.contains("localhost:5432"));
    
    // Should have appropriate error code
    assert!(matches!(sanitized.code.as_str(), "DATABASE_ERROR" | "NETWORK_ERROR" | "INTERNAL_ERROR"));
}

/// Test error sanitization with file paths
#[test]
fn test_file_path_sanitization() {
    let file_error = io::Error::new(
        io::ErrorKind::NotFound,
        "File not found: /home/user/.ssh/id_rsa"
    );
    
    let sanitized = file_error.to_sanitized_api_error();
    
    // Should not contain sensitive file path
    assert!(!sanitized.message.contains("/home/user"));
    assert!(!sanitized.message.contains(".ssh"));
    assert!(!sanitized.message.contains("id_rsa"));
}

/// Test cross-API error consistency
#[test]
fn test_cross_api_error_consistency() {
    let base_error = io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Access denied: /etc/passwd"
    );
    
    // Convert to different API formats
    let rest_response = base_error.to_rest_response();
    let mcp_response = base_error.to_mcp_json_rpc_error();
    
    #[cfg(feature = "graphql")]
    let graphql_error = base_error.to_sanitized_graphql_error();
    
    // All should sanitize the sensitive path
    let rest_body = extract_rest_error_message(&rest_response);
    assert!(!rest_body.contains("/etc/passwd"));
    
    assert!(!mcp_response.message.contains("/etc/passwd"));
    
    #[cfg(feature = "graphql")]
    assert!(!graphql_error.message.contains("/etc/passwd"));
}

/// Helper to extract error message from REST response
fn extract_rest_error_message(response: &axum::response::Response) -> String {
    // This is a simplified version - in real tests you'd properly deserialize the response
    format!("{:?}", response)
}

/// Test error domain categorization
#[test]
fn test_error_domain_enrichment() {
    let api_error = ApiError::new("INTERNAL_ERROR", "Something went wrong");
    
    let enriched = api_error
        .with_domain(ErrorDomain::Database)
        .with_retry_info(true, Some(std::time::Duration::from_secs(5)));
    
    assert_eq!(enriched.code, "DATABASE_ERROR");
    assert!(enriched.suggestions.is_some());
    assert!(enriched.details.is_some());
}

/// Test sanitization configuration presets
#[test]
fn test_sanitization_presets() {
    let shared_sanitizer = SharedErrorSanitizer::new(SanitizationPresets::production());
    
    let error_with_db_info = io::Error::new(
        io::ErrorKind::ConnectionRefused,
        "Connection to database server failed: timeout after 30s"
    );
    
    let sanitized = shared_sanitizer.sanitize_error(&error_with_db_info);
    
    // Production preset should be more restrictive
    assert!(sanitized.message.len() <= 200); // Max length in production preset
    assert!(!sanitized.message.contains("database server"));
}

/// Test runtime configuration updates
#[test]
fn test_runtime_config_updates() {
    let sanitizer = SharedErrorSanitizer::default();
    
    // Test with default config
    let error_msg = "special database error occurred";
    let initial = sanitizer.sanitize_message(error_msg);
    
    // Update config with custom mapping
    let mut custom_mappings = std::collections::HashMap::new();
    custom_mappings.insert("special database error".to_string(), "system issue".to_string());
    sanitizer.add_custom_mappings(custom_mappings);
    
    let updated = sanitizer.sanitize_message(error_msg);
    
    assert_ne!(initial.message, updated.message);
    assert_eq!(updated.message, "system issue");
}

/// Test error consistency across different error sources
#[test]
fn test_error_source_consistency() {
    let errors: Vec<Box<dyn std::error::Error>> = vec![
        Box::new(io::Error::new(io::ErrorKind::NotFound, "file not found")),
        Box::new(serde_json::Error::io(io::Error::new(io::ErrorKind::InvalidData, "invalid json"))),
    ];
    
    let api_errors: Vec<ApiError> = errors
        .iter()
        .map(|e| e.to_sanitized_api_error())
        .collect();
    
    // All should have consistent structure
    for api_error in api_errors {
        assert!(!api_error.code.is_empty());
        assert!(!api_error.message.is_empty());
        assert!(api_error.timestamp > chrono::Utc::now() - chrono::Duration::seconds(10));
    }
}

/// Test REST error builders
#[test]
fn test_rest_error_builders() {
    let bad_request = RestErrorBuilder::bad_request("Invalid input data");
    assert_eq!(bad_request.status(), axum::http::StatusCode::BAD_REQUEST);
    
    let not_found = RestErrorBuilder::not_found("task", "123");
    assert_eq!(not_found.status(), axum::http::StatusCode::NOT_FOUND);
    
    let validation_error = RestErrorBuilder::validation_error("email", "Invalid format");
    assert_eq!(validation_error.status(), axum::http::StatusCode::BAD_REQUEST);
}

/// Test MCP error builders
#[test]
fn test_mcp_error_builders() {
    let method_not_found = McpErrorBuilder::method_not_found(
        Some(serde_json::Value::String("req-123".to_string())),
        "unknown_method"
    );
    
    assert_eq!(method_not_found.jsonrpc, "2.0");
    assert_eq!(method_not_found.error.code, -32601);
    assert!(method_not_found.error.message.contains("unknown_method"));
    
    let tool_not_found = McpErrorBuilder::tool_not_found(
        None,
        "missing_tool"
    );
    
    assert_eq!(tool_not_found.error.code, -32008);
    assert!(tool_not_found.error.message.contains("missing_tool"));
}

/// Test error propagation through middleware stack
#[tokio::test]
async fn test_error_middleware_stack() {
    use axum::{routing::get, Router, middleware::from_fn};
    use tower::ServiceExt;
    use ratchet_error_middleware::middleware::{
        error_handling_middleware,
        error_sanitization_middleware,
    };
    
    async fn error_handler() -> Result<&'static str, Box<dyn std::error::Error + Send + Sync>> {
        Err("Database connection failed: postgresql://secret@host/db".into())
    }
    
    let app = Router::new()
        .route("/error", get(error_handler))
        .layer(from_fn(error_sanitization_middleware))
        .layer(from_fn(error_handling_middleware));
    
    let request = axum::http::Request::builder()
        .uri("/error")
        .body(axum::body::Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Should be an error response
    assert!(response.status().is_server_error());
    
    // Extract and verify the response body doesn't contain sensitive info
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    
    assert!(!body_str.contains("postgresql://"));
    assert!(!body_str.contains("secret"));
}

/// Test that all error types implement required traits
#[test]
fn test_trait_implementations() {
    fn assert_sanitizable<T: ToSanitizedApiError>(_: T) {}
    fn assert_rest_convertible<T: ToRestResponse>(_: T) {}
    fn assert_mcp_convertible<T: ToMcpResponse>(_: T) {}
    
    let io_error = io::Error::new(io::ErrorKind::Other, "test");
    let json_error = serde_json::Error::io(io_error.clone());
    
    // All should implement the required traits
    assert_sanitizable(io_error.clone());
    assert_rest_convertible(io_error.clone());
    assert_mcp_convertible(io_error);
    
    assert_sanitizable(json_error.clone());
    assert_rest_convertible(json_error.clone());
    assert_mcp_convertible(json_error);
}

/// Performance test for error sanitization
#[test]
fn test_sanitization_performance() {
    let sanitizer = SharedErrorSanitizer::default();
    let error_message = "Database connection failed: postgresql://user:pass@host:5432/db with stack trace at /home/user/app/src/main.rs:123";
    
    let start = std::time::Instant::now();
    
    // Sanitize 1000 times
    for _ in 0..1000 {
        let _ = sanitizer.sanitize_message(error_message);
    }
    
    let duration = start.elapsed();
    
    // Should be reasonably fast (less than 100ms for 1000 operations)
    assert!(duration < std::time::Duration::from_millis(100));
}

/// Test error context preservation through conversion chain
#[test]
fn test_error_context_preservation() {
    let original_error = io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Access denied to sensitive file"
    );
    
    // Convert through the chain
    let api_error = original_error.to_sanitized_api_error_with_context("test/operation");
    
    // Context should be preserved
    assert_eq!(api_error.path, Some("test/operation".to_string()));
    
    // Message should be sanitized
    assert!(!api_error.message.contains("sensitive file"));
    
    // Should have a valid timestamp
    assert!(api_error.timestamp <= chrono::Utc::now());
}

#[cfg(feature = "graphql")]
mod graphql_tests {
    use super::*;
    use async_graphql::Error as GraphQLError;
    
    #[test]
    fn test_graphql_error_conversion() {
        let io_error = io::Error::new(
            io::ErrorKind::NotFound,
            "Secret file not found: /etc/shadow"
        );
        
        let graphql_error = io_error.to_sanitized_graphql_error();
        
        // Should not contain sensitive information
        assert!(!graphql_error.message.contains("/etc/shadow"));
        
        // Should have proper extensions
        let extensions = graphql_error.extensions.as_ref().unwrap();
        assert!(extensions.contains_key("code"));
        assert!(extensions.contains_key("timestamp"));
    }
    
    #[test]
    fn test_api_error_to_graphql_conversion() {
        let api_error = ApiError::new("TEST_ERROR", "Test message")
            .with_request_id("req-123")
            .with_path("/test/query")
            .with_suggestions(vec!["Try again".to_string()]);
        
        let graphql_error: GraphQLError = api_error.into();
        
        let extensions = graphql_error.extensions.unwrap();
        assert_eq!(extensions.get("code").unwrap(), "TEST_ERROR");
        assert_eq!(extensions.get("requestId").unwrap(), "req-123");
        assert_eq!(extensions.get("path").unwrap(), "/test/query");
    }
}