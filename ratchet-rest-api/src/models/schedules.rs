//! Schedule-related request and response models

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use ratchet_api_types::ApiId;

/// Request to create a new schedule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "taskId": "task_123",
    "name": "daily-report",
    "description": "Generate daily analytics report",
    "cronExpression": "0 9 * * *",
    "enabled": true
}))]
pub struct CreateScheduleRequest {
    /// ID of the task to schedule
    #[schema(example = "task_123")]
    pub task_id: ApiId,
    
    /// Human-readable name for the schedule
    #[schema(example = "daily-report")]
    pub name: String,
    
    /// Optional description of the schedule purpose
    #[schema(example = "Generate daily analytics report")]
    pub description: Option<String>,
    
    /// Cron expression defining the schedule
    #[schema(example = "0 9 * * *")]
    pub cron_expression: String,
    
    /// Whether the schedule is enabled
    #[schema(example = true)]
    pub enabled: Option<bool>,
}

/// Request to update a schedule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "weekly-report",
    "description": "Generate weekly analytics report",
    "cronExpression": "0 9 * * 1",
    "enabled": false
}))]
pub struct UpdateScheduleRequest {
    /// Updated name for the schedule
    #[schema(example = "weekly-report")]
    pub name: Option<String>,
    
    /// Updated description
    #[schema(example = "Generate weekly analytics report")]
    pub description: Option<String>,
    
    /// Updated cron expression
    #[schema(example = "0 9 * * 1")]
    pub cron_expression: Option<String>,
    
    /// Updated enabled status
    #[schema(example = false)]
    pub enabled: Option<bool>,
}

/// Schedule statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "totalSchedules": 45,
    "enabledSchedules": 38,
    "disabledSchedules": 7,
    "schedulesReadyToRun": 3,
    "averageExecutionIntervalMinutes": 120.5,
    "lastExecution": "2023-12-07T14:30:00Z",
    "nextExecution": "2023-12-07T16:30:00Z"
}))]
pub struct ScheduleStats {
    /// Total number of schedules in the system
    #[schema(example = 45)]
    pub total_schedules: u64,
    
    /// Number of enabled schedules
    #[schema(example = 38)]
    pub enabled_schedules: u64,
    
    /// Number of disabled schedules
    #[schema(example = 7)]
    pub disabled_schedules: u64,
    
    /// Number of schedules ready to run now
    #[schema(example = 3)]
    pub schedules_ready_to_run: u64,
    
    /// Average time between executions in minutes
    #[schema(example = 120.5)]
    pub average_execution_interval_minutes: Option<f64>,
    
    /// Most recent execution time
    #[schema(example = "2023-12-07T14:30:00Z")]
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Next scheduled execution time
    #[schema(example = "2023-12-07T16:30:00Z")]
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,
}