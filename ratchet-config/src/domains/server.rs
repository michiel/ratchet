//! Server configuration for REST and GraphQL APIs

use crate::error::ConfigResult;
use crate::validation::{validate_positive, validate_required_string, validate_url, Validatable};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub database: super::database::DatabaseConfig,

    /// Authentication configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,

    /// CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// TLS configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,

    /// Token expiration time
    #[serde(with = "crate::domains::utils::serde_duration")]
    pub token_expiration: Duration,

    /// Token issuer
    #[serde(default = "default_token_issuer")]
    pub issuer: String,

    /// Token audience
    #[serde(default = "default_token_audience")]
    pub audience: String,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    /// Allowed origins
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    #[serde(default = "default_cors_headers")]
    pub allowed_headers: Vec<String>,

    /// Whether to allow credentials
    #[serde(default = "crate::domains::utils::default_false")]
    pub allow_credentials: bool,

    /// Max age for preflight requests
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_cors_max_age")]
    pub max_age: Duration,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,

    /// Requests per minute per IP
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    /// Burst size
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Time window for rate limiting
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_time_window")]
    pub time_window: Duration,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_file: String,

    /// Path to private key file
    pub key_file: String,

    /// Minimum TLS version
    #[serde(default = "default_min_tls_version")]
    pub min_version: String,

    /// Certificate chain file (optional)
    pub chain_file: Option<String>,
}

impl TlsConfig {
    /// Create a new TLS configuration builder
    pub fn builder() -> TlsConfigBuilder {
        TlsConfigBuilder::new()
    }
}

/// Builder for TLS configuration
#[derive(Debug)]
pub struct TlsConfigBuilder {
    cert_file: Option<String>,
    key_file: Option<String>,
    min_version: String,
    chain_file: Option<String>,
}

impl TlsConfigBuilder {
    /// Create a new TLS configuration builder
    pub fn new() -> Self {
        Self {
            cert_file: None,
            key_file: None,
            min_version: default_min_tls_version(),
            chain_file: None,
        }
    }

    /// Set the certificate file path
    pub fn cert_file(mut self, path: impl Into<String>) -> Self {
        self.cert_file = Some(path.into());
        self
    }

    /// Set the private key file path
    pub fn key_file(mut self, path: impl Into<String>) -> Self {
        self.key_file = Some(path.into());
        self
    }

    /// Set the minimum TLS version
    pub fn min_version(mut self, version: impl Into<String>) -> Self {
        self.min_version = version.into();
        self
    }

    /// Set the certificate chain file path
    pub fn chain_file(mut self, path: impl Into<String>) -> Self {
        self.chain_file = Some(path.into());
        self
    }

    /// Build the TLS configuration
    pub fn build(self) -> Result<TlsConfig, String> {
        let cert_file = self.cert_file.ok_or("Certificate file path is required")?;
        let key_file = self.key_file.ok_or("Private key file path is required")?;

        Ok(TlsConfig {
            cert_file,
            key_file,
            min_version: self.min_version,
            chain_file: self.chain_file,
        })
    }
}

impl Default for TlsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            port: default_port(),
            database: super::database::DatabaseConfig::default(),
            auth: None,
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            tls: None,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_cors_origins(),
            allowed_methods: default_cors_methods(),
            allowed_headers: default_cors_headers(),
            allow_credentials: false,
            max_age: default_cors_max_age(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: default_requests_per_minute(),
            burst_size: default_burst_size(),
            time_window: default_time_window(),
        }
    }
}

impl Validatable for ServerConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.bind_address, "bind_address", self.domain_name())?;
        validate_positive(self.port, "port", self.domain_name())?;

        self.database.validate()?;
        self.cors.validate()?;
        self.rate_limit.validate()?;

        if let Some(ref auth) = self.auth {
            auth.validate()?;
        }

        if let Some(ref tls) = self.tls {
            tls.validate()?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "server"
    }
}

