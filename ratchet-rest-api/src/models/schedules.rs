//! Schedule-related request and response models

use serde::{Deserialize, Serialize};
// use utoipa::ToSchema; // temporarily disabled
use ratchet_api_types::{ApiId, UnifiedOutputDestination};

/// Request to create a new schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleRequest {
    /// ID of the task to schedule
    pub task_id: ApiId,

    /// Human-readable name for the schedule
    pub name: String,

    /// Optional description of the schedule purpose
    pub description: Option<String>,

    /// Cron expression defining the schedule
    pub cron_expression: String,

    /// Whether the schedule is enabled
    pub enabled: Option<bool>,

    /// Optional output destinations for execution results (webhooks, files, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_destinations: Option<Vec<UnifiedOutputDestination>>,
}

/// Request to update a schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRequest {
    /// Updated name for the schedule
    pub name: Option<String>,

    /// Updated description
    pub description: Option<String>,

    /// Updated cron expression
    pub cron_expression: Option<String>,

    /// Updated enabled status
    pub enabled: Option<bool>,

    /// Updated output destinations for execution results (webhooks, files, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_destinations: Option<Vec<UnifiedOutputDestination>>,
}

/// Schedule statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleStats {
    /// Total number of schedules in the system
    pub total_schedules: u64,

    /// Number of enabled schedules
    pub enabled_schedules: u64,

    /// Number of disabled schedules
    pub disabled_schedules: u64,

    /// Number of schedules ready to run now
    pub schedules_ready_to_run: u64,

    /// Average time between executions in minutes
    pub average_execution_interval_minutes: Option<f64>,

    /// Most recent execution time
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,

    /// Next scheduled execution time
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,
}
