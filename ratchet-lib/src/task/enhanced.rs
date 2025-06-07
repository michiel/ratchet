//! Enhanced task module that uses ratchet-core types with ratchet-lib implementation

// Re-export core types from ratchet-core
pub use ratchet_core::task::{TaskId, TaskMetadata as CoreTaskMetadata};

use ratchet_core::task::TaskSource;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

// Re-export errors and cache from current implementation
pub use super::{TaskError, CONTENT_CACHE};

/// Enhanced task metadata that extends core metadata with ratchet-lib specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Core metadata from ratchet-core
    #[serde(flatten)]
    pub core: CoreTaskMetadata,
    
    /// Legacy label field for backward compatibility
    pub label: String,
    
    /// Legacy UUID field for backward compatibility
    pub uuid: Uuid,
}

impl TaskMetadata {
    /// Create new metadata from core metadata
    pub fn from_core(core: CoreTaskMetadata) -> Self {
        Self {
            label: core.name.clone(),
            uuid: *core.id.as_uuid(),
            core,
        }
    }
    
    /// Create metadata with backward compatibility
    pub fn new(uuid: Uuid, version: String, label: String, description: String) -> Self {
        let mut core = CoreTaskMetadata::new(&label, &version)
            .with_description(description);
        core.id = TaskId(uuid);
            
        Self {
            label: label.clone(),
            uuid,
            core,
        }
    }
}

/// Type of task to be executed (enhanced with more sources)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    /// JavaScript task with lazy loading
    JsTask {
        path: String,
        #[serde(skip)]
        content: Option<Arc<String>>,
    },
    /// Task from ratchet-core source
    CoreTask {
        source: TaskSource,
    },
}

impl From<TaskSource> for TaskType {
    fn from(source: TaskSource) -> Self {
        TaskType::CoreTask { source }
    }
}

/// Enhanced task representation
#[derive(Debug, Clone)]
pub struct Task {
    /// Enhanced metadata
    pub metadata: TaskMetadata,
    
    /// Task type and source
    pub task_type: TaskType,
    
    /// Input JSON schema
    pub input_schema: JsonValue,
    
    /// Output JSON schema  
    pub output_schema: JsonValue,
    
    /// File system path
    pub path: PathBuf,
    
    /// Temporary directory for ZIP extraction
    #[doc(hidden)]
    pub(crate) _temp_dir: Option<Arc<TempDir>>,
    
    /// Cached content for performance
    pub content: Option<String>,
}

impl Task {
    /// Load a task from the filesystem or ZIP file
    pub fn from_fs(path: impl AsRef<std::path::Path>) -> Result<Self, TaskError> {
        super::loader::load_from_fs(path)
    }
    
    /// Get the task ID
    pub fn id(&self) -> TaskId {
        self.metadata.core.id
    }
    
    /// Get the UUID for backward compatibility
    pub fn uuid(&self) -> Uuid {
        self.metadata.uuid
    }
    
    /// Get the task name
    pub fn name(&self) -> &str {
        &self.metadata.core.name
    }
    
    /// Get the task version
    pub fn version(&self) -> &str {
        &self.metadata.core.version
    }
    
    /// Get the task description
    pub fn description(&self) -> Option<&str> {
        self.metadata.core.description.as_deref()
    }
    
    /// Get the path to the JavaScript file for JS tasks
    pub fn js_file_path(&self) -> Option<PathBuf> {
        match &self.task_type {
            TaskType::JsTask { path, .. } => Some(PathBuf::from(path)),
            TaskType::CoreTask { source } => {
                match source {
                    TaskSource::File { path } => Some(PathBuf::from(path)),
                    _ => None,
                }
            }
        }
    }
    
    /// Ensure content is loaded for JS tasks
    pub fn ensure_content_loaded(&mut self) -> Result<(), TaskError> {
        match &mut self.task_type {
            TaskType::JsTask { path, content } => {
                if content.is_none() {
                    let js_content = std::fs::read_to_string(path)?;
                    *content = Some(Arc::new(js_content.clone()));
                    self.content = Some(js_content);
                }
                Ok(())
            }
            TaskType::CoreTask { source } => {
                match source {
                    TaskSource::JavaScript { code } => {
                        self.content = Some(code.clone());
                        Ok(())
                    }
                    TaskSource::File { path } => {
                        if self.content.is_none() {
                            let js_content = std::fs::read_to_string(path)?;
                            self.content = Some(js_content);
                        }
                        Ok(())
                    }
                    _ => Ok(())
                }
            }
        }
    }
    
    /// Get the JavaScript content if available
    pub fn js_content(&self) -> Option<&str> {
        match &self.task_type {
            TaskType::JsTask { content, .. } => {
                content.as_ref().map(|s| s.as_str())
            }
            TaskType::CoreTask { .. } => {
                self.content.as_deref()
            }
        }
    }
    
    /// Validate that the task is properly structured and syntactically correct
    pub fn validate(&mut self) -> Result<(), crate::task::TaskError> {
        super::validation::validate_task(self)
    }
    
    /// Purge content from memory to save space
    pub fn purge_content(&mut self) {
        super::cache::purge_content(&mut self.task_type)
    }
}

/// Task builder for easier construction
pub struct TaskBuilder {
    metadata: Option<TaskMetadata>,
    task_type: Option<TaskType>,
    input_schema: JsonValue,
    output_schema: JsonValue,
    path: Option<PathBuf>,
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new() -> Self {
        Self {
            metadata: None,
            task_type: None,
            input_schema: JsonValue::Object(Default::default()),
            output_schema: JsonValue::Object(Default::default()),
            path: None,
        }
    }
    
    /// Set the task metadata
    pub fn with_metadata(mut self, metadata: TaskMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Set the task type
    pub fn with_task_type(mut self, task_type: TaskType) -> Self {
        self.task_type = Some(task_type);
        self
    }
    
    /// Set the input schema
    pub fn with_input_schema(mut self, schema: JsonValue) -> Self {
        self.input_schema = schema;
        self
    }
    
    /// Set the output schema
    pub fn with_output_schema(mut self, schema: JsonValue) -> Self {
        self.output_schema = schema;
        self
    }
    
    /// Set the file path
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }
    
    /// Build the task
    pub fn build(self) -> Result<Task, TaskError> {
        let metadata = self.metadata.ok_or_else(|| {
            TaskError::InvalidTaskStructure("Missing task metadata".to_string())
        })?;
        
        let task_type = self.task_type.ok_or_else(|| {
            TaskError::InvalidTaskStructure("Missing task type".to_string())
        })?;
        
        let path = self.path.ok_or_else(|| {
            TaskError::InvalidTaskStructure("Missing task path".to_string())
        })?;
        
        Ok(Task {
            metadata,
            task_type,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            path,
            _temp_dir: None,
            content: None,
        })
    }
}