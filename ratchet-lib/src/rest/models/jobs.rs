use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::database::entities::jobs::{JobStatus, JobPriority as Priority};
use crate::output::OutputDestinationConfig;

#[derive(Debug, Serialize)]
pub struct JobResponse {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub execution_id: Option<i32>,
    pub schedule_id: Option<i32>,
    pub priority: Priority,
    pub status: JobStatus,
    pub input_data: Value,
    pub retry_count: i32,
    pub max_retries: i32,
    pub retry_delay_seconds: i32,
    pub error_message: Option<String>,
    pub error_details: Option<Value>,
    pub queued_at: DateTime<Utc>,
    pub process_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: Option<Value>,
    pub output_destinations: Option<Vec<OutputDestinationConfig>>,
}

#[derive(Debug, Serialize)]
pub struct JobDetailResponse {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub task_name: Option<String>,
    pub execution_id: Option<i32>,
    pub schedule_id: Option<i32>,
    pub priority: Priority,
    pub status: JobStatus,
    pub input_data: Value,
    pub retry_count: i32,
    pub max_retries: i32,
    pub retry_delay_seconds: i32,
    pub error_message: Option<String>,
    pub error_details: Option<Value>,
    pub queued_at: DateTime<Utc>,
    pub process_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: Option<Value>,
    pub output_destinations: Option<Vec<OutputDestinationConfig>>,
    pub queue_position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct JobCreateRequest {
    pub task_id: i32,
    pub input_data: Value,
    pub priority: Option<Priority>,
    pub process_at: Option<DateTime<Utc>>,
    pub max_retries: Option<i32>,
    pub retry_delay_seconds: Option<i32>,
    pub metadata: Option<Value>,
    pub output_destinations: Option<Vec<OutputDestinationConfig>>,
}

#[derive(Debug, Deserialize)]
pub struct JobUpdateRequest {
    pub priority: Option<Priority>,
    pub process_at: Option<DateTime<Utc>>,
    pub max_retries: Option<i32>,
    pub retry_delay_seconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct JobFilters {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub task_id: Option<i32>,
    pub schedule_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct JobQueueStats {
    pub total: u64,
    pub queued: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub retrying: u64,
    pub by_priority: PriorityStats,
}

#[derive(Debug, Serialize)]
pub struct PriorityStats {
    pub urgent: u64,
    pub high: u64,
    pub normal: u64,
    pub low: u64,
}

/// Request to test output destination configurations
#[derive(Debug, Deserialize)]
pub struct TestOutputDestinationsRequest {
    pub destinations: Vec<OutputDestinationConfig>,
}

/// Result of testing an output destination configuration
#[derive(Debug, Serialize)]
pub struct TestDestinationResult {
    pub index: usize,
    pub destination_type: String,
    pub success: bool,
    pub error: Option<String>,
    pub estimated_time_ms: u64,
}

/// Response for testing output destinations
#[derive(Debug, Serialize)]
pub struct TestOutputDestinationsResponse {
    pub results: Vec<TestDestinationResult>,
    pub overall_success: bool,
}

impl From<crate::database::entities::jobs::Model> for JobResponse {
    fn from(job: crate::database::entities::jobs::Model) -> Self {
        let output_destinations = job.output_destinations
            .and_then(|json| serde_json::from_value(json.into()).ok());
        
        Self {
            id: job.id,
            uuid: job.uuid,
            task_id: job.task_id,
            execution_id: job.execution_id,
            schedule_id: job.schedule_id,
            priority: job.priority,
            status: job.status,
            input_data: job.input_data.clone(),
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            retry_delay_seconds: job.retry_delay_seconds,
            error_message: job.error_message,
            error_details: job.error_details.clone(),
            queued_at: job.queued_at,
            process_at: job.process_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            metadata: job.metadata.clone(),
            output_destinations,
        }
    }
}