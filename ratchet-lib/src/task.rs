use anyhow::Result;
use lazy_static::lazy_static;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

// Define a global LRU cache for file contents
// Cache size is 100 entries - adjust based on expected number of tasks
lazy_static! {
    static ref CONTENT_CACHE: Mutex<LruCache<String, Arc<String>>> = {
        let cache_size = NonZeroUsize::new(100).unwrap();
        Mutex::new(LruCache::new(cache_size))
    };
}

/// Errors that can occur during task operations
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Invalid task structure: {0}")]
    InvalidTaskStructure(String),

    #[error("Task file not found: {0}")]
    TaskFileNotFound(String),
}

/// Type of task to be executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    JsTask {
        path: String, // Path to the JS file (for reference/debugging)
        #[serde(skip)] // Skip content during serialization
        content: Option<Arc<String>>, // Lazily loaded content
    },
}

/// Metadata for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub uuid: Uuid,
    pub version: String,
    pub label: String,
    pub description: String,
}

/// Representation of a complete task with all its components
#[derive(Debug, Clone)]
pub struct Task {
    pub metadata: TaskMetadata,
    pub task_type: TaskType,
    pub input_schema: JsonValue,
    pub output_schema: JsonValue,
    pub path: PathBuf,
}

impl Task {
    /// Load a task from the filesystem
    pub fn from_fs(path: impl AsRef<Path>) -> Result<Self, TaskError> {
        let path = path.as_ref();
        
        // Check if path exists and is a directory
        if !path.exists() {
            return Err(TaskError::TaskFileNotFound(path.to_string_lossy().to_string()));
        }
        
        if !path.is_dir() {
            return Err(TaskError::InvalidTaskStructure(format!(
                "Path {} is not a directory", path.to_string_lossy()
            )));
        }
        
        // Read metadata.json
        let metadata_path = path.join("metadata.json");
        if !metadata_path.exists() {
            return Err(TaskError::TaskFileNotFound(format!(
                "Metadata file not found at {}", metadata_path.to_string_lossy()
            )));
        }
        
        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata: TaskMetadata = serde_json::from_str(&metadata_json)?;
        
        // Read input schema
        let input_schema_path = path.join("input.schema.json");
        if !input_schema_path.exists() {
            return Err(TaskError::TaskFileNotFound(format!(
                "Input schema file not found at {}", input_schema_path.to_string_lossy()
            )));
        }
        
        let input_schema_json = fs::read_to_string(&input_schema_path)?;
        let input_schema: JsonValue = serde_json::from_str(&input_schema_json)?;
        
        // Read output schema
        let output_schema_path = path.join("output.schema.json");
        if !output_schema_path.exists() {
            return Err(TaskError::TaskFileNotFound(format!(
                "Output schema file not found at {}", output_schema_path.to_string_lossy()
            )));
        }
        
        let output_schema_json = fs::read_to_string(&output_schema_path)?;
        let output_schema: JsonValue = serde_json::from_str(&output_schema_json)?;
        
        // Check for JS file
        let js_file_path = path.join("main.js");
        if !js_file_path.exists() {
            return Err(TaskError::TaskFileNotFound(format!(
                "JavaScript file not found at {}", js_file_path.to_string_lossy()
            )));
        }
        
        // Create the task type (without loading content initially)
        let task_type = TaskType::JsTask {
            path: js_file_path.to_string_lossy().to_string(),
            content: None, // Content is loaded lazily
        };
        
        Ok(Task {
            metadata,
            task_type,
            input_schema,
            output_schema,
            path: path.to_path_buf(),
        })
    }
    
    /// Get the path to the JavaScript file for JS tasks
    pub fn js_file_path(&self) -> Option<PathBuf> {
        match &self.task_type {
            TaskType::JsTask { path, .. } => Some(PathBuf::from(path)),
        }
    }
    
    /// Get the UUID of the task
    pub fn uuid(&self) -> Uuid {
        self.metadata.uuid
    }
    
