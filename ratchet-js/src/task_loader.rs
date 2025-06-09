//! Task loading from filesystem
//! 
//! This module provides functionality to load JavaScript tasks from filesystem
//! directories, compatible with the ratchet task format.

use crate::{JsTask, JsExecutionError};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when loading tasks
#[derive(Error, Debug)]
pub enum TaskLoadError {
    #[error("Task not found at path: {0}")]
    TaskNotFound(String),

    #[error("Invalid task structure: {0}")]
    InvalidStructure(String),

    #[error("File read error: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Missing required file: {0}")]
    MissingFile(String),
}

/// Task metadata structure  
#[derive(Debug, serde::Deserialize)]
pub struct TaskMetadata {
    pub label: String,
    pub description: Option<String>,
    pub version: String,
    pub core: Option<TaskCore>,
    // Legacy fields for backward compatibility
    pub uuid: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TaskCore {
    pub version: String,
}

/// Enhanced JavaScript task with metadata and file paths
#[derive(Debug)]
pub struct FileSystemTask {
    pub name: String,
    pub content: String,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub metadata: TaskMetadata,
    pub task_path: String,
}

impl FileSystemTask {
    /// Load a task from filesystem path
    pub fn from_fs<P: AsRef<Path>>(path: P) -> Result<Self, TaskLoadError> {
        let task_path = path.as_ref();
        
        if !task_path.exists() {
            return Err(TaskLoadError::TaskNotFound(
                task_path.display().to_string()
            ));
        }

        // Check if it's a directory with task structure
        if task_path.is_dir() {
            Self::load_from_directory(task_path)
        } else {
            Err(TaskLoadError::InvalidStructure(
                "Task path must be a directory".to_string()
            ))
        }
    }

    /// Load task from directory structure
    fn load_from_directory(dir: &Path) -> Result<Self, TaskLoadError> {
        // Load metadata.json
        let metadata_path = dir.join("metadata.json");
        if !metadata_path.exists() {
            return Err(TaskLoadError::MissingFile("metadata.json".to_string()));
        }

        let metadata_content = fs::read_to_string(&metadata_path)?;
        let metadata: TaskMetadata = serde_json::from_str(&metadata_content)?;

        // Load main.js
        let main_js_path = dir.join("main.js");
        if !main_js_path.exists() {
            return Err(TaskLoadError::MissingFile("main.js".to_string()));
        }

        let js_content = fs::read_to_string(&main_js_path)?;

        // Load optional schema files
        let input_schema = Self::load_schema_file(dir, "input.schema.json")?;
        let output_schema = Self::load_schema_file(dir, "output.schema.json")?;

        Ok(FileSystemTask {
            name: metadata.label.clone(),
            content: js_content,
            input_schema,
            output_schema,
            metadata,
            task_path: dir.display().to_string(),
        })
    }

    /// Load optional schema file
    fn load_schema_file(dir: &Path, filename: &str) -> Result<Option<JsonValue>, TaskLoadError> {
        let schema_path = dir.join(filename);
        if schema_path.exists() {
            let schema_content = fs::read_to_string(&schema_path)?;
            let schema: JsonValue = serde_json::from_str(&schema_content)?;
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Validate the task structure and JavaScript content
    pub fn validate(&self) -> Result<(), JsExecutionError> {
        // Basic validation - check if the task has executable content
        // Accept multiple formats:
        // 1. Named function: function main(input) { ... }
        // 2. Const function: const main = (input) => { ... }
        // 3. Anonymous function: (function(input) { ... })
        // 4. Arrow function: (input) => { ... }
        let has_executable_content = self.content.contains("function main") 
            || self.content.contains("const main") 
            || self.content.contains("(function(") 
            || self.content.trim().starts_with("(") 
            || self.content.contains("=> {");

        if !has_executable_content {
            return Err(JsExecutionError::ValidationError(
                "Task must contain executable JavaScript code (function main, anonymous function, or arrow function)".to_string()
            ));
        }

        // Ensure the content is not empty
        if self.content.trim().is_empty() {
            return Err(JsExecutionError::ValidationError(
                "Task content cannot be empty".to_string()
            ));
        }

        // TODO: Add more sophisticated validation
        // - Schema validation against input/output schemas
        // - JavaScript syntax validation
        // - Required function signature validation

        Ok(())
    }

    /// Convert to JsTask for execution
    pub fn to_js_task(&self) -> JsTask {
        JsTask {
            name: self.name.clone(),
            content: self.content.clone(),
            input_schema: self.input_schema.clone(),
            output_schema: self.output_schema.clone(),
        }
    }

    /// Get task label for display
    pub fn label(&self) -> &str {
        &self.metadata.label
    }

    /// Get task version
    pub fn version(&self) -> &str {
        &self.metadata.version
    }

    /// Get task description
    pub fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }
}

/// Convenience function to load and execute a task from filesystem
pub async fn load_and_execute_task<P: AsRef<Path>>(
    path: P,
    input_data: JsonValue,
) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
    let fs_task = FileSystemTask::from_fs(path)?;
    fs_task.validate()?;
    
    let js_task = fs_task.to_js_task();
    let runner = crate::JsTaskRunner::new();
    
    let result = runner.execute_task(&js_task, input_data, None).await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_task(dir: &Path) -> std::io::Result<()> {
        // Create metadata.json
        let metadata = r#"
        {
            "label": "Test Task",
            "description": "A test task",
            "version": "1.0.0",
            "core": {
                "version": "0.3.0"
            }
        }
        "#;
        fs::write(dir.join("metadata.json"), metadata)?;

        // Create main.js
        let main_js = r#"
        function main(input) {
            return { result: input.a + input.b };
        }
        "#;
        fs::write(dir.join("main.js"), main_js)?;

        // Create input schema
        let input_schema = r#"
        {
            "type": "object",
            "properties": {
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        }
        "#;
        fs::write(dir.join("input.schema.json"), input_schema)?;

        Ok(())
    }

    #[test]
    fn test_load_task_from_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join("test_task");
        fs::create_dir(&task_dir).unwrap();
        
        create_test_task(&task_dir).unwrap();

        let task = FileSystemTask::from_fs(&task_dir).unwrap();
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.metadata.version, "1.0.0");
        assert!(task.input_schema.is_some());
        assert!(task.content.contains("function main"));
    }

    #[test]
    fn test_task_validation() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join("test_task");
        fs::create_dir(&task_dir).unwrap();
        
        create_test_task(&task_dir).unwrap();

        let task = FileSystemTask::from_fs(&task_dir).unwrap();
        assert!(task.validate().is_ok());
    }

    #[tokio::test]
    async fn test_load_and_execute() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join("test_task");
        fs::create_dir(&task_dir).unwrap();
        
        create_test_task(&task_dir).unwrap();

        let input = serde_json::json!({ "a": 5, "b": 3 });
        let result = load_and_execute_task(&task_dir, input).await.unwrap();
        
        assert_eq!(result["result"], 8);
    }
}