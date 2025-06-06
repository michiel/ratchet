//! Simple in-memory cache implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    cache::{Cache, CacheEntry, CacheKey, CacheValue},
    stats::{create_stats_collector, SharedStatsCollector},
    CacheResult, CacheStats,
};

/// Simple in-memory cache
pub struct InMemoryCache<K, V> {
    store: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    stats: SharedStatsCollector,
}

impl<K: CacheKey + 'static, V: CacheValue + 'static> InMemoryCache<K, V> {
    /// Create a new in-memory cache
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            stats: create_stats_collector(),
        }
    }

    /// Create with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            stats: create_stats_collector(),
        }
    }
}

impl<K: CacheKey + 'static, V: CacheValue + 'static> Default for InMemoryCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<K: CacheKey + 'static, V: CacheValue + 'static> Cache<K, V> for InMemoryCache<K, V> {
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let start = std::time::Instant::now();

        let result = {
            let mut store = self.store.write();
            match store.get_mut(key) {
                Some(entry) => {
                    if entry.is_expired() {
                        store.remove(key);
                        self.stats.record_miss();
                        None
                    } else {
                        entry.record_access();
                        self.stats.record_hit();
                        Some(entry.value.clone())
                    }
                }
                None => {
                    self.stats.record_miss();
                    None
                }
            }
        };

        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_get_latency(latency_ns);

        Ok(result)
    }

    async fn put(&self, key: K, value: V) -> CacheResult<()> {
        let start = std::time::Instant::now();

        {
            let mut store = self.store.write();
            store.insert(key, CacheEntry::new(value));
            self.stats.record_put();
        }

        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_put_latency(latency_ns);

        Ok(())
    }

    async fn put_with_ttl(&self, key: K, value: V, ttl: std::time::Duration) -> CacheResult<()> {
        let start = std::time::Instant::now();

        {
            let mut store = self.store.write();
            store.insert(key, CacheEntry::with_ttl(value, ttl));
            self.stats.record_put();
        }

        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_put_latency(latency_ns);

        Ok(())
    }

    async fn remove(&self, key: &K) -> CacheResult<Option<V>> {
        let mut store = self.store.write();
        match store.remove(key) {
            Some(entry) => {
                if entry.is_expired() {
                    Ok(None)
                } else {
                    Ok(Some(entry.value))
                }
            }
            None => Ok(None),
        }
    }

    async fn clear(&self) -> CacheResult<()> {
        let mut store = self.store.write();
        let count = store.len();
        store.clear();

        // Record evictions
        for _ in 0..count {
            self.stats.record_eviction();
        }

        Ok(())
    }

    async fn len(&self) -> CacheResult<usize> {
        let store = self.store.read();

        // Count non-expired entries
        let count = store.values().filter(|entry| !entry.is_expired()).count();

        Ok(count)
    }

    async fn stats(&self) -> CacheResult<CacheStats> {
        let len = self.len().await?;
        Ok(self.stats.get_stats(len, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = InMemoryCache::new();

        // Test put and get
        cache.put("key1", "value1").await.unwrap();
        let value = cache.get(&"key1").await.unwrap();
        assert_eq!(value, Some("value1"));

        // Test missing key
        let value = cache.get(&"key2").await.unwrap();
        assert_eq!(value, None);

        // Test remove
        let removed = cache.remove(&"key1").await.unwrap();
        assert_eq!(removed, Some("value1"));

        let value = cache.get(&"key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_ttl() {
        let cache = InMemoryCache::new();

        // Put with short TTL
        cache
            .put_with_ttl("key1", "value1", Duration::from_millis(50))
            .await
            .unwrap();

        // Should exist immediately
        let value = cache.get(&"key1").await.unwrap();
        assert_eq!(value, Some("value1"));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let value = cache.get(&"key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = InMemoryCache::new();

        // Put some values
        cache.put("key1", "value1").await.unwrap();
        cache.put("key2", "value2").await.unwrap();

        // Get operations
        cache.get(&"key1").await.unwrap(); // Hit
        cache.get(&"key2").await.unwrap(); // Hit
        cache.get(&"key3").await.unwrap(); // Miss

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.total_gets, 3);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_puts, 2);
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.hit_rate, 2.0 / 3.0);
    }
}
