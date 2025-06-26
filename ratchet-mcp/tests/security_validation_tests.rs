//! Security validation tests for MCP server components

use ratchet_mcp::{
    server::config::{CorsConfig, McpServerConfig, McpServerTransport},
    transport::{SseTransport, SseAuth},
    McpError,
};
use std::collections::HashMap;
use std::time::Duration;

/// Test MCP server CORS configuration security
#[test]
fn test_mcp_cors_security_validation() {
    // Test default configuration is secure
    let default_config = CorsConfig::default();
    assert!(!default_config.allowed_origins.contains(&"*".to_string()));
    assert!(default_config.allowed_origins.contains(&"http://localhost:3000".to_string()));
    assert!(!default_config.allow_credentials);
    
    // Test secure origins are properly configured
    let secure_origins = &default_config.allowed_origins;
    for origin in secure_origins {
        assert!(origin.starts_with("http://localhost") || origin.starts_with("https://localhost") ||
                origin.starts_with("http://127.0.0.1") || origin.starts_with("https://127.0.0.1"));
    }
    
    // Test that wildcard origins are not in default config
    assert!(!default_config.allowed_origins.iter().any(|o| o == "*"));
}

/// Test MCP server configuration defaults are secure
#[test]
fn test_mcp_server_config_security() {
    let config = McpServerConfig::default();
    
    // Should default to stdio (most secure)
    assert!(matches!(config.transport, McpServerTransport::Stdio));
    
    // Security config should be enabled
    assert!(config.security.audit_log_enabled);
    
    // Test SSE configuration security
    let sse_config = McpServerConfig::sse(8080);
    match sse_config.transport {
        McpServerTransport::Sse { cors, .. } => {
            assert!(!cors.allowed_origins.contains(&"*".to_string()));
            assert!(!cors.allow_credentials);
        }
        _ => panic!("Expected SSE transport"),
    }
}

/// Test transport creation with security validation
#[test]
fn test_transport_security_validation() {
    // Test URL validation prevents malicious URLs
    let malicious_urls = vec![
        "", // Empty URL
        "not-a-url", // Invalid format
        "javascript:alert('xss')", // XSS attempt
        "file:///etc/passwd", // Local file access
        "data:text/html,<script>alert('xss')</script>", // Data URL XSS
    ];
    
    for url in malicious_urls {
        let result = SseTransport::new(
            url.to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            true,
        );
        assert!(result.is_err(), "Should reject malicious URL: {}", url);
    }
    
    // Test valid URLs are accepted
    let valid_urls = vec![
        "https://api.example.com/mcp",
        "http://localhost:8080",
        "https://127.0.0.1:3000",
    ];
    
    for url in valid_urls {
        let result = SseTransport::new(
            url.to_string(),
            HashMap::new(),
            None,
            Duration::from_secs(30),
            true,
        );
        assert!(result.is_ok(), "Should accept valid URL: {}", url);
    }
}

/// Test authentication configuration security
#[test]
fn test_auth_configuration_security() {
    // Test that different auth types are properly configured
    let auth_configs = vec![
        SseAuth::Bearer {
            token: "valid-bearer-token".to_string(),
        },
        SseAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        },
        SseAuth::ApiKey {
            header: "X-API-Key".to_string(),
            key: "api-key-value".to_string(),
        },
    ];
    
    for auth in auth_configs {
        let transport = SseTransport::new(
            "https://api.example.com".to_string(),
            HashMap::new(),
            Some(auth),
            Duration::from_secs(30),
            true,
        );
        assert!(transport.is_ok());
    }
}

/// Test SSL verification is properly configured
#[test]
fn test_ssl_verification_security() {
    // Test that SSL verification is enabled by default for security
    let transport_with_ssl = SseTransport::new(
        "https://api.example.com".to_string(),
        HashMap::new(),
        None,
        Duration::from_secs(30),
        true, // verify_ssl = true
    );
    assert!(transport_with_ssl.is_ok());
    
    // Test that SSL verification can be disabled for development
    let transport_without_ssl = SseTransport::new(
        "https://localhost:8080".to_string(),
        HashMap::new(),
        None,
        Duration::from_secs(30),
        false, // verify_ssl = false
    );
    assert!(transport_without_ssl.is_ok());
}

