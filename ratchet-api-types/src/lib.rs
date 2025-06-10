//! Unified API types for Ratchet REST and GraphQL APIs
//!
//! This crate provides consistent type definitions that can be used across
//! both REST and GraphQL API implementations, reducing duplication and
//! ensuring API consistency.

pub mod ids;
pub mod domain;
pub mod enums;
pub mod pagination;
pub mod errors;
pub mod conversions;

// Re-export main types for convenience
pub use ids::ApiId;
pub use domain::{
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    UnifiedOutputDestination, UnifiedFilesystemConfig, UnifiedWebhookConfig,
    UnifiedRetryPolicy, UnifiedWebhookAuth, UnifiedBearerAuth,
    UnifiedBasicAuth, UnifiedApiKeyAuth, UnifiedWorkerStatus
};
pub use enums::{
    ExecutionStatus, JobPriority, JobStatus, OutputFormat,
    CompressionType, HttpMethod, WorkerStatusType
};
pub use errors::ApiError;
pub use pagination::{PaginationInput, ListResponse};