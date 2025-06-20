//! Task domain model and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a task (newtype pattern for type safety)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Create a new random task ID
    pub fn new() -> Self {
        TaskId(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TaskId {
    fn from(uuid: Uuid) -> Self {
        TaskId(uuid)
    }
}

impl From<TaskId> for Uuid {
    fn from(id: TaskId) -> Self {
        id.0
    }
}

/// Task metadata including name, version, and description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskMetadata {
    /// Unique identifier for the task
    pub id: TaskId,

    /// Human-readable name of the task
    pub name: String,

    /// Semantic version of the task
    pub version: String,

    /// Brief description of what the task does
    pub description: Option<String>,

    /// Longer documentation or usage instructions
    pub documentation: Option<String>,

    /// Author or maintainer of the task
    pub author: Option<String>,

    /// Tags for categorization and discovery
    pub tags: Vec<String>,

    /// Whether this task is deprecated
    pub deprecated: bool,

    /// Deprecation message if deprecated
    pub deprecation_message: Option<String>,
}

impl TaskMetadata {
    /// Create a new TaskMetadata with minimal required fields
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: TaskId::new(),
            name: name.into(),
            version: version.into(),
            description: None,
            documentation: None,
            author: None,
            tags: Vec::new(),
            deprecated: false,
            deprecation_message: None,
        }
    }

    /// Builder pattern for adding description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder pattern for adding tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Mark task as deprecated with a message
    pub fn deprecate(mut self, message: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_message = Some(message.into());
        self
    }
}

/// Complete task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task metadata
    pub metadata: TaskMetadata,

    /// JSON schema for input validation
    pub input_schema: serde_json::Value,

    /// JSON schema for output validation
    pub output_schema: serde_json::Value,

    /// The source code or reference to the task implementation
    pub source: TaskSource,

    /// Whether this task is enabled for execution
    pub enabled: bool,

    /// When the task was created
    pub created_at: DateTime<Utc>,

    /// When the task was last updated
    pub updated_at: DateTime<Utc>,

    /// When the task was last validated
    pub validated_at: Option<DateTime<Utc>>,

    /// Registry source if loaded from a registry
    pub registry_source: Option<String>,
}

/// Source of task implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskSource {
    /// JavaScript source code
    JavaScript { code: String },

    /// Reference to a file on disk
    File { path: String },

    /// Reference to a URL
    Url { url: String, checksum: Option<String> },

    /// Plugin-based task
    Plugin { plugin_id: String, task_name: String },
}

impl Task {
    /// Create a new task with JavaScript source
    pub fn new_javascript(
        metadata: TaskMetadata,
        code: impl Into<String>,
        input_schema: serde_json::Value,
        output_schema: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            metadata,
            input_schema,
            output_schema,
            source: TaskSource::JavaScript { code: code.into() },
            enabled: true,
            created_at: now,
            updated_at: now,
            validated_at: None,
            registry_source: None,
        }
    }

    /// Check if the task is valid for execution
    pub fn is_executable(&self) -> bool {
        self.enabled && self.validated_at.is_some()
    }

    /// Get a display name for the task
    pub fn display_name(&self) -> String {
        format!("{} v{}", self.metadata.name, self.metadata.version)
    }
}

/// Builder for constructing tasks
pub struct TaskBuilder {
    metadata: TaskMetadata,
    input_schema: Option<serde_json::Value>,
    output_schema: Option<serde_json::Value>,
    source: Option<TaskSource>,
    enabled: bool,
    registry_source: Option<String>,
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            metadata: TaskMetadata::new(name, version),
            input_schema: None,
            output_schema: None,
            source: None,
            enabled: true,
            registry_source: None,
        }
    }

    /// Set the task metadata
    pub fn metadata(mut self, metadata: TaskMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the input schema
    pub fn input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set the output schema  
    pub fn output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set JavaScript source code
    pub fn javascript_source(mut self, code: impl Into<String>) -> Self {
        self.source = Some(TaskSource::JavaScript { code: code.into() });
        self
    }

    /// Set file source
    pub fn file_source(mut self, path: impl Into<String>) -> Self {
        self.source = Some(TaskSource::File { path: path.into() });
        self
    }

    /// Set whether the task is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the registry source
    pub fn registry_source(mut self, source: impl Into<String>) -> Self {
        self.registry_source = Some(source.into());
        self
    }

    /// Build the task
    pub fn build(self) -> Result<Task, String> {
        let input_schema = self.input_schema.ok_or("Input schema is required")?;
        let output_schema = self.output_schema.ok_or("Output schema is required")?;
        let source = self.source.ok_or("Task source is required")?;

        let now = Utc::now();
        Ok(Task {
            metadata: self.metadata,
            input_schema,
            output_schema,
            source,
            enabled: self.enabled,
            created_at: now,
            updated_at: now,
            validated_at: None,
            registry_source: self.registry_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);

        let uuid = Uuid::new_v4();
        let id = TaskId::from(uuid);
        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn test_task_metadata_builder() {
        let metadata = TaskMetadata::new("test-task", "1.0.0")
            .with_description("A test task")
            .with_tags(vec!["test".to_string(), "example".to_string()])
            .deprecate("Use test-task-v2 instead");

        assert_eq!(metadata.name, "test-task");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description.as_deref(), Some("A test task"));
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.deprecated);
        assert_eq!(
            metadata.deprecation_message.as_deref(),
            Some("Use test-task-v2 instead")
        );
    }

    #[test]
    fn test_task_builder() {
        let task = TaskBuilder::new("example", "1.0.0")
            .input_schema(serde_json::json!({"type": "object"}))
            .output_schema(serde_json::json!({"type": "object"}))
            .javascript_source("(function(input) { return input; })")
            .enabled(true)
            .build()
            .unwrap();

        assert_eq!(task.metadata.name, "example");
        assert_eq!(task.metadata.version, "1.0.0");
        assert!(task.enabled);
        assert_eq!(task.display_name(), "example v1.0.0");

        match task.source {
            TaskSource::JavaScript { code } => {
                assert_eq!(code, "(function(input) { return input; })");
            }
            _ => panic!("Expected JavaScript source"),
        }
    }

    #[test]
    fn test_task_is_executable() {
        let mut task = TaskBuilder::new("test", "1.0.0")
            .input_schema(serde_json::json!({}))
            .output_schema(serde_json::json!({}))
            .javascript_source("return {};")
            .build()
            .unwrap();

        // Not executable without validation
        assert!(!task.is_executable());

        // Executable after validation
        task.validated_at = Some(Utc::now());
        assert!(task.is_executable());

        // Not executable if disabled
        task.enabled = false;
        assert!(!task.is_executable());
    }
}
