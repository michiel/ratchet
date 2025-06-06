//! Domain-specific configuration modules

pub mod cache;
pub mod database;
pub mod execution;
pub mod http;
pub mod logging;
pub mod mcp;
pub mod output;
pub mod registry;
pub mod server;
pub mod utils;

use crate::error::ConfigResult;
use crate::validation::Validatable;
use serde::{Deserialize, Serialize};

/// Main Ratchet configuration combining all domains
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RatchetConfig {
    /// Task execution configuration
    #[serde(default)]
    pub execution: execution::ExecutionConfig,

    /// HTTP client configuration
    #[serde(default)]
    pub http: http::HttpConfig,

    /// Caching configuration
    #[serde(default)]
    pub cache: cache::CacheConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: logging::LoggingConfig,

    /// Output destinations configuration
    #[serde(default)]
    pub output: output::OutputConfig,

    /// Server configuration (optional, for server mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<server::ServerConfig>,

    /// Registry configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<registry::RegistryConfig>,

    /// MCP server configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<mcp::McpConfig>,
}

impl RatchetConfig {
    /// Validate all domain configurations
    pub fn validate_all(&self) -> ConfigResult<()> {
        // Validate each domain
        self.execution.validate()?;
        self.http.validate()?;
        self.cache.validate()?;
        self.logging.validate()?;
        self.output.validate()?;

        if let Some(ref server) = self.server {
            server.validate()?;
        }

        if let Some(ref registry) = self.registry {
            registry.validate()?;
        }

        if let Some(ref mcp) = self.mcp {
            mcp.validate()?;
        }

        Ok(())
    }

    /// Generate a sample configuration file
    pub fn generate_sample() -> String {
        let config = RatchetConfig::default();
        serde_yaml::to_string(&config)
            .unwrap_or_else(|_| "# Failed to generate sample config".to_string())
    }
}
