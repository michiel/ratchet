use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub name: String,
    pub cron_expression: String,
    pub input_data: serde_json::Value,
    pub enabled: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub execution_count: i32,
    pub max_executions: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleDetailResponse {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub task_name: Option<String>,
    pub name: String,
    pub cron_expression: String,
    pub input_data: serde_json::Value,
    pub enabled: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub execution_count: i32,
    pub max_executions: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_exhausted: bool,
    pub runs_remaining: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleCreateRequest {
    pub task_id: i32,
    pub name: String,
    pub cron_expression: String,
    pub input_data: Value,
    pub enabled: Option<bool>,
    pub max_executions: Option<i32>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleUpdateRequest {
    pub name: Option<String>,
    pub cron_expression: Option<String>,
    pub input_data: Option<Value>,
    pub enabled: Option<bool>,
    pub max_executions: Option<i32>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleFilters {
    pub task_id: Option<i32>,
    pub enabled: Option<bool>,
    pub name_like: Option<String>,
}

impl From<ratchet_storage::Schedule> for ScheduleResponse {
    fn from(schedule: ratchet_storage::Schedule) -> Self {
        Self {
            id: schedule.id,
            uuid: schedule.uuid,
            task_id: schedule.task_id,
            name: schedule.name,
            cron_expression: schedule.cron_expression,
            input_data: schedule.input_data,
            enabled: schedule.enabled,
            next_run_at: schedule.next_run_at,
            last_run_at: schedule.last_run_at,
            execution_count: schedule.execution_count,
            max_executions: schedule.max_executions,
            metadata: Some(schedule.metadata),
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}
