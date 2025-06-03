//! Time-based TTL cache implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{
    cache::{Cache, CacheEntry, CacheKey, CacheValue},
    stats::{create_stats_collector, SharedStatsCollector},
    CacheResult, CacheStats,
};

/// TTL-based cache implementation
pub struct TtlCache<K, V> {
    /// Default TTL for entries
    default_ttl: Duration,
    
    /// Store with entries
    store: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    
    /// Statistics collector
    stats: SharedStatsCollector,
}

impl<K: CacheKey + 'static, V: CacheValue + 'static> TtlCache<K, V> {
    /// Create a new TTL cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            default_ttl,
            store: Arc::new(RwLock::new(HashMap::new())),
            stats: create_stats_collector(),
        }
    }
    
    /// Create with capacity hint
    pub fn with_capacity(default_ttl: Duration, capacity: usize) -> Self {
        Self {
            default_ttl,
            store: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            stats: create_stats_collector(),
        }
    }
    
    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> CacheResult<usize> {
        let mut store = self.store.write();
        let now = Instant::now();
        
        let expired_keys: Vec<K> = store
            .iter()
            .filter(|(_, entry)| {
                entry.expires_at.is_some_and(|expires_at| now > expires_at)
            })
            .map(|(k, _)| k.clone())
            .collect();
        
        let count = expired_keys.len();
        
        for key in expired_keys {
            store.remove(&key);
            self.stats.record_eviction();
        }
        
        Ok(count)
    }
    
    /// Start a background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                let _ = self.cleanup_expired().await;
            }
        })
    }
}

#[async_trait]
impl<K: CacheKey + 'static, V: CacheValue + 'static> Cache<K, V> for TtlCache<K, V> {
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let start = Instant::now();
        
        let result = {
            let mut store = self.store.write();
            match store.get_mut(key) {
                Some(entry) => {
                    if entry.is_expired() {
                        store.remove(key);
                        self.stats.record_eviction();
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
        self.put_with_ttl(key, value, self.default_ttl).await
    }
    
    async fn put_with_ttl(
        &self,
        key: K,
        value: V,
        ttl: Duration,
    ) -> CacheResult<()> {
        let start = Instant::now();
        
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
                    self.stats.record_eviction();
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
        let count = store
            .values()
            .filter(|entry| !entry.is_expired())
            .count();
        
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
    
    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = TtlCache::new(Duration::from_millis(100));
        
        // Put with default TTL
        cache.put("key1", "value1").await.unwrap();
        
        // Should exist immediately
        assert_eq!(cache.get(&"key1").await.unwrap(), Some("value1"));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get(&"key1").await.unwrap(), None);
    }
    
    #[tokio::test]
    async fn test_custom_ttl() {
        let cache = TtlCache::new(Duration::from_secs(10)); // Default 10s
        
        // Put with custom short TTL
        cache
            .put_with_ttl("key1", "value1", Duration::from_millis(50))
            .await
            .unwrap();
        
        // Put with default TTL
        cache.put("key2", "value2").await.unwrap();
        
        // Both should exist
        assert_eq!(cache.get(&"key1").await.unwrap(), Some("value1"));
        assert_eq!(cache.get(&"key2").await.unwrap(), Some("value2"));
        
        // Wait for first to expire
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // First should be expired, second still valid
        assert_eq!(cache.get(&"key1").await.unwrap(), None);
        assert_eq!(cache.get(&"key2").await.unwrap(), Some("value2"));
    }
    
    #[tokio::test]
    async fn test_cleanup() {
        let cache = TtlCache::new(Duration::from_millis(50));
        
        // Add multiple entries
        for i in 0..5 {
            cache.put(i, i * 10).await.unwrap();
        }
        
        assert_eq!(cache.len().await.unwrap(), 5);
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Cleanup
        let cleaned = cache.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 5);
        assert_eq!(cache.len().await.unwrap(), 0);
    }
}