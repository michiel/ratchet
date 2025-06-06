pub mod cache;
pub mod loader;
pub mod validation;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use thiserror::Error;
use uuid::Uuid;

// Re-export cache for use in the Task implementation
pub use cache::CONTENT_CACHE;

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
    #[cfg(feature = "output")]
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
    pub fn from_fs(path: impl AsRef<std::path::Path>) -> Result<Self, TaskError> {
        loader::load_from_fs(path)
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
        cache::ensure_content_loaded(&mut self.task_type)
    }

    /// Get the JavaScript content if loaded, or load it if not
    pub fn get_js_content(&mut self) -> Result<Arc<String>, TaskError> {
        self.ensure_content_loaded()?;

        match &self.task_type {
            TaskType::JsTask { content, .. } => content.clone().ok_or_else(|| {
                TaskError::InvalidTaskStructure("Failed to load JavaScript content".to_string())
            }),
        }
    }

    /// Pre-load the JavaScript content
    pub fn preload(&mut self) -> Result<(), TaskError> {
        self.ensure_content_loaded()
    }

    /// Purge content from memory to save space
    pub fn purge_content(&mut self) {
        cache::purge_content(&mut self.task_type)
    }

    /// Validate that the task is properly structured and syntactically correct
    pub fn validate(&mut self) -> Result<(), TaskError> {
        validation::validate_task(self)
    }
}
