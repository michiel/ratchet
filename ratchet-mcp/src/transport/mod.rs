//! Transport layer abstractions for MCP communication

pub mod connection;
pub mod sse;
pub mod stdio;
pub mod streamable_http;

pub use connection::{ConnectionHealth, ConnectionPool, HealthMonitor};
pub use sse::SseTransport;
pub use stdio::StdioTransport;
pub use streamable_http::{StreamableHttpTransport, SessionManager, EventStore, InMemoryEventStore, McpEvent};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::{McpError, McpResult};

/// Transport type configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransportType {
    /// Standard I/O transport for local processes
    #[serde(rename = "stdio")]
    Stdio {
        /// Command to execute
        command: String,

        /// Command arguments
        #[serde(default)]
        args: Vec<String>,

        /// Environment variables
        #[serde(default)]
        env: std::collections::HashMap<String, String>,

        /// Working directory
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },

    /// Server-Sent Events transport for HTTP connections
    #[serde(rename = "sse")]
    Sse {
        /// Base URL for the SSE endpoint
        url: String,

        /// HTTP headers to include
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,

        /// Authentication configuration
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<SseAuth>,

        /// Connection timeout
        #[serde(default = "default_timeout")]
        timeout: Duration,

        /// Whether to verify SSL certificates
        #[serde(default = "default_true")]
        verify_ssl: bool,
    },

    /// Streamable HTTP transport for Claude MCP compatibility
    #[serde(rename = "streamable_http")]
    StreamableHttp {
        /// Base URL for the MCP endpoint
        url: String,

        /// HTTP headers to include
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,

        /// Authentication configuration
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<SseAuth>,

        /// Connection timeout
        #[serde(default = "default_timeout")]
        timeout: Duration,

        /// Whether to verify SSL certificates
        #[serde(default = "default_true")]
        verify_ssl: bool,

        /// Maximum events per session
        #[serde(default = "default_max_events")]
        max_events_per_session: usize,

        /// Session timeout
        #[serde(default = "default_session_timeout")]
        session_timeout: Duration,
    },
}

/// SSE authentication configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SseAuth {
    #[serde(rename = "bearer")]
    Bearer { token: String },

    #[serde(rename = "basic")]
    Basic { username: String, password: String },

    #[serde(rename = "api_key")]
    ApiKey { header: String, key: String },
}

/// Transport trait for MCP communication
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Connect to the remote endpoint
    async fn connect(&mut self) -> McpResult<()>;

    /// Send a JSON-RPC message
    async fn send(&mut self, message: JsonRpcRequest) -> McpResult<()>;

    /// Receive a JSON-RPC message
    async fn receive(&mut self) -> McpResult<JsonRpcResponse>;

    /// Send a message and wait for response with timeout
    async fn send_and_receive(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Duration,
    ) -> McpResult<JsonRpcResponse> {
        // Send the request
        self.send(request).await?;

        // Wait for response with timeout
        timeout(timeout_duration, self.receive())
            .await
            .map_err(|_| McpError::ServerTimeout {
                timeout: timeout_duration,
            })?
    }

    /// Check if the transport is connected
    async fn is_connected(&self) -> bool;

    /// Get transport health information
    async fn health(&self) -> TransportHealth;

    /// Close the transport connection
    async fn close(&mut self) -> McpResult<()>;
}

/// Transport health information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportHealth {
    /// Whether the transport is connected
    pub connected: bool,

    /// Last successful message timestamp
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,

    /// Last error encountered
    pub last_error: Option<String>,

    /// Number of consecutive failures
    pub consecutive_failures: u32,

    /// Round-trip latency (if available)
    pub latency: Option<Duration>,

    /// Additional transport-specific metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl TransportHealth {
    /// Create a new healthy transport state
    pub fn healthy() -> Self {
        Self {
            connected: true,
            last_success: Some(chrono::Utc::now()),
            last_error: None,
            consecutive_failures: 0,
            latency: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a new unhealthy transport state
    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            connected: false,
            last_success: None,
            last_error: Some(error.into()),
            consecutive_failures: 1,
            latency: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Mark a successful operation
    pub fn mark_success(&mut self, latency: Option<Duration>) {
        self.connected = true;
        self.last_success = Some(chrono::Utc::now());
        self.last_error = None;
        self.consecutive_failures = 0;
        self.latency = latency;
    }

    /// Mark a failed operation
    pub fn mark_failure(&mut self, error: impl Into<String>) {
        self.connected = false;
        self.last_error = Some(error.into());
        self.consecutive_failures += 1;
    }

    /// Check if the transport is healthy
    pub fn is_healthy(&self) -> bool {
        self.connected && self.consecutive_failures == 0
    }
}

/// Transport factory for creating transport instances
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport from configuration
    pub async fn create(config: TransportType) -> McpResult<Box<dyn McpTransport>> {
        match config {
            TransportType::Stdio {
                command,
                args,
                env,
                cwd,
            } => Ok(Box::new(StdioTransport::new(command, args, env, cwd)?)),
            TransportType::Sse {
                url,
                headers,
                auth,
                timeout,
                verify_ssl,
            } => Ok(Box::new(SseTransport::new(url, headers, auth, timeout, verify_ssl)?)),
            TransportType::StreamableHttp {
                max_events_per_session,
                session_timeout,
                ..
            } => {
                let event_store = std::sync::Arc::new(InMemoryEventStore::new(
                    max_events_per_session,
                    session_timeout,
                ));
                let session_manager = std::sync::Arc::new(SessionManager::new(
                    event_store,
                    session_timeout,
                    Duration::from_secs(60), // cleanup interval
                ));
                Ok(Box::new(StreamableHttpTransport::new(session_manager)))
            }
        }
    }
}

