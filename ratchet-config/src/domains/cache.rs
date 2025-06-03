//! Caching configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::validation::{Validatable, validate_positive, validate_enum_choice};
use crate::error::ConfigResult;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Whether caching is enabled globally
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,
    
    /// Task cache configuration
    #[serde(default)]
    pub task_cache: TaskCacheConfig,
    
    /// HTTP response cache configuration
    #[serde(default)]
    pub http_cache: HttpCacheConfig,
    
    /// Result cache configuration
    #[serde(default)]
    pub result_cache: ResultCacheConfig,
}

/// Task cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskCacheConfig {
    /// Cache implementation type
    #[serde(default = "default_task_cache_type")]
    pub cache_type: String,
    
    /// LRU cache size for task content
    #[serde(default = "default_task_content_cache_size")]
    pub task_content_cache_size: usize,
    
    /// Memory limit in bytes
    #[serde(default = "default_task_memory_limit")]
    pub memory_limit_bytes: usize,
    
    /// TTL for cached task definitions
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_task_ttl")]
    pub ttl: Duration,
}

/// HTTP cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpCacheConfig {
    /// Whether HTTP response caching is enabled
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,
    
    /// Maximum cache size in bytes
    #[serde(default = "default_http_cache_size")]
    pub max_size_bytes: usize,
    
    /// Default TTL for responses without cache headers
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_http_ttl")]
    pub default_ttl: Duration,
    
    /// Whether to respect cache-control headers
    #[serde(default = "crate::domains::utils::default_true")]
    pub respect_cache_control: bool,
}

/// Result cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResultCacheConfig {
    /// Whether result caching is enabled
    #[serde(default = "crate::domains::utils::default_true")]
    pub enabled: bool,
    
    /// Whether to cache only successful results
    #[serde(default = "crate::domains::utils::default_true")]
    pub cache_only_success: bool,
    
    /// Maximum number of cached results
    #[serde(default = "default_result_cache_capacity")]
    pub max_entries: usize,
    
    /// TTL for cached results
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_result_ttl")]
    pub ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            task_cache: TaskCacheConfig::default(),
            http_cache: HttpCacheConfig::default(),
            result_cache: ResultCacheConfig::default(),
        }
    }
}

impl Default for TaskCacheConfig {
    fn default() -> Self {
        Self {
            cache_type: default_task_cache_type(),
            task_content_cache_size: default_task_content_cache_size(),
            memory_limit_bytes: default_task_memory_limit(),
            ttl: default_task_ttl(),
        }
    }
}

impl Default for HttpCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_bytes: default_http_cache_size(),
            default_ttl: default_http_ttl(),
            respect_cache_control: true,
        }
    }
}

impl Default for ResultCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_only_success: true,
            max_entries: default_result_cache_capacity(),
            ttl: default_result_ttl(),
        }
    }
}

impl Validatable for CacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.task_cache.validate()?;
        self.http_cache.validate()?;
        self.result_cache.validate()?;
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "cache"
    }
}

impl Validatable for TaskCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate cache type
        let valid_types = ["lru", "ttl", "moka", "inmemory"];
        validate_enum_choice(&self.cache_type, &valid_types, "cache_type", self.domain_name())?;
        
        validate_positive(
            self.task_content_cache_size,
            "task_content_cache_size",
            self.domain_name()
        )?;
        
        validate_positive(
            self.memory_limit_bytes,
            "memory_limit_bytes",
            self.domain_name()
        )?;
        
        validate_positive(
            self.ttl.as_secs(),
            "ttl",
            self.domain_name()
        )?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "cache.task_cache"
    }
}

impl Validatable for HttpCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(
            self.max_size_bytes,
            "max_size_bytes",
            self.domain_name()
        )?;
        
        validate_positive(
            self.default_ttl.as_secs(),
            "default_ttl",
            self.domain_name()
        )?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "cache.http_cache"
    }
}

impl Validatable for ResultCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_positive(
            self.max_entries,
            "max_entries",
            self.domain_name()
        )?;
        
        validate_positive(
            self.ttl.as_secs(),
            "ttl",
            self.domain_name()
        )?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "cache.result_cache"
    }
}

// Default value functions
fn default_task_cache_type() -> String {
    "lru".to_string()
}

fn default_task_content_cache_size() -> usize {
    100
}

fn default_task_memory_limit() -> usize {
    64 * 1024 * 1024 // 64MB
}

fn default_task_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_http_cache_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_http_ttl() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_result_cache_capacity() -> usize {
    1000
}

fn default_result_ttl() -> Duration {
    Duration::from_secs(1800) // 30 minutes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.task_cache.task_content_cache_size, 100);
        assert_eq!(config.task_cache.cache_type, "lru");
    }

    #[test]
    fn test_task_cache_validation() {
        let mut config = TaskCacheConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid cache type
        config.cache_type = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Test zero cache size
        config = TaskCacheConfig::default();
        config.task_content_cache_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_http_cache_validation() {
        let mut config = HttpCacheConfig::default();
        assert!(config.validate().is_ok());
        
        // Test zero size
        config.max_size_bytes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_result_cache_validation() {
        let mut config = ResultCacheConfig::default();
        assert!(config.validate().is_ok());
        
        // Test zero entries
        config.max_entries = 0;
        assert!(config.validate().is_err());
    }
}