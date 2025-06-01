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
#[serde(default)]
pub struct RatchetConfig {
    /// Task execution configuration
    #[serde(default)]
    pub execution: ExecutionConfig,
    
    /// HTTP client configuration
    #[serde(default)]
    pub http: HttpConfig,
    
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
    
    /// Registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistryConfig>,
}

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionConfig {
    /// JavaScript variable names used for fetch operations
    #[serde(default)]
    pub fetch_variables: FetchVariables,
    
    /// Maximum execution time for JavaScript tasks
    #[serde(with = "serde_duration_seconds", default = "default_max_execution_duration")]
    pub max_execution_duration: Duration,
    
    /// Whether to validate schemas during execution
    #[serde(default = "default_true")]
    pub validate_schemas: bool,
}

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Request timeout
    #[serde(with = "serde_duration_seconds", default = "default_http_timeout")]
    pub timeout: Duration,
    
    /// Maximum number of redirects to follow
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,
    
    /// User agent string
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    
    /// Whether to verify SSL certificates
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// LRU cache size for task content
    #[serde(default = "default_cache_size")]
    pub task_content_cache_size: usize,
    
    /// Whether caching is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Output destinations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Maximum number of concurrent deliveries
    #[serde(default = "default_max_concurrent_deliveries")]
    pub max_concurrent_deliveries: usize,
    
    /// Default timeout for deliveries
    #[serde(with = "serde_duration_seconds", default = "default_delivery_timeout")]
    pub default_timeout: Duration,
    
    /// Whether to validate destination configurations on startup
    #[serde(default = "default_true")]
    pub validate_on_startup: bool,
    
    /// Global output destination templates
    #[serde(default)]
    pub global_destinations: Vec<OutputDestinationTemplate>,
    
    /// Default retry policy for failed deliveries
    #[serde(default)]
    pub default_retry_policy: RetryPolicyConfig,
}

/// Output destination template for reuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDestinationTemplate {
    /// Template name for reference
    pub name: String,
    
    /// Template description
    pub description: Option<String>,
    
    /// Destination configuration
    pub destination: OutputDestinationConfigTemplate,
}

/// Output destination configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutputDestinationConfigTemplate {
    Filesystem {
        /// Path template with variables
        path: String,
        /// Output format
        #[serde(default = "default_output_format")]
        format: String,
        /// File permissions (octal as string)
        #[serde(default = "default_file_permissions")]
        permissions: String,
        /// Whether to create directories
        #[serde(default = "default_true")]
        create_dirs: bool,
        /// Whether to overwrite existing files
        #[serde(default = "default_true")]
        overwrite: bool,
        /// Whether to backup existing files
        #[serde(default = "default_false")]
        backup_existing: bool,
    },
    Webhook {
        /// Webhook URL template
        url: String,
        /// HTTP method
        #[serde(default = "default_http_method")]
        method: String,
        /// HTTP headers
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        /// Request timeout in seconds
        #[serde(default = "default_webhook_timeout")]
        timeout_seconds: u64,
        /// Content type header
        content_type: Option<String>,
        /// Authentication configuration
        auth: Option<WebhookAuthConfig>,
    },
    Database {
        /// Database connection string
        connection_string: String,
        /// Target table name
        table_name: String,
        /// Column mappings
        column_mappings: std::collections::HashMap<String, String>,
    },
    S3 {
        /// S3 bucket name
        bucket: String,
        /// Object key template
        key_template: String,
        /// AWS region
        region: String,
        /// AWS access key ID (optional, can use environment)
        access_key_id: Option<String>,
        /// AWS secret access key (optional, can use environment)
        secret_access_key: Option<String>,
    },
}

/// Webhook authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WebhookAuthConfig {
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        header: String,
        value: String,
    },
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryPolicyConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: i32,
    
    /// Initial delay between retries in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    
    /// Backoff multiplier for exponential backoff
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}