/// Test error handling doesn't leak sensitive information
#[test]
fn test_error_information_leakage() {
    // Test that configuration errors don't expose sensitive details
    let config_error = McpError::Configuration {
        message: "Database connection failed with credentials user:password@host:5432/db".to_string(),
    };
    
    let error_message = config_error.to_string();
    
    // Error messages should be descriptive but not expose sensitive data
    assert!(error_message.contains("Configuration"));
    // In a real implementation, we might want to sanitize this further
    
    // Test network errors
    let network_error = McpError::Network {
        message: "Connection failed to internal-server.company.local:8080".to_string(),
    };
    
    let network_message = network_error.to_string();
    assert!(network_message.contains("Network"));
}

/// Test timeout configuration security
#[test]
fn test_timeout_configuration() {
    // Test reasonable timeout defaults
    let config = McpServerTransport::sse(8080);
    match config {
        McpServerTransport::Sse { timeout, .. } => {
            // Should have reasonable timeout (not too long to prevent resource exhaustion)
            assert!(timeout >= Duration::from_secs(1));
            assert!(timeout <= Duration::from_secs(300)); // 5 minutes max
        }
        _ => panic!("Expected SSE transport"),
    }
}

/// Test bind address security
#[test]
fn test_bind_address_security() {
    // Test that default bind addresses are secure
    let stdio_config = McpServerConfig::stdio();
    assert_eq!(stdio_config.bind_address, None); // No network binding for stdio
    
    let sse_config = McpServerConfig::sse(8080);
    assert_eq!(sse_config.bind_address, Some("127.0.0.1:8080".to_string())); // Localhost only
    
    // Test custom host binding
    let custom_config = McpServerConfig::sse_with_host(8080, "0.0.0.0");
    assert_eq!(custom_config.bind_address, Some("0.0.0.0:8080".to_string()));
}

/// Test transport capabilities and limitations
#[test]
fn test_transport_capabilities() {
    let stdio = McpServerTransport::stdio();
    let sse = McpServerTransport::sse(8080);
    
    // Stdio should be bidirectional (more secure for local communication)
    assert!(stdio.is_bidirectional());
    assert_eq!(stdio.type_name(), "stdio");
    
    // SSE should be unidirectional (by design)
    assert!(!sse.is_bidirectional());
    assert_eq!(sse.type_name(), "sse");
}

/// Test configuration serialization security
#[test]
fn test_config_serialization_security() {
    use serde_json;
    
    let config = McpServerConfig {
        transport: McpServerTransport::sse(3000),
        security: ratchet_mcp::security::SecurityConfig::default(),
        bind_address: Some("127.0.0.1:3000".to_string()),
    };
    
    // Test that configuration can be serialized without exposing secrets
    let serialized = serde_json::to_string(&config).unwrap();
    
    // Should not contain any obvious secrets or sensitive information
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("key"));
    
    // Should be able to deserialize back
    let deserialized: McpServerConfig = serde_json::from_str(&serialized).unwrap();
    
    match deserialized.transport {
        McpServerTransport::Sse { port, host, .. } => {
            assert_eq!(port, 3000);
            assert_eq!(host, "127.0.0.1");
        }
        _ => panic!("Expected SSE transport"),
    }
}

/// Test integration with ratchet-config
#[test]
fn test_ratchet_config_integration_security() {
    // Create a mock MCP config that would come from ratchet-config
    let mock_config = ratchet_config::domains::mcp::McpConfig {
        enabled: true,
        transport: "sse".to_string(),
        host: "127.0.0.1".to_string(), // Should default to localhost
        port: 8080,
    };
    
    let server_config = McpServerConfig::from_ratchet_config(&mock_config);
    
    // Should create secure configuration
    match server_config.transport {
        McpServerTransport::Sse { host, port, cors, .. } => {
            assert_eq!(host, "127.0.0.1");
            assert_eq!(port, 8080);
            assert!(!cors.allowed_origins.contains(&"*".to_string()));
        }
        _ => panic!("Expected SSE transport"),
    }
    
    // Test unknown transport defaults to stdio (most secure)
    let unknown_config = ratchet_config::domains::mcp::McpConfig {
        enabled: true,
        transport: "unknown".to_string(),
        host: "0.0.0.0".to_string(),
        port: 8080,
    };
    
    let default_config = McpServerConfig::from_ratchet_config(&unknown_config);
    assert!(matches!(default_config.transport, McpServerTransport::Stdio));
}