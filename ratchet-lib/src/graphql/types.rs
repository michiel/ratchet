use async_graphql::*;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use std::collections::HashMap;

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
    pub output_destinations: Option<Vec<OutputDestination>>,
}

/// Task execution result for direct execution
#[derive(SimpleObject)]
pub struct TaskExecutionResult {
    pub success: bool,
    pub output: Option<JsonValue>,
    pub error: Option<String>,
    pub duration_ms: i64,
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
    pub output_destinations: Option<Vec<OutputDestinationInput>>,
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

/// Unified task representation combining registry and database information
#[derive(SimpleObject)]
pub struct UnifiedTask {
    /// Database ID (if task exists in database)
    pub id: Option<i32>,
    /// Task UUID
    pub uuid: Uuid,
    /// Current version
    pub version: String,
    /// Task label/name
    pub label: String,
    /// Task description
    pub description: String,
    /// All available versions in registry
    pub available_versions: Vec<String>,
    /// Whether task is from registry
    pub registry_source: bool,
    /// Whether task is enabled for execution
    pub enabled: bool,
    /// When task was first created (in database)
    pub created_at: Option<DateTime<Utc>>,
    /// When task was last updated (in database)
    pub updated_at: Option<DateTime<Utc>>,
    /// When task was last validated
    pub validated_at: Option<DateTime<Utc>>,
    /// Whether task is synced between registry and database
    pub in_sync: bool,
}

/// Unified task list response
#[derive(SimpleObject)]
pub struct UnifiedTaskListResponse {
    pub tasks: Vec<UnifiedTask>,
    pub total: u64,
}

impl From<crate::services::UnifiedTask> for UnifiedTask {
    fn from(task: crate::services::UnifiedTask) -> Self {
        Self {
            id: task.id,
            uuid: task.uuid,
            version: task.version,
            label: task.label,
            description: task.description,
            available_versions: task.available_versions,
            registry_source: task.registry_source,
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
            in_sync: task.in_sync,
        }
    }
}

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

/// Output format enum
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum OutputFormat {
    Json,
    JsonCompact,
    Yaml,
    Csv,
    Raw,
    Template, // Note: GraphQL doesn't support variant with data, so template string needs to be separate
}

/// HTTP method enum
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

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