/// Server configuration (for future server mode)
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
    
    /// Authentication configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://ratchet.db")
    #[serde(default = "default_database_url")]
    pub url: String,
    
    /// Maximum number of database connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Connection timeout
    #[serde(with = "serde_duration_seconds", default = "default_connection_timeout")]
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
#[serde(default)]
pub struct FetchVariables {
    /// Variable name for fetch URL
    #[serde(default = "default_url_var")]
    pub url_var: String,
    
    /// Variable name for fetch parameters
    #[serde(default = "default_params_var")]
    pub params_var: String,
    
    /// Variable name for fetch body
    #[serde(default = "default_body_var")]
    pub body_var: String,
    
    /// Variable name for HTTP result
    #[serde(default = "default_result_var")]
    pub result_var: String,
    
    /// Variable name for temporary result
    #[serde(default = "default_temp_result_var")]
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
            logging: crate::logging::LoggingConfig::default(),
            output: OutputConfig::default(),
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

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_concurrent_deliveries: 10,
            default_timeout: Duration::from_secs(30),
            validate_on_startup: true,
            global_destinations: Vec::new(),
            default_retry_policy: RetryPolicyConfig::default(),
        }
    }
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            database: DatabaseConfig::default(),
            auth: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            connection_timeout: default_connection_timeout(),
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
        if let Ok(log_level_str) = std::env::var("RATCHET_LOG_LEVEL") {
            use std::str::FromStr;
            if let Ok(log_level) = crate::logging::LogLevel::from_str(&log_level_str) {
                self.logging.level = log_level;
            } else {
                return Err(ConfigError::EnvError(format!("Invalid RATCHET_LOG_LEVEL: {}", log_level_str)));
            }
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
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Log level is now an enum, so it's always valid
        
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
        
        // Validate output configuration
        self.validate_output_config()?;
        
        Ok(())
    }
    
    /// Validate output configuration
    fn validate_output_config(&self) -> Result<(), ConfigError> {
        let output = &self.output;
        
        // Validate max concurrent deliveries
        if output.max_concurrent_deliveries == 0 {
            return Err(ConfigError::ValidationError(
                "Max concurrent deliveries must be greater than 0".to_string()
            ));
        }
        
        // Validate default timeout
        if output.default_timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError(
                "Default delivery timeout must be greater than 0 seconds".to_string()
            ));
        }
        
        // Validate retry policy
        let retry = &output.default_retry_policy;
        if retry.max_attempts <= 0 {
            return Err(ConfigError::ValidationError(
                "Max retry attempts must be greater than 0".to_string()
            ));
        }
        
        if retry.initial_delay_ms == 0 {
            return Err(ConfigError::ValidationError(
                "Initial retry delay must be greater than 0 milliseconds".to_string()
            ));
        }
        
        if retry.max_delay_ms < retry.initial_delay_ms {
            return Err(ConfigError::ValidationError(
                "Max retry delay must be greater than or equal to initial delay".to_string()
            ));
        }
        
        if retry.backoff_multiplier <= 1.0 {
            return Err(ConfigError::ValidationError(
                "Backoff multiplier must be greater than 1.0".to_string()
            ));
        }
        
        // Validate global destination templates
        for (index, template) in output.global_destinations.iter().enumerate() {
            if template.name.is_empty() {
                return Err(ConfigError::ValidationError(
                    format!("Global destination template {} has empty name", index)
                ));
            }
            
            self.validate_destination_template(&template.destination, &template.name)?;
        }
        
        Ok(())
    }
    
    /// Validate a destination template configuration
    fn validate_destination_template(&self, template: &OutputDestinationConfigTemplate, name: &str) -> Result<(), ConfigError> {
        match template {
            OutputDestinationConfigTemplate::Filesystem { path, format, .. } => {
                if path.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Filesystem destination '{}' has empty path", name)
                    ));
                }
                
                let valid_formats = ["json", "json_compact", "yaml", "csv", "raw", "template"];
                if !valid_formats.contains(&format.as_str()) {
                    return Err(ConfigError::ValidationError(
                        format!("Filesystem destination '{}' has invalid format '{}'. Valid formats: {}", 
                            name, format, valid_formats.join(", "))
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::Webhook { url, method, .. } => {
                if url.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has empty URL", name)
                    ));
                }
                
                // Basic URL validation
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has invalid URL format", name)
                    ));
                }
                
                let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
                if !valid_methods.contains(&method.to_uppercase().as_str()) {
                    return Err(ConfigError::ValidationError(
                        format!("Webhook destination '{}' has invalid HTTP method '{}'. Valid methods: {}", 
                            name, method, valid_methods.join(", "))
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::Database { connection_string, table_name, .. } => {
                if connection_string.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Database destination '{}' has empty connection string", name)
                    ));
                }
                
                if table_name.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("Database destination '{}' has empty table name", name)
                    ));
                }
            }
            
            OutputDestinationConfigTemplate::S3 { bucket, key_template, region, .. } => {
                if bucket.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty bucket name", name)
                    ));
                }
                
                if key_template.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty key template", name)
                    ));
                }
                
                if region.is_empty() {
                    return Err(ConfigError::ValidationError(
                        format!("S3 destination '{}' has empty region", name)
                    ));
                }
            }
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
        assert_eq!(config.logging.level, crate::logging::LogLevel::Info);
        assert_eq!(config.execution.fetch_variables.url_var, "__fetch_url");
        assert!(config.execution.validate_schemas);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = RatchetConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Test zero cache size
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
        assert_eq!(config.logging.level, crate::logging::LogLevel::Debug);
        
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
    
    #[test]
    fn test_partial_config_loading() {
        // Test that a partial config loads with defaults
        let yaml = r#"
logging:
  level: debug
"#;
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        
        // Check that defaults were applied
        assert_eq!(config.logging.level, crate::logging::LogLevel::Debug);  // Our override
        assert_eq!(config.http.timeout, Duration::from_secs(30));  // Default
        assert_eq!(config.cache.task_content_cache_size, 100);  // Default
        assert!(config.execution.validate_schemas);  // Default
    }
    
    #[test]
    fn test_empty_config_loading() {
        // Test that an empty config loads with all defaults
        let yaml = "{}";
        let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
        
        // Check that all defaults were applied
        assert_eq!(config.logging.level, crate::logging::LogLevel::Info);
        assert_eq!(config.http.timeout, Duration::from_secs(30));
        assert_eq!(config.cache.task_content_cache_size, 100);
        assert!(config.execution.validate_schemas);
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

// Default value functions for serde
fn default_true() -> bool {
    true
}


fn default_cache_size() -> usize {
    100
}

fn default_http_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_redirects() -> u32 {
    10
}

fn default_user_agent() -> String {
    "Ratchet/1.0".to_string()
}

fn default_max_execution_duration() -> Duration {
    Duration::from_secs(300)
}

fn default_url_var() -> String {
    "__fetch_url".to_string()
}

fn default_params_var() -> String {
    "__fetch_params".to_string()
}

fn default_body_var() -> String {
    "__fetch_body".to_string()
}

fn default_result_var() -> String {
    "__http_result".to_string()
}

fn default_temp_result_var() -> String {
    "__temp_result".to_string()
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_database_url() -> String {
    "sqlite://ratchet.db".to_string()
}

fn default_max_connections() -> u32 {
    10
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

// Output configuration defaults
fn default_max_concurrent_deliveries() -> usize {
    10
}

fn default_delivery_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_output_format() -> String {
    "json".to_string()
}

fn default_file_permissions() -> String {
    "644".to_string()
}

fn default_false() -> bool {
    false
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn default_webhook_timeout() -> u64 {
    30
}

fn default_max_retries() -> i32 {
    3
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    30000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}