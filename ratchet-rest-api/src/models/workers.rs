//! Worker-related request and response models

use serde::{Deserialize, Serialize};

/// Worker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStats {
    pub total_workers: u64,
    pub idle_workers: u64,
    pub running_workers: u64,
    pub stopping_workers: u64,
    pub error_workers: u64,
    pub total_tasks_processed: u64,
    pub average_task_duration_ms: Option<f64>,
    pub system_load: Option<f64>,
}

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub memory_usage_mb: u64,
    pub memory_total_mb: u64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: Option<f64>,
    pub uptime_seconds: u64,
    pub active_connections: u64,
}