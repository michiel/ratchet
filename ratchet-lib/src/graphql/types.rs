use async_graphql::*;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Task representation in GraphQL
#[derive(SimpleObject)]
pub struct Task {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub path: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

/// Execution representation in GraphQL
#[derive(SimpleObject)]
pub struct Execution {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

/// Job representation in GraphQL
#[derive(SimpleObject)]
pub struct Job {
    pub id: i32,
    pub task_id: i32,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub queued_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// Schedule representation in GraphQL
#[derive(SimpleObject)]
pub struct Schedule {
    pub id: i32,
    pub task_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Execution status enum for GraphQL
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Job priority enum for GraphQL
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Job status enum for GraphQL
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Retrying,
    Cancelled,
}

/// Task statistics
#[derive(SimpleObject)]
pub struct TaskStats {
    pub total_tasks: u64,
    pub enabled_tasks: u64,
    pub disabled_tasks: u64,
}

/// Execution statistics
#[derive(SimpleObject)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub pending: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
}

/// Job queue statistics
#[derive(SimpleObject)]
pub struct JobStats {
    pub total_jobs: u64,
    pub queued: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub retrying: u64,
}

/// System health status
#[derive(SimpleObject)]
pub struct HealthStatus {
    pub database: bool,
    pub job_queue: bool,
    pub scheduler: bool,
    pub message: String,
}

/// Input types for mutations

/// Create task input
#[derive(InputObject)]
pub struct CreateTaskInput {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub path: String,
}

/// Update task input
#[derive(InputObject)]
pub struct UpdateTaskInput {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub path: Option<String>,
    pub enabled: Option<bool>,
}

/// Execute task input
#[derive(InputObject)]
pub struct ExecuteTaskInput {
    pub task_id: i32,
    pub input_data: JsonValue,
    pub priority: Option<JobPriority>,
}

/// Create schedule input
#[derive(InputObject)]
pub struct CreateScheduleInput {
    pub task_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub input_data: JsonValue,
}

/// Update schedule input
#[derive(InputObject)]
pub struct UpdateScheduleInput {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
    pub input_data: Option<JsonValue>,
}

/// Pagination input
#[derive(InputObject)]
pub struct PaginationInput {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

/// Task list response with pagination
#[derive(SimpleObject)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

/// Execution list response with pagination
#[derive(SimpleObject)]
pub struct ExecutionListResponse {
    pub executions: Vec<Execution>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

/// Job list response with pagination
#[derive(SimpleObject)]
pub struct JobListResponse {
    pub jobs: Vec<Job>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}