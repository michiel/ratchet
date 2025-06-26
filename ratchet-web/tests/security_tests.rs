//! Security tests for web middleware and error handling

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::get,
    Router,
};
use ratchet_api_types::errors::ApiError;
use ratchet_web::{
    errors::WebError,
    middleware::{
        cors::{cors_layer_with_config, CorsConfig},
        cors_layer, error_handler_layer,
    },
};
use serde_json::Value;
use tower::ServiceExt;

/// Test error sanitization prevents information leakage
#[tokio::test]
async fn test_error_sanitization_enforcement() {
    // Create a test handler that returns various error types
    async fn test_handler() -> Result<(), WebError> {
        // Simulate a database connection error with sensitive information
        Err(WebError::Internal {
            message: "Database connection failed: password=secret123, host=internal-db.company.com".to_string(),
        })
    }

    let app = Router::new().route("/test", get(test_handler));

    let request = Request::builder()
        .method(Method::GET)
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Convert response body to string
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // Parse JSON response
    let json: Value = serde_json::from_str(&body_str).unwrap();
    
    // Ensure sensitive information is not leaked
    let error_obj = json.get("error").unwrap();
    let message = error_obj.get("message").unwrap().as_str().unwrap();
    
    assert!(!message.contains("password=secret123"));
    assert!(!message.contains("internal-db.company.com"));
    assert!(!message.contains("Database connection failed"));
    
    // Should contain sanitized message - either categorized as database error or generic
    let is_sanitized = message.contains("Database operation failed") || 
                      message.contains("Internal server error") ||
                      message.contains("[REDACTED]") ||
                      message.contains("An error occurred");
    assert!(is_sanitized, "Message should be sanitized: {}", message);
}

/// Test API error conversion maintains sanitization
#[tokio::test]
async fn test_api_error_conversion_sanitization() {
    // Test WebError to ApiError conversion
    let internal_error = WebError::Internal {
        message: "SQL injection detected: DROP TABLE users; --".to_string(),
    };
    
    let api_error: ApiError = internal_error.into();
    
    // Should not contain the SQL injection attempt
    assert!(!api_error.message.contains("DROP TABLE"));
    assert!(!api_error.message.contains("SQL injection"));
    
    // Test other sensitive error types
    let jwt_error = WebError::Internal {
        message: "JWT secret key: super_secret_key_123".to_string(),
    };
    let jwt_api_error: ApiError = jwt_error.into();
    assert!(!jwt_api_error.message.contains("super_secret_key_123"));
    assert!(!jwt_api_error.message.contains("super_secret_key"));
    // Should contain [REDACTED] after sanitization
    assert!(jwt_api_error.message.contains("[REDACTED]") || jwt_api_error.message == "An error occurred");
    
    let api_key_error = WebError::Internal {
        message: "API key validation failed: key=sk_live_abc123".to_string(),
    };
    let api_key_api_error: ApiError = api_key_error.into();
    assert!(!api_key_api_error.message.contains("sk_live_abc123"));
    // This should be categorized as validation error
    assert!(api_key_api_error.message.contains("validation") || api_key_api_error.message.contains("Input"));
}

/// Test user-facing errors are not sanitized
#[tokio::test]
async fn test_user_facing_errors_not_sanitized() {
    let user_errors = vec![
        WebError::BadRequest {
            message: "Invalid email format".to_string(),
        },
        WebError::Unauthorized {
            message: "Invalid credentials".to_string(),
        },
        WebError::Forbidden {
            message: "Access denied to this resource".to_string(),
        },
        WebError::NotFound {
            message: "User not found".to_string(),
        },
    ];
    
    for error in user_errors {
        let original_message = match &error {
            WebError::BadRequest { message } => message.clone(),
            WebError::Unauthorized { message } => message.clone(),
            WebError::Forbidden { message } => message.clone(),
            WebError::NotFound { message } => message.clone(),
            _ => unreachable!(),
        };
        
        let api_error: ApiError = error.into();
        
        // User-facing errors should preserve their original message
        assert_eq!(api_error.message, original_message);
    }
}

/// Test CORS security validation
#[tokio::test]
async fn test_cors_security_validation() {
    // Test secure default configuration
    let default_config = CorsConfig::default();
    assert!(default_config.validate().is_ok());
    assert!(!default_config.allowed_origins.contains(&"*".to_string()));
    assert!(default_config.allowed_origins.contains(&"http://localhost:3000".to_string()));
    
    // Test invalid configuration: wildcard with credentials
    let invalid_config = CorsConfig {
        allowed_origins: vec!["*".to_string()],
        allow_credentials: true,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());
    
    // Test development configuration validation
    let dev_config = CorsConfig::development();
    assert!(dev_config.validate().is_ok()); // Should be valid but warn
    assert!(dev_config.allowed_origins.contains(&"*".to_string()));
    
    // Test production configuration
    let prod_config = CorsConfig::production(vec![
        "https://myapp.com".to_string(),
        "https://www.myapp.com".to_string(),
    ]);
    assert!(prod_config.validate().is_ok());
    assert!(!prod_config.allowed_origins.contains(&"*".to_string()));
}

