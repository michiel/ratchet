//! Task-related request and response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    /// Unique name for the task (alphanumeric characters, hyphens, and underscores only)
    pub name: String,

    /// Optional description of what the task does
    pub description: Option<String>,

    /// Semantic version of the task (e.g., 1.0.0)
    pub version: String,

    /// Whether the task is enabled for execution (defaults to true)
    pub enabled: Option<bool>,

    /// JSON schema defining the expected input format
    pub input_schema: Option<serde_json::Value>,

    /// JSON schema defining the expected output format
    pub output_schema: Option<serde_json::Value>,

    /// Additional metadata for the task
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    /// Updated name for the task
    pub name: Option<String>,

    /// Updated description
    pub description: Option<String>,

    /// Updated version
    pub version: Option<String>,

    /// Updated enabled status
    pub enabled: Option<bool>,

    /// Updated input schema
    pub input_schema: Option<serde_json::Value>,

    /// Updated output schema
    pub output_schema: Option<serde_json::Value>,

    /// Updated metadata
    pub metadata: Option<serde_json::Value>,
}

/// Task validation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidateTaskRequest {
    /// Input data to validate against the task's input schema
    pub input: serde_json::Value,
}

/// Task validation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidateTaskResponse {
    /// Whether the input data is valid according to the task schema
    pub valid: bool,

    /// List of validation errors found in the input
    pub errors: Vec<ValidationErrorDetail>,

    /// List of validation warnings (non-blocking issues)
    pub warnings: Vec<ValidationWarningDetail>,
}

/// Validation error detail
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorDetail {
    /// The field name that failed validation (optional for global errors)
    pub field: Option<String>,

    /// Human-readable error message
    pub message: String,

    /// Error code for programmatic handling
    pub code: String,
}

/// Validation warning detail
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWarningDetail {
    /// The field name that generated the warning (optional for global warnings)
    pub field: Option<String>,

    /// Human-readable warning message
    pub message: String,

    /// Warning code for programmatic handling
    pub code: String,
}

/// Task synchronization response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncTasksResponse {
    /// Names of tasks that were added during synchronization
    pub added: Vec<String>,

    /// Names of tasks that were updated during synchronization
    pub updated: Vec<String>,

    /// Names of tasks that were removed during synchronization
    pub removed: Vec<String>,

    /// Errors encountered during synchronization
    pub errors: Vec<TaskSyncError>,
}

/// Task synchronization error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskSyncError {
    /// Name of the task that failed to synchronize
    pub task_name: String,

    /// Error message describing what went wrong
    pub error: String,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskStats {
    /// Total number of tasks in the system
    pub total_tasks: u64,

    /// Number of enabled tasks
    pub enabled_tasks: u64,

    /// Number of disabled tasks
    pub disabled_tasks: u64,

    /// Number of tasks loaded from registry
    pub registry_tasks: u64,

    /// Number of tasks stored in database
    pub database_tasks: u64,

    /// Number of tasks with validation errors
    pub validation_errors: u64,

    /// Timestamp of last synchronization with registry
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}
