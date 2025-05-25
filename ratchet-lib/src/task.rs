use anyhow::Result;
use lazy_static::lazy_static;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::io::{self};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;
use zip::ZipArchive;

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
    
    #[error("Invalid JSON schema: {0}")]
    InvalidJsonSchema(String),
    
    #[error("JavaScript parse error: {0}")]
    JavaScriptParseError(String),
    
    #[error("ZIP error: {0}")]
    ZipError(#[from] zip::result::ZipError),
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
    /// Temporary directory used for extracted ZIP files
    #[doc(hidden)]
    pub(crate) _temp_dir: Option<Arc<TempDir>>,
}

impl Task {
    /// Load a task from the filesystem or a ZIP file
    pub fn from_fs(path: impl AsRef<Path>) -> Result<Self, TaskError> {
        let path = path.as_ref();
        
        debug!("Loading task from path: {:?}", path);
        
        // Check if path exists
        if !path.exists() {
            warn!("Task path does not exist: {:?}", path);
            return Err(TaskError::TaskFileNotFound(path.to_string_lossy().to_string()));
        }
        
        // Check if the path is a file (potentially a ZIP) or a directory
        if path.is_file() {
            // Check if it's a ZIP file
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extension.to_lowercase() == "zip" {
                debug!("Loading task from ZIP file: {:?}", path);
                return Self::from_zip(path);
            } else {
                warn!("File is not a ZIP file: {:?}", path);
                return Err(TaskError::InvalidTaskStructure(format!(
                    "File {} is not a ZIP file", path.to_string_lossy()
                )));
            }
        } else if !path.is_dir() {
            warn!("Path is neither a directory nor a ZIP file: {:?}", path);
            return Err(TaskError::InvalidTaskStructure(format!(
                "Path {} is neither a directory nor a ZIP file", path.to_string_lossy()
            )));
        }
        
        // Path is a directory, process it directly
        debug!("Loading task from directory: {:?}", path);
        Self::from_directory(path)
    }
    
    /// Load a task from a directory
    fn from_directory(path: &Path) -> Result<Self, TaskError> {
        debug!("Loading task from directory: {:?}", path);
        
        // Read metadata.json
        let metadata_path = path.join("metadata.json");
        if !metadata_path.exists() {
            warn!("Metadata file not found: {:?}", metadata_path);
            return Err(TaskError::TaskFileNotFound(format!(
                "Metadata file not found at {}", metadata_path.to_string_lossy()
            )));
        }
        
        let metadata_json = fs::read_to_string(&metadata_path)?;
        let metadata: TaskMetadata = serde_json::from_str(&metadata_json)?;
        
        debug!("Task metadata loaded: {} ({})", metadata.label, metadata.uuid);
        
        // Read input schema
        let input_schema_path = path.join("input.schema.json");
        if !input_schema_path.exists() {
            warn!("Input schema file not found: {:?}", input_schema_path);
            return Err(TaskError::TaskFileNotFound(format!(
                "Input schema file not found at {}", input_schema_path.to_string_lossy()
            )));
        }
        
        let input_schema_json = fs::read_to_string(&input_schema_path)?;
        let input_schema: JsonValue = serde_json::from_str(&input_schema_json)?;
        
        // Read output schema
        let output_schema_path = path.join("output.schema.json");
        if !output_schema_path.exists() {
            warn!("Output schema file not found: {:?}", output_schema_path);
            return Err(TaskError::TaskFileNotFound(format!(
                "Output schema file not found at {}", output_schema_path.to_string_lossy()
            )));
        }
        
        let output_schema_json = fs::read_to_string(&output_schema_path)?;
        let output_schema: JsonValue = serde_json::from_str(&output_schema_json)?;
        
        // Check for JS file
        let js_file_path = path.join("main.js");
        if !js_file_path.exists() {
            warn!("JavaScript file not found: {:?}", js_file_path);
            return Err(TaskError::TaskFileNotFound(format!(
                "JavaScript file not found at {}", js_file_path.to_string_lossy()
            )));
        }
        
        // Create the task type (without loading content initially)
        let task_type = TaskType::JsTask {
            path: js_file_path.to_string_lossy().to_string(),
            content: None, // Content is loaded lazily
        };
        
        info!("Successfully loaded task: {} ({})", metadata.label, metadata.uuid);
        
        Ok(Task {
            metadata,
            task_type,
            input_schema,
            output_schema,
            path: path.to_path_buf(),
            _temp_dir: None,
        })
    }
    