/// Test CORS layer creation with invalid configuration falls back to secure defaults
#[tokio::test]
async fn test_cors_layer_fallback_to_secure_defaults() {
    // Create invalid CORS config
    let invalid_config = CorsConfig {
        allowed_origins: vec!["*".to_string()],
        allow_credentials: true,
        ..Default::default()
    };
    
    // Should not panic and should create a working CORS layer
    let cors_layer = cors_layer_with_config(invalid_config);
    
    // Create a simple app with the CORS layer
    let app = Router::new()
        .route("/test", get(|| async { "OK" }))
        .layer(cors_layer);
    
    // Test preflight request
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/test")
        .header("Origin", "http://malicious-site.com")
        .header("Access-Control-Request-Method", "GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Should handle the request without error
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NO_CONTENT);
}

/// Test CORS with specific origins
#[tokio::test]
async fn test_cors_specific_origins() {
    let config = CorsConfig::production(vec![
        "https://trusted-site.com".to_string(),
        "https://another-trusted.com".to_string(),
    ]);
    
    let cors_layer = cors_layer_with_config(config);
    let app = Router::new()
        .route("/api/test", get(|| async { "OK" }))
        .layer(cors_layer);
    
    // Test allowed origin
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/test")
        .header("Origin", "https://trusted-site.com")
        .body(Body::empty())
        .unwrap();
    
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test disallowed origin (should still work but without CORS headers)
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/test")
        .header("Origin", "https://malicious-site.com")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    // The request should succeed but CORS headers may not be present
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test configuration parsing security
#[tokio::test]
async fn test_configuration_security() {
    // Test that default configurations are secure
    let default_cors = CorsConfig::default();
    assert!(!default_cors.allowed_origins.contains(&"*".to_string()));
    assert!(!default_cors.allow_credentials); // Should be false by default for security
    
    // Test production configuration enforces specific origins
    let prod_config = CorsConfig::production(vec!["https://prod.example.com".to_string()]);
    assert!(!prod_config.allowed_origins.contains(&"*".to_string()));
    assert!(prod_config.allow_credentials); // Should be true for production with specific origins
    
    // Ensure development config warns about insecure settings
    let dev_config = CorsConfig::development();
    assert!(dev_config.allowed_origins.contains(&"*".to_string()));
    // Development config should still validate successfully but with warnings
    assert!(dev_config.validate().is_ok());
}

/// Test error handling robustness
#[tokio::test]
async fn test_error_handling_robustness() {
    // Test various error scenarios that could expose sensitive information
    let sensitive_errors = vec![
        "Database password: admin123",
        "API key: sk_live_1234567890",
        "Internal server details: /var/www/app/config/database.yml",
        "Stack trace: /home/user/.cargo/registry/src/github.com",
        "Environment variable: STRIPE_SECRET_KEY=sk_test_123",
    ];
    
    for sensitive_msg in sensitive_errors {
        let error = WebError::Internal {
            message: sensitive_msg.to_string(),
        };
        
        let api_error: ApiError = error.into();
        
        // Should not contain any sensitive raw information
        assert!(!api_error.message.contains("admin123"));
        assert!(!api_error.message.contains("sk_live_1234567890"));
        assert!(!api_error.message.contains("sk_test_123"));
        assert!(!api_error.message.contains("STRIPE_SECRET_KEY="));
        assert!(!api_error.message.contains("/var/www/app/config"));
        assert!(!api_error.message.contains("/home/user/.cargo"));
        assert!(!api_error.message.contains("database.yml"));
        
        // Should either be sanitized with [REDACTED] or categorized as a specific error type
        let msg_lower = api_error.message.to_lowercase();
        let is_sanitized = api_error.message.contains("[REDACTED]") || 
                          msg_lower.contains("database operation failed") ||
                          msg_lower.contains("file operation failed") ||
                          msg_lower.contains("configuration error") ||
                          msg_lower.contains("an error occurred") ||
                          msg_lower.contains("internal server error");
        assert!(is_sanitized, "Message should be sanitized: {}", api_error.message);
    }
}

/// Integration test: Full request cycle with security middleware
#[tokio::test]
async fn test_full_security_integration() {
    
    // Handler that simulates various error conditions
    async fn security_test_handler() -> Result<String, WebError> {
        // This would typically come from request parameters
        let error_type = "internal"; // Simulate getting this from request
        
        match error_type {
            "internal" => Err(WebError::Internal {
                message: "Critical system failure: root password exposed in logs".to_string(),
            }),
            "bad_request" => Err(WebError::BadRequest {
                message: "Invalid input format".to_string(),
            }),
            _ => Ok("Success".to_string()),
        }
    }
    
    let app = Router::new()
        .route("/security-test", get(security_test_handler))
        .layer(error_handler_layer())
        .layer(cors_layer());
    
    // Test internal error sanitization
    let request = Request::builder()
        .method(Method::GET)
        .uri("/security-test")
        .header("Origin", "http://localhost:3000")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // Should not leak the actual sensitive information
    assert!(!body_str.contains("root password"));
    // But should either redact the sensitive part or provide generic message
    let is_safe = body_str.contains("[REDACTED]") || 
                  body_str.contains("Internal server error") ||
                  body_str.contains("An error occurred");
    assert!(is_safe, "Response should be sanitized: {}", body_str);
}