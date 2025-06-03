//! Moka-based high-performance cache implementation

use async_trait::async_trait;
use moka::future::Cache as MokaInner;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::{Cache, CacheKey, CacheValue},
    stats::{create_stats_collector, SharedStatsCollector},
    CacheResult, CacheStats,
};

/// Moka-based cache wrapper
pub struct MokaCache<K, V> {
    /// Inner Moka cache
    inner: MokaInner<K, V>,
    
    /// Statistics collector
    stats: SharedStatsCollector,
}

impl<K, V> MokaCache<K, V>
where
    K: CacheKey + Send + Sync + 'static,
    V: CacheValue + Send + Sync + 'static,
{
    /// Create a new Moka cache with max capacity
    pub fn new(max_capacity: u64) -> Self {
        let inner = MokaInner::builder()
            .max_capacity(max_capacity)
            .build();
        
        Self {
            inner,
            stats: create_stats_collector(),
        }
    }
    
    /// Create a new Moka cache with advanced configuration
    pub fn builder() -> MokaCacheBuilder<K, V> {
        MokaCacheBuilder::new()
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for MokaCache<K, V>
where
    K: CacheKey + Send + Sync + 'static,
    V: CacheValue + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let start = std::time::Instant::now();
        
        let result = self.inner.get(key).await;
        
        if result.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }
        
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_get_latency(latency_ns);
        
        Ok(result)
    }
    
    async fn put(&self, key: K, value: V) -> CacheResult<()> {
        let start = std::time::Instant::now();
        
        self.inner.insert(key, value).await;
        self.stats.record_put();
        
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_put_latency(latency_ns);
        
        Ok(())
    }
    
    async fn put_with_ttl(
        &self,
        key: K,
        value: V,
        _ttl: Duration,
    ) -> CacheResult<()> {
        let start = std::time::Instant::now();
        
        // Moka doesn't have insert_with_ttl, need to use a builder or invalidate after TTL
        self.inner.insert(key, value).await;
        // Note: TTL should be configured at cache creation time in Moka
        self.stats.record_put();
        
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_put_latency(latency_ns);
        
        Ok(())
    }
    
    async fn remove(&self, key: &K) -> CacheResult<Option<V>> {
        let result = self.inner.remove(key).await;
        
        if result.is_some() {
            self.stats.record_eviction();
        }
        
        Ok(result)
    }
    
    async fn clear(&self) -> CacheResult<()> {
        let count = self.inner.entry_count() as usize;
        
        self.inner.invalidate_all();
        
        // Record evictions
        for _ in 0..count {
            self.stats.record_eviction();
        }
        
        Ok(())
    }
    
    async fn len(&self) -> CacheResult<usize> {
        Ok(self.inner.entry_count() as usize)
    }
    
    async fn stats(&self) -> CacheResult<CacheStats> {
        let len = self.len().await?;
        let inner_stats = self.inner.entry_count();
        
        Ok(self.stats.get_stats(len, Some(inner_stats as usize * 100))) // Rough estimate
    }
}

/// Builder for Moka cache
pub struct MokaCacheBuilder<K, V> {
    max_capacity: Option<u64>,
    time_to_live: Option<Duration>,
    time_to_idle: Option<Duration>,
    weigher: Option<Arc<dyn Fn(&K, &V) -> u32 + Send + Sync + 'static>>,
}

impl<K, V> MokaCacheBuilder<K, V>
where
    K: CacheKey + Send + Sync + 'static,
    V: CacheValue + Send + Sync + 'static,
{
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            max_capacity: None,
            time_to_live: None,
            time_to_idle: None,
            weigher: None,
        }
    }
    
    /// Set max capacity
    pub fn max_capacity(mut self, capacity: u64) -> Self {
        self.max_capacity = Some(capacity);
        self
    }
    
    /// Set time to live
    pub fn time_to_live(mut self, ttl: Duration) -> Self {
        self.time_to_live = Some(ttl);
        self
    }
    
    /// Set time to idle
    pub fn time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }
    
    /// Set custom weigher function
    pub fn weigher<F>(mut self, weigher: F) -> Self
    where
        F: Fn(&K, &V) -> u32 + Send + Sync + 'static,
    {
        self.weigher = Some(Arc::new(weigher));
        self
    }
    
    /// Build the cache
    pub fn build(self) -> MokaCache<K, V> {
        let mut builder = MokaInner::builder();
        
        if let Some(capacity) = self.max_capacity {
            builder = builder.max_capacity(capacity);
        }
        
        if let Some(ttl) = self.time_to_live {
            builder = builder.time_to_live(ttl);
        }
        
        if let Some(tti) = self.time_to_idle {
            builder = builder.time_to_idle(tti);
        }
        
        if let Some(weigher) = self.weigher {
            builder = builder.weigher(move |k: &K, v: &V| weigher(k, v));
        }
        
        MokaCache {
            inner: builder.build(),
            stats: create_stats_collector(),
        }
    }
}

impl<K, V> Default for MokaCacheBuilder<K, V>
where
    K: CacheKey + Send + Sync + 'static,
    V: CacheValue + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_moka_basic() {
        let cache = MokaCache::new(10);
        
        // Put and get
        cache.put("key1", "value1").await.unwrap();
        assert_eq!(cache.get(&"key1").await.unwrap(), Some("value1"));
        
        // Remove
        let removed = cache.remove(&"key1").await.unwrap();
        assert_eq!(removed, Some("value1"));
        assert_eq!(cache.get(&"key1").await.unwrap(), None);
    }
    
    #[tokio::test]
    async fn test_moka_ttl() {
        let cache = MokaCache::builder()
            .max_capacity(10)
            .time_to_live(Duration::from_millis(50))
            .build();
        
        cache.put("key1", "value1").await.unwrap();
        assert_eq!(cache.get(&"key1").await.unwrap(), Some("value1"));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Moka runs expiration in background, so we need to trigger it
        cache.inner.run_pending_tasks().await;
        
        assert_eq!(cache.get(&"key1").await.unwrap(), None);
    }
    
    #[tokio::test]
    async fn test_moka_capacity() {
        let cache = MokaCache::new(2);
        
        cache.put("a", 1).await.unwrap();
        cache.put("b", 2).await.unwrap();
        cache.put("c", 3).await.unwrap();
        
        // Let Moka process evictions
        cache.inner.run_pending_tasks().await;
        
        // Check that capacity is respected
        assert!(cache.len().await.unwrap() <= 2);
    }
}