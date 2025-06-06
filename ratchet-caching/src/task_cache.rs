//! Task-specific cache implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::{Cache, CacheWarmer},
    config::TaskCacheConfig,
    stores::{InMemoryCache, LruCache, MokaCache, TtlCache},
    CacheError, CacheResult, CacheStats,
};

/// Task metadata for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTask {
    /// Task UUID
    pub uuid: uuid::Uuid,
    
    /// Task name
    pub name: String,
    
    /// Task version
    pub version: String,
    
    /// Task metadata
    pub metadata: serde_json::Value,
    
    /// Input schema
    pub input_schema: serde_json::Value,
    
    /// Output schema
    pub output_schema: serde_json::Value,
    
    /// JavaScript content
    pub js_content: Option<String>,
    
    /// Estimated memory size
    pub memory_size: usize,
}

impl CachedTask {
    /// Estimate memory usage
    pub fn estimate_memory_size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        
        // Add string sizes
        size += self.name.capacity();
        size += self.version.capacity();
        
        // Add JSON sizes (rough estimate)
        size += self.metadata.to_string().len();
        size += self.input_schema.to_string().len();
        size += self.output_schema.to_string().len();
        
        // Add JS content size
        if let Some(ref content) = self.js_content {
            size += content.capacity();
        }
        
        size
    }
}

/// Task cache implementation
pub struct TaskCache {
    /// Inner cache implementation
    inner: TaskCacheImpl,
    
    /// Configuration
    config: TaskCacheConfig,
}

/// Inner cache implementation enum
enum TaskCacheImpl {
    InMemory(InMemoryCache<String, Arc<CachedTask>>),
    Lru(LruCache<String, Arc<CachedTask>>),
    Ttl(TtlCache<String, Arc<CachedTask>>),
    Moka(MokaCache<String, Arc<CachedTask>>),
}

impl TaskCache {
    /// Create a new task cache from configuration
    pub fn from_config(config: TaskCacheConfig) -> Self {
        let inner = match config.cache_type {
            crate::config::CacheType::InMemory => {
                TaskCacheImpl::InMemory(InMemoryCache::new())
            }
            crate::config::CacheType::Lru => {
                TaskCacheImpl::Lru(LruCache::new(config.max_entries))
            }
            crate::config::CacheType::Ttl => {
                let ttl = Duration::from_secs(config.ttl_seconds.unwrap_or(3600));
                TaskCacheImpl::Ttl(TtlCache::new(ttl))
            }
            crate::config::CacheType::Moka => {
                let mut builder = MokaCache::builder();
                builder = builder.max_capacity(config.max_entries as u64);
                
                if let Some(ttl_secs) = config.ttl_seconds {
                    builder = builder.time_to_live(Duration::from_secs(ttl_secs));
                }
                
                // Custom weigher based on memory size
                builder = builder.weigher(|_k, v: &Arc<CachedTask>| {
                    (v.memory_size / 1024) as u32 // Convert to KB
                });
                
                TaskCacheImpl::Moka(builder.build())
            }
        };
        
        Self { inner, config }
    }
    
