//! HTTP configuration

use ratchet_config::domains::http::HttpConfig as ConfigHttpConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout
    pub timeout: Duration,

    /// Maximum number of redirects to follow
    pub max_redirects: u32,

    /// User agent string
    pub user_agent: String,

    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
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

impl From<ConfigHttpConfig> for HttpConfig {
    fn from(config: ConfigHttpConfig) -> Self {
        Self {
            timeout: config.timeout,
            max_redirects: config.max_redirects,
            user_agent: config.user_agent,
            verify_ssl: config.verify_ssl,
        }
    }
}
