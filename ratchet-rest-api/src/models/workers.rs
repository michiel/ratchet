//! Worker-related request and response models

use serde::{Deserialize, Serialize};

/// Worker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerStats {
    pub total_workers: i32,
    pub active_workers: i32,
    pub idle_workers: i32,
    pub running_workers: i32,
    pub stopping_workers: i32,
    pub error_workers: i32,
    pub total_tasks: i64,
    pub average_uptime_seconds: Option<f64>,
    pub total_memory_usage_mb: Option<u64>,
}

/// Workers list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkersListResponse {
    pub workers: Vec<serde_json::Value>,
    pub total: i32,
    pub page: u32,
    pub limit: u32,
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