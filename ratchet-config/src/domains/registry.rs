//! Registry configuration for task sources

use crate::error::ConfigResult;
use crate::validation::{validate_positive, validate_required_string, validate_url, Validatable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RegistryConfig {
    /// List of registry sources
    #[serde(default)]
    pub sources: Vec<RegistrySourceConfig>,

    /// Default polling interval for sources
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_polling_interval"
    )]
    pub default_polling_interval: Duration,

    /// Cache configuration for registry data
    #[serde(default)]
    pub cache: RegistryCacheConfig,

    /// Authentication configuration for registry sources
    #[serde(default)]
    pub auth: HashMap<String, RegistryAuthConfig>,
}

/// Registry source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySourceConfig {
    /// Source name for identification
    pub name: String,

    /// Source URI (e.g., "file://./tasks", "https://registry.example.com", "git://github.com/user/repo")
    pub uri: String,

    /// Source type
    #[serde(default = "default_source_type")]
    pub source_type: RegistrySourceType,

    /// Polling interval for this source (overrides default)
    #[serde(
        with = "crate::domains::utils::serde_duration_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub polling_interval: Option<Duration>,

    /// Whether this source is enabled
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,

    /// Authentication name (references auth config)
    pub auth_name: Option<String>,

    /// Source-specific configuration
    #[serde(default)]
    pub config: SourceSpecificConfig,
}

/// Registry source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RegistrySourceType {
    /// Local filesystem source
    #[default]
    Filesystem,
    /// HTTP/HTTPS source
    Http,
    /// Git repository source
    Git,
    /// S3 bucket source
    S3,
}

/// Source-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SourceSpecificConfig {
    /// Filesystem-specific configuration
    pub filesystem: FilesystemSourceConfig,

    /// HTTP-specific configuration
    pub http: HttpSourceConfig,

    /// Git-specific configuration
    pub git: GitSourceConfig,

    /// S3-specific configuration
    pub s3: S3SourceConfig,
}

/// Filesystem source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesystemSourceConfig {
    /// Whether to watch for changes
    #[serde(default = "crate::domains::utils::default_true")]
    pub watch_changes: bool,

    /// File patterns to include
    #[serde(default = "default_file_patterns")]
    pub include_patterns: Vec<String>,

    /// File patterns to exclude
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Whether to follow symbolic links
    #[serde(default = "crate::domains::utils::default_false")]
    pub follow_symlinks: bool,
}

/// HTTP source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpSourceConfig {
    /// Request timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_http_timeout"
    )]
    pub timeout: Duration,

    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Whether to verify SSL certificates
    #[serde(default = "crate::domains::utils::default_true")]
    pub verify_ssl: bool,

    /// User agent string
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

/// Git source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitSourceConfig {
    /// Git branch name (e.g., "main", "master", "develop")
    /// Also accepts tags or commit hashes
    #[serde(alias = "git_ref", default = "default_git_branch")]
    pub branch: String,

    /// Subdirectory within repository
    pub subdirectory: Option<String>,

    /// Whether to use shallow clones
    #[serde(default = "crate::domains::utils::default_true")]
    pub shallow: bool,

    /// Clone depth for shallow clones
    pub depth: Option<u32>,

    /// Sync strategy
    #[serde(default)]
    pub sync_strategy: GitSyncStrategy,

    /// Cleanup on error
    #[serde(default = "crate::domains::utils::default_true")]
    pub cleanup_on_error: bool,

    /// Verify Git commit signatures
    #[serde(default = "crate::domains::utils::default_false")]
    pub verify_signatures: bool,

    /// Allowed Git refs (for security)
    pub allowed_refs: Option<Vec<String>>,

    /// Git operation timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_git_timeout"
    )]
    pub timeout: Duration,

    /// Maximum repository size
    pub max_repo_size: Option<String>,

    /// Local cache path
    pub local_cache_path: Option<String>,

    /// Cache TTL
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_cache_ttl"
    )]
    pub cache_ttl: Duration,

    /// Keep Git history
    #[serde(default = "crate::domains::utils::default_false")]
    pub keep_history: bool,
}

/// Git sync strategy
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

/// S3 source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct S3SourceConfig {
    /// S3 region
    #[serde(default = "default_s3_region")]
    pub region: String,

    /// Object prefix filter
    pub prefix: Option<String>,

    /// Request timeout
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_s3_timeout"
    )]
    pub timeout: Duration,
}

/// Registry cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RegistryCacheConfig {
    /// Whether caching is enabled
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,

    /// Cache TTL for registry metadata
    #[serde(
        with = "crate::domains::utils::serde_duration",
        default = "default_cache_ttl"
    )]
    pub ttl: Duration,

    /// Maximum cache size in entries
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: usize,
}