impl Validatable for AuthConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.jwt_secret, "jwt_secret", self.domain_name())?;
        validate_required_string(&self.issuer, "issuer", self.domain_name())?;
        validate_required_string(&self.audience, "audience", self.domain_name())?;

        validate_positive(self.token_expiration.as_secs(), "token_expiration", self.domain_name())?;

        // Validate JWT secret strength
        if self.jwt_secret.len() < 32 {
            return Err(self.validation_error("jwt_secret must be at least 32 characters long"));
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "server.auth"
    }
}

impl Validatable for CorsConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate origins
        for origin in &self.allowed_origins {
            if origin != "*" && !origin.is_empty() {
                validate_url(origin, "allowed_origins", self.domain_name())?;
            }
        }

        // Validate methods
        let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
        for method in &self.allowed_methods {
            if !valid_methods.contains(&method.as_str()) {
                return Err(self.validation_error(format!("Invalid HTTP method in allowed_methods: {}", method)));
            }
        }

        validate_positive(self.max_age.as_secs(), "max_age", self.domain_name())?;

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "server.cors"
    }
}

impl Validatable for RateLimitConfig {
    fn validate(&self) -> ConfigResult<()> {
        if self.enabled {
            validate_positive(self.requests_per_minute, "requests_per_minute", self.domain_name())?;
            validate_positive(self.burst_size, "burst_size", self.domain_name())?;
            validate_positive(self.time_window.as_secs(), "time_window", self.domain_name())?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "server.rate_limit"
    }
}

impl Validatable for TlsConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.cert_file, "cert_file", self.domain_name())?;
        validate_required_string(&self.key_file, "key_file", self.domain_name())?;
        validate_required_string(&self.min_version, "min_version", self.domain_name())?;

        // Validate TLS version
        let valid_versions = ["1.0", "1.1", "1.2", "1.3"];
        if !valid_versions.contains(&self.min_version.as_str()) {
            return Err(self.validation_error(format!(
                "Invalid TLS version: {}. Valid versions: {}",
                self.min_version,
                valid_versions.join(", ")
            )));
        }

        // Check if files exist
        if !std::path::Path::new(&self.cert_file).exists() {
            return Err(self.validation_error(format!("Certificate file not found: {}", self.cert_file)));
        }

        if !std::path::Path::new(&self.key_file).exists() {
            return Err(self.validation_error(format!("Private key file not found: {}", self.key_file)));
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "server.tls"
    }
}

// Default value functions
fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_token_issuer() -> String {
    "ratchet".to_string()
}

fn default_token_audience() -> String {
    "ratchet-api".to_string()
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
    ]
}

fn default_cors_headers() -> Vec<String> {
    vec![
        "Content-Type".to_string(),
        "Authorization".to_string(),
        "X-Requested-With".to_string(),
    ]
}

fn default_cors_max_age() -> Duration {
    Duration::from_secs(3600)
}

fn default_requests_per_minute() -> u32 {
    60
}

fn default_burst_size() -> u32 {
    10
}

fn default_time_window() -> Duration {
    Duration::from_secs(60)
}

fn default_min_tls_version() -> String {
    "1.2".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.rate_limit.enabled);
    }

    #[test]
    fn test_auth_config_validation() {
        let mut auth = AuthConfig {
            jwt_secret: "a".repeat(32),
            token_expiration: Duration::from_secs(3600),
            issuer: "test".to_string(),
            audience: "test".to_string(),
        };
        assert!(auth.validate().is_ok());

        // Test short secret
        auth.jwt_secret = "short".to_string();
        assert!(auth.validate().is_err());
    }

    #[test]
    fn test_cors_config_validation() {
        let mut cors = CorsConfig::default();
        assert!(cors.validate().is_ok());

        // Test invalid method
        cors.allowed_methods.push("INVALID".to_string());
        assert!(cors.validate().is_err());
    }

    #[test]
    fn test_rate_limit_config_validation() {
        let mut rate_limit = RateLimitConfig::default();
        assert!(rate_limit.validate().is_ok());

        // Test zero requests per minute
        rate_limit.requests_per_minute = 0;
        assert!(rate_limit.validate().is_err());

        // Test disabled rate limiting
        rate_limit = RateLimitConfig::default();
        rate_limit.enabled = false;
        rate_limit.requests_per_minute = 0; // Should be ok when disabled
        assert!(rate_limit.validate().is_ok());
    }
}
