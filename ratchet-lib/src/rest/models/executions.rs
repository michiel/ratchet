use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::entities::{executions::ExecutionStatus, Execution};

/// Execution response model for REST API
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub id: String,
    pub uuid: String,
    pub task_id: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub http_requests: Option<serde_json::Value>,
    pub recording_path: Option<String>,
}

/// Detailed execution response with additional metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionDetailResponse {
    pub id: String,
    pub uuid: String,
    pub task_id: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub http_requests: Option<serde_json::Value>,
    pub recording_path: Option<String>,
    /// Additional execution metadata for detailed view
    pub metadata: ExecutionMetadata,
}

/// Additional execution metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Whether the execution can be retried
    pub can_retry: bool,
    /// Whether the execution can be cancelled
    pub can_cancel: bool,
    /// Execution progress if available
    pub progress: Option<f32>,
}

/// Request model for creating a new execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionCreateRequest {
    pub task_id: String,
    pub input: serde_json::Value,
    /// Optional execution options
    pub options: Option<ExecutionOptions>,
}

/// Request model for updating an execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionUpdateRequest {
    /// Only status updates are typically allowed
    pub status: Option<ExecutionStatus>,
    /// For manual completion
    pub output: Option<serde_json::Value>,
    /// For manual failure
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
}

/// Execution options for creation
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// Whether to enable recording
    pub enable_recording: Option<bool>,
    /// Priority level (if supported)
    pub priority: Option<i32>,
    /// Execution timeout in seconds
    pub timeout_seconds: Option<i32>,
}

/// Filter options for execution queries
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionFilters {
    /// Filter by task ID
    pub task_id: Option<String>,
    /// Filter by status
    pub status: Option<ExecutionStatus>,
    /// Filter by status (multiple values)
    pub status_in: Option<Vec<ExecutionStatus>>,
    /// Filter executions queued after this date
    pub queued_after: Option<DateTime<Utc>>,
    /// Filter executions queued before this date
    pub queued_before: Option<DateTime<Utc>>,
    /// Filter executions started after this date
    pub started_after: Option<DateTime<Utc>>,
    /// Filter executions started before this date
    pub started_before: Option<DateTime<Utc>>,
    /// Filter executions completed after this date
    pub completed_after: Option<DateTime<Utc>>,
    /// Filter executions completed before this date
    pub completed_before: Option<DateTime<Utc>>,
    /// Filter by minimum duration in milliseconds
    pub min_duration_ms: Option<i32>,
    /// Filter by maximum duration in milliseconds
    pub max_duration_ms: Option<i32>,
    /// Search in error messages
    pub error_message_like: Option<String>,
    /// Filter by UUIDs
    pub uuid_in: Option<Vec<String>>,
}

impl From<Execution> for ExecutionResponse {
    fn from(execution: Execution) -> Self {
        Self {
            id: execution.id.to_string(),
            uuid: execution.uuid.to_string(),
            task_id: execution.task_id.to_string(),
            input: execution.input.clone(),
            output: execution.output.clone(),
            status: execution.status,
            error_message: execution.error_message,
            error_details: execution.error_details.clone(),
            queued_at: execution.queued_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
            http_requests: execution.http_requests.clone(),
            recording_path: execution.recording_path,
        }
    }
}

impl From<Execution> for ExecutionDetailResponse {
    fn from(execution: Execution) -> Self {
        let metadata = ExecutionMetadata {
            can_retry: matches!(execution.status, ExecutionStatus::Failed | ExecutionStatus::Cancelled),
            can_cancel: matches!(execution.status, ExecutionStatus::Pending | ExecutionStatus::Running),
            progress: None, // TODO: Implement progress tracking
        };

        Self {
            id: execution.id.to_string(),
            uuid: execution.uuid.to_string(),
            task_id: execution.task_id.to_string(),
            input: execution.input.clone(),
            output: execution.output.clone(),
            status: execution.status,
            error_message: execution.error_message,
            error_details: execution.error_details.clone(),
            queued_at: execution.queued_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
            http_requests: execution.http_requests.clone(),
            recording_path: execution.recording_path,
            metadata,
        }
    }
}

impl ExecutionFilters {
    pub fn get_valid_sort_fields() -> Vec<&'static str> {
        vec![
            "id",
            "uuid", 
            "task_id",
            "status",
            "queued_at",
            "started_at", 
            "completed_at",
            "duration_ms"
        ]
    }
}