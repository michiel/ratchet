use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{error::McpResult, security::McpAuth};

/// Simple transport type for basic configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimpleTransportType {
    #[serde(rename = "stdio")]
    Stdio,
    #[serde(rename = "sse")]
    Sse,
}

impl Default for SimpleTransportType {
    fn default() -> Self {
        Self::Stdio
    }
}

/// Configuration for the MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Transport type to use
    #[serde(default)]
    pub transport_type: SimpleTransportType,

    /// Server host for SSE transport
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port for SSE transport
    #[serde(default = "default_port")]
    pub port: u16,

    /// Authentication configuration
    #[serde(default)]
    pub auth: McpAuth,

    /// Connection limits
    #[serde(default)]
    pub limits: ConnectionLimits,

    /// Timeouts configuration
    #[serde(default)]
    pub timeouts: Timeouts,

    /// Tool configuration
    #[serde(default)]
    pub tools: ToolConfig,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport_type: SimpleTransportType::Stdio,
            host: default_host(),
            port: default_port(),
            auth: McpAuth::default(),
            limits: ConnectionLimits::default(),
            timeouts: Timeouts::default(),
            tools: ToolConfig::default(),
        }
    }
}

/// Connection limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLimits {
    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum message size in bytes
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,

    /// Rate limit: requests per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            max_message_size: default_max_message_size(),
            rate_limit: default_rate_limit(),
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeouts {
    /// Request processing timeout
    #[serde(with = "humantime_serde", default = "default_request_timeout")]
    pub request_timeout: Duration,

    /// Connection idle timeout
    #[serde(with = "humantime_serde", default = "default_idle_timeout")]
    pub idle_timeout: Duration,

    /// Health check interval
    #[serde(with = "humantime_serde", default = "default_health_check_interval")]
    pub health_check_interval: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            request_timeout: default_request_timeout(),
            idle_timeout: default_idle_timeout(),
            health_check_interval: default_health_check_interval(),
        }
    }
}

/// Tool-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Enable execution tools
    #[serde(default = "default_true")]
    pub enable_execution: bool,

    /// Enable logging tools
    #[serde(default = "default_true")]
    pub enable_logging: bool,

    /// Enable monitoring tools
    #[serde(default = "default_true")]
    pub enable_monitoring: bool,

    /// Enable debugging tools
    #[serde(default = "default_false")]
    pub enable_debugging: bool,

    /// Maximum execution time for tasks
    #[serde(with = "humantime_serde", default = "default_max_execution_time")]
    pub max_execution_time: Duration,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enable_execution: true,
            enable_logging: true,
            enable_monitoring: true,
            enable_debugging: false,
            max_execution_time: default_max_execution_time(),
        }
    }
}

// Default value functions
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_max_connections() -> usize {
    100
}

fn default_max_message_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_rate_limit() -> u32 {
    60 // 60 requests per minute
}

fn default_request_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_health_check_interval() -> Duration {
    Duration::from_secs(30)
}

fn default_max_execution_time() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl McpConfig {
    /// Load configuration from a file
    pub async fn from_file(path: &str) -> McpResult<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            crate::error::McpError::Configuration {
                message: format!("Failed to read config file '{}': {}", path, e),
            }
        })?;

        let config: Self =
            serde_yaml::from_str(&content).map_err(|e| crate::error::McpError::Configuration {
                message: format!("Failed to parse config file '{}': {}", path, e),
            })?;

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> McpResult<()> {
        if self.port == 0 {
            return Err(crate::error::McpError::Configuration {
                message: "Port cannot be 0".to_string(),
            });
        }

        if self.limits.max_connections == 0 {
            return Err(crate::error::McpError::Configuration {
                message: "max_connections cannot be 0".to_string(),
            });
        }

        if self.limits.max_message_size == 0 {
            return Err(crate::error::McpError::Configuration {
                message: "max_message_size cannot be 0".to_string(),
            });
        }

        if self.timeouts.request_timeout.is_zero() {
            return Err(crate::error::McpError::Configuration {
                message: "request_timeout cannot be 0".to_string(),
            });
        }

        Ok(())
    }

    /// Merge with environment variables
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(host) = std::env::var("MCP_HOST") {
            self.host = host;
        }

        if let Ok(port) = std::env::var("MCP_PORT") {
            if let Ok(port) = port.parse() {
                self.port = port;
            }
        }

        if let Ok(transport) = std::env::var("MCP_TRANSPORT") {
            match transport.to_lowercase().as_str() {
                "stdio" => self.transport_type = SimpleTransportType::Stdio,
                "sse" => self.transport_type = SimpleTransportType::Sse,
                _ => {}
            }
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = McpConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = McpConfig::default();

        // Test invalid port
        config.port = 0;
        assert!(config.validate().is_err());

        config.port = 3000;
        assert!(config.validate().is_ok());

        // Test invalid max_connections
        config.limits.max_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("MCP_HOST", "0.0.0.0");
        std::env::set_var("MCP_PORT", "8080");
        std::env::set_var("MCP_TRANSPORT", "sse");

        let config = McpConfig::default().with_env_overrides();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(matches!(config.transport_type, SimpleTransportType::Sse));

        // Clean up
        std::env::remove_var("MCP_HOST");
        std::env::remove_var("MCP_PORT");
        std::env::remove_var("MCP_TRANSPORT");
    }
}
