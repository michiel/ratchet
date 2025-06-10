use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    ttl: Duration,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Duration) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

pub struct MemoryCache<K, V> {
    data: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    max_size: usize,
    default_ttl: Duration,
}

impl<K, V> Clone for MemoryCache<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            max_size: self.max_size,
            default_ttl: self.default_ttl,
        }
    }
}

impl<K, V> MemoryCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            default_ttl,
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        // First try to read without write lock
        {
            let data = self.data.read().await;
            if let Some(entry) = data.get(key) {
                if !entry.is_expired() {
                    return Some(entry.value.clone());
                }
            }
        }

        // If expired or not found, clean up expired entries
        self.cleanup_expired().await;
        None
    }

    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }

    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut data = self.data.write().await;

        // If we're at capacity and adding a new key, remove the oldest entry
        if data.len() >= self.max_size && !data.contains_key(&key) {
            if let Some(oldest_key) = self.find_oldest_key(&data) {
                data.remove(&oldest_key);
            }
        }

        data.insert(key, CacheEntry::new(value, ttl));
    }

    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut data = self.data.write().await;
        data.remove(key).map(|entry| entry.value)
    }

    pub async fn contains_key(&self, key: &K) -> bool {
        let data = self.data.read().await;
        if let Some(entry) = data.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    pub async fn len(&self) -> usize {
        let data = self.data.read().await;
        data.len()
    }

    pub async fn clear(&self) {
        let mut data = self.data.write().await;
        data.clear();
    }

    pub async fn cleanup_expired(&self) {
        let mut data = self.data.write().await;
        data.retain(|_, entry| !entry.is_expired());
    }

    fn find_oldest_key(&self, data: &HashMap<K, CacheEntry<V>>) -> Option<K> {
        data.iter()
            .min_by_key(|(_, entry)| entry.inserted_at)
            .map(|(key, _)| key.clone())
    }

    pub async fn stats(&self) -> CacheStats {
        let data = self.data.read().await;
        let total_entries = data.len();
        let expired_entries = data.values().filter(|entry| entry.is_expired()).count();

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            max_size: self.max_size,
            hit_ratio: 0.0, // TODO: Track hits/misses for accurate ratio
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub max_size: usize,
    pub hit_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = MemoryCache::new(3, Duration::from_secs(60));

        // Test insert and get
        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        // Test contains_key
        assert!(cache.contains_key(&"key1".to_string()).await);
        assert!(!cache.contains_key(&"key2".to_string()).await);

        // Test remove
        assert_eq!(cache.remove(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = MemoryCache::new(10, Duration::from_millis(100));

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let cache = MemoryCache::new(2, Duration::from_secs(60));

        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;
        cache.insert("key3".to_string(), "value3".to_string()).await;

        // Should have evicted the oldest entry
        assert_eq!(cache.len().await, 2);
    }
}