    /// Load a task from a ZIP file
    fn from_zip(zip_path: &Path) -> Result<Self, TaskError> {
        debug!("Loading task from ZIP file: {:?}", zip_path);
        
        // Create a temporary directory to extract the ZIP
        let temp_dir = TempDir::new()?;
        let temp_dir_arc = Arc::new(temp_dir);
        let extract_path = temp_dir_arc.path();
        
        debug!("Created temporary directory: {:?}", extract_path);
        
        // Open the ZIP file
        let zip_file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(zip_file)?;
        
        // Extract all files from the ZIP to the temporary directory
        debug!("Extracting {} files from ZIP", archive.len());
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => {
                    warn!("Skipping file with unsafe name at index {}", i);
                    continue; // Skip files with unsafe names
                }
            };
            
            let output_path = extract_path.join(&file_path);
            
            // Create directory structure if needed
            if file.is_dir() {
                fs::create_dir_all(&output_path)?;
            } else {
                if let Some(parent) = output_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                let mut output_file = fs::File::create(&output_path)?;
                io::copy(&mut file, &mut output_file)?;
            }
        }
        
        // Determine the root directory of the task within the extracted ZIP
        // We look for a directory that contains metadata.json
        let root_dir = if extract_path.join("metadata.json").exists() {
            extract_path.to_path_buf()
        } else {
            // Try to find a subdirectory with metadata.json
            let entries = fs::read_dir(extract_path)?;
            let mut task_dir = None;
            
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() && path.join("metadata.json").exists() {
                    task_dir = Some(path);
                    break;
                }
            }
            
            task_dir.ok_or_else(|| TaskError::TaskFileNotFound(format!(
                "Could not find metadata.json in ZIP file {}", zip_path.to_string_lossy()
            )))?
        };
        
        // Now load the task from the extracted directory
        debug!("Loading task from extracted directory: {:?}", root_dir);
        let mut task = Self::from_directory(&root_dir)?;
        
        // Store the temp_dir in the task to keep it alive as long as the task exists
        task._temp_dir = Some(temp_dir_arc);
        
        info!("Successfully loaded task from ZIP: {} ({})", task.metadata.label, task.metadata.uuid);
        
        Ok(task)
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
                        let file_content = fs::read_to_string(&path_str)?;
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
            TaskType::JsTask { path, content } => {
                if content.is_some() {
                    debug!("Purging JavaScript content from memory for: {}", path);
                }
                *content = None;
            }
        }
    }
    
    /// Validate that the task is properly structured and syntactically correct
    pub fn validate(&mut self) -> Result<(), TaskError> {
        debug!("Validating task: {} ({})", self.metadata.label, self.metadata.uuid);
        
        // 1. Validate input schema is valid JSON Schema
        debug!("Validating input schema");
        if !self.input_schema.is_object() {
            warn!("Input schema is not a valid JSON object");
            return Err(TaskError::InvalidJsonSchema(
                "Input schema must be a valid JSON object".to_string()
            ));
        }
        
        // 2. Validate output schema is valid JSON Schema
        debug!("Validating output schema");
        if !self.output_schema.is_object() {
            warn!("Output schema is not a valid JSON object");
            return Err(TaskError::InvalidJsonSchema(
                "Output schema must be a valid JSON object".to_string()
            ));
        }
        
        // 3. Validate that the JavaScript code can be parsed
        debug!("Validating JavaScript content");
        self.ensure_content_loaded()?;
        let js_content = self.get_js_content()?;
        
        // We'll use a basic heuristic first - check if it contains a function definition
        if !js_content.contains("function") {
            warn!("JavaScript code does not contain a function definition");
            return Err(TaskError::JavaScriptParseError(
                "JavaScript code does not contain a function definition".to_string()
            ));
        }
        
        // 4. Try to parse the JavaScript code using BoaJS
        // This will catch syntax errors in the JavaScript code
        debug!("Parsing JavaScript with BoaJS engine");
        let mut context = boa_engine::Context::default();
        let result = context.eval(boa_engine::Source::from_bytes(js_content.as_ref()));
        if result.is_err() {
            let error = result.err().unwrap();
            warn!("JavaScript syntax error: {}", error);
            return Err(TaskError::JavaScriptParseError(
                format!("JavaScript syntax error: {}", error)
            ));
        }
        
        // 5. Validate that the code returns a function or is a callable object
        let js_result = result.unwrap();
        if !js_result.is_callable() && !js_result.is_object() {
            warn!("JavaScript code does not return a callable function or object");
            return Err(TaskError::JavaScriptParseError(
                "JavaScript code must return a callable function or object".to_string()
            ));
        }
        
        // All validations passed
        info!("Task validation completed successfully: {} ({})", self.metadata.label, self.metadata.uuid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;
    
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
    
    // Creates a ZIP file with the specified task files
    fn create_zip_task() -> Result<(PathBuf, TempDir), std::io::Error> {
        // Create a temporary directory to store the ZIP file
        let temp_dir = tempdir()?;
        
        // Create task files in a subdirectory
        let task_dir = temp_dir.path().join("task");
        fs::create_dir(&task_dir)?;
        
        // Create metadata.json
        let metadata = r#"{
            "uuid": "bd6c6f98-4896-44cc-8c82-30328c3aefda",
            "version": "1.0.0",
            "label": "ZIP Task",
            "description": "This is a sample task in a ZIP file."
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
        let main_js = r#"(function(input) {
            const {num1, num2} = input;
            
            if (typeof num1 !== 'number' || typeof num2 !== 'number') {
              throw new Error('num1 and num2 must be numbers');
            }
            
            return {
              sum: num1 + num2
            }
        })"#;
        fs::write(task_dir.join("main.js"), main_js)?;
        
        // Create the ZIP file
        let zip_path = temp_dir.path().join("task.zip");
        let zip_file = fs::File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(zip_file);
        
        // Add files to the ZIP
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);
        
        // Add metadata.json
        zip.start_file("task/metadata.json", options)?;
        zip.write_all(metadata.as_bytes())?;
        
        // Add input.schema.json
        zip.start_file("task/input.schema.json", options)?;
        zip.write_all(input_schema.as_bytes())?;
        
        // Add output.schema.json
        zip.start_file("task/output.schema.json", options)?;
        zip.write_all(output_schema.as_bytes())?;
        
        // Add main.js
        zip.start_file("task/main.js", options)?;
        zip.write_all(main_js.as_bytes())?;
        
        // Finalize the ZIP file
        zip.finish()?;
        
        Ok((zip_path, temp_dir))
    }
    
    #[test]
    fn test_from_zip() {
        // Create a ZIP file with a task
        let (zip_path, _temp_dir) = create_zip_task().unwrap();
        
        // Load the task from the ZIP file
        let mut task = Task::from_fs(&zip_path).unwrap();
        
        // Verify the task properties
        assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
        assert_eq!(task.metadata.version, "1.0.0");
        assert_eq!(task.metadata.label, "ZIP Task");
        assert_eq!(task.metadata.description, "This is a sample task in a ZIP file.");
        
        // The temporary directory should be stored in the task
        assert!(task._temp_dir.is_some());
        
        // Test loading content
        task.ensure_content_loaded().unwrap();
        let content = task.get_js_content().unwrap();
        assert!(content.contains("function"));
        
        // Check schema properties
        let input_props = task.input_schema.get("properties").unwrap();
        assert!(input_props.get("num1").is_some());
        assert!(input_props.get("num2").is_some());
        
        let output_props = task.output_schema.get("properties").unwrap();
        assert!(output_props.get("sum").is_some());
    }
    
    #[test]
    fn test_sample_zip() {
        // Test with the sample ZIP task in the project
        let sample_path = Path::new("/home/michiel/dev/ratchet/sample/js-tasks/addition.zip");
        if sample_path.exists() {
            let mut task = Task::from_fs(sample_path).unwrap();
            
            assert_eq!(task.metadata.uuid.to_string(), "bd6c6f98-4896-44cc-8c82-30328c3aefda");
            assert_eq!(task.metadata.version, "1.0.0");
            assert_eq!(task.metadata.label, "Addition Task");
            
            // The temporary directory should be stored in the task
            assert!(task._temp_dir.is_some());
            
            // Test content loading
            task.ensure_content_loaded().unwrap();
            let content = task.get_js_content().unwrap();
            assert!(!content.is_empty());
            
            // Check schema properties
            assert!(task.input_schema.get("properties").is_some());
            assert!(task.output_schema.get("properties").is_some());
        }
    }
}