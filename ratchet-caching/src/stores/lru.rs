//! LRU (Least Recently Used) cache implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::{
    cache::{Cache, CacheEntry, CacheKey, CacheValue},
    stats::{create_stats_collector, SharedStatsCollector},
    CacheError, CacheResult, CacheStats,
};

/// LRU cache node
struct LruNode<K, V> {
    key: K,
    entry: CacheEntry<V>,
}

/// LRU cache implementation
pub struct LruCache<K, V> {
    /// Maximum capacity
    capacity: usize,
    
    /// Map from key to index in the queue
    map: Arc<RwLock<HashMap<K, usize>>>,
    
    /// Queue of entries (front = most recent, back = least recent)
    queue: Arc<RwLock<VecDeque<LruNode<K, V>>>>,
    
    /// Statistics collector
    stats: SharedStatsCollector,
}

impl<K: CacheKey + 'static, V: CacheValue + 'static> LruCache<K, V> {
    /// Create a new LRU cache with specified capacity
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("LRU cache capacity must be greater than 0");
        }
        
        Self {
            capacity,
            map: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            stats: create_stats_collector(),
        }
    }
    
    /// Move an entry to the front of the queue
    fn move_to_front(&self, index: usize) -> CacheResult<()> {
        let mut queue = self.queue.write();
        
        if index >= queue.len() {
            return Err(CacheError::BackendError("Invalid LRU index".to_string()));
        }
        
        // Remove from current position
        if let Some(node) = queue.remove(index) {
            // Update indices for items that moved
            let mut map = self.map.write();
            for (_, idx) in map.iter_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }
            
            // Add to front
            queue.push_front(node);
            
            // Update index
            if let Some(front) = queue.front() {
                map.insert(front.key.clone(), 0);
            }
            
            // Update all indices
            for (i, node) in queue.iter().enumerate() {
                map.insert(node.key.clone(), i);
            }
        }
        
        Ok(())
    }
    
}

#[async_trait]
impl<K: CacheKey + 'static, V: CacheValue + 'static> Cache<K, V> for LruCache<K, V> {
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let start = std::time::Instant::now();
        
        // Get index while holding lock, then release before async operations
        let maybe_index = {
            let map = self.map.read();
            map.get(key).copied()
        };
        
        let result = match maybe_index {
            Some(index) => {
                // Check if entry is expired
                let is_expired = {
                    let queue = self.queue.read();
                    queue.get(index).map_or(true, |node| node.entry.is_expired())
                };
                
                if is_expired {
                    // Remove expired entry
                    self.remove(key).await?;
                    self.stats.record_miss();
                    None
                } else {
                    // Move to front and get value
                    self.move_to_front(index)?;
                    
                    let value = {
                        let mut queue = self.queue.write();
                        if let Some(node) = queue.get_mut(0) {
                            node.entry.record_access();
                            Some(node.entry.value.clone())
                        } else {
                            None
                        }
                    };
                    
                    if value.is_some() {
                        self.stats.record_hit();
                    } else {
                        self.stats.record_miss();
                    }
                    
                    value
                }
            }
            None => {
                self.stats.record_miss();
                None
            }
        };
        
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_get_latency(latency_ns);
        
        Ok(result)
    }
    
    async fn put(&self, key: K, value: V) -> CacheResult<()> {
        self.put_with_ttl(key, value, std::time::Duration::MAX).await
    }
    
    async fn put_with_ttl(
        &self,
        key: K,
        value: V,
        ttl: std::time::Duration,
    ) -> CacheResult<()> {
        let start = std::time::Instant::now();
        
        {
            let mut map = self.map.write();
            let mut queue = self.queue.write();
            
            // If key already exists, remove it first
            if let Some(&index) = map.get(&key) {
                queue.remove(index);
                
                // Update indices
                for (_, idx) in map.iter_mut() {
                    if *idx > index {
                        *idx -= 1;
                    }
                }
            }
            
            // Create new entry
            let entry = if ttl == std::time::Duration::MAX {
                CacheEntry::new(value)
            } else {
                CacheEntry::with_ttl(value, ttl)
            };
            
            let node = LruNode {
                key: key.clone(),
                entry,
            };
            
            // Add to front
            queue.push_front(node);
            
            // Update map
            map.insert(key, 0);
            
            // Update all indices
            for (i, node) in queue.iter().enumerate() {
                map.insert(node.key.clone(), i);
            }
            
            // Evict if over capacity
            while queue.len() > self.capacity {
                if let Some(evicted) = queue.pop_back() {
                    map.remove(&evicted.key);
                    self.stats.record_eviction();
                }
            }
            
            self.stats.record_put();
        }
        
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.stats.record_put_latency(latency_ns);
        
        Ok(())
    }
    
    async fn remove(&self, key: &K) -> CacheResult<Option<V>> {
        let mut map = self.map.write();
        let mut queue = self.queue.write();
        
        if let Some(index) = map.remove(key) {
            if let Some(node) = queue.remove(index) {
                // Update indices
                for (_, idx) in map.iter_mut() {
                    if *idx > index {
                        *idx -= 1;
                    }
                }
                
                if !node.entry.is_expired() {
                    Ok(Some(node.entry.value))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    async fn clear(&self) -> CacheResult<()> {
        let mut map = self.map.write();
        let mut queue = self.queue.write();
        
        let count = queue.len();
        
        map.clear();
        queue.clear();
        
        // Record evictions
        for _ in 0..count {
            self.stats.record_eviction();
        }
        
        Ok(())
    }
    
    async fn len(&self) -> CacheResult<usize> {
        let queue = self.queue.read();
        
        // Count non-expired entries
        let count = queue
            .iter()
            .filter(|node| !node.entry.is_expired())
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
    async fn test_lru_eviction() {
        let cache = LruCache::new(3);
        
        // Fill cache
        cache.put("a", 1).await.unwrap();
        cache.put("b", 2).await.unwrap();
        cache.put("c", 3).await.unwrap();
        
        // Access 'a' to make it most recent
        assert_eq!(cache.get(&"a").await.unwrap(), Some(1));
        
        // Add new item, should evict 'b' (least recently used)
        cache.put("d", 4).await.unwrap();
        
        assert_eq!(cache.get(&"a").await.unwrap(), Some(1));
        assert_eq!(cache.get(&"b").await.unwrap(), None); // Evicted
        assert_eq!(cache.get(&"c").await.unwrap(), Some(3));
        assert_eq!(cache.get(&"d").await.unwrap(), Some(4));
    }
    
    #[tokio::test]
    async fn test_lru_update() {
        let cache = LruCache::new(2);
        
        cache.put("a", 1).await.unwrap();
        cache.put("b", 2).await.unwrap();
        
        // Update 'a' with new value
        cache.put("a", 10).await.unwrap();
        
        assert_eq!(cache.get(&"a").await.unwrap(), Some(10));
        assert_eq!(cache.get(&"b").await.unwrap(), Some(2));
        assert_eq!(cache.len().await.unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_lru_capacity() {
        let cache = LruCache::new(1);
        
        cache.put("a", 1).await.unwrap();
        assert_eq!(cache.len().await.unwrap(), 1);
        
        cache.put("b", 2).await.unwrap();
        assert_eq!(cache.len().await.unwrap(), 1);
        assert_eq!(cache.get(&"a").await.unwrap(), None); // Evicted
        assert_eq!(cache.get(&"b").await.unwrap(), Some(2));
    }
}