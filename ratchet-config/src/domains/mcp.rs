//! MCP (Model Context Protocol) configuration

use crate::error::ConfigResult;
use crate::validation::Validatable;
use serde::{Deserialize, Serialize};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    /// Whether MCP server is enabled
    #[serde(default = "crate::domains::utils::default_false")]
    pub enabled: bool,

    /// Transport protocol ("stdio" or "sse")
    #[serde(default = "default_mcp_transport")]
    pub transport: String,

    /// Host address for SSE transport
    #[serde(default = "default_mcp_host")]
    pub host: String,

    /// Port for SSE transport
    #[serde(default = "default_mcp_port")]
    pub port: u16,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: default_mcp_transport(),
            host: default_mcp_host(),
            port: default_mcp_port(),
        }
    }
}

impl Validatable for McpConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate transport type
        let valid_transports = ["stdio", "sse"];
        crate::validation::validate_enum_choice(
            &self.transport,
            &valid_transports,
            "transport",
            self.domain_name(),
        )?;

        // Validate port range if using SSE
        if self.transport == "sse" {
            crate::validation::validate_port_range(self.port, "port", self.domain_name())?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "mcp"
    }
}

// Default value functions
fn default_mcp_transport() -> String {
    "stdio".to_string()
}

fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}

fn default_mcp_port() -> u16 {
    3001
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_defaults() {
        let config = McpConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.transport, "stdio");
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3001);
    }

    #[test]
    fn test_mcp_config_validation() {
        let mut config = McpConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid transport
        config.transport = "invalid".to_string();
        assert!(config.validate().is_err());

        // Test valid SSE transport
        config.transport = "sse".to_string();
        assert!(config.validate().is_ok());
    }
}
