use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTask {
    pub task_ref: TaskReference,
    pub metadata: TaskMetadata,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskReference {
    pub name: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub uuid: Uuid,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub reference: TaskReference,
    pub metadata: TaskMetadata,
    pub script: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub dependencies: Vec<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub tasks_added: usize,
    pub tasks_updated: usize,
    pub tasks_removed: usize,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub task_ref: TaskReference,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistryEvent {
    TaskAdded(DiscoveredTask),
    TaskUpdated(DiscoveredTask),
    TaskRemoved(TaskReference),
    BulkSync(SyncResult),
    Error(String),
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, field: String, message: String, code: String) {
        self.is_valid = false;
        self.errors.push(ValidationError { field, message, code });
    }

    pub fn add_warning(&mut self, field: String, message: String, code: String) {
        self.warnings.push(ValidationWarning { field, message, code });
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncResult {
    pub fn new() -> Self {
        Self {
            tasks_added: 0,
            tasks_updated: 0,
            tasks_removed: 0,
            errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, task_ref: TaskReference, error: String) {
        self.errors.push(SyncError { task_ref, error });
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}