use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ids::ApiId;
use crate::enums::*;

#[cfg(feature = "graphql")]
use async_graphql::*;

/// Unified Task representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedOutputDestination {
    pub destination_type: String,
    pub template: Option<String>,
    pub filesystem: Option<UnifiedFilesystemConfig>,
    pub webhook: Option<UnifiedWebhookConfig>,
}

/// Unified Filesystem Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedFilesystemConfig {
    pub path: String,
    pub format: OutputFormat,
    pub compression: Option<CompressionType>,
    pub permissions: Option<String>,
}

/// Unified Webhook Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedRetryPolicy {
    pub max_attempts: i32,
    pub initial_delay_seconds: i32,
    pub max_delay_seconds: i32,
    pub backoff_multiplier: f64,
}

/// Unified Webhook Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWebhookAuth {
    pub auth_type: String,
    pub bearer: Option<UnifiedBearerAuth>,
    pub basic: Option<UnifiedBasicAuth>,
    pub api_key: Option<UnifiedApiKeyAuth>,
}

/// Bearer Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
pub struct UnifiedBearerAuth {
    pub token: String,
}

/// Basic Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
pub struct UnifiedBasicAuth {
    pub username: String,
    pub password: String,
}

/// API Key Authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[serde(rename_all = "camelCase")]
pub struct UnifiedApiKeyAuth {
    pub key: String,
    pub header_name: String,
}

/// Worker status representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
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