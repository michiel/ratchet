//! Task-related request and response models

use serde::{Deserialize, Serialize};
use ratchet_api_types::{UnifiedTask, ApiId};

/// Request to create a new task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub enabled: Option<bool>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to update a task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Task validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateTaskRequest {
    pub input: serde_json::Value,
}

/// Task validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateTaskResponse {
    pub valid: bool,
    pub errors: Vec<ValidationErrorDetail>,
    pub warnings: Vec<ValidationWarningDetail>,
}

/// Validation error detail
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorDetail {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

/// Validation warning detail
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWarningDetail {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

/// Task synchronization response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTasksResponse {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub errors: Vec<TaskSyncError>,
}

/// Task synchronization error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSyncError {
    pub task_name: String,
    pub error: String,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStats {
    pub total_tasks: u64,
    pub enabled_tasks: u64,
    pub disabled_tasks: u64,
    pub registry_tasks: u64,
    pub database_tasks: u64,
    pub validation_errors: u64,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}