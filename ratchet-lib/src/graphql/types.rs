use async_graphql::*;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use std::collections::HashMap;

// Re-export unified types for GraphQL
pub use crate::api::types::*;
pub use crate::api::pagination::*;
pub use crate::api::errors::ApiError as UnifiedApiError;

/// Legacy Task type (use UnifiedTask instead)
#[derive(SimpleObject)]
pub struct LegacyTask {
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

/// Use unified task type
pub type Task = UnifiedTask;

/// Legacy Execution type (use UnifiedExecution instead)
#[derive(SimpleObject)]
pub struct LegacyExecution {
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

/// Use unified execution type
pub type Execution = UnifiedExecution;

/// Legacy Job type (use UnifiedJob instead)
#[derive(SimpleObject)]
pub struct LegacyJob {
    pub id: i32,
    pub task_id: i32,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub queued_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub output_destinations: Option<Vec<UnifiedOutputDestination>>,
}

/// Use unified job type
pub type Job = UnifiedJob;

/// Task execution result for direct execution
#[derive(SimpleObject)]
pub struct TaskExecutionResult {
    pub success: bool,
    pub output: Option<JsonValue>,
    pub error: Option<String>,
    pub duration_ms: i64,
}

/// Legacy Schedule type (use UnifiedSchedule instead)
#[derive(SimpleObject)]
pub struct LegacySchedule {
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

/// Use unified schedule type
pub type Schedule = UnifiedSchedule;

// Re-export unified enums (already defined in api::types)
// pub use crate::api::types::{ExecutionStatus, JobPriority, JobStatus};

/// Legacy enums (kept for backward compatibility during migration)
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum LegacyExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum LegacyJobPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum LegacyJobStatus {
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
    pub id: ApiId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
}

/// Execute task input
#[derive(InputObject)]
pub struct ExecuteTaskInput {
    pub task_id: ApiId,
    pub input_data: JsonValue,
    pub priority: Option<JobPriority>,
    pub output_destinations: Option<Vec<OutputDestinationInput>>,
}

/// Create schedule input
#[derive(InputObject)]
pub struct CreateScheduleInput {
    pub task_id: ApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub input_data: JsonValue,
}

/// Update schedule input
#[derive(InputObject)]
pub struct UpdateScheduleInput {
    pub id: ApiId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
    pub input_data: Option<JsonValue>,
}

// Use unified pagination input
// pub use crate::api::pagination::PaginationInput;

/// Task list response with pagination (use unified ListResponse<UnifiedTask> instead)
pub type TaskListResponse = ListResponse<UnifiedTask>;

/// Execution list response with pagination (use unified ListResponse<UnifiedExecution> instead)
pub type ExecutionListResponse = ListResponse<UnifiedExecution>;

/// Job list response with pagination (use unified ListResponse<UnifiedJob> instead)
pub type JobListResponse = ListResponse<UnifiedJob>;

/// Legacy UnifiedTask type - use crate::api::types::UnifiedTask instead
/// Kept for backward compatibility during migration
#[derive(SimpleObject)]
pub struct LegacyUnifiedTask {
    pub id: Option<i32>,
    pub uuid: Uuid,
    pub version: String,
    pub label: String,
    pub description: String,
    pub available_versions: Vec<String>,
    pub registry_source: bool,
    pub enabled: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub validated_at: Option<DateTime<Utc>>,
    pub in_sync: bool,
}

/// Legacy UnifiedTaskListResponse - use ListResponse<UnifiedTask> instead
pub type UnifiedTaskListResponse = ListResponse<UnifiedTask>;

// Conversion is handled in api::conversions module

/// Output destination types for GraphQL
#[derive(Union)]
pub enum OutputDestination {
    Filesystem(FilesystemDestination),
    Webhook(WebhookDestination),
    Database(DatabaseDestination),
    S3(S3Destination),
}

/// Filesystem output destination
#[derive(SimpleObject)]
pub struct FilesystemDestination {
    pub path: String,
    pub format: OutputFormat,
    pub permissions: String, // Octal representation as string
    pub create_dirs: bool,
    pub overwrite: bool,
    pub backup_existing: bool,
}

/// Webhook output destination
#[derive(SimpleObject)]
pub struct WebhookDestination {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: i32,
    pub retry_policy: RetryPolicy,
    pub auth: Option<WebhookAuth>,
    pub content_type: Option<String>,
}

/// Database output destination
#[derive(SimpleObject)]
pub struct DatabaseDestination {
    pub connection_string: String,
    pub table_name: String,
    pub column_mappings: HashMap<String, String>,
}

/// S3 output destination
#[derive(SimpleObject)]
pub struct S3Destination {
    pub bucket: String,
    pub key_template: String,
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

// Use unified types from api module
pub use crate::api::types::{OutputFormat, HttpMethod};

/// Retry policy for delivery
#[derive(SimpleObject)]
pub struct RetryPolicy {
    pub max_attempts: i32,
    pub initial_delay_ms: i32,
    pub max_delay_ms: i32,
    pub backoff_multiplier: f64,
}

/// Webhook authentication
#[derive(SimpleObject)]
pub struct WebhookAuth {
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

/// Input types for output destinations
#[derive(InputObject)]
pub struct OutputDestinationInput {
    pub destination_type: DestinationType,
    pub filesystem: Option<FilesystemDestinationInput>,
    pub webhook: Option<WebhookDestinationInput>,
    pub database: Option<DatabaseDestinationInput>,
    pub s3: Option<S3DestinationInput>,
}

/// Destination type enum for input
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum DestinationType {
    Filesystem,
    Webhook,
    Database,
    S3,
}

/// Filesystem destination input
#[derive(InputObject)]
pub struct FilesystemDestinationInput {
    pub path: String,
    pub format: OutputFormat,
    pub permissions: Option<String>,
    pub create_dirs: Option<bool>,
    pub overwrite: Option<bool>,
    pub backup_existing: Option<bool>,
}

/// Webhook destination input
#[derive(InputObject)]
pub struct WebhookDestinationInput {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<i32>,
    pub retry_policy: Option<RetryPolicyInput>,
    pub auth: Option<WebhookAuthInput>,
    pub content_type: Option<String>,
}

/// Database destination input
#[derive(InputObject)]
pub struct DatabaseDestinationInput {
    pub connection_string: String,
    pub table_name: String,
    pub column_mappings: HashMap<String, String>,
}

/// S3 destination input
#[derive(InputObject)]
pub struct S3DestinationInput {
    pub bucket: String,
    pub key_template: String,
    pub region: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

/// Retry policy input
#[derive(InputObject)]
pub struct RetryPolicyInput {
    pub max_attempts: i32,
    pub initial_delay_ms: i32,
    pub max_delay_ms: i32,
    pub backoff_multiplier: f64,
}

/// Webhook auth input
#[derive(InputObject)]
pub struct WebhookAuthInput {
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

/// Test output destinations input
#[derive(InputObject)]
pub struct TestOutputDestinationsInput {
    pub destinations: Vec<OutputDestinationInput>,
}

/// Test destination result
#[derive(SimpleObject)]
pub struct TestDestinationResult {
    pub index: i32,
    pub destination_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub estimated_time_ms: i32,
}