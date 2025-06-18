//! Job entity definition

use super::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Job entity representing a queued task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Primary key
    pub id: i32,

    /// Unique identifier
    pub uuid: Uuid,

    /// Foreign key to the task
    pub task_id: i32,

    /// Foreign key to the execution (when job is processed)
    pub execution_id: Option<i32>,

    /// Foreign key to the schedule (if job was created by a schedule)
    pub schedule_id: Option<i32>,

    /// Job priority
    pub priority: JobPriority,

    /// Job status
    pub status: JobStatus,

    /// Input data for the task
    pub input_data: serde_json::Value,

    /// Number of retry attempts
    pub retry_count: i32,

    /// Maximum number of retries allowed
    pub max_retries: i32,

    /// Retry delay in seconds
    pub retry_delay_seconds: i32,

    /// Error message if job failed
    pub error_message: Option<String>,

    /// Detailed error information
    pub error_details: Option<serde_json::Value>,

    /// When the job was queued
    pub queued_at: DateTime<Utc>,

    /// When the job should be processed (for delayed jobs)
    pub process_at: Option<DateTime<Utc>>,

    /// When the job started processing
    pub started_at: Option<DateTime<Utc>>,

    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Job metadata
    pub metadata: serde_json::Value,

    /// Output destinations for job results
    pub output_destinations: Option<serde_json::Value>,

    /// When the job was created
    pub created_at: DateTime<Utc>,

    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
}

/// Job priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum JobPriority {
    /// Low priority job
    Low = 1,

    /// Normal priority job (default)
    #[default]
    Normal = 2,

    /// High priority job
    High = 3,

    /// Urgent priority job
    Urgent = 4,
}

/// Job status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum JobStatus {
    /// Job is queued and waiting to be processed
    #[default]
    Queued,

    /// Job is currently being processed
    Processing,

    /// Job completed successfully
    Completed,

    /// Job failed
    Failed,

    /// Job was cancelled
    Cancelled,

    /// Job is being retried
    Retrying,

    /// Job is scheduled for future processing
    Scheduled,
}

impl Entity for Job {
    fn id(&self) -> i32 {
        self.id
    }

    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl Job {
    /// Create a new job
    pub fn new(task_id: i32, input_data: serde_json::Value) -> Self {
        let now = Utc::now();

        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            task_id,
            execution_id: None,
            schedule_id: None,
            priority: JobPriority::Normal,
            status: JobStatus::Queued,
            input_data,
            retry_count: 0,
            max_retries: 3,
            retry_delay_seconds: 5,
            error_message: None,
            error_details: None,
            queued_at: now,
            process_at: None,
            started_at: None,
            completed_at: None,
            metadata: serde_json::json!({}),
            output_destinations: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a scheduled job
    pub fn new_scheduled(
        task_id: i32,
        input_data: serde_json::Value,
        process_at: DateTime<Utc>,
    ) -> Self {
        let mut job = Self::new(task_id, input_data);
        job.status = JobStatus::Scheduled;
        job.process_at = Some(process_at);
        job
    }

    /// Create a job from a schedule
    pub fn from_schedule(task_id: i32, schedule_id: i32, input_data: serde_json::Value) -> Self {
        let mut job = Self::new(task_id, input_data);
        job.schedule_id = Some(schedule_id);
        job
    }

    /// Set job priority
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self.updated_at = Utc::now();
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, max_retries: i32, delay_seconds: i32) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_seconds = delay_seconds;
        self.updated_at = Utc::now();
        self
    }

    /// Set output destinations
    pub fn with_output_destinations(mut self, destinations: serde_json::Value) -> Self {
        self.output_destinations = Some(destinations);
        self.updated_at = Utc::now();
        self
    }

