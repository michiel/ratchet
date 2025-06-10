//! Execution-related request and response models

use serde::{Deserialize, Serialize};
use ratchet_api_types::{UnifiedExecution, ApiId, ExecutionStatus};

/// Request to create a new execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExecutionRequest {
    pub task_id: ApiId,
    pub input: serde_json::Value,
    pub priority: Option<String>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to update execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExecutionRequest {
    pub status: Option<ExecutionStatus>,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub progress: Option<f32>,
}

/// Execution retry request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryExecutionRequest {
    pub input: Option<serde_json::Value>, // Optional new input
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub pending_executions: u64,
    pub running_executions: u64,
    pub completed_executions: u64,
    pub failed_executions: u64,
    pub cancelled_executions: u64,
    pub average_duration_ms: Option<f64>,
    pub success_rate: f64,
    pub executions_last_24h: u64,
}