/// Registry authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RegistryAuthConfig {
    /// HTTP Basic authentication
    Basic { username: String, password: String },
    /// Bearer token authentication
    Bearer { token: String },
    /// API key authentication
    ApiKey { header: String, value: String },
    /// Git token authentication (for HTTPS Git)
    GitToken { token: String },
    /// Git SSH key authentication
    SshKey {
        private_key_path: String,
        passphrase: Option<String>,
    },
    /// GitHub App authentication
    GitHubApp {
        app_id: String,
        private_key_path: String,
        installation_id: String,
    },
    /// Client certificate authentication
    ClientCertificate {
        cert_path: String,
        key_path: String,
        ca_cert_path: Option<String>,
    },
    /// AWS credentials for S3
    AwsCredentials {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
    },
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            default_polling_interval: default_polling_interval(),
            cache: RegistryCacheConfig::default(),
            auth: HashMap::new(),
        }
    }
}

impl Default for FilesystemSourceConfig {
    fn default() -> Self {
        Self {
            watch_changes: true,
            include_patterns: default_file_patterns(),
            exclude_patterns: Vec::new(),
            follow_symlinks: false,
        }
    }
}

impl Default for HttpSourceConfig {
    fn default() -> Self {
        Self {
            timeout: default_http_timeout(),
            headers: HashMap::new(),
            verify_ssl: true,
            user_agent: default_user_agent(),
        }
    }
}

impl Default for GitSourceConfig {
    fn default() -> Self {
        Self {
            branch: default_git_branch(),
            subdirectory: None,
            shallow: true,
            depth: Some(1),
            sync_strategy: GitSyncStrategy::default(),
            cleanup_on_error: true,
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

impl Default for S3SourceConfig {
    fn default() -> Self {
        Self {
            region: default_s3_region(),
            prefix: None,
            timeout: default_s3_timeout(),
        }
    }
}

impl Default for RegistryCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: default_cache_ttl(),
            max_entries: default_cache_max_entries(),
        }
    }
}

impl Validatable for RegistryConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(
            self.default_polling_interval.as_secs(),
            "default_polling_interval",
            self.domain_name(),
        )?;

        self.cache.validate()?;

        // Validate sources
        for (index, source) in self.sources.iter().enumerate() {
            source.validate_with_context(&format!("sources[{}]", index))?;
        }

        // Validate auth configs
        for (name, auth) in &self.auth {
            auth.validate_with_name(name)?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "registry"
    }
}

impl Validatable for RegistryCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        if self.enabled {
            validate_positive(self.ttl.as_secs(), "ttl", self.domain_name())?;
            validate_positive(self.max_entries, "max_entries", self.domain_name())?;
        }

        Ok(())
    }

    fn domain_name(&self) -> &'static str {
        "registry.cache"
    }
}

impl RegistrySourceConfig {
    pub fn validate_with_context(&self, context: &str) -> ConfigResult<()> {
        validate_required_string(&self.name, "name", "registry")?;
        validate_required_string(&self.uri, "uri", "registry")?;

        // Validate URI format based on source type
        match self.source_type {
            RegistrySourceType::Http => {
                validate_url(&self.uri, "uri", "registry")?;
            }
            RegistrySourceType::Filesystem => {
                if !self.uri.starts_with("file://") && !std::path::Path::new(&self.uri).exists() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!(
                            "{}: filesystem path does not exist: {}",
                            context, self.uri
                        ),
                    });
                }
            }
            RegistrySourceType::Git => {
                // Basic Git URL validation
                if !self.uri.starts_with("git://")
                    && !self.uri.starts_with("https://")
                    && !self.uri.starts_with("ssh://")
                    && !self.uri.contains("@")
                {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!("{}: invalid Git URL format: {}", context, self.uri),
                    });
                }
            }
            RegistrySourceType::S3 => {
                if !self.uri.starts_with("s3://") {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!("{}: S3 URI must start with s3://: {}", context, self.uri),
                    });
                }
            }
        }

        // Validate polling interval if specified
        if let Some(interval) = self.polling_interval {
            if interval.as_secs() == 0 {
                return Err(crate::error::ConfigError::DomainError {
                    domain: "registry".to_string(),
                    message: format!("{}: polling_interval must be greater than 0", context),
                });
            }
        }

        Ok(())
    }
}

