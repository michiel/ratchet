//! MCP server configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::security::SecurityConfig;

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport configuration
    pub transport: McpServerTransport,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Bind address for network transports
    pub bind_address: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            transport: McpServerTransport::Stdio,
            security: SecurityConfig::default(),
            bind_address: None,
        }
    }
}

/// MCP server transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerTransport {
    /// Standard I/O transport (for local processes)
    #[serde(rename = "stdio")]
    Stdio,
    
    /// Server-Sent Events transport (for HTTP connections)
    #[serde(rename = "sse")]
    Sse {
        /// Port to bind to
        port: u16,
        
        /// Host to bind to
        host: String,
        
        /// Whether to use TLS
        tls: bool,
        
        /// CORS configuration
        cors: CorsConfig,
        
        /// Request timeout
        timeout: Duration,
    },
}

/// CORS configuration for SSE transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    
    /// Whether to allow credentials
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: false,
        }
    }
}

impl Default for McpServerTransport {
    fn default() -> Self {
        Self::Stdio
    }
}

impl McpServerTransport {
    /// Create stdio transport
    pub fn stdio() -> Self {
        Self::Stdio
    }
    
    /// Create SSE transport with default settings
    pub fn sse(port: u16) -> Self {
        Self::Sse {
            port,
            host: "127.0.0.1".to_string(),
            tls: false,
            cors: CorsConfig::default(),
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Create SSE transport with TLS
    pub fn sse_tls(port: u16, host: impl Into<String>) -> Self {
        Self::Sse {
            port,
            host: host.into(),
            tls: true,
            cors: CorsConfig::default(),
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Check if this transport supports bidirectional communication
    pub fn is_bidirectional(&self) -> bool {
        match self {
            Self::Stdio => true,
            Self::Sse { .. } => false, // SSE is typically unidirectional
        }
    }
    
    /// Get the transport type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse { .. } => "sse",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpServerConfig::default();
        assert!(matches!(config.transport, McpServerTransport::Stdio));
        assert!(config.security.audit_log_enabled);
    }

    #[test]
    fn test_transport_creation() {
        let stdio = McpServerTransport::stdio();
        assert!(stdio.is_bidirectional());
        assert_eq!(stdio.type_name(), "stdio");
        
        let sse = McpServerTransport::sse(8080);
        assert!(!sse.is_bidirectional());
        assert_eq!(sse.type_name(), "sse");
        
        match sse {
            McpServerTransport::Sse { port, host, tls, .. } => {
                assert_eq!(port, 8080);
                assert_eq!(host, "127.0.0.1");
                assert!(!tls);
            }
            _ => panic!("Expected SSE transport"),
        }
    }

    #[test]
    fn test_cors_config() {
        let cors = CorsConfig::default();
        assert!(cors.allowed_origins.contains(&"*".to_string()));
        assert!(cors.allowed_methods.contains(&"GET".to_string()));
        assert!(!cors.allow_credentials);
    }

    #[test]
    fn test_config_serialization() {
        let config = McpServerConfig {
            transport: McpServerTransport::sse(3000),
            security: SecurityConfig::default(),
            bind_address: Some("0.0.0.0:3000".to_string()),
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: McpServerConfig = serde_json::from_str(&serialized).unwrap();
        
        match deserialized.transport {
            McpServerTransport::Sse { port, .. } => assert_eq!(port, 3000),
            _ => panic!("Expected SSE transport"),
        }
    }
}