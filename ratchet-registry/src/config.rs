use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

use crate::error::{RegistryError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub sources: Vec<TaskSource>,
    pub sync_interval: Duration,
    pub enable_auto_sync: bool,
    pub enable_validation: bool,
    pub cache_config: CacheConfig,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            sync_interval: Duration::from_secs(300), // 5 minutes
            enable_auto_sync: true,
            enable_validation: true,
            cache_config: CacheConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskSource {
    #[serde(rename = "filesystem")]
    Filesystem { 
        path: String,
        recursive: bool,
        watch: bool,
    },
    #[serde(rename = "http")]
    Http { 
        url: String,
        auth: Option<HttpAuth>,
        polling_interval: Duration,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAuth {
    #[serde(flatten)]
    pub auth_type: HttpAuthType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HttpAuthType {
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    #[serde(rename = "api_key")]
    ApiKey { header_name: String, api_key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub ignore_patterns: Vec<String>,
    pub max_concurrent_reloads: usize,
    pub retry_on_error: bool,
    pub retry_delay_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 500,
            ignore_patterns: vec![
                "*.tmp".to_string(),
                "*.swp".to_string(),
                ".git/**".to_string(),
                ".DS_Store".to_string(),
            ],
            max_concurrent_reloads: 5,
            retry_on_error: true,
            retry_delay_ms: 1000,
        }
    }
}

impl TaskSource {
    pub fn from_uri(uri: &str) -> Result<Self> {
        if uri.starts_with("file://") {
            let path_str = uri.strip_prefix("file://").unwrap();
            Ok(TaskSource::Filesystem {
                path: path_str.to_string(),
                recursive: true,
                watch: false,
            })
        } else if uri.starts_with("http://") || uri.starts_with("https://") {
            Ok(TaskSource::Http {
                url: uri.to_string(),
                auth: None,
                polling_interval: Duration::from_secs(300),
            })
        } else {
            Err(RegistryError::Configuration(format!(
                "Unsupported registry source URI: {}",
                uri
            )))
        }
    }

    pub fn filesystem_path(&self) -> Option<PathBuf> {
        match self {
            TaskSource::Filesystem { path, .. } => Some(PathBuf::from(path)),
            _ => None,
        }
    }

    pub fn url(&self) -> Option<Result<Url>> {
        match self {
            TaskSource::Http { url, .. } => Some(
                url.parse()
                    .map_err(|e| RegistryError::Configuration(format!("Invalid URL: {}", e)))
            ),
            _ => None,
        }
    }
}