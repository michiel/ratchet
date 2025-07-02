//! MCP server configuration

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,
    
    /// Server version
    pub version: String,
    
    /// Host to bind to
    pub host: String,
    
    /// Port to bind to
    pub port: u16,
    
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Whether to enable batch operations
    pub enable_batch: bool,
    
    /// Maximum batch size
    pub max_batch_size: usize,
    
    /// Session configuration
    pub session: SessionConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Additional server metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: "0.1.0".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            enable_batch: true,
            max_batch_size: 100,
            session: SessionConfig::default(),
            security: SecurityConfig::default(),
            metadata: HashMap::new(),
        }
    }
}

impl McpServerConfig {
    /// Create a new server config with SSE transport
    pub fn sse_with_host(port: u16, host: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            ..Default::default()
        }
    }
    
    /// Create a new server config with stdio transport
    pub fn stdio() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 0, // Not used for stdio
            ..Default::default()
        }
    }
    
    /// Enable batch operations with custom settings
    pub fn with_batch(mut self, max_size: usize) -> Self {
        self.enable_batch = true;
        self.max_batch_size = max_size;
        self
    }
    
    /// Set connection limits
    pub fn with_connection_limits(mut self, max_connections: usize, timeout: Duration) -> Self {
        self.max_connections = max_connections;
        self.connection_timeout = timeout;
        self
    }
    
    /// Add custom metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout
    pub timeout: Duration,
    
    /// Maximum events per session
    pub max_events: usize,
    
    /// Cleanup interval
    pub cleanup_interval: Duration,
    
    /// Whether to enable session resumability
    pub enable_resumability: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(1800), // 30 minutes
            max_events: 1000,
            cleanup_interval: Duration::from_secs(60),
            enable_resumability: true,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether to require authentication
    pub require_auth: bool,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    
    /// CORS configuration
    pub cors: CorsConfig,
    
    /// Whether to enable audit logging
    pub enable_audit: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_auth: false,
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            enable_audit: false,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    
    /// Maximum requests per window
    pub max_requests: u32,
    
    /// Time window for rate limiting
    pub window: Duration,
    
    /// Burst allowance
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_requests: 100,
            window: Duration::from_secs(60),
            burst: 10,
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Whether CORS is enabled
    pub enabled: bool,
    
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
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpServerConfig::default();
        assert_eq!(config.name, "MCP Server");
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.enable_batch);
        assert_eq!(config.max_batch_size, 100);
    }

    #[test]
    fn test_sse_config() {
        let config = McpServerConfig::sse_with_host(3000, "0.0.0.0");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_config_builder() {
        let config = McpServerConfig::default()
            .with_batch(50)
            .with_connection_limits(500, Duration::from_secs(15))
            .with_metadata("custom", serde_json::json!({"key": "value"}));
        
        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.max_connections, 500);
        assert_eq!(config.connection_timeout, Duration::from_secs(15));
        assert!(config.metadata.contains_key("custom"));
    }
}