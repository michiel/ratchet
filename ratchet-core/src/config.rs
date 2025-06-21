//! Configuration types - simplified for now

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExecutionConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HttpConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StorageConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggingConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OutputConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ServerConfig {}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PluginConfig {}

/// Development mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentConfig {
    /// Enable development mode features
    pub enabled: bool,
    /// Allow webhooks to use localhost URLs (security risk in production)
    pub allow_localhost_webhooks: bool,
    /// Allow insecure HTTP webhooks (security risk in production)
    pub allow_http_webhooks: bool,
    /// Skip SSL certificate verification for webhooks (security risk in production)
    pub skip_webhook_ssl_verification: bool,
    /// Enable verbose error messages with stack traces
    pub verbose_errors: bool,
}

impl Default for DevelopmentConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Development mode disabled by default for security
            allow_localhost_webhooks: false,
            allow_http_webhooks: false,
            skip_webhook_ssl_verification: false,
            verbose_errors: false,
        }
    }
}
