//! Cache configuration

use serde::{Deserialize, Serialize};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,

    /// Default cache type
    pub default_cache_type: CacheType,

    /// Task cache configuration
    pub task_cache: TaskCacheConfig,

    /// HTTP cache configuration
    pub http_cache: HttpCacheConfig,

    /// Result cache configuration
    pub result_cache: ResultCacheConfig,

    /// Global cache settings
    pub global: GlobalCacheConfig,
}

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    /// Simple in-memory cache
    InMemory,

    /// LRU cache with fixed capacity
    Lru,

    /// Time-based cache with TTL
    Ttl,

    /// High-performance Moka cache
    Moka,
}

/// Task cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCacheConfig {
    /// Enable task caching
    pub enabled: bool,

    /// Maximum number of tasks to cache
    pub max_entries: usize,

    /// Maximum memory usage in MB
    pub max_memory_mb: usize,

    /// Cache type for tasks
    pub cache_type: CacheType,

    /// TTL for cached tasks
    pub ttl_seconds: Option<u64>,
}

/// HTTP cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpCacheConfig {
    /// Enable HTTP response caching
    pub enabled: bool,

    /// Maximum number of responses to cache
    pub max_entries: usize,

    /// Maximum response size to cache (in bytes)
    pub max_response_size: usize,

    /// Default TTL for cached responses
    pub default_ttl_seconds: u64,

    /// Honor cache-control headers
    pub honor_cache_control: bool,

    /// Cache type for HTTP responses
    pub cache_type: CacheType,
}

/// Result cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultCacheConfig {
    /// Enable execution result caching
    pub enabled: bool,

    /// Maximum number of results to cache
    pub max_entries: usize,

    /// Maximum result size to cache (in bytes)
    pub max_result_size: usize,

    /// TTL for cached results
    pub ttl_seconds: u64,

    /// Only cache successful results
    pub cache_only_success: bool,

    /// Cache type for results
    pub cache_type: CacheType,
}

/// Global cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCacheConfig {
    /// Enable cache statistics collection
    pub collect_stats: bool,

    /// Stats collection interval
    pub stats_interval_seconds: u64,

    /// Maximum total memory usage across all caches (in MB)
    pub max_total_memory_mb: Option<usize>,

    /// Enable cache warming on startup
    pub warm_on_startup: bool,

    /// Enable cache persistence
    pub persist_to_disk: bool,

    /// Cache persistence directory
    pub persistence_dir: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_cache_type: CacheType::Lru,
            task_cache: TaskCacheConfig::default(),
            http_cache: HttpCacheConfig::default(),
            result_cache: ResultCacheConfig::default(),
            global: GlobalCacheConfig::default(),
        }
    }
}

impl Default for TaskCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            max_memory_mb: 100,
            cache_type: CacheType::Lru,
            ttl_seconds: None, // No expiration by default
        }
    }
}

impl Default for HttpCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 500,
            max_response_size: 1024 * 1024, // 1MB
            default_ttl_seconds: 300,       // 5 minutes
            honor_cache_control: true,
            cache_type: CacheType::Ttl,
        }
    }
}

impl Default for ResultCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default as not all tasks are deterministic
            max_entries: 100,
            max_result_size: 10 * 1024 * 1024, // 10MB
            ttl_seconds: 3600,                 // 1 hour
            cache_only_success: true,
            cache_type: CacheType::Ttl,
        }
    }
}

impl Default for GlobalCacheConfig {
    fn default() -> Self {
        Self {
            collect_stats: true,
            stats_interval_seconds: 60,
            max_total_memory_mb: Some(500),
            warm_on_startup: false,
            persist_to_disk: false,
            persistence_dir: None,
        }
    }
}

impl CacheConfig {
    /// Create a development configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.global.collect_stats = true;
        config.global.persist_to_disk = false;
        config.http_cache.default_ttl_seconds = 60; // Shorter TTL for dev
        config
    }

    /// Create a production configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.global.warm_on_startup = true;
        config.global.persist_to_disk = true;
        config.http_cache.honor_cache_control = true;
        config.result_cache.enabled = true;
        config
    }

    /// Create a minimal configuration (caching mostly disabled)
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            default_cache_type: CacheType::InMemory,
            task_cache: TaskCacheConfig {
                enabled: true,
                max_entries: 100,
                max_memory_mb: 10,
                ..Default::default()
            },
            http_cache: HttpCacheConfig {
                enabled: false,
                ..Default::default()
            },
            result_cache: ResultCacheConfig {
                enabled: false,
                ..Default::default()
            },
            global: GlobalCacheConfig {
                collect_stats: false,
                persist_to_disk: false,
                ..Default::default()
            },
        }
    }
}
