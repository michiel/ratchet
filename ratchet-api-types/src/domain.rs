use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::*;
use crate::ids::ApiId;

#[cfg(feature = "graphql")]
use async_graphql::*;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

// #[cfg(feature = "openapi")]
// use serde_json::json;

/// Unified Task representation with full repository support
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedTask {
    pub id: ApiId,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub enabled: bool,
    pub registry_source: bool,
    pub available_versions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
    pub in_sync: bool,

    // New fields for full task storage
    pub source_code: String,
    pub source_type: String,
    pub repository_info: TaskRepositoryInfo,
    pub is_editable: bool,
    pub sync_status: String,
    pub needs_push: bool,
    pub last_synced_at: Option<DateTime<Utc>>,

    // Additional fields for detailed view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Task repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskRepositoryInfo {
    pub repository_id: ApiId,
    pub repository_name: String,
    pub repository_type: String,
    pub repository_path: String,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub can_push: bool,
    pub auto_push: bool,
}

/// Repository representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedTaskRepository {
    pub id: ApiId,
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub sync_enabled: bool,
    pub sync_interval_minutes: Option<i32>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub is_default: bool,
    pub is_writable: bool,
    pub watch_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub push_on_change: bool,
    pub task_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create task request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_code: String,
    pub source_type: Option<String>,
    pub version: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub repository_id: Option<ApiId>,
    pub repository_path: Option<String>,
}

/// Update task source request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskSourceRequest {
    pub source_code: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub version: Option<String>,
    pub change_description: Option<String>,
}

/// Create repository request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub is_default: Option<bool>,
    pub is_writable: Option<bool>,
    pub watch_patterns: Option<Vec<String>>,
    pub ignore_patterns: Option<Vec<String>>,
    pub push_on_change: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

/// Update repository request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UpdateRepositoryRequest {
    pub name: Option<String>,
    pub uri: Option<String>,
    pub branch: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub is_default: Option<bool>,
    pub is_writable: Option<bool>,
    pub watch_patterns: Option<Vec<String>>,
    pub ignore_patterns: Option<Vec<String>>,
    pub push_on_change: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

/// Repository connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Sync result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub repository_id: ApiId,
    pub tasks_added: u32,
    pub tasks_updated: u32,
    pub tasks_deleted: u32,
    pub conflicts: Vec<TaskConflict>,
    pub errors: Vec<String>,
}

/// Push result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub task_id: ApiId,
    pub repository_id: ApiId,
    pub repository_path: String,
    pub success: bool,
    pub commit_hash: Option<String>,
    pub error: Option<String>,
}

/// Task conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskConflict {
    pub task_id: ApiId,
    pub repository_id: ApiId,
    pub conflict_type: String,
    pub local_checksum: String,
    pub remote_checksum: String,
    pub auto_resolvable: bool,
}

/// Repository sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct RepositorySyncStatus {
    pub repository_id: ApiId,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub sync_error: Option<String>,
    pub health: RepositoryHealth,
    pub task_count: u64,
}

/// Repository health information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct RepositoryHealth {
    pub accessible: bool,
    pub writable: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub error_count: u32,
    pub message: String,
}

/// Task assignment request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct AssignTaskRequest {
    pub repository_id: ApiId,
    pub repository_path: Option<String>,
    pub sync_immediately: Option<bool>,
}

/// Task repository assignment information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskRepositoryAssignment {
    pub task_id: ApiId,
    pub repository_id: ApiId,
    pub repository_name: String,
    pub repository_path: String,
    pub can_push: bool,
    pub auto_push: bool,
    pub sync_status: String,
    pub needs_push: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// Enhanced repository creation request with sync options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CreateRepositoryWithSyncRequest {
    #[serde(flatten)]
    pub repository: CreateRepositoryRequest,
    pub test_connection: Option<bool>,
    pub initial_sync: Option<bool>,
}

/// Enhanced repository update request with sync options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UpdateRepositoryWithSyncRequest {
    #[serde(flatten)]
    pub repository: UpdateRepositoryRequest,
    pub test_connection: Option<bool>,
    pub sync_after_update: Option<bool>,
}

/// Unified Execution representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedExecution {
    pub id: ApiId,
    pub uuid: Uuid,
    pub task_id: ApiId,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub http_requests: Option<serde_json::Value>,
    pub recording_path: Option<String>,

    // Computed fields
    pub can_retry: bool,
    pub can_cancel: bool,
    pub progress: Option<f32>,
}