/// Transport configuration validation
impl TransportType {
    /// Validate the transport configuration
    pub fn validate(&self) -> McpResult<()> {
        match self {
            TransportType::Stdio { command, .. } => {
                if command.trim().is_empty() {
                    return Err(McpError::Configuration {
                        message: "Stdio transport command cannot be empty".to_string(),
                    });
                }
            }
            TransportType::Sse { url, .. } => {
                if url.trim().is_empty() {
                    return Err(McpError::Configuration {
                        message: "SSE transport URL cannot be empty".to_string(),
                    });
                }

                // Validate URL format
                if let Err(e) = url::Url::parse(url) {
                    return Err(McpError::Configuration {
                        message: format!("Invalid SSE URL: {}", e),
                    });
                }
            }
            TransportType::StreamableHttp { url, .. } => {
                if url.trim().is_empty() {
                    return Err(McpError::Configuration {
                        message: "StreamableHttp transport URL cannot be empty".to_string(),
                    });
                }

                // Validate URL format
                if let Err(e) = url::Url::parse(url) {
                    return Err(McpError::Configuration {
                        message: format!("Invalid StreamableHttp URL: {}", e),
                    });
                }
            }
        }
        Ok(())
    }

    /// Get transport type name
    pub fn type_name(&self) -> &'static str {
        match self {
            TransportType::Stdio { .. } => "stdio",
            TransportType::Sse { .. } => "sse",
            TransportType::StreamableHttp { .. } => "streamable_http",
        }
    }

    /// Check if this transport supports bidirectional communication
    pub fn is_bidirectional(&self) -> bool {
        match self {
            TransportType::Stdio { .. } => true,
            TransportType::Sse { .. } => false, // SSE is typically unidirectional
            TransportType::StreamableHttp { .. } => true, // POST + SSE = bidirectional
        }
    }
}

// Helper functions for serde defaults
fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_true() -> bool {
    true
}

fn default_max_events() -> usize {
    1000
}

fn default_session_timeout() -> Duration {
    Duration::from_secs(1800) // 30 minutes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_serialization() {
        let stdio_config = TransportType::Stdio {
            command: "test-command".to_string(),
            args: vec!["--arg1".to_string(), "--arg2".to_string()],
            env: [("KEY".to_string(), "value".to_string())].into(),
            cwd: Some("/tmp".to_string()),
        };

        let serialized = serde_json::to_value(&stdio_config).unwrap();
        let deserialized: TransportType = serde_json::from_value(serialized).unwrap();

        assert_eq!(stdio_config, deserialized);
    }

    #[test]
    fn test_transport_validation() {
        let valid_stdio = TransportType::Stdio {
            command: "echo".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            cwd: None,
        };
        assert!(valid_stdio.validate().is_ok());

        let invalid_stdio = TransportType::Stdio {
            command: "".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            cwd: None,
        };
        assert!(invalid_stdio.validate().is_err());

        let valid_sse = TransportType::Sse {
            url: "https://example.com/sse".to_string(),
            headers: std::collections::HashMap::new(),
            auth: None,
            timeout: Duration::from_secs(30),
            verify_ssl: true,
        };
        assert!(valid_sse.validate().is_ok());

        let invalid_sse = TransportType::Sse {
            url: "not-a-url".to_string(),
            headers: std::collections::HashMap::new(),
            auth: None,
            timeout: Duration::from_secs(30),
            verify_ssl: true,
        };
        assert!(invalid_sse.validate().is_err());
    }

    #[test]
    fn test_transport_health() {
        let mut health = TransportHealth::healthy();
        assert!(health.is_healthy());
        assert!(health.connected);
        assert_eq!(health.consecutive_failures, 0);

        health.mark_failure("Connection failed");
        assert!(!health.is_healthy());
        assert!(!health.connected);
        assert_eq!(health.consecutive_failures, 1);

        health.mark_success(Some(Duration::from_millis(100)));
        assert!(health.is_healthy());
        assert!(health.connected);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.latency, Some(Duration::from_millis(100)));
    }
}
