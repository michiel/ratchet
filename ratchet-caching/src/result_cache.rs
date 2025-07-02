//! Task execution result cache implementation

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::{
    cache::Cache,
    config::ResultCacheConfig,
    stores::{MokaCache, TtlCache},
    CacheError, CacheResult, CacheStats,
};

/// Result cache key
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultCacheKey {
    /// Task ID
    pub task_id: String,

    /// Task version
    pub task_version: String,

    /// Input hash (deterministic hash of input data)
    pub input_hash: String,
}

impl ResultCacheKey {
    /// Create a new result cache key
    pub fn new(task_id: impl Into<String>, task_version: impl Into<String>, input_data: &serde_json::Value) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create deterministic hash of input
        let mut hasher = DefaultHasher::new();
        // Hash the JSON value directly without string conversion for better performance
        input_data.hash(&mut hasher);
        let input_hash = format!("{:x}", hasher.finish());

        Self {
            task_id: task_id.into(),
            task_version: task_version.into(),
            input_hash,
        }
    }
}

/// Cached execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    /// Execution ID that produced this result
    pub execution_id: uuid::Uuid,

    /// Task output
    pub output: serde_json::Value,

    /// Whether execution was successful
    pub success: bool,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// When the result was cached
    pub cached_at: chrono::DateTime<chrono::Utc>,

    /// Result size in bytes
    pub size_bytes: usize,
}

impl CachedResult {
    /// Create a successful result
    pub fn success(execution_id: uuid::Uuid, output: serde_json::Value, duration_ms: u64) -> Self {
        let size_bytes = output.to_string().len();

        Self {
            execution_id,
            output,
            success: true,
            error_message: None,
            duration_ms,
            cached_at: chrono::Utc::now(),
            size_bytes,
        }
    }

    /// Create a failed result
    pub fn failure(execution_id: uuid::Uuid, error_message: String, duration_ms: u64) -> Self {
        let size_bytes = error_message.len();
        Self {
            execution_id,
            output: serde_json::Value::Null,
            success: false,
            error_message: Some(error_message),
            duration_ms,
            cached_at: chrono::Utc::now(),
            size_bytes,
        }
    }
}

/// Result cache implementation
pub struct ResultCache {
    /// Inner cache
    inner: ResultCacheImpl,

    /// Configuration
    config: ResultCacheConfig,
}

/// Inner cache implementation
enum ResultCacheImpl {
    Ttl(TtlCache<ResultCacheKey, Arc<CachedResult>>),
    Moka(MokaCache<ResultCacheKey, Arc<CachedResult>>),
}

impl ResultCache {
    /// Create a new result cache from configuration
    pub fn from_config(config: ResultCacheConfig) -> Self {
        let inner = match config.cache_type {
            crate::config::CacheType::Moka => {
                let mut builder = MokaCache::builder();
                builder = builder
                    .max_capacity(config.max_entries as u64)
                    .time_to_live(Duration::from_secs(config.ttl_seconds))
                    .weigher(|_k, v: &Arc<CachedResult>| {
                        (v.size_bytes / 1024) as u32 // KB
                    });

                ResultCacheImpl::Moka(builder.build())
            }
            _ => {
                // Default to TTL cache
                let ttl = Duration::from_secs(config.ttl_seconds);
                ResultCacheImpl::Ttl(TtlCache::new(ttl))
            }
        };

        Self { inner, config }
    }

    /// Get a cached result
    pub async fn get(&self, key: &ResultCacheKey) -> CacheResult<Option<Arc<CachedResult>>> {
        if !self.config.enabled {
            return Ok(None);
        }

        match &self.inner {
            ResultCacheImpl::Ttl(cache) => cache.get(key).await,
            ResultCacheImpl::Moka(cache) => cache.get(key).await,
        }
    }

    /// Cache a result
    pub async fn put(&self, key: ResultCacheKey, result: CachedResult) -> CacheResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check if we should cache this result
        if self.config.cache_only_success && !result.success {
            return Ok(());
        }

