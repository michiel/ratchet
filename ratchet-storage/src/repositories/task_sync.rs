//! Repository synchronization abstraction layer
//!
//! This module defines the traits and structures for repository synchronization
//! operations, enabling bidirectional sync between the database and various
//! repository types (filesystem, Git, HTTP).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use anyhow::Result;

/// Repository task representation for sync operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryTask {
    /// Path within the repository
    pub path: String,
    /// Task name
    pub name: String,
    /// JavaScript source code
    pub source_code: String,
    /// Input schema definition
    pub input_schema: JsonValue,
    /// Output schema definition
    pub output_schema: JsonValue,
    /// Task metadata
    pub metadata: TaskMetadata,
    /// SHA256 checksum of source code
    pub checksum: String,
    /// File modification timestamp
    pub modified_at: DateTime<Utc>,
    /// File creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Task metadata for repository operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Task version
    pub version: String,
    /// Task description
    pub description: Option<String>,
    /// Author information
    pub author: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata fields
    pub custom: HashMap<String, JsonValue>,
}

/// Repository metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// Repository name
    pub name: String,
    /// Repository type
    pub repository_type: String,
    /// Repository URI
    pub uri: String,
    /// Current branch (for Git repositories)
    pub branch: Option<String>,
    /// Last commit hash (for Git repositories)
    pub commit: Option<String>,
    /// Whether repository supports write operations
    pub is_writable: bool,
    /// Repository-specific metadata
    pub metadata: HashMap<String, JsonValue>,
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Repository ID
    pub repository_id: i32,
    /// Number of tasks added
    pub tasks_added: u32,
    /// Number of tasks updated
    pub tasks_updated: u32,
    /// Number of tasks deleted
    pub tasks_deleted: u32,
    /// Conflicts encountered during sync
    pub conflicts: Vec<TaskConflict>,
    /// Errors encountered during sync
    pub errors: Vec<SyncError>,
    /// Sync duration in milliseconds
    pub duration_ms: u64,
    /// Sync timestamp
    pub synced_at: DateTime<Utc>,
}

/// Result of a push operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    /// Task ID
    pub task_id: i32,
    /// Repository ID
    pub repository_id: i32,
    /// Path within repository
    pub repository_path: String,
    /// Whether push was successful
    pub success: bool,
    /// Commit hash if applicable
    pub commit_hash: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Push timestamp
    pub pushed_at: DateTime<Utc>,
}

/// Sync conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConflict {
    /// Task ID
    pub task_id: i32,
    /// Repository ID
    pub repository_id: i32,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Local (database) version
    pub local_version: TaskVersion,
    /// Remote (repository) version
    pub remote_version: TaskVersion,
    /// Whether conflict can be auto-resolved
    pub auto_resolvable: bool,
    /// Conflict timestamp
    pub detected_at: DateTime<Utc>,
}

/// Types of sync conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Both local and remote versions have been modified
    ModificationConflict,
    /// Task exists locally but not in repository
    LocalOnly,
    /// Task exists in repository but not locally
    RemoteOnly,
    /// Task deleted locally but modified in repository
    DeleteModifyConflict,
    /// Task deleted in repository but modified locally
    ModifyDeleteConflict,
}

/// Task version for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskVersion {
    /// Source code
    pub source_code: String,
    /// Input schema
    pub input_schema: JsonValue,
    /// Output schema
    pub output_schema: JsonValue,
    /// Metadata
    pub metadata: TaskMetadata,
    /// Checksum
    pub checksum: String,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
}

/// Sync error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Task path that caused the error (if applicable)
    pub task_path: Option<String>,
    /// Error timestamp
    pub occurred_at: DateTime<Utc>,
}

/// Repository abstraction trait for task operations
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// List all tasks in the repository
    async fn list_tasks(&self) -> Result<Vec<RepositoryTask>>;
    
    /// Get a specific task by path
    async fn get_task(&self, path: &str) -> Result<Option<RepositoryTask>>;
    
    /// Store/update a task in the repository
    async fn put_task(&self, task: &RepositoryTask) -> Result<()>;
    
    /// Delete a task from the repository
    async fn delete_task(&self, path: &str) -> Result<()>;
    
    /// Get repository metadata
    async fn get_metadata(&self) -> Result<RepositoryMetadata>;
    
    /// Check if repository supports write operations
    async fn is_writable(&self) -> bool;
    
    /// Test repository connection
    async fn test_connection(&self) -> Result<bool>;
    
    /// Get repository health status
    async fn health_check(&self) -> Result<RepositoryHealth>;
}

