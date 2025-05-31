use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use crate::task::Task;

/// LRU cache node for doubly linked list
#[derive(Debug)]
struct CacheNode {
    task: Task,
    prev: Option<String>,
    next: Option<String>,
}

/// LRU cache for tasks with memory-aware eviction
pub struct TaskCache {
    /// Maximum number of entries
    max_entries: usize,
    /// Maximum memory usage in bytes (estimated)
    max_memory_bytes: usize,
    /// Current memory usage estimate
    current_memory_bytes: usize,
    /// Cache storage
    cache: HashMap<String, CacheNode>,
    /// Head of LRU list (most recently used)
    head: Option<String>,
    /// Tail of LRU list (least recently used)
    tail: Option<String>,
    /// Cache statistics
    stats: CacheStats,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub memory_bytes: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl TaskCache {
    /// Create a new task cache
    pub fn new(max_entries: usize, max_memory_mb: usize) -> Self {
        Self {
            max_entries,
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            current_memory_bytes: 0,
            cache: HashMap::new(),
            head: None,
            tail: None,
            stats: CacheStats::default(),
        }
    }

    /// Get a task from cache
    pub fn get(&mut self, key: &str) -> Option<Task> {
        if let Some(node) = self.cache.get(key) {
            let task = node.task.clone();
            
            // Move to head (mark as most recently used)
            self.move_to_head(key);
            
            self.stats.hits += 1;
            debug!("Cache hit for task: {}", key);
            Some(task)
        } else {
            self.stats.misses += 1;
            debug!("Cache miss for task: {}", key);
            None
        }
    }

    /// Put a task into cache
    pub fn put(&mut self, key: String, task: Task) {
        let task_size = self.estimate_task_size(&task);

        // Check if key already exists
        if self.cache.contains_key(&key) {
            // Update existing entry
            self.update_existing(&key, task, task_size);
        } else {
            // Add new entry
            self.add_new(key, task, task_size);
        }

        // Ensure cache constraints are met
        self.enforce_constraints();
        
        self.update_stats();
    }

    /// Remove a task from cache
    pub fn remove(&mut self, key: &str) -> Option<Task> {
        if let Some(node) = self.cache.remove(key) {
            let task = node.task.clone();
            let task_size = self.estimate_task_size(&task);
            
            // Update linked list
            self.remove_from_list(key);
            
            // Update memory usage
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(task_size);
            
            self.update_stats();
            Some(task)
        } else {
            None
        }
    }

    /// Clear all cached tasks
    pub fn clear(&mut self) {
        self.cache.clear();
        self.head = None;
        self.tail = None;
        self.current_memory_bytes = 0;
        self.stats = CacheStats::default();
        info!("Task cache cleared");
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Check if cache contains a key
    pub fn contains_key(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    fn update_existing(&mut self, key: &str, task: Task, new_size: usize) {
        let old_size = if let Some(node) = self.cache.get(key) {
            self.estimate_task_size(&node.task)
        } else {
            return;
        };
        
        if let Some(node) = self.cache.get_mut(key) {
            // Update task
            node.task = task;
            
            // Update memory usage
            self.current_memory_bytes = self.current_memory_bytes
                .saturating_sub(old_size)
                .saturating_add(new_size);
            
            // Move to head
            self.move_to_head(key);
        }
    }

    fn add_new(&mut self, key: String, task: Task, task_size: usize) {
        let node = CacheNode {
            task,
            prev: None,
            next: self.head.clone(),
        };

        // Insert into cache
        self.cache.insert(key.clone(), node);
        
        // Update linked list
        if let Some(old_head) = &self.head {
            if let Some(old_head_node) = self.cache.get_mut(old_head) {
                old_head_node.prev = Some(key.clone());
            }
        }
        
        self.head = Some(key.clone());
        
        if self.tail.is_none() {
            self.tail = Some(key);
        }

        // Update memory usage
        self.current_memory_bytes += task_size;
    }

    fn move_to_head(&mut self, key: &str) {
        if self.head.as_ref() == Some(&key.to_string()) {
            return; // Already at head
        }

        // Remove from current position
        self.remove_from_list(key);
        
        // Add to head
        if let Some(node) = self.cache.get_mut(key) {
            node.prev = None;
            node.next = self.head.clone();
        }

        if let Some(old_head) = &self.head {
            if let Some(old_head_node) = self.cache.get_mut(old_head) {
                old_head_node.prev = Some(key.to_string());
            }
        }

        self.head = Some(key.to_string());
        
        if self.tail.is_none() {
            self.tail = Some(key.to_string());
        }
    }

    fn remove_from_list(&mut self, key: &str) {
        let (prev, next) = if let Some(node) = self.cache.get(key) {
            (node.prev.clone(), node.next.clone())
        } else {
            return;
        };

        // Update previous node
        if let Some(prev_key) = &prev {
            if let Some(prev_node) = self.cache.get_mut(prev_key) {
                prev_node.next = next.clone();
            }
        } else {
            // This was the head
            self.head = next.clone();
        }

        // Update next node
        if let Some(next_key) = &next {
            if let Some(next_node) = self.cache.get_mut(next_key) {
                next_node.prev = prev.clone();
            }
        } else {
            // This was the tail
            self.tail = prev;
        }
    }

    fn enforce_constraints(&mut self) {
        // Evict entries if we exceed limits
        while self.should_evict() {
            if let Some(tail_key) = self.tail.clone() {
                self.evict_entry(&tail_key);
            } else {
                break;
            }
        }
    }

    fn should_evict(&self) -> bool {
        self.cache.len() > self.max_entries || 
        self.current_memory_bytes > self.max_memory_bytes
    }

    fn evict_entry(&mut self, key: &str) {
        if let Some(node) = self.cache.remove(key) {
            let task_size = self.estimate_task_size(&node.task);
            
            // Remove from linked list
            self.remove_from_list(key);
            
            // Update memory usage
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(task_size);
            
            self.stats.evictions += 1;
            
            debug!("Evicted task from cache: {} (size: {} bytes)", key, task_size);
        }
    }

    fn estimate_task_size(&self, task: &Task) -> usize {
        // Rough estimation of task memory footprint
        let base_size = std::mem::size_of::<Task>();
        let label_size = task.metadata.label.len();
        let description_size = task.metadata.description.len();
        let version_size = task.metadata.version.len();
        let path_size = task.path.to_string_lossy().len();
        
        // Estimate JSON schema sizes
        let input_schema_size = task.input_schema.to_string().len();
        let output_schema_size = task.output_schema.to_string().len();
        
        // Estimate task type size
        let task_type_size = match &task.task_type {
            crate::task::TaskType::JsTask { path, content } => {
                path.len() + content.as_ref().map_or(0, |c| c.len())
            }
        };

        base_size + 
        label_size + 
        description_size + 
        version_size + 
        path_size + 
        input_schema_size + 
        output_schema_size + 
        task_type_size
    }

    fn update_stats(&mut self) {
        self.stats.entries = self.cache.len();
        self.stats.memory_bytes = self.current_memory_bytes;
    }
}

/// Thread-safe wrapper around TaskCache
pub struct SharedTaskCache {
    cache: Arc<RwLock<TaskCache>>,
}

impl SharedTaskCache {
    pub fn new(max_entries: usize, max_memory_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(TaskCache::new(max_entries, max_memory_mb))),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Task> {
        let mut cache = self.cache.write().await;
        cache.get(key)
    }

    pub async fn put(&self, key: String, task: Task) {
        let mut cache = self.cache.write().await;
        cache.put(key, task);
    }

    pub async fn remove(&self, key: &str) -> Option<Task> {
        let mut cache = self.cache.write().await;
        cache.remove(key)
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        cache.stats()
    }

    pub async fn contains_key(&self, key: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(key)
    }

    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Clone for SharedTaskCache {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;
    use uuid::Uuid;

    fn create_test_task(name: &str) -> Task {
        use std::path::PathBuf;
        use crate::task::{TaskMetadata, TaskType};
        
        Task {
            metadata: TaskMetadata {
                uuid: Uuid::new_v4(),
                version: "1.0.0".to_string(),
                label: name.to_string(),
                description: format!("Test task {}", name),
            },
            task_type: TaskType::JsTask {
                path: format!("/test/{}/main.js", name),
                content: None,
            },
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            path: PathBuf::from(format!("/test/{}", name)),
            _temp_dir: None,
        }
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = TaskCache::new(3, 10); // 3 entries, 10MB
        
        let task1 = create_test_task("task1");
        let task2 = create_test_task("task2");
        
        // Test put and get
        cache.put("key1".to_string(), task1.clone());
        cache.put("key2".to_string(), task2.clone());
        
        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key("key1"));
        assert!(cache.contains_key("key2"));
        
        let retrieved = cache.get("key1").unwrap();
        assert_eq!(retrieved.metadata.label, task1.metadata.label);
        
        // Test remove
        let removed = cache.remove("key1").unwrap();
        assert_eq!(removed.metadata.label, task1.metadata.label);
        assert!(!cache.contains_key("key1"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = TaskCache::new(2, 100); // Only 2 entries allowed
        
        let task1 = create_test_task("task1");
        let task2 = create_test_task("task2");
        let task3 = create_test_task("task3");
        
        cache.put("key1".to_string(), task1);
        cache.put("key2".to_string(), task2);
        cache.put("key3".to_string(), task3); // Should evict key1
        
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains_key("key1")); // Evicted
        assert!(cache.contains_key("key2"));
        assert!(cache.contains_key("key3"));
        
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_lru_ordering() {
        let mut cache = TaskCache::new(3, 100);
        
        let task1 = create_test_task("task1");
        let task2 = create_test_task("task2");
        let task3 = create_test_task("task3");
        let task4 = create_test_task("task4");
        
        cache.put("key1".to_string(), task1);
        cache.put("key2".to_string(), task2);
        cache.put("key3".to_string(), task3);
        
        // Access key1 to make it most recently used
        cache.get("key1");
        
        // Add key4, should evict key2 (least recently used)
        cache.put("key4".to_string(), task4);
        
        assert!(cache.contains_key("key1")); // Recently accessed
        assert!(!cache.contains_key("key2")); // Evicted
        assert!(cache.contains_key("key3"));
        assert!(cache.contains_key("key4"));
    }

    #[tokio::test]
    async fn test_shared_cache() {
        let cache = SharedTaskCache::new(2, 10);
        
        let task1 = create_test_task("task1");
        let task2 = create_test_task("task2");
        
        cache.put("key1".to_string(), task1.clone()).await;
        cache.put("key2".to_string(), task2.clone()).await;
        
        assert_eq!(cache.len().await, 2);
        
        let retrieved = cache.get("key1").await.unwrap();
        assert_eq!(retrieved.metadata.label, task1.metadata.label);
        
        let stats = cache.stats().await;
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }
}