/// Unified Job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedJob {
    pub id: ApiId,
    pub task_id: ApiId,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub queued_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub output_destinations: Option<Vec<UnifiedOutputDestination>>,
}

/// Unified Schedule representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSchedule {
    pub id: ApiId,
    pub task_id: ApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub output_destinations: Option<Vec<UnifiedOutputDestination>>,
}

/// Unified Output Destination representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Configuration for where to send execution results",
    example = json!({
        "destinationType": "webhook",
        "webhook": {
            "url": "https://your-webhook-endpoint.com/api/notifications",
            "method": "POST",
            "contentType": "application/json",
            "timeoutSeconds": 30,
            "retryPolicy": {
                "maxAttempts": 3,
                "initialDelaySeconds": 1,
                "maxDelaySeconds": 5,
                "backoffMultiplier": 2.0
            },
            "authentication": {
                "authType": "bearer",
                "bearer": {
                    "token": "your-bearer-token"
                }
            }
        }
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedOutputDestination {
    /// Type of destination: "webhook", "filesystem", "database", or "stdio"
    #[cfg_attr(feature = "openapi", schema(example = "webhook"))]
    pub destination_type: String,

    /// Optional template for formatting output
    #[cfg_attr(feature = "openapi", schema(example = "Execution completed: {{status}}"))]
    pub template: Option<String>,

    /// Filesystem configuration (when destination_type is "filesystem")
    pub filesystem: Option<UnifiedFilesystemConfig>,

    /// Webhook configuration (when destination_type is "webhook")
    pub webhook: Option<UnifiedWebhookConfig>,

    /// Stdio configuration (when destination_type is "stdio")
    pub stdio: Option<UnifiedStdioConfig>,
}

/// Unified Filesystem Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Configuration for filesystem output destinations",
    example = json!({
        "path": "/var/ratchet/outputs/results.json",
        "format": "JSON",
        "compression": "GZIP",
        "permissions": "644"
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedFilesystemConfig {
    /// File path to write results to
    #[cfg_attr(
        feature = "openapi",
        schema(example = "/var/ratchet/outputs/results.json", max_length = 4096)
    )]
    pub path: String,

    /// Output format for the file
    pub format: OutputFormat,

    /// Optional compression
    pub compression: Option<CompressionType>,

    /// Optional file permissions
    #[cfg_attr(feature = "openapi", schema(example = "644"))]
    pub permissions: Option<String>,
}

/// Unified Webhook Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Configuration for webhook output destinations",
    example = json!({
        "url": "https://your-webhook-endpoint.com/api/notifications",
        "method": "POST",
        "timeoutSeconds": 30,
        "contentType": "application/json",
        "retryPolicy": {
            "maxAttempts": 3,
            "initialDelaySeconds": 1,
            "maxDelaySeconds": 5,
            "backoffMultiplier": 2.0
        },
        "authentication": {
            "authType": "bearer",
            "bearer": {
                "token": "your-bearer-token"
            }
        }
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWebhookConfig {
    /// The webhook URL to send results to
    #[cfg_attr(
        feature = "openapi",
        schema(example = "https://your-webhook-endpoint.com/api/notifications", max_length = 2048)
    )]
    pub url: String,

    /// HTTP method to use for the webhook
    pub method: HttpMethod,

    /// Request timeout in seconds
    #[cfg_attr(feature = "openapi", schema(example = 30, minimum = 1, maximum = 300))]
    pub timeout_seconds: i32,

    /// Content type for the request
    #[cfg_attr(feature = "openapi", schema(example = "application/json"))]
    pub content_type: Option<String>,

    /// Retry policy for failed requests
    pub retry_policy: Option<UnifiedRetryPolicy>,

    /// Authentication configuration
    pub authentication: Option<UnifiedWebhookAuth>,
}

/// Unified Retry Policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Configuration for retrying failed webhook requests",
    example = json!({
        "maxAttempts": 3,
        "initialDelaySeconds": 1,
        "maxDelaySeconds": 5,
        "backoffMultiplier": 2.0
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedRetryPolicy {
    /// Maximum number of retry attempts
    #[cfg_attr(feature = "openapi", schema(example = 3, minimum = 1, maximum = 10))]
    pub max_attempts: i32,

    /// Initial delay before first retry
    #[cfg_attr(feature = "openapi", schema(example = 1))]
    pub initial_delay_seconds: i32,

    /// Maximum delay between retries
    #[cfg_attr(feature = "openapi", schema(example = 5))]
    pub max_delay_seconds: i32,

    /// Backoff multiplier for exponential backoff
    #[cfg_attr(feature = "openapi", schema(example = 2.0, minimum = 1.0, maximum = 10.0))]
    pub backoff_multiplier: f64,
}

/// Unified Webhook Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Authentication configuration for webhook requests",
    example = json!({
        "authType": "bearer",
        "bearer": {
            "token": "your-bearer-token"
        }
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWebhookAuth {
    /// Type of authentication: "bearer", "basic", or "api_key"
    #[cfg_attr(feature = "openapi", schema(example = "bearer"))]
    pub auth_type: String,

    /// Bearer token authentication (when auth_type is "bearer")
    pub bearer: Option<UnifiedBearerAuth>,

    /// Basic authentication (when auth_type is "basic")
    pub basic: Option<UnifiedBasicAuth>,

    /// API key authentication (when auth_type is "api_key")
    pub api_key: Option<UnifiedApiKeyAuth>,
}

/// Bearer Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Bearer token authentication configuration",
    example = json!({
        "token": "your-bearer-token"
    })
))]
pub struct UnifiedBearerAuth {
    /// The bearer token
    #[cfg_attr(feature = "openapi", schema(example = "your-bearer-token", max_length = 1024))]
    pub token: String,
}

/// Basic Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Basic authentication configuration",
    example = json!({
        "username": "your-username",
        "password": "your-password"
    })
))]
pub struct UnifiedBasicAuth {
    /// Username for basic authentication
    #[cfg_attr(feature = "openapi", schema(example = "your-username", max_length = 255))]
    pub username: String,

