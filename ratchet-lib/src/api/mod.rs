pub mod conversions;
pub mod errors;
pub mod pagination;
/// Unified API types and utilities for consistent REST and GraphQL APIs
pub mod types;

// Re-export types from ratchet-api-types for backward compatibility
pub use ratchet_api_types::{
    ApiId,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    UnifiedOutputDestination, UnifiedFilesystemConfig, UnifiedWebhookConfig,
    UnifiedRetryPolicy, UnifiedWebhookAuth, UnifiedBearerAuth,
    UnifiedBasicAuth, UnifiedApiKeyAuth, UnifiedWorkerStatus,
    ExecutionStatus, JobPriority, JobStatus, OutputFormat,
    CompressionType, HttpMethod, WorkerStatusType,
    PaginationInput, ListResponse
};

pub use ratchet_api_types::errors::{ApiError, ApiResult};
pub use ratchet_api_types::pagination::PaginationMeta;
