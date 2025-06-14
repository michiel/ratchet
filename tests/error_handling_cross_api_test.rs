//! Cross-API error handling consistency tests
//!
//! This test validates that error handling is consistent across
//! GraphQL, REST, and MCP APIs.

use anyhow::Result;
use ratchet_error_middleware::{ToSanitizedApiError, rest::ToRestResponse, mcp::ToMcpResponse};
use ratchet_api_types::errors::ApiError;
use std::io;
use serde_json::json;
use axum::{
    routing::{get, post},
    Router, Json,
    response::{IntoResponse, Response},
    extract::Request,
};
use tower::ServiceExt;

#[cfg(feature = "graphql")]
use ratchet_error_middleware::graphql::GraphQLErrorExtensions;

/// Test that sensitive information is consistently sanitized across all APIs
#[tokio::test]
async fn test_sensitive_data_sanitization_consistency() -> Result<()> {
    let sensitive_errors = vec![
        io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "Database connection failed: postgresql://admin:secret123@prod-db.internal:5432/sensitive_data"
        ),
        io::Error::new(
            io::ErrorKind::NotFound,
            "Config file not found: /home/user/.aws/credentials"
        ),
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Access denied to /etc/shadow - invalid token: jwt_abc123_secret"
        ),
    ];
    
    for error in sensitive_errors {
        // Test REST API sanitization
        let rest_response = error.to_rest_response();
        let rest_body = extract_response_body(rest_response).await;
        assert_no_sensitive_data(&rest_body);
        
        // Test MCP sanitization  
        let mcp_error = error.to_mcp_json_rpc_error();
        assert_no_sensitive_data(&mcp_error.message);
        if let Some(data) = &mcp_error.data {
            assert_no_sensitive_data(&data.to_string());
        }
        
        #[cfg(feature = "graphql")]
        {
            // Test GraphQL sanitization
            let graphql_error = error.to_sanitized_graphql_error();
            assert_no_sensitive_data(&graphql_error.message);
        }
        
        // Test unified API error
        let api_error = error.to_sanitized_api_error();
        assert_no_sensitive_data(&api_error.message);
    }
    
    Ok(())
}

/// Test error code consistency across APIs
#[test]
fn test_error_code_consistency() {
    let test_cases = vec![
        (
            io::Error::new(io::ErrorKind::NotFound, "Resource not found"),
            vec!["NOT_FOUND", "FILESYSTEM_ERROR", "INTERNAL_ERROR"]
        ),
        (
            io::Error::new(io::ErrorKind::PermissionDenied, "Access denied"),
            vec!["FORBIDDEN", "PERMISSION_DENIED", "AUTH_ERROR", "INTERNAL_ERROR"]
        ),
        (
            io::Error::new(io::ErrorKind::ConnectionRefused, "Connection failed"),
            vec!["SERVICE_UNAVAILABLE", "NETWORK_ERROR", "DATABASE_ERROR", "INTERNAL_ERROR"]
        ),
    ];
    
    for (error, expected_codes) in test_cases {
        let api_error = error.to_sanitized_api_error();
        
        // Error code should be one of the expected codes
        assert!(
            expected_codes.contains(&api_error.code.as_str()),
            "Error code '{}' not in expected codes for error: {}",
            api_error.code,
            error
        );
        
        // All API formats should derive from the same base
        let rest_response = error.to_rest_response();
        let mcp_error = error.to_mcp_json_rpc_error();
        
        // Verify consistent categorization (not necessarily same codes due to protocol differences)
        assert!(!rest_response.status().is_success());
        assert!(mcp_error.code < 0); // MCP uses negative error codes
    }
}

/// Test that error structure is consistent across APIs
#[test]
fn test_error_structure_consistency() {
    let base_error = io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid JSON format in configuration"
    );
    
    // Convert to all API formats
    let api_error = base_error.to_sanitized_api_error()
        .with_request_id("test-req-123")
        .with_path("/test/operation");
    
    // All should have required fields
    assert!(!api_error.code.is_empty());
    assert!(!api_error.message.is_empty());
    assert!(api_error.request_id.is_some());
    assert!(api_error.path.is_some());
    assert!(api_error.timestamp <= chrono::Utc::now());
    
    // REST conversion
    let rest_response = base_error.to_rest_response();
    assert!(!rest_response.status().is_success());
    
    // MCP conversion
    let mcp_error = base_error.to_mcp_json_rpc_error();
    assert!(!mcp_error.message.is_empty());
    assert!(mcp_error.code != 0);
}

/// Test error message quality and user-friendliness
#[test]
fn test_error_message_quality() {
    let technical_errors = vec![
        "std::io::Error: No such file or directory (os error 2)",
        "thread 'main' panicked at 'assertion failed: x == y'",
        "ERROR 1045 (28000): Access denied for user 'root'@'localhost'",
        "java.lang.NullPointerException at line 42",
    ];
    
    for error_msg in technical_errors {
        let error = io::Error::new(io::ErrorKind::Other, error_msg);
        let api_error = error.to_sanitized_api_error();
        
        // Message should be user-friendly
        assert!(!api_error.message.contains("std::io::Error"));
        assert!(!api_error.message.contains("thread 'main' panicked"));
        assert!(!api_error.message.contains("java.lang"));
        assert!(!api_error.message.contains("NullPointerException"));
        assert!(!api_error.message.contains("os error"));
        assert!(!api_error.message.contains("line 42"));
        
        // Should still be informative
        assert!(api_error.message.len() > 10);
        assert!(!api_error.message.is_empty());
    }
}

