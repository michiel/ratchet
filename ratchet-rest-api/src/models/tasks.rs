//! Task-related request and response models

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "data-processor",
    "description": "Process incoming data files",
    "version": "1.0.0",
    "enabled": true,
    "inputSchema": {
        "type": "object",
        "properties": {
            "filename": {"type": "string"}
        },
        "required": ["filename"]
    },
    "outputSchema": {
        "type": "object",
        "properties": {
            "processedCount": {"type": "number"}
        }
    },
    "metadata": {
        "author": "Development Team",
        "category": "data-processing"
    }
}))]
pub struct CreateTaskRequest {
    /// Unique name for the task (alphanumeric characters, hyphens, and underscores only)
    #[schema(example = "data-processor", pattern = "^[a-zA-Z0-9_-]+$")]
    pub name: String,
    
    /// Optional description of what the task does
    #[schema(example = "Process incoming data files and generate reports")]
    pub description: Option<String>,
    
    /// Semantic version of the task (e.g., 1.0.0)
    #[schema(example = "1.0.0", pattern = r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)")]
    pub version: String,
    
    /// Whether the task is enabled for execution (defaults to true)
    #[schema(example = true)]
    pub enabled: Option<bool>,
    
    /// JSON schema defining the expected input format
    #[schema(example = json!({"type": "object", "properties": {"filename": {"type": "string"}}}))]
    pub input_schema: Option<serde_json::Value>,
    
    /// JSON schema defining the expected output format
    #[schema(example = json!({"type": "object", "properties": {"result": {"type": "string"}}}))]
    pub output_schema: Option<serde_json::Value>,
    
    /// Additional metadata for the task
    #[schema(example = json!({"author": "Development Team", "category": "data-processing"}))]
    pub metadata: Option<serde_json::Value>,
}

/// Request to update an existing task
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "description": "Updated description for the task",
    "enabled": false
}))]
pub struct UpdateTaskRequest {
    /// Updated name for the task
    #[schema(example = "updated-task-name")]
    pub name: Option<String>,
    
    /// Updated description
    #[schema(example = "Updated description for the task")]
    pub description: Option<String>,
    
    /// Updated version
    #[schema(example = "1.1.0")]
    pub version: Option<String>,
    
    /// Updated enabled status
    #[schema(example = false)]
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
#[schema(example = json!({
    "input": {
        "filename": "data.csv",
        "format": "csv"
    }
}))]
pub struct ValidateTaskRequest {
    /// Input data to validate against the task's input schema
    #[schema(example = json!({"filename": "data.csv", "format": "csv"}))]
    pub input: serde_json::Value,
}

/// Task validation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "valid": true,
    "errors": [],
    "warnings": [
        {
            "field": "format",
            "message": "Using deprecated format option",
            "code": "W001"
        }
    ]
}))]
pub struct ValidateTaskResponse {
    /// Whether the input data is valid according to the task schema
    #[schema(example = true)]
    pub valid: bool,
    
    /// List of validation errors found in the input
    pub errors: Vec<ValidationErrorDetail>,
    
    /// List of validation warnings (non-blocking issues)
    pub warnings: Vec<ValidationWarningDetail>,
}

/// Validation error detail
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "field": "filename",
    "message": "Field is required but missing",
    "code": "E001"
}))]
pub struct ValidationErrorDetail {
    /// The field name that failed validation (optional for global errors)
    #[schema(example = "filename")]
    pub field: Option<String>,
    
    /// Human-readable error message
    #[schema(example = "Field is required but missing")]
    pub message: String,
    
    /// Error code for programmatic handling
    #[schema(example = "E001")]
    pub code: String,
}

/// Validation warning detail
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "field": "format",
    "message": "Using deprecated format option",
    "code": "W001"
}))]
pub struct ValidationWarningDetail {
    /// The field name that generated the warning (optional for global warnings)
    #[schema(example = "format")]
    pub field: Option<String>,
    
    /// Human-readable warning message
    #[schema(example = "Using deprecated format option")]
    pub message: String,
    
    /// Warning code for programmatic handling
    #[schema(example = "W001")]
    pub code: String,
}

/// Task synchronization response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "added": ["new-task-1", "new-task-2"],
    "updated": ["existing-task-1"],
    "removed": ["old-task-1"],
    "errors": [
        {
            "taskName": "broken-task",
            "error": "Invalid JavaScript syntax"
        }
    ]
}))]
pub struct SyncTasksResponse {
    /// Names of tasks that were added during synchronization
    #[schema(example = json!(["new-task-1", "new-task-2"]))]
    pub added: Vec<String>,
    
    /// Names of tasks that were updated during synchronization
    #[schema(example = json!(["existing-task-1"]))]
    pub updated: Vec<String>,
    
    /// Names of tasks that were removed during synchronization
    #[schema(example = json!(["old-task-1"]))]
    pub removed: Vec<String>,
    
    /// Errors encountered during synchronization
    pub errors: Vec<TaskSyncError>,
}

/// Task synchronization error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "taskName": "broken-task",
    "error": "Invalid JavaScript syntax"
}))]
pub struct TaskSyncError {
    /// Name of the task that failed to synchronize
    #[schema(example = "broken-task")]
    pub task_name: String,
    
    /// Error message describing what went wrong
    #[schema(example = "Invalid JavaScript syntax")]
    pub error: String,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "totalTasks": 42,
    "enabledTasks": 38,
    "disabledTasks": 4,
    "registryTasks": 25,
    "databaseTasks": 42,
    "validationErrors": 2,
    "lastSync": "2023-12-07T14:30:00Z"
}))]
pub struct TaskStats {
    /// Total number of tasks in the system
    #[schema(example = 42)]
    pub total_tasks: u64,
    
    /// Number of enabled tasks
    #[schema(example = 38)]
    pub enabled_tasks: u64,
    
    /// Number of disabled tasks
    #[schema(example = 4)]
    pub disabled_tasks: u64,
    
    /// Number of tasks loaded from registry
    #[schema(example = 25)]
    pub registry_tasks: u64,
    
    /// Number of tasks stored in database
    #[schema(example = 42)]
    pub database_tasks: u64,
    
    /// Number of tasks with validation errors
    #[schema(example = 2)]
    pub validation_errors: u64,
    
    /// Timestamp of last synchronization with registry
    #[schema(example = "2023-12-07T14:30:00Z")]
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}