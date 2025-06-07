pub mod cache;
pub mod enhanced;
pub mod loader;
pub mod validation;

#[cfg(test)]
mod tests;

use thiserror::Error;

// Re-export enhanced types as the primary interface
pub use enhanced::{Task, TaskBuilder, TaskMetadata, TaskType};

// Re-export core types for advanced usage
pub use ratchet_core::task::{TaskId, TaskMetadata as CoreTaskMetadata, TaskSource};

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

// Legacy types are now in the enhanced module
