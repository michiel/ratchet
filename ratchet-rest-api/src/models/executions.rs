//! Execution-related request and response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use ratchet_api_types::{ApiId, ExecutionStatus};

/// Request to create a new execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "taskId": "task_123",
    "input": {
        "filename": "data.csv",
        "format": "csv"
    },
    "priority": "high",
    "scheduledFor": "2023-12-07T15:30:00Z"
}))]
pub struct CreateExecutionRequest {
    /// ID of the task to execute
    #[schema(example = "task_123")]
    pub task_id: ApiId,
    
    /// Input data for the task execution
    #[schema(example = json!({"filename": "data.csv", "format": "csv"}))]
    pub input: serde_json::Value,
    
    /// Optional priority level (low, normal, high, urgent)
    #[schema(example = "high")]
    pub priority: Option<String>,
    
    /// Optional scheduled execution time (ISO 8601 format)
    #[schema(example = "2023-12-07T15:30:00Z")]
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to update execution status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "status": "completed",
    "output": {
        "processedRecords": 1250,
        "outputFile": "processed_data.json"
    },
    "progress": 100.0
}))]
pub struct UpdateExecutionRequest {
    /// New execution status
    #[schema(example = "completed")]
    pub status: Option<ExecutionStatus>,
    
    /// Execution output data
    #[schema(example = json!({"processedRecords": 1250, "outputFile": "processed_data.json"}))]
    pub output: Option<serde_json::Value>,
    
    /// Error message if execution failed
    #[schema(example = "Validation failed: missing required field 'id'")]
    pub error_message: Option<String>,
    
    /// Additional error details
    #[schema(example = json!({"field": "id", "expected": "string", "actual": "null"}))]
    pub error_details: Option<serde_json::Value>,
    
    /// Execution progress percentage (0.0 to 100.0)
    #[schema(example = 100.0)]
    pub progress: Option<f32>,
}

/// Execution retry request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "input": {
        "filename": "corrected_data.csv",
        "format": "csv",
        "validateOnly": false
    }
}))]
pub struct RetryExecutionRequest {
    /// Optional new input data for the retry (uses original input if not provided)
    #[schema(example = json!({"filename": "corrected_data.csv", "format": "csv"}))]
    pub input: Option<serde_json::Value>,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "totalExecutions": 1523,
    "pendingExecutions": 12,
    "runningExecutions": 5,
    "completedExecutions": 1401,
    "failedExecutions": 98,
    "cancelledExecutions": 7,
    "averageDurationMs": 2456.7,
    "successRate": 92.1,
    "executionsLast24h": 247
}))]
pub struct ExecutionStats {
    /// Total number of executions in the system
    #[schema(example = 1523)]
    pub total_executions: u64,
    
    /// Number of executions waiting to start
    #[schema(example = 12)]
    pub pending_executions: u64,
    
    /// Number of executions currently running
    #[schema(example = 5)]
    pub running_executions: u64,
    
    /// Number of successfully completed executions
    #[schema(example = 1401)]
    pub completed_executions: u64,
    
    /// Number of failed executions
    #[schema(example = 98)]
    pub failed_executions: u64,
    
    /// Number of cancelled executions
    #[schema(example = 7)]
    pub cancelled_executions: u64,
    
    /// Average execution duration in milliseconds
    #[schema(example = 2456.7)]
    pub average_duration_ms: Option<f64>,
    
    /// Success rate as a percentage (0.0 to 100.0)
    #[schema(example = 92.1)]
    pub success_rate: f64,
    
    /// Number of executions in the last 24 hours
    #[schema(example = 247)]
    pub executions_last_24h: u64,
}