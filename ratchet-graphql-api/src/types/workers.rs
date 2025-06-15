//! GraphQL types for workers

use async_graphql::{SimpleObject, InputObject, Enum};
use ratchet_api_types::{UnifiedWorkerStatus, WorkerStatusType};
use chrono::{DateTime, Utc};

/// GraphQL Worker type
#[derive(SimpleObject, Clone, Debug)]
pub struct Worker {
    pub id: String,
    pub status: WorkerStatusGraphQL,
    pub task_count: i32,
    pub current_task: Option<String>,
    pub uptime_seconds: i64,
    pub memory_usage_mb: Option<u64>,
    pub cpu_usage_percent: Option<f32>,
    pub last_heartbeat: DateTime<Utc>,
}

/// GraphQL enum for worker status
#[derive(Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerStatusGraphQL {
    Idle,
    Running,
    Stopping,
    Error,
}

impl From<WorkerStatusType> for WorkerStatusGraphQL {
    fn from(status: WorkerStatusType) -> Self {
        match status {
            WorkerStatusType::Idle => WorkerStatusGraphQL::Idle,
            WorkerStatusType::Running => WorkerStatusGraphQL::Running,
            WorkerStatusType::Stopping => WorkerStatusGraphQL::Stopping,
            WorkerStatusType::Error => WorkerStatusGraphQL::Error,
        }
    }
}

impl From<WorkerStatusGraphQL> for WorkerStatusType {
    fn from(status: WorkerStatusGraphQL) -> Self {
        match status {
            WorkerStatusGraphQL::Idle => WorkerStatusType::Idle,
            WorkerStatusGraphQL::Running => WorkerStatusType::Running,
            WorkerStatusGraphQL::Stopping => WorkerStatusType::Stopping,
            WorkerStatusGraphQL::Error => WorkerStatusType::Error,
        }
    }
}

impl From<UnifiedWorkerStatus> for Worker {
    fn from(worker: UnifiedWorkerStatus) -> Self {
        Self {
            id: worker.id,
            status: worker.status.into(),
            task_count: worker.task_count,
            current_task: worker.current_task,
            uptime_seconds: worker.uptime_seconds,
            memory_usage_mb: worker.memory_usage_mb,
            cpu_usage_percent: worker.cpu_usage_percent,
            last_heartbeat: worker.last_heartbeat,
        }
    }
}

/// Input type for worker filtering
#[derive(InputObject)]
pub struct WorkerFiltersInput {
    pub status: Option<WorkerStatusGraphQL>,
    pub last_heartbeat_after: Option<DateTime<Utc>>,
}

/// Worker statistics
#[derive(SimpleObject)]
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