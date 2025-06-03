//! Configuration error types

use thiserror::Error;

/// Configuration result type
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// IO error reading configuration file
    #[error("Failed to read config file: {0}")]
    FileReadError(#[from] std::io::Error),
    
    /// YAML parsing error
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_yaml::Error),
    
    /// JSON parsing error
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    
    /// Validation error
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    
    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(String),
    
    /// URL parsing error
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),
    
    /// Domain-specific configuration error
    #[error("Domain configuration error in {domain}: {message}")]
    DomainError {
        domain: String,
        message: String,
    },
}