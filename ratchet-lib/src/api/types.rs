/// Unified API types for consistent representation across REST and GraphQL
use async_graphql::{*, scalar};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unified ID type that works consistently across both APIs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiId(pub String);

impl ApiId {
    /// Create from database integer ID
    pub fn from_i32(id: i32) -> Self {
        Self(id.to_string())
    }
    
    /// Create from UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid.to_string())
    }
    
    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    
    /// Get as string (always available)
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Try to parse as integer (for database IDs)
    pub fn as_i32(&self) -> Option<i32> {
        self.0.parse().ok()
    }
    
    /// Try to parse as UUID
    pub fn as_uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.0).ok()
    }
}

impl std::fmt::Display for ApiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for ApiId {
    fn from(id: i32) -> Self {
        Self::from_i32(id)
    }
}

impl From<Uuid> for ApiId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

impl From<String> for ApiId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ApiId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// GraphQL scalar implementation
scalar!(
    ApiId,
    "ApiId",
    "A unified ID that accepts both strings and numbers"
);

/// Unified Task representation
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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
    
    // Additional fields for detailed view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Unified Execution representation
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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
}

/// Unified Output Destination representation
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedOutputDestination {
    pub destination_type: String,
    pub template: Option<String>,
    pub filesystem: Option<UnifiedFilesystemConfig>,
    pub webhook: Option<UnifiedWebhookConfig>,
}

/// Unified Filesystem Configuration
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedFilesystemConfig {
    pub path: String,
    pub format: OutputFormat,
    pub compression: Option<CompressionType>,
    pub permissions: Option<String>,
}

/// Unified Webhook Configuration
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWebhookConfig {
    pub url: String,
    pub method: HttpMethod,
    pub timeout_seconds: i32,
    pub content_type: Option<String>,
    pub retry_policy: Option<UnifiedRetryPolicy>,
    pub authentication: Option<UnifiedWebhookAuth>,
}

/// Unified Retry Policy
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedRetryPolicy {
    pub max_attempts: i32,
    pub initial_delay_seconds: i32,
    pub max_delay_seconds: i32,
    pub backoff_multiplier: f64,
}

/// Unified Webhook Authentication
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWebhookAuth {
    pub auth_type: String,
    pub bearer: Option<UnifiedBearerAuth>,
    pub basic: Option<UnifiedBasicAuth>,
    pub api_key: Option<UnifiedApiKeyAuth>,
}

/// Bearer Authentication
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct UnifiedBearerAuth {
    pub token: String,
}

/// Basic Authentication
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct UnifiedBasicAuth {
    pub username: String,
    pub password: String,
}

/// API Key Authentication
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedApiKeyAuth {
    pub key: String,
    pub header_name: String,
}

/// Unified enums that work in both REST and GraphQL

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutputFormat {
    Json,
    Yaml,
    Csv,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompressionType {
    Gzip,
    Zstd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Worker status representation
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Enum)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerStatusType {
    Idle,
    Running,
    Stopping,
    Error,
}