/// Repository health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryHealth {
    /// Whether repository is accessible
    pub accessible: bool,
    /// Whether repository is writable
    pub writable: bool,
    /// Last successful operation timestamp
    pub last_success: Option<DateTime<Utc>>,
    /// Number of recent errors
    pub error_count: u32,
    /// Health status message
    pub message: String,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep database version
    TakeLocal,
    /// Use repository version
    TakeRemote,
    /// Attempt automatic merge
    Merge,
    /// Require manual resolution
    Manual,
}

impl RepositoryTask {
    /// Calculate SHA256 checksum of source code
    pub fn calculate_checksum(source_code: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(source_code.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    /// Create a new repository task with calculated checksum
    pub fn new(
        path: String,
        name: String,
        source_code: String,
        input_schema: JsonValue,
        output_schema: JsonValue,
        metadata: TaskMetadata,
    ) -> Self {
        let checksum = Self::calculate_checksum(&source_code);
        let now = Utc::now();
        
        Self {
            path,
            name,
            source_code,
            input_schema,
            output_schema,
            metadata,
            checksum,
            modified_at: now,
            created_at: now,
        }
    }
    
    /// Update source code and recalculate checksum
    pub fn update_source_code(&mut self, source_code: String) {
        self.source_code = source_code;
        self.checksum = Self::calculate_checksum(&self.source_code);
        self.modified_at = Utc::now();
    }
    
    /// Check if task has been modified since given timestamp
    pub fn is_modified_since(&self, timestamp: DateTime<Utc>) -> bool {
        self.modified_at > timestamp
    }
}

impl TaskMetadata {
    /// Create new metadata with minimal fields
    pub fn minimal(version: String) -> Self {
        Self {
            version,
            description: None,
            author: None,
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }
    
    /// Create metadata from JSON value
    pub fn from_json(value: JsonValue) -> Result<Self> {
        serde_json::from_value(value).map_err(|e| anyhow::anyhow!("Failed to parse metadata: {}", e))
    }
}

impl SyncResult {
    /// Create a new sync result
    pub fn new(repository_id: i32) -> Self {
        Self {
            repository_id,
            tasks_added: 0,
            tasks_updated: 0,
            tasks_deleted: 0,
            conflicts: Vec::new(),
            errors: Vec::new(),
            duration_ms: 0,
            synced_at: Utc::now(),
        }
    }
    
    /// Check if sync was successful (no conflicts or errors)
    pub fn is_successful(&self) -> bool {
        self.conflicts.is_empty() && self.errors.is_empty()
    }
    
    /// Get total number of tasks processed
    pub fn total_tasks_processed(&self) -> u32 {
        self.tasks_added + self.tasks_updated + self.tasks_deleted
    }
}

impl PushResult {
    /// Create a successful push result
    pub fn success(task_id: i32, repository_id: i32, repository_path: String, commit_hash: Option<String>) -> Self {
        Self {
            task_id,
            repository_id,
            repository_path,
            success: true,
            commit_hash,
            error: None,
            pushed_at: Utc::now(),
        }
    }
    
    /// Create a failed push result
    pub fn failure(task_id: i32, repository_id: i32, repository_path: String, error: String) -> Self {
        Self {
            task_id,
            repository_id,
            repository_path,
            success: false,
            commit_hash: None,
            error: Some(error),
            pushed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_repository_task_checksum() {
        let source_code = "function test() { return 'hello'; }";
        let checksum = RepositoryTask::calculate_checksum(source_code);
        
        // Verify checksum is consistent
        let checksum2 = RepositoryTask::calculate_checksum(source_code);
        assert_eq!(checksum, checksum2);
        
        // Verify different code produces different checksum
        let different_code = "function test() { return 'world'; }";
        let different_checksum = RepositoryTask::calculate_checksum(different_code);
        assert_ne!(checksum, different_checksum);
    }
    
    #[test]
    fn test_repository_task_creation() {
        let metadata = TaskMetadata::minimal("1.0.0".to_string());
        let task = RepositoryTask::new(
            "test/task.js".to_string(),
            "test_task".to_string(),
            "function test() { return 'hello'; }".to_string(),
            json!({"type": "object"}),
            json!({"type": "string"}),
            metadata,
        );
        
        assert_eq!(task.path, "test/task.js");
        assert_eq!(task.name, "test_task");
        assert!(!task.checksum.is_empty());
        assert!(task.created_at <= Utc::now());
    }
    
    #[test]
    fn test_sync_result_creation() {
        let mut result = SyncResult::new(1);
        assert_eq!(result.repository_id, 1);
        assert_eq!(result.total_tasks_processed(), 0);
        assert!(result.is_successful());
        
        result.tasks_added = 5;
        result.tasks_updated = 3;
        assert_eq!(result.total_tasks_processed(), 8);
        
        result.errors.push(SyncError {
            error_type: "test".to_string(),
            message: "test error".to_string(),
            task_path: None,
            occurred_at: Utc::now(),
        });
        assert!(!result.is_successful());
    }
}