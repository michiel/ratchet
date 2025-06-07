use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Serde helper for Duration serialization as seconds
mod serde_duration_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(seconds))
    }
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Environment variable error: {0}")]
    EnvError(String),
}

/// Main Ratchet configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RatchetConfig {
    /// Task execution configuration
    #[serde(default)]
    pub execution: ExecutionConfig,

    /// HTTP client configuration
    #[serde(default)]
    pub http: ratchet_http::HttpConfig,

    /// Caching configuration
    #[serde(default)]
    pub cache: CacheConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: crate::logging::LoggingConfig,

    /// Output destinations configuration
    #[serde(default)]
    pub output: OutputConfig,

    /// Server configuration (optional, for future server mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerConfig>,

    /// MCP server configuration (optional, for LLM integration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpServerConfig>,

    /// Registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistryConfig>,
}

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// Maximum execution duration in seconds
    #[serde(default = "default_execution_duration")]
    pub max_execution_duration: u64,

    /// Whether to validate JSON schemas
    #[serde(default = "default_true")]
    pub validate_schemas: bool,

    /// Maximum concurrent tasks
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,

    /// Grace period for task termination
    #[serde(default = "default_grace_period")]
    pub timeout_grace_period: u64,
}


/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Whether caching is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Task content cache size
    #[serde(default = "default_cache_size")]
    pub task_content_cache_size: usize,
}

/// Output destinations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Maximum number of concurrent deliveries
    #[serde(default = "default_concurrent_deliveries")]
    pub max_concurrent_deliveries: usize,

    /// Default timeout for deliveries
    #[serde(default = "default_delivery_timeout")]
    pub default_timeout: u64,

    /// Whether to validate destinations on startup
    #[serde(default = "default_true")]
    pub validate_on_startup: bool,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Server bind address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
    // JWT configuration removed - not implemented
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database URL
    #[serde(default = "default_database_url")]
    pub url: String,

    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Connection timeout in seconds
    #[serde(
        with = "serde_duration_seconds",
        default = "default_connection_timeout_duration"
    )]
    pub connection_timeout: Duration,
}

// JWT configuration removed - not implemented

/// Simple MCP server configuration - basic settings only
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerConfig {
    /// Whether MCP server is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Transport type (stdio, sse)
    #[serde(default = "default_mcp_transport")]
    pub transport: String,

    /// Host for network transports (ignored for stdio)
    #[serde(default = "default_mcp_host")]
    pub host: String,

    /// Port for network transports (ignored for stdio)
    #[serde(default = "default_mcp_port")]
    pub port: u16,
}

// MCP authentication and complex configurations removed - use ratchet-mcp crate for full MCP configuration

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct RegistryConfig {
    /// Registry sources
    #[serde(default)]
    pub sources: Vec<RegistrySource>,
}

/// Registry source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct RegistrySource {
    /// Source name
    pub name: String,

    /// Source URI
    pub uri: String,
}

/// Registry source configuration (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct RegistrySourceConfig {
    /// Source name
    pub name: String,

    /// Source URI
    pub uri: String,

    /// Source-specific configuration
    #[serde(default)]
    pub config: RegistrySourceSettings,
}

/// Registry source settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RegistrySourceSettings {
    /// Whether to watch for changes
    #[serde(default = "default_false")]
    pub watch_for_changes: bool,

    /// Whether to auto-reload on changes
    #[serde(default = "default_false")]
    pub auto_reload: bool,
}

// Default value functions
fn default_execution_duration() -> u64 {
    300
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_max_concurrent_tasks() -> usize {
    10
}
fn default_grace_period() -> u64 {
    5
}
fn default_cache_size() -> usize {
    100
}
fn default_concurrent_deliveries() -> usize {
    10
}
fn default_delivery_timeout() -> u64 {
    30
}
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_database_url() -> String {
    "sqlite::memory:".to_string()
}
fn default_max_connections() -> u32 {
    10
}
fn default_connection_timeout_duration() -> Duration {
    Duration::from_secs(30)
}
// JWT default function removed
fn default_mcp_transport() -> String {
    "stdio".to_string()
}
fn default_mcp_host() -> String {
    "127.0.0.1".to_string()
}
fn default_mcp_port() -> u16 {
    3000
}
// Removed unused MCP default functions

// Default implementations
impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_duration: default_execution_duration(),
            validate_schemas: default_true(),
            max_concurrent_tasks: default_max_concurrent_tasks(),
            timeout_grace_period: default_grace_period(),
        }
    }
}


impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            task_content_cache_size: default_cache_size(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_concurrent_deliveries: default_concurrent_deliveries(),
            default_timeout: default_delivery_timeout(),
            validate_on_startup: default_true(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            database: DatabaseConfig::default(),
            // JWT removed
        }
    }
}

// JWT default implementation removed

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout_duration(),
        }
    }
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            transport: default_mcp_transport(),
            host: default_mcp_host(),
            port: default_mcp_port(),
        }
    }
}

impl Default for RegistrySourceSettings {
    fn default() -> Self {
        Self {
            watch_for_changes: default_false(),
            auto_reload: default_false(),
        }
    }
}

impl RatchetConfig {
    /// Load configuration from a YAML file with environment variable overrides
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: RatchetConfig = serde_yaml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from environment variables only
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = RatchetConfig::default();
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // HTTP timeout override
        if let Ok(timeout) = std::env::var("RATCHET_HTTP_TIMEOUT") {
            let timeout_secs: u64 = timeout.parse().map_err(|e| {
                ConfigError::EnvError(format!("Invalid RATCHET_HTTP_TIMEOUT: {}", e))
            })?;
            self.http.timeout = Duration::from_secs(timeout_secs);
        }

        // Database URL override
        if let Ok(url) = std::env::var("RATCHET_DATABASE_URL") {
            if let Some(ref mut server) = self.server {
                server.database.url = url;
            }
        }

        // Server port override
        if let Ok(port) = std::env::var("RATCHET_SERVER_PORT") {
            if let Some(ref mut server) = self.server {
                server.port = port.parse().map_err(|e| {
                    ConfigError::EnvError(format!("Invalid RATCHET_SERVER_PORT: {}", e))
                })?;
            }
        }

        // MCP database configuration removed - database is managed centrally through server.database

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate execution config
        if self.execution.max_execution_duration == 0 {
            return Err(ConfigError::ValidationError(
                "max_execution_duration must be greater than 0".to_string(),
            ));
        }

        if self.execution.max_concurrent_tasks == 0 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_tasks must be greater than 0".to_string(),
            ));
        }

        // Validate HTTP config
        if self.http.timeout.is_zero() {
            return Err(ConfigError::ValidationError(
                "HTTP timeout must be greater than 0".to_string(),
            ));
        }

        // Validate output config
        if self.output.max_concurrent_deliveries == 0 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_deliveries must be greater than 0".to_string(),
            ));
        }

        if self.output.default_timeout == 0 {
            return Err(ConfigError::ValidationError(
                "default_timeout must be greater than 0".to_string(),
            ));
        }

        // Validate server config if present
        if let Some(ref server) = self.server {
            if server.port == 0 {
                return Err(ConfigError::ValidationError(
                    "Server port must be greater than 0".to_string(),
                ));
            }
        }

        // Validate MCP config if present and enabled
        if let Some(ref mcp) = self.mcp {
            if mcp.enabled {
                if mcp.transport != "stdio" && mcp.transport != "sse" {
                    return Err(ConfigError::ValidationError(
                        "MCP transport must be 'stdio' or 'sse'".to_string(),
                    ));
                }

                if mcp.transport != "stdio" && mcp.port == 0 {
                    return Err(ConfigError::ValidationError(
                        "MCP port must be greater than 0 for network transports".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get default configuration for development
    pub fn development() -> Self {
        let mut config = Self::default();

        // Enable server for development
        config.server = Some(ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            database: DatabaseConfig {
                url: "sqlite:./dev-ratchet.db".to_string(),
                max_connections: 10,
                connection_timeout: Duration::from_secs(30),
            },
            // JWT removed
        });

        // Enable MCP server for development
        config.mcp = Some(McpServerConfig {
            enabled: true,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
        });

        config
    }
}
