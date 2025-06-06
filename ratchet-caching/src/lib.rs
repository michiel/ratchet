//! Caching abstractions and implementations for Ratchet
//!
//! This crate provides a unified caching layer with multiple backend implementations
//! and specialized caches for different types of data.

pub mod cache;
pub mod config;
pub mod errors;
pub mod stats;
pub mod stores;

// Specialized cache implementations
pub mod task_cache;
pub mod http_cache;
pub mod result_cache;

// Re-export main types
pub use cache::{Cache, CacheKey, CacheValue};
pub use config::CacheConfig;
pub use errors::{CacheError, CacheResult};
pub use stats::CacheStats;

// Re-export store implementations
pub use stores::InMemoryCache;

#[cfg(feature = "lru")]
pub use stores::LruCache;

#[cfg(feature = "ttl")]
pub use stores::TtlCache;

#[cfg(feature = "moka")]
pub use stores::MokaCache;

// Re-export specialized caches
pub use task_cache::TaskCache;
pub use http_cache::HttpCache;
pub use result_cache::ResultCache;

/// Create a default in-memory cache
pub fn create_default_cache<K, V>() -> impl Cache<K, V>
where
    K: CacheKey + 'static,
    V: CacheValue + 'static,
{
    InMemoryCache::new()
}

/// Create an LRU cache with specified capacity
#[cfg(feature = "lru")]
pub fn create_lru_cache<K, V>(capacity: usize) -> impl Cache<K, V>
where
    K: CacheKey + 'static,
    V: CacheValue + 'static,
{
    LruCache::new(capacity)
}

/// Create a TTL-based cache with default TTL
#[cfg(feature = "ttl")]
pub fn create_ttl_cache<K, V>(default_ttl: std::time::Duration) -> impl Cache<K, V>
where
    K: CacheKey + 'static,
    V: CacheValue + 'static,
{
    TtlCache::new(default_ttl)
}

/// Create a high-performance Moka-based cache
#[cfg(feature = "moka")]
pub fn create_moka_cache<K, V>(max_capacity: u64) -> impl Cache<K, V>
where
    K: CacheKey + Send + Sync + 'static,
    V: CacheValue + Send + Sync + 'static,
{
    MokaCache::new(max_capacity)
}