//! Integration tests for SSE (Server-Sent Events) transport

use std::collections::HashMap;
use std::time::Duration;

use crate::{
    config::{McpConfig, SimpleTransportType},
    protocol::{ClientCapabilities, ClientInfo, InitializeParams, JsonRpcRequest},
    security::McpAuth,
    transport::{McpTransport, SseAuth, SseTransport, TransportType},
    McpError,
};

/// Test helper to create a basic MCP config for SSE testing
fn create_test_sse_config(port: u16) -> McpConfig {
    McpConfig {
        transport_type: SimpleTransportType::Sse,
        host: "127.0.0.1".to_string(),
        port,
        auth: McpAuth::None,
        limits: crate::config::ConnectionLimits::default(),
        timeouts: crate::config::Timeouts {
            request_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(30),
        },
        tools: crate::config::ToolConfig::default(),
    }
}

/// Test helper to create an initialize request
#[allow(dead_code)]
fn create_initialize_request() -> JsonRpcRequest {
    let params = InitializeParams {
        protocol_version: "2024-11-05".to_string(),
        capabilities: ClientCapabilities::default(),
        client_info: Some(ClientInfo {
            name: "Test Client".to_string(),
            version: "1.0.0".to_string(),
            metadata: HashMap::new(),
        }),
    };

    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(serde_json::to_value(params).unwrap()),
        id: Some(serde_json::Value::String("test-1".to_string())),
    }
}

#[tokio::test]
async fn test_sse_server_startup() {
    // Start SSE server on a test port
    let config = create_test_sse_config(3001); // Use a test port

    // Create a simple test adapter (we can't easily create a real one in tests)
    // So we'll just test that the config is valid
    assert!(config.validate().is_ok());
    assert_eq!(config.transport_type, SimpleTransportType::Sse);
}

#[tokio::test]
async fn test_sse_transport_creation() {
    let transport = SseTransport::new(
        "http://localhost:8080".to_string(),
        HashMap::new(),
        None,
        Duration::from_secs(30),
        false, // Don't verify SSL for tests
    );

    assert!(transport.is_ok());
    let transport = transport.unwrap();
    assert!(!transport.is_connected().await);
}

#[tokio::test]
async fn test_sse_transport_with_auth() {
    let auth_bearer = SseAuth::Bearer {
        token: "test-bearer-token".to_string(),
    };

    let transport = SseTransport::new(
        "http://localhost:8080".to_string(),
        HashMap::new(),
        Some(auth_bearer),
        Duration::from_secs(30),
        false,
    );

    assert!(transport.is_ok());
}

#[tokio::test]
async fn test_sse_transport_with_custom_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
    headers.insert("User-Agent".to_string(), "RatchetMCP/1.0".to_string());

    let transport = SseTransport::new(
        "http://localhost:8080".to_string(),
        headers,
        None,
        Duration::from_secs(30),
        false,
    );

    assert!(transport.is_ok());
}

#[tokio::test]
async fn test_sse_transport_invalid_url() {
    let transport = SseTransport::new(
        "invalid-url".to_string(),
        HashMap::new(),
        None,
        Duration::from_secs(30),
        false,
    );

    assert!(transport.is_err());
    assert!(matches!(transport.unwrap_err(), McpError::Configuration { .. }));
}

#[tokio::test]
async fn test_sse_transport_empty_url() {
    let transport = SseTransport::new("".to_string(), HashMap::new(), None, Duration::from_secs(30), false);

    assert!(transport.is_err());
    assert!(matches!(transport.unwrap_err(), McpError::Configuration { .. }));
}

#[tokio::test]
async fn test_sse_health_tracking() {
    let transport = SseTransport::new(
        "http://localhost:8080".to_string(),
        HashMap::new(),
        None,
        Duration::from_secs(30),
        false,
    )
    .unwrap();

    let health = transport.health().await;
    assert!(!health.is_healthy());
    assert!(!health.connected);
    assert!(health.last_success.is_none());
    assert!(health.consecutive_failures > 0);
}

#[tokio::test]
async fn test_transport_factory_sse() {
    let config = TransportType::Sse {
        url: "http://localhost:8080".to_string(),
        headers: HashMap::new(),
        auth: None,
        timeout: Duration::from_secs(30),
        verify_ssl: false,
    };

    let transport = crate::transport::TransportFactory::create(config).await;
    assert!(transport.is_ok());
}

#[test]
fn test_sse_auth_serialization() {
    let auth_configs = vec![
        SseAuth::Bearer {
            token: "test-token".to_string(),
        },
        SseAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        },
        SseAuth::ApiKey {
            header: "X-API-Key".to_string(),
            key: "api-key".to_string(),
        },
    ];

    for auth in auth_configs {
        let serialized = serde_json::to_value(&auth).unwrap();
        let deserialized: SseAuth = serde_json::from_value(serialized).unwrap();
        assert_eq!(auth, deserialized);
    }
}

#[test]
fn test_transport_type_sse_validation() {
    let valid_config = TransportType::Sse {
        url: "https://example.com/mcp".to_string(),
        headers: HashMap::new(),
        auth: None,
        timeout: Duration::from_secs(30),
        verify_ssl: true,
    };

    assert!(valid_config.validate().is_ok());

    let invalid_config = TransportType::Sse {
        url: "".to_string(),
        headers: HashMap::new(),
        auth: None,
        timeout: Duration::from_secs(30),
        verify_ssl: true,
    };

    assert!(invalid_config.validate().is_err());
}

/// Test that demonstrates SSE endpoint structure
#[test]
fn test_sse_endpoint_structure() {
    // This test documents the expected SSE endpoint structure:
    // - GET /sse/{session_id} - SSE connection endpoint
    // - POST /message/{session_id} - Message sending endpoint
    // - GET /health - Health check endpoint

    let base_url = "http://localhost:8080";
    let session_id = "test-session-123";

    let expected_sse_url = format!("{}/sse/{}", base_url, session_id);
    let expected_message_url = format!("{}/message/{}", base_url, session_id);
    let expected_health_url = format!("{}/health", base_url);

    assert_eq!(expected_sse_url, "http://localhost:8080/sse/test-session-123");
    assert_eq!(expected_message_url, "http://localhost:8080/message/test-session-123");
    assert_eq!(expected_health_url, "http://localhost:8080/health");
}

/// Test that verifies SSE transport configuration
#[test]
fn test_sse_transport_configuration() {
    let config = TransportType::Sse {
        url: "http://localhost:8080".to_string(),
        headers: [
            ("Authorization".to_string(), "Bearer token123".to_string()),
            ("X-Client-Version".to_string(), "1.0.0".to_string()),
        ]
        .into(),
        auth: Some(SseAuth::Bearer {
            token: "test-token".to_string(),
        }),
        timeout: Duration::from_secs(60),
        verify_ssl: false,
    };

    assert_eq!(config.type_name(), "sse");
    assert!(!config.is_bidirectional()); // SSE is typically unidirectional from server to client
    assert!(config.validate().is_ok());
}
