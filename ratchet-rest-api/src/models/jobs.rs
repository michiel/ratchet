//! Job-related request and response models

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use ratchet_api_types::{ApiId, JobPriority, JobStatus};

/// Request to create a new job
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "taskId": "task_123",
    "input": {
        "filename": "data.csv",
        "format": "csv"
    },
    "priority": "high",
    "maxRetries": 3,
    "scheduledFor": "2023-12-07T15:30:00Z"
}))]
pub struct CreateJobRequest {
    /// ID of the task to queue for execution
    #[schema(example = "task_123")]
    pub task_id: ApiId,
    
    /// Input data for the task execution
    #[schema(example = json!({"filename": "data.csv", "format": "csv"}))]
    pub input: serde_json::Value,
    
    /// Job priority level
    #[schema(example = "high")]
    pub priority: Option<JobPriority>,
    
    /// Maximum number of retry attempts
    #[schema(example = 3)]
    pub max_retries: Option<i32>,
    
    /// Optional scheduled execution time (ISO 8601 format)
    #[schema(example = "2023-12-07T15:30:00Z")]
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Optional output destinations for job results
    pub output_destinations: Option<Vec<ratchet_api_types::UnifiedOutputDestination>>,
}

/// Request to update job status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "status": "processing",
    "priority": "urgent",
    "maxRetries": 5
}))]
pub struct UpdateJobRequest {
    /// New job status
    #[schema(example = "processing")]
    pub status: Option<JobStatus>,
    
    /// Updated job priority
    #[schema(example = "urgent")]
    pub priority: Option<JobPriority>,
    
    /// Updated maximum retry count
    #[schema(example = 5)]
    pub max_retries: Option<i32>,
    
    /// Updated scheduled execution time
    #[schema(example = "2023-12-07T16:00:00Z")]
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Error message if job failed
    #[schema(example = "Task validation failed: missing required field")]
    pub error_message: Option<String>,
}

/// Job statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "totalJobs": 2456,
    "queuedJobs": 23,
    "processingJobs": 7,
    "completedJobs": 2301,
    "failedJobs": 89,
    "cancelledJobs": 36,
    "retryingJobs": 5,
    "averageWaitTimeMs": 1234.5,
    "jobsLast24h": 156
}))]
pub struct JobStats {
    /// Total number of jobs in the system
    #[schema(example = 2456)]
    pub total_jobs: u64,
    
    /// Number of jobs waiting in queue
    #[schema(example = 23)]
    pub queued_jobs: u64,
    
    /// Number of jobs currently being processed
    #[schema(example = 7)]
    pub processing_jobs: u64,
    
    /// Number of successfully completed jobs
    #[schema(example = 2301)]
    pub completed_jobs: u64,
    
    /// Number of failed jobs
    #[schema(example = 89)]
    pub failed_jobs: u64,
    
    /// Number of cancelled jobs
    #[schema(example = 36)]
    pub cancelled_jobs: u64,
    
    /// Number of jobs waiting to be retried
    #[schema(example = 5)]
    pub retrying_jobs: u64,
    
    /// Average job wait time in milliseconds
    #[schema(example = 1234.5)]
    pub average_wait_time_ms: Option<f64>,
    
    /// Number of jobs processed in the last 24 hours
    #[schema(example = 156)]
    pub jobs_last_24h: u64,
}