    /// Ensure the JavaScript content is loaded in memory
    pub fn ensure_content_loaded(&mut self) -> Result<(), TaskError> {
        match &mut self.task_type {
            TaskType::JsTask { path, content } => {
                if content.is_none() {
                    // Make a clone of the path for use in file operations
                    let path_str = path.clone();
                    
                    // Try to get content from cache first
                    let mut cache = CONTENT_CACHE.lock().unwrap();
                    
                    if let Some(cached_content) = cache.get(&path_str) {
                        // Content found in cache, use it
                        *content = Some(cached_content.clone());
                    } else {
                        // Content not in cache, load from filesystem
                        let file_content = fs::read_to_string(&path_str)?;
                        let arc_content = Arc::new(file_content);
                        
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
    
    /// Get the JavaScript content if loaded, or load it if not
    pub fn get_js_content(&mut self) -> Result<Arc<String>, TaskError> {
        self.ensure_content_loaded()?;
        
        match &self.task_type {
            TaskType::JsTask { content, .. } => {
                content.clone().ok_or_else(|| 
                    TaskError::InvalidTaskStructure("Failed to load JavaScript content".to_string())
                )
            }
        }
    }
    
    /// Pre-load the JavaScript content
    pub fn preload(&mut self) -> Result<(), TaskError> {
        self.ensure_content_loaded()
    }
    
    /// Purge content from memory to save space
    pub fn purge_content(&mut self) {
        match &mut self.task_type {
            TaskType::JsTask { content, .. } => {
                *content = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_task_files() -> Result<PathBuf, std::io::Error> {
        let temp_dir = tempdir()?;
        let task_dir = temp_dir.path().to_path_buf();
        
        // Create metadata.json
        let metadata = r#"{
            "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
            "version": "1.0.0",
            "label": "Addition Task",
            "description": "This is a sample task that adds two numbers together."
        }"#;
        fs::write(task_dir.join("metadata.json"), metadata)?;
        
        // Create input.schema.json
        let input_schema = r#"{
            "type": "object",
            "properties": {
                "num1": { "type": "number" },
                "num2": { "type": "number" }
            },
            "required": ["num1", "num2"]
        }"#;
        fs::write(task_dir.join("input.schema.json"), input_schema)?;
        
        // Create output.schema.json
        let output_schema = r#"{
            "type": "object",
            "properties": {
                "sum": { "type": "number" }
            },
            "required": ["sum"]
        }"#;
        fs::write(task_dir.join("output.schema.json"), output_schema)?;
        
        // Create main.js
        let main_js = r#"function(input) {
            const {num1, num2} = input;
            
            if (typeof num1 !== 'number' || typeof num2 !== 'number') {
              throw new Error('num1 and num2 must be numbers');
            }
            
            return {
              sum: num1 + num2
            }
        }"#;
        fs::write(task_dir.join("main.js"), main_js)?;
        
        // Prevent temp_dir from being dropped so the files remain
        std::mem::forget(temp_dir);
        
        Ok(task_dir)
    }
    
    #[test]
    fn test_load_from_sample() {
        // Test with the sample task in the project
        let sample_path = Path::new("/home/michiel/dev/ratchet/sample/js-tasks/addition");
        if sample_path.exists() {
            let mut task = Task::from_fs(sample_path).unwrap();
            
            assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
            assert_eq!(task.metadata.version, "1.0.0");
            assert_eq!(task.metadata.label, "Addition Task");
            
            match &task.task_type {
                TaskType::JsTask { path, content } => {
                    assert!(path.contains("main.js"));
                    assert!(content.is_none()); // Content should not be loaded initially
                }
            }
            
            // Test content loading
            task.ensure_content_loaded().unwrap();
            let content = task.get_js_content().unwrap();
            assert!(!content.is_empty());
            
            // Check schema properties
            assert!(task.input_schema.get("properties").is_some());
            assert!(task.output_schema.get("properties").is_some());
        }
    }
    
    #[test]
    fn test_from_fs() {
        // Create test files
        let test_dir = create_test_task_files().unwrap();
        
        // Load the task
        let mut task = Task::from_fs(&test_dir).unwrap();
        
        // Verify the task properties
        assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
        assert_eq!(task.metadata.version, "1.0.0");
        assert_eq!(task.metadata.label, "Addition Task");
        assert_eq!(task.metadata.description, "This is a sample task that adds two numbers together.");
        
        match &task.task_type {
            TaskType::JsTask { path, content } => {
                assert!(path.contains("main.js"));
                assert!(content.is_none()); // Content should not be loaded initially
            }
        }
        
        // Test loading content
        task.ensure_content_loaded().unwrap();
        
        match &task.task_type {
            TaskType::JsTask { content, .. } => {
                assert!(content.is_some()); // Content should be loaded now
                let js_content = content.as_ref().unwrap();
                assert!(js_content.contains("function")); // Check content contains expected text
            }
        }
        
        // Test get_js_content
        let js_content = task.get_js_content().unwrap();
        assert!(js_content.contains("function"));
        
        // Test purge_content
        task.purge_content();
        
        match &task.task_type {
            TaskType::JsTask { content, .. } => {
                assert!(content.is_none()); // Content should be purged
            }
        }
        
        // Check schema properties
        let input_props = task.input_schema.get("properties").unwrap();
        assert!(input_props.get("num1").is_some());
        assert!(input_props.get("num2").is_some());
        
        let output_props = task.output_schema.get("properties").unwrap();
        assert!(output_props.get("sum").is_some());
    }
    
    #[test]
    fn test_missing_files() {
        let temp_dir = tempdir().unwrap();
        let result = Task::from_fs(temp_dir.path());
        assert!(result.is_err());
        
        // Create just metadata.json
        let metadata = r#"{
            "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
            "version": "1.0.0",
            "label": "Addition Task",
            "description": "This is a sample task that adds two numbers together."
        }"#;
        fs::write(temp_dir.path().join("metadata.json"), metadata).unwrap();
        
        // Should still fail due to missing input schema
        let result = Task::from_fs(temp_dir.path());
        assert!(result.is_err());
    }
}