    /// Start processing the job
    pub fn start_processing(&mut self, execution_id: i32) {
        self.status = JobStatus::Processing;
        self.execution_id = Some(execution_id);
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Complete the job successfully
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Fail the job
    pub fn fail(&mut self, error_message: String, error_details: Option<serde_json::Value>) {
        self.status = JobStatus::Failed;
        self.error_message = Some(error_message);
        self.error_details = error_details;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Cancel the job
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Retry the job
    pub fn retry(&mut self) -> Result<(), String> {
        if self.retry_count >= self.max_retries {
            return Err("Maximum retries exceeded".to_string());
        }

        self.retry_count += 1;
        self.status = JobStatus::Retrying;
        self.error_message = None;
        self.error_details = None;
        self.execution_id = None;
        self.started_at = None;
        self.completed_at = None;

        // Schedule retry with delay
        let retry_delay = std::time::Duration::from_secs(
            (self.retry_delay_seconds * (2_i32.pow(self.retry_count as u32 - 1))) as u64,
        );
        self.process_at = Some(Utc::now() + chrono::Duration::from_std(retry_delay).unwrap());
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Reset job for retry
    pub fn reset_for_retry(&mut self) {
        self.status = JobStatus::Queued;
        self.process_at = None;
        self.updated_at = Utc::now();
    }

    /// Check if the job is ready to be processed
    pub fn is_ready_to_process(&self) -> bool {
        match self.status {
            JobStatus::Queued => true,
            JobStatus::Scheduled | JobStatus::Retrying => self
                .process_at
                .is_none_or(|process_time| Utc::now() >= process_time),
            _ => false,
        }
    }

    /// Check if the job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        )
    }

    /// Check if the job is active (processing or queued)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            JobStatus::Queued | JobStatus::Processing | JobStatus::Retrying | JobStatus::Scheduled
        )
    }

    /// Check if the job was successful
    pub fn is_successful(&self) -> bool {
        matches!(self.status, JobStatus::Completed)
    }

