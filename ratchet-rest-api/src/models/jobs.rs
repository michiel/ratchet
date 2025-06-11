//! Job-related request and response models

use serde::{Deserialize, Serialize};
use ratchet_api_types::{ApiId, JobPriority, JobStatus};

/// Request to create a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub task_id: ApiId,
    pub input: serde_json::Value,
    pub priority: Option<JobPriority>,
    pub max_retries: Option<i32>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub output_destinations: Option<Vec<ratchet_api_types::UnifiedOutputDestination>>,
}

/// Request to update job status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobRequest {
    pub status: Option<JobStatus>,
    pub priority: Option<JobPriority>,
    pub max_retries: Option<i32>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

/// Job statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStats {
    pub total_jobs: u64,
    pub queued_jobs: u64,
    pub processing_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub cancelled_jobs: u64,
    pub retrying_jobs: u64,
    pub average_wait_time_ms: Option<f64>,
    pub jobs_last_24h: u64,
}