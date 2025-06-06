//! Configuration loading and environment variable handling

use crate::domains::RatchetConfig;
use crate::error::{ConfigError, ConfigResult};
use std::path::Path;

/// Configuration loader with environment variable support
pub struct ConfigLoader {
    /// Environment variable prefix
    prefix: String,
}

impl ConfigLoader {
    /// Create a new config loader with default prefix
    pub fn new() -> Self {
        Self {
            prefix: "RATCHET".to_string(),
        }
    }

    /// Create a new config loader with custom prefix
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Load configuration from a YAML file with environment overrides
    pub fn from_file(&self, path: impl AsRef<Path>) -> ConfigResult<RatchetConfig> {
        let content = std::fs::read_to_string(path)?;
        let mut config: RatchetConfig = serde_yaml::from_str(&content)?;

        // Apply environment variable overrides
        self.apply_env_overrides(&mut config)?;

        // Validate all domains
        config.validate_all()?;

        Ok(config)
    }

    /// Load configuration from environment variables only
    pub fn from_env(&self) -> ConfigResult<RatchetConfig> {
        let mut config = RatchetConfig::default();
        self.apply_env_overrides(&mut config)?;
        config.validate_all()?;
        Ok(config)
    }

    /// Load configuration with fallback chain
    pub fn load(&self, config_path: Option<impl AsRef<Path>>) -> ConfigResult<RatchetConfig> {
        match config_path {
            Some(path) => self.from_file(path),
            None => self.from_env(),
        }
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(&self, config: &mut RatchetConfig) -> ConfigResult<()> {
        // Apply domain-specific overrides
        self.apply_execution_overrides(&mut config.execution)?;
        self.apply_http_overrides(&mut config.http)?;
        self.apply_cache_overrides(&mut config.cache)?;
        self.apply_logging_overrides(&mut config.logging)?;
        self.apply_output_overrides(&mut config.output)?;

        if let Some(ref mut server) = config.server {
            self.apply_server_overrides(server)?;
        }

        Ok(())
    }

    /// Apply execution config overrides
    fn apply_execution_overrides(
        &self,
        config: &mut crate::domains::execution::ExecutionConfig,
    ) -> ConfigResult<()> {
        if let Ok(max_exec) = self.get_env_var("MAX_EXECUTION_SECONDS") {
            let seconds: u64 = max_exec.parse().map_err(|e| {
                ConfigError::EnvError(format!("Invalid MAX_EXECUTION_SECONDS: {}", e))
            })?;
            config.max_execution_duration = std::time::Duration::from_secs(seconds);
        }

        if let Ok(validate) = self.get_env_var("VALIDATE_SCHEMAS") {
            config.validate_schemas = validate
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid VALIDATE_SCHEMAS: {}", e)))?;
        }

        Ok(())
    }

    /// Apply HTTP config overrides
    fn apply_http_overrides(
        &self,
        config: &mut crate::domains::http::HttpConfig,
    ) -> ConfigResult<()> {
        if let Ok(timeout) = self.get_env_var("HTTP_TIMEOUT") {
            let seconds: u64 = timeout
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid HTTP_TIMEOUT: {}", e)))?;
            config.timeout = std::time::Duration::from_secs(seconds);
        }

        if let Ok(user_agent) = self.get_env_var("HTTP_USER_AGENT") {
            config.user_agent = user_agent;
        }

        if let Ok(verify_ssl) = self.get_env_var("HTTP_VERIFY_SSL") {
            config.verify_ssl = verify_ssl
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid HTTP_VERIFY_SSL: {}", e)))?;
        }

        Ok(())
    }

    /// Apply cache config overrides
    fn apply_cache_overrides(
        &self,
        config: &mut crate::domains::cache::CacheConfig,
    ) -> ConfigResult<()> {
        if let Ok(cache_size) = self.get_env_var("CACHE_SIZE") {
            let size: usize = cache_size
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid CACHE_SIZE: {}", e)))?;
            config.task_cache.task_content_cache_size = size;
        }

        if let Ok(enabled) = self.get_env_var("CACHE_ENABLED") {
            config.enabled = enabled
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid CACHE_ENABLED: {}", e)))?;
        }

        Ok(())
    }

    /// Apply logging config overrides
    fn apply_logging_overrides(
        &self,
        config: &mut crate::domains::logging::LoggingConfig,
    ) -> ConfigResult<()> {
        if let Ok(log_level) = self.get_env_var("LOG_LEVEL") {
            use std::str::FromStr;
            config.level = crate::domains::logging::LogLevel::from_str(&log_level)
                .map_err(|_| ConfigError::EnvError(format!("Invalid LOG_LEVEL: {}", log_level)))?;
        }

        if let Ok(format) = self.get_env_var("LOG_FORMAT") {
            use std::str::FromStr;
            config.format = crate::domains::logging::LogFormat::from_str(&format)
                .map_err(|_| ConfigError::EnvError(format!("Invalid LOG_FORMAT: {}", format)))?;
        }

        Ok(())
    }

    /// Apply output config overrides
    fn apply_output_overrides(
        &self,
        config: &mut crate::domains::output::OutputConfig,
    ) -> ConfigResult<()> {
        if let Ok(max_deliveries) = self.get_env_var("OUTPUT_MAX_CONCURRENT_DELIVERIES") {
            config.max_concurrent_deliveries = max_deliveries.parse().map_err(|e| {
                ConfigError::EnvError(format!("Invalid OUTPUT_MAX_CONCURRENT_DELIVERIES: {}", e))
            })?;
        }

        if let Ok(timeout) = self.get_env_var("OUTPUT_DEFAULT_TIMEOUT") {
            let seconds: u64 = timeout.parse().map_err(|e| {
                ConfigError::EnvError(format!("Invalid OUTPUT_DEFAULT_TIMEOUT: {}", e))
            })?;
            config.default_timeout = std::time::Duration::from_secs(seconds);
        }

        Ok(())
    }

    /// Apply server config overrides
    fn apply_server_overrides(
        &self,
        config: &mut crate::domains::server::ServerConfig,
    ) -> ConfigResult<()> {
        if let Ok(bind) = self.get_env_var("SERVER_BIND_ADDRESS") {
            config.bind_address = bind;
        }

        if let Ok(port) = self.get_env_var("SERVER_PORT") {
            config.port = port
                .parse()
                .map_err(|e| ConfigError::EnvError(format!("Invalid SERVER_PORT: {}", e)))?;
        }

        Ok(())
    }

    /// Get environment variable with prefix
    fn get_env_var(&self, name: &str) -> Result<String, std::env::VarError> {
        std::env::var(format!("{}_{}", self.prefix, name))
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
