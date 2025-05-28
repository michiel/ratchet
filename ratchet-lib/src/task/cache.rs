use super::{TaskError, TaskType};
use lazy_static::lazy_static;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tracing::debug;

// Define a global LRU cache for file contents
// Cache size is 100 entries - adjust based on expected number of tasks
lazy_static! {
    pub static ref CONTENT_CACHE: Mutex<LruCache<String, Arc<String>>> = {
        let cache_size = NonZeroUsize::new(100).unwrap();
        Mutex::new(LruCache::new(cache_size))
    };
}

/// Ensure the JavaScript content is loaded in memory for the given task type
pub fn ensure_content_loaded(task_type: &mut TaskType) -> Result<(), TaskError> {
    match task_type {
        TaskType::JsTask { path, content } => {
            if content.is_none() {
                debug!("Loading JavaScript content for: {}", path);
                
                // Make a clone of the path for use in file operations
                let path_str = path.clone();
                
                // Try to get content from cache first
                let mut cache = CONTENT_CACHE.lock().unwrap();
                
                if let Some(cached_content) = cache.get(&path_str) {
                    debug!("JavaScript content found in cache for: {}", path);
                    // Content found in cache, use it
                    *content = Some(cached_content.clone());
                } else {
                    debug!("Loading JavaScript content from filesystem: {}", path);
                    // Content not in cache, load from filesystem
                    let file_content = std::fs::read_to_string(&path_str)?;
                    let arc_content = Arc::new(file_content);
                    
                    debug!("Storing JavaScript content in cache for: {}", path);
                    // Store in cache for future use
                    cache.put(path_str, arc_content.clone());
                    
                    // Update task with content
                    *content = Some(arc_content);
                }
            }
            
            Ok(())
        }
    }
}

/// Purge content from memory to save space
pub fn purge_content(task_type: &mut TaskType) {
    match task_type {
        TaskType::JsTask { path, content } => {
            if content.is_some() {
                debug!("Purging JavaScript content from memory for: {}", path);
            }
            *content = None;
        }
    }
}