    /// Get a task by ID
    pub async fn get(&self, task_id: &str) -> CacheResult<Option<Arc<CachedTask>>> {
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => cache.get(&task_id.to_string()).await,
            TaskCacheImpl::Lru(cache) => cache.get(&task_id.to_string()).await,
            TaskCacheImpl::Ttl(cache) => cache.get(&task_id.to_string()).await,
            TaskCacheImpl::Moka(cache) => cache.get(&task_id.to_string()).await,
        }
    }
    
    /// Put a task into cache
    pub async fn put(&self, task_id: String, task: CachedTask) -> CacheResult<()> {
        // Check memory limit
        if let Some(max_memory_mb) = self.config.max_memory_mb.checked_mul(1024 * 1024) {
            if task.memory_size > max_memory_mb {
                return Err(CacheError::CapacityExceeded(
                    format!("Task {} exceeds memory limit", task_id)
                ));
            }
        }
        
        let task = Arc::new(task);
        
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => cache.put(task_id, task).await,
            TaskCacheImpl::Lru(cache) => cache.put(task_id, task).await,
            TaskCacheImpl::Ttl(cache) => cache.put(task_id, task).await,
            TaskCacheImpl::Moka(cache) => cache.put(task_id, task).await,
        }
    }
    
    /// Remove a task from cache
    pub async fn remove(&self, task_id: &str) -> CacheResult<Option<Arc<CachedTask>>> {
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => cache.remove(&task_id.to_string()).await,
            TaskCacheImpl::Lru(cache) => cache.remove(&task_id.to_string()).await,
            TaskCacheImpl::Ttl(cache) => cache.remove(&task_id.to_string()).await,
            TaskCacheImpl::Moka(cache) => cache.remove(&task_id.to_string()).await,
        }
    }
    
    /// Clear all tasks
    pub async fn clear(&self) -> CacheResult<()> {
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => cache.clear().await,
            TaskCacheImpl::Lru(cache) => cache.clear().await,
            TaskCacheImpl::Ttl(cache) => cache.clear().await,
            TaskCacheImpl::Moka(cache) => cache.clear().await,
        }
    }
    
    /// Get cache statistics
    pub async fn stats(&self) -> CacheResult<CacheStats> {
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => cache.stats().await,
            TaskCacheImpl::Lru(cache) => cache.stats().await,
            TaskCacheImpl::Ttl(cache) => cache.stats().await,
            TaskCacheImpl::Moka(cache) => cache.stats().await,
        }
    }
    
    /// Get current memory usage
    pub async fn memory_usage(&self) -> CacheResult<usize> {
        match &self.inner {
            TaskCacheImpl::InMemory(cache) => {
                let len = cache.len().await?;
                Ok(len * 1024 * 10) // Rough estimate
            }
            TaskCacheImpl::Lru(cache) => {
                // This is a simplified approach - in production you'd track this differently
                let len = cache.len().await?;
                let total = len * 1024 * 10; // Rough estimate
                Ok(total)
            }
            TaskCacheImpl::Ttl(cache) => {
                let len = cache.len().await?;
                Ok(len * 1024 * 10) // Rough estimate
            }
            TaskCacheImpl::Moka(cache) => {
                let stats = cache.stats().await?;
                Ok(stats.memory_usage_bytes.unwrap_or(0))
            }
        }
    }
}

/// Task cache warmer
pub struct TaskCacheWarmer {
    /// Task IDs to preload
    task_ids: Vec<String>,
}

impl TaskCacheWarmer {
    /// Create a new warmer with task IDs
    pub fn new(task_ids: Vec<String>) -> Self {
        Self { task_ids }
    }
}

#[async_trait]
impl CacheWarmer<String, Arc<CachedTask>> for TaskCacheWarmer {
    async fn warm(&self, cache: &impl Cache<String, Arc<CachedTask>>) -> CacheResult<usize> {
        let mut count = 0;
        
        // In a real implementation, this would load tasks from storage
        for task_id in &self.task_ids {
            // Simulate loading task
            let task = CachedTask {
                uuid: uuid::Uuid::new_v4(),
                name: format!("task_{}", task_id),
                version: "1.0.0".to_string(),
                metadata: serde_json::json!({}),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: serde_json::json!({"type": "object"}),
                js_content: Some("// Task content".to_string()),
                memory_size: 1024,
            };
            
            cache.put(task_id.clone(), Arc::new(task)).await?;
            count += 1;
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CacheType;
    
    #[tokio::test]
    async fn test_task_cache() {
        let config = TaskCacheConfig {
            enabled: true,
            max_entries: 10,
            max_memory_mb: 10,
            cache_type: CacheType::Lru,
            ttl_seconds: None,
        };
        
        let cache = TaskCache::from_config(config);
        
        let task = CachedTask {
            uuid: uuid::Uuid::new_v4(),
            name: "test_task".to_string(),
            version: "1.0.0".to_string(),
            metadata: serde_json::json!({"author": "test"}),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({"type": "object"}),
            js_content: Some("console.log('test');".to_string()),
            memory_size: 1024,
        };
        
        // Put and get
        cache.put("task1".to_string(), task.clone()).await.unwrap();
        let cached = cache.get("task1").await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "test_task");
        
        // Stats
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.entry_count, 1);
    }
    
    #[tokio::test]
    async fn test_memory_limit() {
        let config = TaskCacheConfig {
            enabled: true,
            max_entries: 10,
            max_memory_mb: 1, // 1MB limit
            cache_type: CacheType::Lru,
            ttl_seconds: None,
        };
        
        let cache = TaskCache::from_config(config);
        
        let large_task = CachedTask {
            uuid: uuid::Uuid::new_v4(),
            name: "large_task".to_string(),
            version: "1.0.0".to_string(),
            metadata: serde_json::json!({}),
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            js_content: Some("x".repeat(2 * 1024 * 1024)), // 2MB content
            memory_size: 2 * 1024 * 1024,
        };
        
        // Should fail due to memory limit
        let result = cache.put("large".to_string(), large_task).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CacheError::CapacityExceeded(_)));
    }
}