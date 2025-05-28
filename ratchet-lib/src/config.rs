use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::path::PathBuf;
use thiserror::Error;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetConfig {
    /// Task execution configuration
    pub execution: ExecutionConfig,
    
    /// HTTP client configuration
    pub http: HttpConfig,
    
    /// Caching configuration
    pub cache: CacheConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
    
    /// Server configuration (optional, for future server mode)
    pub server: Option<ServerConfig>,
    
    /// Registry configuration
    pub registry: Option<RegistryConfig>,
}

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// JavaScript variable names used for fetch operations
    pub fetch_variables: FetchVariables,
    
    /// Maximum execution time for JavaScript tasks
    #[serde(with = "serde_duration_seconds")]
    pub max_execution_duration: Duration,
    
    /// Whether to validate schemas during execution
    pub validate_schemas: bool,
}

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout
    #[serde(with = "serde_duration_seconds")]
    pub timeout: Duration,
    
    /// Maximum number of redirects to follow
    pub max_redirects: u32,
    
    /// User agent string
    pub user_agent: String,
    
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// LRU cache size for task content
    pub task_content_cache_size: usize,
    
    /// Whether caching is enabled
    pub enabled: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    
    /// Whether to log to file
    pub log_to_file: bool,
    
    /// Log file path (if log_to_file is true)
    pub log_file_path: Option<PathBuf>,
}

/// Server configuration (for future server mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    
    /// Server port
    pub port: u16,
    
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://ratchet.db")
    pub url: String,
    
    /// Maximum number of database connections
    pub max_connections: u32,
    
    /// Connection timeout
    #[serde(with = "serde_duration_seconds")]
    pub connection_timeout: Duration,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,
    
    /// Token expiration time
    #[serde(with = "serde_duration_seconds")]
    pub token_expiration: Duration,
}

/// JavaScript fetch variables configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchVariables {
    /// Variable name for fetch URL
    pub url_var: String,
    
    /// Variable name for fetch parameters
    pub params_var: String,
    
    /// Variable name for fetch body
    pub body_var: String,
    
    /// Variable name for HTTP result
    pub result_var: String,
    
    /// Variable name for temporary result
    pub temp_result_var: String,
}

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

impl Default for RatchetConfig {
    fn default() -> Self {
        Self {
            execution: ExecutionConfig::default(),
            http: HttpConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
            server: None,
            registry: None,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            fetch_variables: FetchVariables::default(),
            max_execution_duration: Duration::from_secs(300), // 5 minutes
            validate_schemas: true,
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: "Ratchet/1.0".to_string(),
            verify_ssl: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            task_content_cache_size: 100,
            enabled: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file_path: None,
        }
    }
}

impl Default for FetchVariables {
    fn default() -> Self {
        Self {
            url_var: "__fetch_url".to_string(),
            params_var: "__fetch_params".to_string(),
            body_var: "__fetch_body".to_string(),
            result_var: "__http_result".to_string(),
            temp_result_var: "__temp_result".to_string(),
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
    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // HTTP configuration overrides
        if let Ok(timeout) = std::env::var("RATCHET_HTTP_TIMEOUT") {
            let seconds: u64 = timeout.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_HTTP_TIMEOUT: {}", e)))?;
            self.http.timeout = Duration::from_secs(seconds);
        }
        
        if let Ok(user_agent) = std::env::var("RATCHET_HTTP_USER_AGENT") {
            self.http.user_agent = user_agent;
        }
        
        // Cache configuration overrides
        if let Ok(cache_size) = std::env::var("RATCHET_CACHE_SIZE") {
            let size: usize = cache_size.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_CACHE_SIZE: {}", e)))?;
            self.cache.task_content_cache_size = size;
        }
        
        // Logging configuration overrides
        if let Ok(log_level) = std::env::var("RATCHET_LOG_LEVEL") {
            self.logging.level = log_level;
        }
        
        // Execution configuration overrides
        if let Ok(max_exec) = std::env::var("RATCHET_MAX_EXECUTION_SECONDS") {
            let seconds: u64 = max_exec.parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid RATCHET_MAX_EXECUTION_SECONDS: {}", e)))?;
            self.execution.max_execution_duration = Duration::from_secs(seconds);
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate log level
        match self.logging.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(ConfigError::ValidationError(
                format!("Invalid log level: {}. Must be one of: trace, debug, info, warn, error", self.logging.level)
            )),
        }
        
        // Validate cache size
        if self.cache.task_content_cache_size == 0 {
            return Err(ConfigError::ValidationError(
                "Cache size must be greater than 0".to_string()
            ));
        }
        
        // Validate timeout values
        if self.http.timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "HTTP timeout must be greater than 0 seconds".to_string()
            ));
        }
        
        if self.execution.max_execution_duration.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "Max execution duration must be greater than 0 seconds".to_string()
            ));
        }
        
        // Validate fetch variable names are not empty
        let fetch_vars = &self.execution.fetch_variables;
        if fetch_vars.url_var.is_empty() || fetch_vars.params_var.is_empty() || 
           fetch_vars.body_var.is_empty() || fetch_vars.result_var.is_empty() || 
           fetch_vars.temp_result_var.is_empty() {
            return Err(ConfigError::ValidationError(
                "Fetch variable names cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Generate a sample configuration file
    pub fn generate_sample() -> String {
        let config = RatchetConfig::default();
        serde_yaml::to_string(&config).unwrap_or_else(|_| "# Failed to generate sample config".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = RatchetConfig::default();
        
        assert_eq!(config.http.timeout, Duration::from_secs(30));
        assert_eq!(config.cache.task_content_cache_size, 100);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.execution.fetch_variables.url_var, "__fetch_url");
        assert!(config.execution.validate_schemas);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = RatchetConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid log level should fail
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Reset and test zero cache size
        config = RatchetConfig::default();
        config.cache.task_content_cache_size = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_env_overrides() {
        std::env::set_var("RATCHET_HTTP_TIMEOUT", "60");
        std::env::set_var("RATCHET_CACHE_SIZE", "200");
        std::env::set_var("RATCHET_LOG_LEVEL", "debug");
        
        let config = RatchetConfig::from_env().unwrap();
        
        assert_eq!(config.http.timeout, Duration::from_secs(60));
        assert_eq!(config.cache.task_content_cache_size, 200);
        assert_eq!(config.logging.level, "debug");
        
        // Clean up
        std::env::remove_var("RATCHET_HTTP_TIMEOUT");
        std::env::remove_var("RATCHET_CACHE_SIZE");
        std::env::remove_var("RATCHET_LOG_LEVEL");
    }
    
    #[test]
    fn test_fetch_variables() {
        let config = RatchetConfig::default();
        let vars = &config.execution.fetch_variables;
        
        assert_eq!(vars.url_var, "__fetch_url");
        assert_eq!(vars.params_var, "__fetch_params");
        assert_eq!(vars.body_var, "__fetch_body");
        assert_eq!(vars.result_var, "__http_result");
        assert_eq!(vars.temp_result_var, "__temp_result");
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// List of registry sources
    pub sources: Vec<RegistrySourceConfig>,
}

/// Registry source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySourceConfig {
    /// Source name for identification
    pub name: String,
    
    /// Source URI (e.g., "file://./tasks" or "https://registry.example.com")
    pub uri: String,
    
    /// Additional source-specific configuration
    pub config: Option<serde_json::Value>,
}