        // Check size limit
        if result.size_bytes > self.config.max_result_size {
            return Err(CacheError::CapacityExceeded(format!(
                "Result size {} exceeds limit",
                result.size_bytes
            )));
        }

        let result = Arc::new(result);

        match &self.inner {
            ResultCacheImpl::Ttl(cache) => cache.put(key, result).await,
            ResultCacheImpl::Moka(cache) => cache.put(key, result).await,
        }
    }

    /// Remove a cached result
    pub async fn remove(&self, key: &ResultCacheKey) -> CacheResult<Option<Arc<CachedResult>>> {
        match &self.inner {
            ResultCacheImpl::Ttl(cache) => cache.remove(key).await,
            ResultCacheImpl::Moka(cache) => cache.remove(key).await,
        }
    }

    /// Clear all cached results
    pub async fn clear(&self) -> CacheResult<()> {
        match &self.inner {
            ResultCacheImpl::Ttl(cache) => cache.clear().await,
            ResultCacheImpl::Moka(cache) => cache.clear().await,
        }
    }

    /// Clear results for a specific task
    pub async fn clear_task(&self, _task_id: &str) -> CacheResult<usize> {
        // This is a simplified implementation
        // In production, you'd maintain an index of keys by task_id
        self.clear().await?;
        Ok(0)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheResult<CacheStats> {
        match &self.inner {
            ResultCacheImpl::Ttl(cache) => cache.stats().await,
            ResultCacheImpl::Moka(cache) => cache.stats().await,
        }
    }
}

/// Check if a task is deterministic (cacheable)
pub fn is_task_deterministic(task_metadata: &serde_json::Value) -> bool {
    // Check for deterministic flag in metadata
    if let Some(deterministic) = task_metadata.get("deterministic") {
        return deterministic.as_bool().unwrap_or(false);
    }

    // Check for side effects
    if let Some(side_effects) = task_metadata.get("side_effects") {
        if let Some(effects) = side_effects.as_array() {
            return effects.is_empty();
        }
    }

    // Default to not cacheable
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_result_cache() {
        let config = ResultCacheConfig {
            enabled: true,
            max_entries: 100,
            max_result_size: 1024 * 1024,
            ttl_seconds: 3600,
            cache_only_success: true,
            cache_type: crate::config::CacheType::Ttl,
        };

        let cache = ResultCache::from_config(config);

        let input = serde_json::json!({
            "x": 10,
            "y": 20
        });

        let key = ResultCacheKey::new("task1", "1.0.0", &input);

        let result = CachedResult::success(uuid::Uuid::new_v4(), serde_json::json!({"sum": 30}), 100);

        // Put and get
        cache.put(key.clone(), result).await.unwrap();
        let cached = cache.get(&key).await.unwrap();

        assert!(cached.is_some());
        assert!(cached.unwrap().success);
    }

    #[tokio::test]
    async fn test_cache_only_success() {
        let config = ResultCacheConfig {
            enabled: true,
            max_entries: 100,
            max_result_size: 1024 * 1024,
            ttl_seconds: 3600,
            cache_only_success: true, // Only cache successful results
            cache_type: crate::config::CacheType::Ttl,
        };

        let cache = ResultCache::from_config(config);

        let input = serde_json::json!({"test": true});
        let key = ResultCacheKey::new("task1", "1.0.0", &input);

        let failed_result = CachedResult::failure(uuid::Uuid::new_v4(), "Task failed".to_string(), 50);

        // Put failed result
        cache.put(key.clone(), failed_result).await.unwrap();

        // Should not be cached
        let cached = cache.get(&key).await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_deterministic_check() {
        let deterministic_task = serde_json::json!({
            "deterministic": true,
            "side_effects": []
        });
        assert!(is_task_deterministic(&deterministic_task));

        let non_deterministic_task = serde_json::json!({
            "deterministic": false
        });
        assert!(!is_task_deterministic(&non_deterministic_task));

        let task_with_side_effects = serde_json::json!({
            "side_effects": ["http", "filesystem"]
        });
        assert!(!is_task_deterministic(&task_with_side_effects));
    }
}
