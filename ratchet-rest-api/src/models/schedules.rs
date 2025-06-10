//! Schedule-related request and response models

use serde::{Deserialize, Serialize};
use ratchet_api_types::{UnifiedSchedule, ApiId};

/// Request to create a new schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleRequest {
    pub task_id: ApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub enabled: Option<bool>,
}

/// Request to update a schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}

/// Schedule statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleStats {
    pub total_schedules: u64,
    pub enabled_schedules: u64,
    pub disabled_schedules: u64,
    pub schedules_ready_to_run: u64,
    pub average_execution_interval_minutes: Option<f64>,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,
}