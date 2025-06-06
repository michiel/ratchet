//! HTTP client configuration

use crate::error::ConfigResult;
use crate::validation::{validate_positive, validate_required_string, Validatable};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Request timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_timeout"
    )]
    pub timeout: Duration,

    /// Maximum number of redirects to follow
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,

    /// User agent string
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    /// Whether to verify SSL certificates
    #[serde(default = "crate::domains::utils::default_true")]
    pub verify_ssl: bool,

    /// Connection pool configuration
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,

    /// Proxy configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxyConfig>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionPoolConfig {
    /// Maximum idle connections per host
    #[serde(default = "default_max_idle_per_host")]
    pub max_idle_per_host: usize,

    /// Idle connection timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_idle_timeout"
    )]
    pub idle_timeout: Duration,

    /// Connection timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_connection_timeout"
    )]
    pub connection_timeout: Duration,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// HTTP proxy URL
    pub http_proxy: Option<String>,

    /// HTTPS proxy URL
    pub https_proxy: Option<String>,

    /// No proxy hosts (comma-separated)
    pub no_proxy: Option<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            max_redirects: default_max_redirects(),
            user_agent: default_user_agent(),
            verify_ssl: true,
            connection_pool: ConnectionPoolConfig::default(),
            proxy: None,
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: default_max_idle_per_host(),
            idle_timeout: default_idle_timeout(),
            connection_timeout: default_connection_timeout(),
        }
    }
}

impl Validatable for HttpConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate timeout
        validate_positive(self.timeout.as_secs(), "timeout", self.domain_name())?;

        // Validate user agent
        validate_required_string(&self.user_agent, "user_agent", self.domain_name())?;

        // Validate connection pool
        self.connection_pool.validate()?;

        // Validate proxy if present
        if let Some(ref proxy) = self.proxy {
            proxy.validate()?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "http"
    }
}

impl Validatable for ConnectionPoolConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(
            self.max_idle_per_host,
            "max_idle_per_host",
            self.domain_name(),
        )?;

        validate_positive(
            self.idle_timeout.as_secs(),
            "idle_timeout",
            self.domain_name(),
        )?;

        validate_positive(
            self.connection_timeout.as_secs(),
            "connection_timeout",
            self.domain_name(),
        )?;

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "http.connection_pool"
    }
}

impl Validatable for ProxyConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate proxy URLs if present
        if let Some(ref url) = self.http_proxy {
            crate::validation::validate_url(url, "http_proxy", self.domain_name())?;
        }

        if let Some(ref url) = self.https_proxy {
            crate::validation::validate_url(url, "https_proxy", self.domain_name())?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "http.proxy"
    }
}

// Default value functions
fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_redirects() -> u32 {
    10
}

fn default_user_agent() -> String {
    "Ratchet/1.0".to_string()
}

fn default_max_idle_per_host() -> usize {
    10
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(90)
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_defaults() {
        let config = HttpConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_redirects, 10);
        assert_eq!(config.user_agent, "Ratchet/1.0");
        assert!(config.verify_ssl);
    }

    #[test]
    fn test_http_config_validation() {
        let mut config = HttpConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid timeout
        config.timeout = Duration::from_secs(0);
        assert!(config.validate().is_err());

        // Test empty user agent
        config = HttpConfig::default();
        config.user_agent = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_proxy_config_validation() {
        let mut proxy = ProxyConfig {
            http_proxy: Some("http://proxy.example.com:8080".to_string()),
            https_proxy: None,
            no_proxy: None,
        };
        assert!(proxy.validate().is_ok());

        // Test invalid URL
        proxy.http_proxy = Some("not-a-url".to_string());
        assert!(proxy.validate().is_err());
    }
}
