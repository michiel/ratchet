use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStatus {
    pub name: String,
    pub source_type: String,
    pub uri: String,
    pub enabled: bool,
    pub sync_state: SyncState,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_result: Option<SyncResult>,
    pub tasks_discovered: usize,
    pub tasks_loaded: usize,
    pub error_count: usize,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncState {
    Idle,
    Syncing,
    Synced,
    Error(String),
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning(String),
    Error(String),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatusReport {
    pub total_repositories: usize,
    pub active_repositories: usize,
    pub total_tasks: usize,
    pub healthy_repositories: usize,
    pub repositories: Vec<RepositoryStatus>,
    pub last_updated: DateTime<Utc>,
}

impl RepositoryStatus {
    pub fn new(name: String, source_type: String, uri: String, enabled: bool) -> Self {
        Self {
            name,
            source_type,
            uri,
            enabled,
            sync_state: if enabled { SyncState::Idle } else { SyncState::Disabled },
            last_sync_at: None,
            last_sync_result: None,
            tasks_discovered: 0,
            tasks_loaded: 0,
            error_count: 0,
            health_status: HealthStatus::Unknown,
        }
    }

    pub fn update_sync_result(&mut self, result: SyncResult) {
        self.last_sync_at = Some(Utc::now());
        self.error_count = result.errors.len();
        self.tasks_loaded = result.tasks_added + result.tasks_updated;

        if result.errors.is_empty() {
            self.sync_state = SyncState::Synced;
            self.health_status = HealthStatus::Healthy;
        } else {
            let error_msg = format!("{} sync errors", result.errors.len());
            self.sync_state = SyncState::Error(error_msg.clone());
            self.health_status = HealthStatus::Error(error_msg);
        }

        self.last_sync_result = Some(result);
    }

    pub fn set_sync_error(&mut self, error: String) {
        self.sync_state = SyncState::Error(error.clone());
        self.health_status = HealthStatus::Error(error);
        self.error_count += 1;
        self.last_sync_at = Some(Utc::now());
    }

    pub fn set_syncing(&mut self) {
        self.sync_state = SyncState::Syncing;
    }

    pub fn update_task_discovery(&mut self, count: usize) {
        self.tasks_discovered = count;
    }
}