impl RegistryAuthConfig {
    pub fn validate_with_name(&self, name: &str) -> ConfigResult<()> {
        let context = &format!("auth.{}", name);

        match self {
            Self::Basic { username, password } => {
                validate_required_string(username, "username", "registry")?;
                validate_required_string(password, "password", "registry")?;
            }
            Self::Bearer { token } => {
                validate_required_string(token, "token", "registry")?;
            }
            Self::ApiKey { header, value } => {
                validate_required_string(header, "header", "registry")?;
                validate_required_string(value, "value", "registry")?;
            }
            Self::GitToken { token } => {
                validate_required_string(token, "token", "registry")?;
            }
            Self::SshKey {
                private_key_path, ..
            } => {
                validate_required_string(private_key_path, "private_key_path", "registry")?;

                // Check if private key file exists
                if !std::path::Path::new(private_key_path).exists() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!(
                            "{}: private key file not found: {}",
                            context, private_key_path
                        ),
                    });
                }
            }
            Self::GitHubApp {
                app_id,
                private_key_path,
                installation_id,
            } => {
                validate_required_string(app_id, "app_id", "registry")?;
                validate_required_string(private_key_path, "private_key_path", "registry")?;
                validate_required_string(installation_id, "installation_id", "registry")?;

                // Check if private key file exists
                if !std::path::Path::new(private_key_path).exists() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!(
                            "{}: GitHub App private key file not found: {}",
                            context, private_key_path
                        ),
                    });
                }
            }
            Self::ClientCertificate {
                cert_path,
                key_path,
                ca_cert_path,
            } => {
                validate_required_string(cert_path, "cert_path", "registry")?;
                validate_required_string(key_path, "key_path", "registry")?;

                // Check if certificate files exist
                if !std::path::Path::new(cert_path).exists() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!(
                            "{}: client certificate file not found: {}",
                            context, cert_path
                        ),
                    });
                }

                if !std::path::Path::new(key_path).exists() {
                    return Err(crate::error::ConfigError::DomainError {
                        domain: "registry".to_string(),
                        message: format!(
                            "{}: client key file not found: {}",
                            context, key_path
                        ),
                    });
                }

                if let Some(ca_path) = ca_cert_path {
                    if !std::path::Path::new(ca_path).exists() {
                        return Err(crate::error::ConfigError::DomainError {
                            domain: "registry".to_string(),
                            message: format!(
                                "{}: CA certificate file not found: {}",
                                context, ca_path
                            ),
                        });
                    }
                }
            }
            Self::AwsCredentials {
                access_key_id,
                secret_access_key,
                ..
            } => {
                validate_required_string(access_key_id, "access_key_id", "registry")?;
                validate_required_string(secret_access_key, "secret_access_key", "registry")?;
            }
        }

        Ok(())
    }
}

// Default value functions
fn default_polling_interval() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_cache_max_entries() -> usize {
    1000
}

fn default_source_type() -> RegistrySourceType {
    RegistrySourceType::Filesystem
}

fn default_file_patterns() -> Vec<String> {
    vec!["**/*.js".to_string(), "**/*.json".to_string()]
}

fn default_http_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_user_agent() -> String {
    "Ratchet Registry Client/1.0".to_string()
}

fn default_git_branch() -> String {
    "main".to_string()
}

fn default_git_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes for clone operations
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

fn default_s3_timeout() -> Duration {
    Duration::from_secs(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_defaults() {
        let config = RegistryConfig::default();
        assert!(config.sources.is_empty());
        assert_eq!(config.default_polling_interval, Duration::from_secs(300));
        assert!(config.cache.enabled);
    }

    #[test]
    fn test_registry_source_validation() {
        let mut source = RegistrySourceConfig {
            name: "test".to_string(),
            uri: "https://example.com/registry".to_string(),
            source_type: RegistrySourceType::Http,
            polling_interval: None,
            enabled: true,
            auth_name: None,
            config: SourceSpecificConfig::default(),
        };
        assert!(source.validate_with_context("test").is_ok());

        // Test empty name
        source.name = String::new();
        assert!(source.validate_with_context("test").is_err());

        // Test invalid URL for HTTP source
        source.name = "test".to_string();
        source.uri = "not-a-url".to_string();
        assert!(source.validate_with_context("test").is_err());
    }

    #[test]
    fn test_registry_auth_validation() {
        let auth = RegistryAuthConfig::Bearer {
            token: "test-token".to_string(),
        };
        assert!(auth.validate_with_name("test").is_ok());

        let invalid_auth = RegistryAuthConfig::Bearer {
            token: String::new(),
        };
        assert!(invalid_auth.validate_with_name("test").is_err());
    }

    #[test]
    fn test_source_type_uri_validation() {
        // Test S3 URI
        let s3_source = RegistrySourceConfig {
            name: "s3-test".to_string(),
            uri: "s3://bucket/prefix".to_string(),
            source_type: RegistrySourceType::S3,
            polling_interval: None,
            enabled: true,
            auth_name: None,
            config: SourceSpecificConfig::default(),
        };
        assert!(s3_source.validate_with_context("test").is_ok());

        // Test invalid S3 URI
        let invalid_s3 = RegistrySourceConfig {
            name: "s3-test".to_string(),
            uri: "http://bucket/prefix".to_string(),
            source_type: RegistrySourceType::S3,
            polling_interval: None,
            enabled: true,
            auth_name: None,
            config: SourceSpecificConfig::default(),
        };
        assert!(invalid_s3.validate_with_context("test").is_err());
    }
}
