use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct WorkerResponse {
    pub id: String,
    pub status: WorkerStatus,
    pub current_task: Option<String>,
    pub current_execution_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
}

#[derive(Debug, Serialize)]
pub struct WorkerDetailResponse {
    pub id: String,
    pub status: WorkerStatus,
    pub current_task: Option<String>,
    pub current_task_name: Option<String>,
    pub current_execution_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    Idle,
    Running,
    Stopping,
    Stopped,
    Error,
}

#[derive(Debug, Serialize)]
pub struct WorkerPoolStats {
    pub total_workers: u64,
    pub idle_workers: u64,
    pub running_workers: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub average_task_duration_ms: Option<f64>,
}