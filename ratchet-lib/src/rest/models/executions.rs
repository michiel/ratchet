use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    api::types::{UnifiedExecution, ExecutionStatus, ApiId},
};

/// Execution response model for REST API (now unified)
pub type ExecutionResponse = UnifiedExecution;

/// Detailed execution response with additional metadata (now unified)
pub type ExecutionDetailResponse = UnifiedExecution;


/// Request model for creating a new execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionCreateRequest {
    pub task_id: ApiId,
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
    pub task_id: Option<ApiId>,
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

// Conversions handled by api::conversions module

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