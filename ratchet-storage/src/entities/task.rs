//! Task entity definition

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::Entity;

/// Task entity representing a task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Primary key
    pub id: i32,
    
    /// Unique identifier
    pub uuid: Uuid,
    
    /// Task name
    pub name: String,
    
    /// Task description
    pub description: Option<String>,
    
    /// Task version
    pub version: String,
    
    /// Path to task files or source
    pub path: String,
    
    /// Task metadata (JSON)
    pub metadata: serde_json::Value,
    
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
    
    /// Output schema (JSON Schema)
    pub output_schema: serde_json::Value,
    
    /// Whether the task is enabled for execution
    pub enabled: bool,
    
    /// Task status
    pub status: TaskStatus,
    
    /// When the task was created
    pub created_at: DateTime<Utc>,
    
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
    
    /// When the task was last validated
    pub validated_at: Option<DateTime<Utc>>,
    
    /// Registry source if loaded from external registry
    pub registry_source: Option<String>,
    
    /// Task tags for categorization
    pub tags: Vec<String>,
    
    /// Whether the task is deprecated
    pub deprecated: bool,
    
    /// Deprecation message
    pub deprecation_message: Option<String>,
}

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is being processed/validated
    Pending,
    
    /// Task is active and ready for execution
    Active,
    
    /// Task is inactive/disabled
    Inactive,
    
    /// Task validation failed
    Invalid,
    
    /// Task is deprecated but still functional
    Deprecated,
    
    /// Task has been archived
    Archived,
}

impl Entity for Task {
    fn id(&self) -> i32 {
        self.id
    }
    
    fn uuid(&self) -> Uuid {
        self.uuid
    }
    
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl Task {
    /// Create a new task
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        path: impl Into<String>,
        input_schema: serde_json::Value,
        output_schema: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            name: name.into(),
            description: None,
            version: version.into(),
            path: path.into(),
            metadata: serde_json::json!({}),
            input_schema,
            output_schema,
            enabled: true,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            validated_at: None,
            registry_source: None,
            tags: Vec::new(),
            deprecated: false,
            deprecation_message: None,
        }
    }
    
    /// Check if the task is executable
    pub fn is_executable(&self) -> bool {
        self.enabled 
            && matches!(self.status, TaskStatus::Active) 
            && self.validated_at.is_some()
            && !self.deprecated
    }
    
    /// Check if the task is ready for validation
    pub fn is_validatable(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::Invalid)
    }
    
    /// Mark the task as validated
    pub fn mark_validated(&mut self) {
        self.validated_at = Some(Utc::now());
        self.status = TaskStatus::Active;
        self.updated_at = Utc::now();
    }
    
    /// Mark the task as invalid
    pub fn mark_invalid(&mut self) {
        self.status = TaskStatus::Invalid;
        self.validated_at = None;
        self.updated_at = Utc::now();
    }
    
    /// Enable the task
    pub fn enable(&mut self) {
        self.enabled = true;
        if matches!(self.status, TaskStatus::Inactive) {
            self.status = TaskStatus::Pending;
        }
        self.updated_at = Utc::now();
    }
    
    /// Disable the task
    pub fn disable(&mut self) {
        self.enabled = false;
        self.status = TaskStatus::Inactive;
        self.updated_at = Utc::now();
    }
    
    /// Deprecate the task
    pub fn deprecate(&mut self, message: Option<String>) {
        self.deprecated = true;
        self.deprecation_message = message;
        self.status = TaskStatus::Deprecated;
        self.updated_at = Utc::now();
    }
    
    /// Archive the task
    pub fn archive(&mut self) {
        self.enabled = false;
        self.status = TaskStatus::Archived;
        self.updated_at = Utc::now();
    }
    
    /// Add a tag to the task
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }
    
    /// Remove a tag from the task
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }
    
    /// Update task metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }
    
    /// Get display name for the task
    pub fn display_name(&self) -> String {
        format!("{} v{}", self.name, self.version)
    }
    
    /// Check if the task matches a search query
    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        
        self.name.to_lowercase().contains(&query) ||
        self.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query)) ||
        self.tags.iter().any(|tag| tag.to_lowercase().contains(&query)) ||
        self.version.to_lowercase().contains(&query)
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Active => write!(f, "active"),
            TaskStatus::Inactive => write!(f, "inactive"),
            TaskStatus::Invalid => write!(f, "invalid"),
            TaskStatus::Deprecated => write!(f, "deprecated"),
            TaskStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = crate::StorageError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "active" => Ok(TaskStatus::Active),
            "inactive" => Ok(TaskStatus::Inactive),
            "invalid" => Ok(TaskStatus::Invalid),
            "deprecated" => Ok(TaskStatus::Deprecated),
            "archived" => Ok(TaskStatus::Archived),
            _ => Err(crate::StorageError::ValidationFailed(
                format!("Invalid task status: {}", s)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "test-task",
            "1.0.0",
            "/path/to/task",
            serde_json::json!({"type": "object"}),
            serde_json::json!({"type": "object"}),
        );
        
        assert_eq!(task.name, "test-task");
        assert_eq!(task.version, "1.0.0");
        assert_eq!(task.path, "/path/to/task");
        assert!(task.enabled);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.is_executable());
    }
    
    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new(
            "test-task",
            "1.0.0",
            "/path/to/task",
            serde_json::json!({"type": "object"}),
            serde_json::json!({"type": "object"}),
        );
        
        // Initially not executable
        assert!(!task.is_executable());
        assert!(task.is_validatable());
        
        // After validation
        task.mark_validated();
        assert!(task.is_executable());
        assert!(!task.is_validatable());
        assert_eq!(task.status, TaskStatus::Active);
        
        // After disabling
        task.disable();
        assert!(!task.is_executable());
        assert_eq!(task.status, TaskStatus::Inactive);
        
        // After enabling
        task.enable();
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.is_validatable());
    }
    
    #[test]
    fn test_task_tags() {
        let mut task = Task::new(
            "test-task",
            "1.0.0",
            "/path/to/task",
            serde_json::json!({"type": "object"}),
            serde_json::json!({"type": "object"}),
        );
        
        task.add_tag("api");
        task.add_tag("http");
        assert_eq!(task.tags.len(), 2);
        
        // Adding duplicate tag should not increase count
        task.add_tag("api");
        assert_eq!(task.tags.len(), 2);
        
        task.remove_tag("api");
        assert_eq!(task.tags.len(), 1);
        assert!(task.tags.contains(&"http".to_string()));
    }
    
    #[test]
    fn test_task_search() {
        let mut task = Task::new(
            "HTTP Client Task",
            "1.0.0",
            "/path/to/task",
            serde_json::json!({"type": "object"}),
            serde_json::json!({"type": "object"}),
        );
        
        task.description = Some("Makes HTTP requests to external APIs".to_string());
        task.add_tag("http");
        task.add_tag("api");
        
        assert!(task.matches_search("http"));
        assert!(task.matches_search("HTTP"));
        assert!(task.matches_search("client"));
        assert!(task.matches_search("api"));
        assert!(task.matches_search("external"));
        assert!(!task.matches_search("database"));
    }
    
    #[test]
    fn test_task_status_conversion() {
        assert_eq!("active".parse::<TaskStatus>().unwrap(), TaskStatus::Active);
        assert_eq!("pending".parse::<TaskStatus>().unwrap(), TaskStatus::Pending);
        assert!("invalid_status".parse::<TaskStatus>().is_err());
        
        assert_eq!(TaskStatus::Active.to_string(), "active");
        assert_eq!(TaskStatus::Deprecated.to_string(), "deprecated");
    }
}