/// Test error suggestions are provided appropriately
#[test]
fn test_error_suggestions() {
    let api_error = ApiError::validation_error("email", "Invalid email format");
    assert!(api_error.suggestions.is_some());
    
    let suggestions = api_error.suggestions.unwrap();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("format") || s.contains("documentation")));
}

/// Test error handling under load
#[tokio::test]
async fn test_error_handling_performance() {
    use std::sync::Arc;
    use tokio::task::JoinSet;
    
    let errors = Arc::new(vec![
        io::Error::new(io::ErrorKind::NotFound, "File not found: /secret/path"),
        io::Error::new(io::ErrorKind::PermissionDenied, "Access denied: sensitive operation"),
        io::Error::new(io::ErrorKind::ConnectionRefused, "DB connection failed: postgresql://..."),
    ]);
    
    let mut tasks = JoinSet::new();
    
    // Spawn 100 concurrent error processing tasks
    for i in 0..100 {
        let errors_clone = errors.clone();
        tasks.spawn(async move {
            let error = &errors_clone[i % errors_clone.len()];
            
            // Process through all API types
            let _api_error = error.to_sanitized_api_error();
            let _rest_response = error.to_rest_response();
            let _mcp_error = error.to_mcp_json_rpc_error();
            
            #[cfg(feature = "graphql")]
            let _graphql_error = error.to_sanitized_graphql_error();
            
            i
        });
    }
    
    let start = std::time::Instant::now();
    
    // Wait for all tasks to complete
    let mut completed = 0;
    while let Some(result) = tasks.join_next().await {
        result.unwrap();
        completed += 1;
    }
    
    let duration = start.elapsed();
    
    assert_eq!(completed, 100);
    // Should complete in reasonable time (less than 1 second)
    assert!(duration < std::time::Duration::from_secs(1));
}

/// Test middleware integration in actual server
#[tokio::test]
async fn test_middleware_integration() -> Result<()> {
    use ratchet_error_middleware::middleware::ErrorMiddlewareBuilder;
    
    async fn error_endpoint() -> Result<Json<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        Err("Database connection failed: postgresql://user:pass@host/db".into())
    }
    
    let app = Router::new()
        .route("/error", get(error_endpoint))
        .layer(ErrorMiddlewareBuilder::new().build());
    
    let request = Request::builder()
        .uri("/error")
        .body(axum::body::Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Should be an error status
    assert!(response.status().is_server_error());
    
    // Extract response body and verify sanitization
    let body = extract_response_body(response).await;
    assert_no_sensitive_data(&body);
    
    // Should be valid JSON with error structure
    let json: serde_json::Value = serde_json::from_str(&body)?;
    assert!(json.get("error").is_some());
    
    Ok(())
}

/// Test configuration-driven sanitization
#[test]
fn test_configurable_sanitization() {
    use ratchet_error_middleware::sanitization::{SharedErrorSanitizer, SanitizationPresets};
    
    let error_msg = "Database error: connection timeout after 30s";
    
    // Test different presets
    let dev_sanitizer = SharedErrorSanitizer::new(SanitizationPresets::development());
    let prod_sanitizer = SharedErrorSanitizer::new(SanitizationPresets::production());
    let security_sanitizer = SharedErrorSanitizer::new(SanitizationPresets::security_focused());
    
    let dev_result = dev_sanitizer.sanitize_message(error_msg);
    let prod_result = prod_sanitizer.sanitize_message(error_msg);
    let security_result = security_sanitizer.sanitize_message(error_msg);
    
    // Development should be least restrictive
    assert!(dev_result.message.len() >= prod_result.message.len());
    
    // Security-focused should be most restrictive
    assert!(security_result.message.len() <= prod_result.message.len());
    
    // All should sanitize appropriately for their context
    assert!(!dev_result.message.is_empty());
    assert!(!prod_result.message.is_empty());
    assert!(!security_result.message.is_empty());
}

// Helper functions

async fn extract_response_body(response: Response) -> String {
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8_lossy(&body_bytes).to_string()
}

fn assert_no_sensitive_data(content: &str) {
    let sensitive_patterns = [
        "postgresql://",
        "mysql://",
        "user:pass",
        "admin:secret",
        "jwt_",
        "Bearer ",
        "/home/user",
        "/etc/",
        "/.aws/",
        "/.ssh/",
        "secret123",
        "password123",
        ".internal",
        "prod-db",
    ];
    
    for pattern in &sensitive_patterns {
        assert!(
            !content.contains(pattern),
            "Content contains sensitive pattern '{}': {}",
            pattern,
            content
        );
    }
}

#[cfg(feature = "graphql")]
mod graphql_integration_tests {
    use super::*;
    use async_graphql::{Schema, Object, Result as GraphQLResult, Error as GraphQLError};
    use ratchet_error_middleware::graphql::GraphQLErrorExtensions;
    
    struct Query;
    
    #[Object]
    impl Query {
        async fn error_field(&self) -> GraphQLResult<String> {
            let error = io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Access denied to /etc/passwd"
            );
            
            Err(error.to_sanitized_graphql_error())
        }
    }
    
    #[tokio::test]
    async fn test_graphql_error_integration() {
        let schema = Schema::new(Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription);
        
        let query = "{ errorField }";
        let result = schema.execute(query).await;
        
        assert!(result.is_err());
        
        let errors = result.errors;
        assert!(!errors.is_empty());
        
        let error = &errors[0];
        
        // Should not contain sensitive path
        assert!(!error.message.contains("/etc/passwd"));
        
        // Should have proper extensions
        assert!(error.extensions.is_some());
        let extensions = error.extensions.as_ref().unwrap();
        assert!(extensions.contains_key("code"));
    }
}