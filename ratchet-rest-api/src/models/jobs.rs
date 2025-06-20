//! Job-related request and response models

use serde::{Deserialize, Serialize};
// use utoipa::ToSchema; // temporarily disabled
use ratchet_api_types::{ApiId, JobPriority, JobStatus};

/// Request to create a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    /// ID of the task to queue for execution
    pub task_id: ApiId,

    /// Input data for the task execution
    pub input: serde_json::Value,

    /// Job priority level
    pub priority: Option<JobPriority>,

    /// Maximum number of retry attempts
    pub max_retries: Option<i32>,

    /// Optional scheduled execution time (ISO 8601 format)
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,

    /// Optional output destinations for job results
    pub output_destinations: Option<Vec<ratchet_api_types::UnifiedOutputDestination>>,
}

/// Request to update job status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobRequest {
    /// New job status
    pub status: Option<JobStatus>,

    /// Updated job priority
    pub priority: Option<JobPriority>,

    /// Updated maximum retry count
    pub max_retries: Option<i32>,

    /// Updated scheduled execution time
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,

    /// Error message if job failed
    pub error_message: Option<String>,
}

/// Job statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStats {
    /// Total number of jobs in the system
    pub total_jobs: u64,

    /// Number of jobs waiting in queue
    pub queued_jobs: u64,

    /// Number of jobs currently being processed
    pub processing_jobs: u64,

    /// Number of successfully completed jobs
    pub completed_jobs: u64,

    /// Number of failed jobs
    pub failed_jobs: u64,

    /// Number of cancelled jobs
    pub cancelled_jobs: u64,

    /// Number of jobs waiting to be retried
    pub retrying_jobs: u64,

    /// Average job wait time in milliseconds
    pub average_wait_time_ms: Option<f64>,

    /// Number of jobs processed in the last 24 hours
    pub jobs_last_24h: u64,
}