    /// Check if the job failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, JobStatus::Failed)
    }

    /// Check if the job can be retried
    pub fn can_retry(&self) -> bool {
        self.is_failed() && self.retry_count < self.max_retries
    }

    /// Get the next retry time
    pub fn next_retry_time(&self) -> Option<DateTime<Utc>> {
        if self.status == JobStatus::Retrying {
            self.process_at
        } else {
            None
        }
    }

    /// Calculate job age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.queued_at).num_seconds()
    }

    /// Calculate processing duration in seconds
    pub fn processing_duration_seconds(&self) -> Option<i64> {
        if let Some(started) = self.started_at {
            let end_time = self.completed_at.unwrap_or_else(Utc::now);
            Some((end_time - started).num_seconds())
        } else {
            None
        }
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    /// Get priority weight for queue ordering
    pub fn priority_weight(&self) -> i32 {
        self.priority as i32 * 1000 - self.age_seconds() as i32
    }
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobPriority::Low => write!(f, "low"),
            JobPriority::Normal => write!(f, "normal"),
            JobPriority::High => write!(f, "high"),
            JobPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for JobPriority {
    type Err = crate::StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(JobPriority::Low),
            "normal" => Ok(JobPriority::Normal),
            "high" => Ok(JobPriority::High),
            "urgent" => Ok(JobPriority::Urgent),
            _ => Err(crate::StorageError::ValidationFailed(format!(
                "Invalid job priority: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "queued"),
            JobStatus::Processing => write!(f, "processing"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
            JobStatus::Retrying => write!(f, "retrying"),
            JobStatus::Scheduled => write!(f, "scheduled"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = crate::StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(JobStatus::Queued),
            "processing" => Ok(JobStatus::Processing),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            "retrying" => Ok(JobStatus::Retrying),
            "scheduled" => Ok(JobStatus::Scheduled),
            _ => Err(crate::StorageError::ValidationFailed(format!(
                "Invalid job status: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new(1, serde_json::json!({"test": "data"}));

        assert_eq!(job.task_id, 1);
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.priority, JobPriority::Normal);
        assert_eq!(job.retry_count, 0);
        assert!(job.is_active());
        assert!(job.is_ready_to_process());
    }

    #[test]
    fn test_job_lifecycle() {
        let mut job = Job::new(1, serde_json::json!({"test": "data"}));

        // Start processing
        job.start_processing(100);
        assert_eq!(job.status, JobStatus::Processing);
        assert_eq!(job.execution_id, Some(100));
        assert!(job.started_at.is_some());
        assert!(job.is_active());
        assert!(!job.is_ready_to_process());

        // Complete job
        job.complete();
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.completed_at.is_some());
        assert!(job.is_terminal());
        assert!(job.is_successful());
    }

    #[test]
    fn test_job_failure_and_retry() {
        let mut job = Job::new(1, serde_json::json!({"test": "data"}));
        job.max_retries = 2;

        // Start and fail job
        job.start_processing(100);
        job.fail(
            "Connection error".to_string(),
            Some(serde_json::json!({"code": "CONN_ERROR"})),
        );

        assert_eq!(job.status, JobStatus::Failed);
        assert!(job.is_failed());
        assert!(job.can_retry());

        // Retry job
        assert!(job.retry().is_ok());
        assert_eq!(job.status, JobStatus::Retrying);
        assert_eq!(job.retry_count, 1);
        assert!(job.process_at.is_some());
        assert!(job.next_retry_time().is_some());

        // Reset for retry
        job.reset_for_retry();
        assert_eq!(job.status, JobStatus::Queued);
        assert!(job.is_ready_to_process());
    }

    #[test]
    fn test_job_scheduled() {
        let future_time = Utc::now() + chrono::Duration::hours(1);
        let job = Job::new_scheduled(1, serde_json::json!({"test": "data"}), future_time);

        assert_eq!(job.status, JobStatus::Scheduled);
        assert!(job.process_at.is_some());
        assert!(!job.is_ready_to_process()); // Not ready yet

        // Test with past time
        let past_time = Utc::now() - chrono::Duration::hours(1);
        let mut job = Job::new_scheduled(1, serde_json::json!({"test": "data"}), past_time);
        job.process_at = Some(past_time);
        assert!(job.is_ready_to_process()); // Ready now
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::Urgent > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Normal);
        assert!(JobPriority::Normal > JobPriority::Low);

        let urgent = Job::new(1, serde_json::json!({})).with_priority(JobPriority::Urgent);
        let normal = Job::new(1, serde_json::json!({})).with_priority(JobPriority::Normal);

        assert!(urgent.priority_weight() > normal.priority_weight());
    }

    #[test]
    fn test_job_retry_delay() {
        let mut job = Job::new(1, serde_json::json!({"test": "data"}));
        job.retry_delay_seconds = 2;

        // First retry: 2 seconds delay
        job.fail("Error 1".to_string(), None);
        job.retry().unwrap();

        let first_retry_time = job.process_at.unwrap();

        // Second retry: 4 seconds delay (exponential backoff)
        job.reset_for_retry();
        job.start_processing(101);
        job.fail("Error 2".to_string(), None);
        job.retry().unwrap();

        let second_retry_time = job.process_at.unwrap();

        // Second retry should be scheduled later than first
        assert!(second_retry_time > first_retry_time);
    }

    #[test]
    fn test_job_from_schedule() {
        let job = Job::from_schedule(1, 5, serde_json::json!({"scheduled": true}));

        assert_eq!(job.task_id, 1);
        assert_eq!(job.schedule_id, Some(5));
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[test]
    fn test_priority_and_status_conversion() {
        assert_eq!("high".parse::<JobPriority>().unwrap(), JobPriority::High);
        assert_eq!(
            "normal".parse::<JobPriority>().unwrap(),
            JobPriority::Normal
        );
        assert!("invalid".parse::<JobPriority>().is_err());

        assert_eq!(
            "processing".parse::<JobStatus>().unwrap(),
            JobStatus::Processing
        );
        assert_eq!(
            "completed".parse::<JobStatus>().unwrap(),
            JobStatus::Completed
        );
        assert!("invalid".parse::<JobStatus>().is_err());

        assert_eq!(JobPriority::Urgent.to_string(), "urgent");
        assert_eq!(JobStatus::Retrying.to_string(), "retrying");
    }
}
