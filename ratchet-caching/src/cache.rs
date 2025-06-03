//! Core cache traits and types

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use crate::{CacheResult, CacheStats};

/// Trait for types that can be used as cache keys
pub trait CacheKey: Clone + Eq + Hash + Debug + Send + Sync {}

/// Trait for types that can be cached
pub trait CacheValue: Clone + Debug + Send + Sync {}

// Blanket implementations
impl<T> CacheKey for T where T: Clone + Eq + Hash + Debug + Send + Sync {}
impl<T> CacheValue for T where T: Clone + Debug + Send + Sync {}

/// Core cache trait
#[async_trait]
pub trait Cache<K: CacheKey + 'static, V: CacheValue + 'static>: Send + Sync {
    /// Get a value from the cache
    async fn get(&self, key: &K) -> CacheResult<Option<V>>;
    
    /// Put a value into the cache
    async fn put(&self, key: K, value: V) -> CacheResult<()>;
    
    /// Put a value with TTL
    async fn put_with_ttl(&self, key: K, value: V, _ttl: Duration) -> CacheResult<()> {
        // Default implementation ignores TTL
        self.put(key, value).await
    }
    
    /// Remove a value from the cache
    async fn remove(&self, key: &K) -> CacheResult<Option<V>>;
    
    /// Check if a key exists
    async fn contains_key(&self, key: &K) -> CacheResult<bool> {
        Ok(self.get(key).await?.is_some())
    }
    
    /// Clear all entries
    async fn clear(&self) -> CacheResult<()>;
    
    /// Get the number of entries
    async fn len(&self) -> CacheResult<usize>;
    
    /// Check if cache is empty
    async fn is_empty(&self) -> CacheResult<bool> {
        Ok(self.len().await? == 0)
    }
    
    /// Get cache statistics
    async fn stats(&self) -> CacheResult<CacheStats>;
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    /// The cached value
    pub value: V,
    
    /// When the entry was created
    pub created_at: std::time::Instant,
    
    /// When the entry expires (if applicable)
    pub expires_at: Option<std::time::Instant>,
    
    /// Number of times accessed
    pub access_count: u64,
    
    /// Last access time
    pub last_accessed: std::time::Instant,
    
    /// Size in bytes (estimated)
    pub size_bytes: Option<usize>,
}

impl<V: CacheValue> CacheEntry<V> {
    /// Create a new cache entry
    pub fn new(value: V) -> Self {
        let now = std::time::Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: None,
            access_count: 0,
            last_accessed: now,
            size_bytes: None,
        }
    }
    
    /// Create a new cache entry with TTL
    pub fn with_ttl(value: V, ttl: Duration) -> Self {
        let mut entry = Self::new(value);
        entry.expires_at = Some(entry.created_at + ttl);
        entry
    }
    
    /// Check if the entry is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            std::time::Instant::now() > expires_at
        } else {
            false
        }
    }
    
    /// Record an access
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = std::time::Instant::now();
    }
    
    /// Get age of the entry
    pub fn age(&self) -> Duration {
        std::time::Instant::now() - self.created_at
    }
}

/// Serializable cache for persistence
#[async_trait]
pub trait SerializableCache<K, V>: Cache<K, V>
where
    K: CacheKey + Serialize + DeserializeOwned + 'static,
    V: CacheValue + Serialize + DeserializeOwned + 'static,
{
    /// Serialize cache to bytes
    async fn serialize(&self) -> CacheResult<Vec<u8>>;
    
    /// Deserialize cache from bytes
    async fn deserialize(data: &[u8]) -> CacheResult<Self>
    where
        Self: Sized;
    
    /// Save cache to file
    async fn save_to_file(&self, path: &std::path::Path) -> CacheResult<()> {
        let data = self.serialize().await?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }
    
    /// Load cache from file
    async fn load_from_file(path: &std::path::Path) -> CacheResult<Self>
    where
        Self: Sized,
    {
        let data = tokio::fs::read(path).await?;
        Self::deserialize(&data).await
    }
}

/// Cache warmer trait for pre-loading cache
#[async_trait]
pub trait CacheWarmer<K: CacheKey + 'static, V: CacheValue + 'static> {
    /// Warm the cache with initial data
    async fn warm(&self, cache: &impl Cache<K, V>) -> CacheResult<usize>;
}

/// Memory-aware cache trait
#[async_trait]
pub trait MemoryAwareCache<K: CacheKey + 'static, V: CacheValue + 'static>: Cache<K, V> {
    /// Get current memory usage in bytes
    async fn memory_usage(&self) -> CacheResult<usize>;
    
    /// Get maximum memory limit in bytes
    async fn memory_limit(&self) -> CacheResult<Option<usize>>;
    
    /// Set maximum memory limit in bytes
    async fn set_memory_limit(&self, limit: Option<usize>) -> CacheResult<()>;
    
    /// Evict entries to free memory
    async fn evict_to_size(&self, target_size: usize) -> CacheResult<usize>;
}