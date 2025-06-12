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
    #[serde(rename = "git")]
    Git {
        url: String,
        auth: Option<GitAuth>,
        config: GitConfig,
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
pub struct GitAuth {
    #[serde(flatten)]
    pub auth_type: GitAuthType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GitAuthType {
    #[serde(rename = "git_token")]
    Token { token: String },
    #[serde(rename = "ssh_key")]
    SshKey { 
        private_key_path: String,
        passphrase: Option<String>,
    },
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    #[serde(rename = "github_app")]
    GitHubApp {
        app_id: String,
        private_key_path: String,
        installation_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Git branch name (e.g., "main", "master", "develop")
    /// Also accepts tags or commit hashes
    #[serde(alias = "git_ref", default = "default_git_branch")]
    pub branch: String,
    
    /// Subdirectory within repository
    pub subdirectory: Option<String>,
    
    /// Use shallow clone for performance
    #[serde(default = "default_shallow")]
    pub shallow: bool,
    
    /// Clone depth for shallow clones
    #[serde(default = "default_depth")]
    pub depth: Option<u32>,
    
    /// Sync strategy
    #[serde(default)]
    pub sync_strategy: GitSyncStrategy,
    
    /// Cleanup on error
    #[serde(default = "default_cleanup_on_error")]
    pub cleanup_on_error: bool,
    
    /// Verify Git commit signatures
    #[serde(default)]
    pub verify_signatures: bool,
    
    /// Allowed Git refs (for security)
    pub allowed_refs: Option<Vec<String>>,
    
    /// Git operation timeout
    #[serde(default = "default_git_timeout")]
    pub timeout: Duration,
    
    /// Maximum repository size
    pub max_repo_size: Option<String>,
    
    /// Local cache path
    pub local_cache_path: Option<String>,
    
    /// Cache TTL
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: Duration,
    
    /// Keep Git history
    #[serde(default)]
    pub keep_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitSyncStrategy {
    Clone,
    Fetch,
    Pull,
}

impl Default for GitSyncStrategy {
    fn default() -> Self {
        Self::Fetch
    }
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            branch: default_git_branch(),
            subdirectory: None,
            shallow: default_shallow(),
            depth: default_depth(),
            sync_strategy: GitSyncStrategy::default(),
            cleanup_on_error: default_cleanup_on_error(),
            verify_signatures: false,
            allowed_refs: None,
            timeout: default_git_timeout(),
            max_repo_size: None,
            local_cache_path: None,
            cache_ttl: default_cache_ttl(),
            keep_history: false,
        }
    }
}

// Default value functions for Git config
fn default_git_branch() -> String {
    "main".to_string()
}

fn default_shallow() -> bool {
    true
}

fn default_depth() -> Option<u32> {
    Some(1)
}

fn default_cleanup_on_error() -> bool {
    true
}

fn default_git_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
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
            // Check if this is a Git repository URL
            if uri.ends_with(".git") || uri.contains("github.com") || uri.contains("gitlab.com") || uri.contains("bitbucket.org") {
                Ok(TaskSource::Git {
                    url: uri.to_string(),
                    auth: None,
                    config: GitConfig::default(),
                })
            } else {
                Ok(TaskSource::Http {
                    url: uri.to_string(),
                    auth: None,
                    polling_interval: Duration::from_secs(300),
                })
            }
        } else if uri.starts_with("git://") || uri.starts_with("ssh://") {
            Ok(TaskSource::Git {
                url: uri.to_string(),
                auth: None,
                config: GitConfig::default(),
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
            TaskSource::Git { url, .. } => Some(
                url.parse()
                    .map_err(|e| RegistryError::Configuration(format!("Invalid Git URL: {}", e)))
            ),
            _ => None,
        }
    }

    pub fn git_url(&self) -> Option<&str> {
        match self {
            TaskSource::Git { url, .. } => Some(url),
            _ => None,
        }
    }

    pub fn git_config(&self) -> Option<&GitConfig> {
        match self {
            TaskSource::Git { config, .. } => Some(config),
            _ => None,
        }
    }

    pub fn git_auth(&self) -> Option<&GitAuth> {
        match self {
            TaskSource::Git { auth, .. } => auth.as_ref(),
            _ => None,
        }
    }
}