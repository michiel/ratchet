//! Execution-related request and response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use ratchet_api_types::{ApiId, ExecutionStatus};

/// Request to create a new execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateExecutionRequest {
    /// ID of the task to execute
    pub task_id: ApiId,

    /// Input data for the task execution
    pub input: serde_json::Value,

    /// Optional priority level (low, normal, high, urgent)
    pub priority: Option<String>,

    /// Optional scheduled execution time (ISO 8601 format)
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to update execution status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExecutionRequest {
    /// New execution status
    pub status: Option<ExecutionStatus>,

    /// Execution output data
    pub output: Option<serde_json::Value>,

    /// Error message if execution failed
    pub error_message: Option<String>,

    /// Additional error details
    pub error_details: Option<serde_json::Value>,

    /// Execution progress percentage (0.0 to 100.0)
    pub progress: Option<f32>,
}

/// Execution retry request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryExecutionRequest {
    /// Optional new input data for the retry (uses original input if not provided)
    pub input: Option<serde_json::Value>,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStats {
    /// Total number of executions in the system
    pub total_executions: u64,

    /// Number of executions waiting to start
    pub pending_executions: u64,

    /// Number of executions currently running
    pub running_executions: u64,

    /// Number of successfully completed executions
    pub completed_executions: u64,

    /// Number of failed executions
    pub failed_executions: u64,

    /// Number of cancelled executions
    pub cancelled_executions: u64,

    /// Average execution duration in milliseconds
    pub average_duration_ms: Option<f64>,

    /// Success rate as a percentage (0.0 to 100.0)
    pub success_rate: f64,

    /// Number of executions in the last 24 hours
    pub executions_last_24h: u64,
}