    /// Password for basic authentication
    #[cfg_attr(feature = "openapi", schema(example = "your-password", max_length = 255))]
    pub password: String,
}

/// API Key Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "API key authentication configuration",
    example = json!({
        "key": "your-api-key",
        "headerName": "X-API-Key"
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedApiKeyAuth {
    /// The API key value
    #[cfg_attr(feature = "openapi", schema(example = "your-api-key", max_length = 1024))]
    pub key: String,

    /// The header name to send the API key in
    #[cfg_attr(feature = "openapi", schema(example = "X-API-Key", max_length = 100))]
    pub header_name: String,
}

/// Unified Stdio Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "openapi", schema(
    description = "Configuration for stdio output destinations",
    example = json!({
        "stream": "stdout",
        "format": "JSON",
        "includeMetadata": false,
        "lineBuffered": true,
        "prefix": "[HEARTBEAT] "
    })
))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedStdioConfig {
    /// Stream to write to: "stdout" or "stderr"
    #[cfg_attr(feature = "openapi", schema(example = "stdout"))]
    pub stream: String,

    /// Output format for the stream
    pub format: OutputFormat,

    /// Whether to include full task metadata
    #[cfg_attr(feature = "openapi", schema(example = false))]
    pub include_metadata: bool,

    /// Whether to use line buffering
    #[cfg_attr(feature = "openapi", schema(example = true))]
    pub line_buffered: bool,

    /// Optional prefix for each output line
    #[cfg_attr(feature = "openapi", schema(example = "[HEARTBEAT] "))]
    pub prefix: Option<String>,
}

/// Worker status representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWorkerStatus {
    pub id: String,
    pub status: WorkerStatusType,
    pub task_count: i32,
    pub current_task: Option<String>,
    pub uptime_seconds: i64,
    pub memory_usage_mb: Option<u64>,
    pub cpu_usage_percent: Option<f32>,
    pub last_heartbeat: DateTime<Utc>,
}

/// Unified User representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedUser {
    pub id: ApiId,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    // Password hash is never included in API responses
}

/// Unified Session representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSession {
    pub id: ApiId,
    pub session_id: String,
    pub user_id: ApiId,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub is_active: bool,
    // JWT ID and metadata are internal fields not exposed via API
}

/// Unified API Key representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedApiKey {
    pub id: ApiId,
    pub name: String,
    pub user_id: ApiId,
    pub key_prefix: String, // Only prefix shown for security
    pub permissions: ApiKeyPermissions,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub usage_count: i64,
    // Key hash is never included in API responses
}
