//! Unified API types for Ratchet REST and GraphQL APIs
//!
//! This crate provides consistent type definitions that can be used across
//! both REST and GraphQL API implementations, reducing duplication and
//! ensuring API consistency.

pub mod conversions;
pub mod domain;
pub mod enums;
pub mod errors;
pub mod ids;
pub mod pagination;

// Re-export main types for convenience
pub use domain::{
    ConnectionTestResult, CreateRepositoryRequest, CreateTaskRequest, PushResult, SyncResult, TaskConflict,
    TaskRepositoryInfo, UnifiedApiKey, UnifiedApiKeyAuth, UnifiedBasicAuth, UnifiedBearerAuth, UnifiedExecution, 
    UnifiedFilesystemConfig, UnifiedJob, UnifiedOutputDestination, UnifiedRetryPolicy, UnifiedSchedule, 
    UnifiedSession, UnifiedStdioConfig, UnifiedTask, UnifiedTaskRepository, UnifiedUser, UnifiedWebhookAuth, 
    UnifiedWebhookConfig, UnifiedWorkerStatus, UpdateRepositoryRequest, UpdateTaskSourceRequest,
};
pub use enums::{
    ApiKeyPermissions, CompressionType, ExecutionStatus, HttpMethod, JobPriority, JobStatus, OutputFormat, UserRole,
    WorkerStatusType,
};
pub use errors::ApiError;
pub use ids::ApiId;
pub use pagination::{ListResponse, PaginationInput};
