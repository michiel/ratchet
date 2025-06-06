//! Execution entity definition

use super::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Execution entity representing a task execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    /// Primary key
    pub id: i32,

    /// Unique identifier
    pub uuid: Uuid,

    /// Foreign key to the task
    pub task_id: i32,

    /// Input data provided to the task
    pub input: serde_json::Value,

    /// Output data produced by the task
    pub output: Option<serde_json::Value>,

    /// Execution status
    pub status: ExecutionStatus,

    /// Error message if execution failed
    pub error_message: Option<String>,

    /// Detailed error information
    pub error_details: Option<serde_json::Value>,

    /// When the execution was queued
    pub queued_at: DateTime<Utc>,

    /// When the execution started
    pub started_at: Option<DateTime<Utc>>,

    /// When the execution completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<i32>,

    /// HTTP requests made during execution
    pub http_requests: Option<serde_json::Value>,

    /// Path to execution recording/logs
    pub recording_path: Option<String>,

    /// Worker ID that executed this task
    pub worker_id: Option<String>,

    /// Execution metadata
    pub metadata: serde_json::Value,

    /// Number of retry attempts
    pub retry_count: i32,

    /// Maximum number of retries allowed
    pub max_retries: i32,

    /// When the execution was created
    pub created_at: DateTime<Utc>,

    /// When the execution was last updated
    pub updated_at: DateTime<Utc>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExecutionStatus {
    /// Execution is queued and waiting to be processed
    #[default]
    Pending,

    /// Execution is currently running
    Running,

    /// Execution completed successfully
    Completed,

    /// Execution failed
    Failed,

    /// Execution was cancelled
    Cancelled,

    /// Execution timed out
    TimedOut,

    /// Execution is being retried
    Retrying,
}

impl Entity for Execution {
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

impl Execution {
    /// Create a new execution
    pub fn new(task_id: i32, input: serde_json::Value) -> Self {
        let now = Utc::now();

        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            task_id,
            input,
            output: None,
            status: ExecutionStatus::Pending,
            error_message: None,
            error_details: None,
            queued_at: now,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            http_requests: None,
            recording_path: None,
            worker_id: None,
            metadata: serde_json::json!({}),
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
        }
    }

    /// Start the execution
    pub fn start(&mut self, worker_id: Option<String>) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
        self.worker_id = worker_id;
        self.updated_at = Utc::now();
    }

    /// Complete the execution successfully
    pub fn complete(&mut self, output: serde_json::Value) {
        self.status = ExecutionStatus::Completed;
        self.output = Some(output);
        self.completed_at = Some(Utc::now());
        self.calculate_duration();
        self.updated_at = Utc::now();
    }

    /// Fail the execution
    pub fn fail(&mut self, error_message: String, error_details: Option<serde_json::Value>) {
        self.status = ExecutionStatus::Failed;
        self.error_message = Some(error_message);
        self.error_details = error_details;
        self.completed_at = Some(Utc::now());
        self.calculate_duration();
        self.updated_at = Utc::now();
    }

    /// Cancel the execution
    pub fn cancel(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.calculate_duration();
        self.updated_at = Utc::now();
    }

    /// Timeout the execution
    pub fn timeout(&mut self) {
        self.status = ExecutionStatus::TimedOut;
        self.error_message = Some("Execution timed out".to_string());
        self.completed_at = Some(Utc::now());
        self.calculate_duration();
        self.updated_at = Utc::now();
    }

    /// Retry the execution
    pub fn retry(&mut self) -> Result<(), String> {
        if self.retry_count >= self.max_retries {
            return Err("Maximum retries exceeded".to_string());
        }

        self.retry_count += 1;
        self.status = ExecutionStatus::Retrying;
        self.error_message = None;
        self.error_details = None;
        self.started_at = None;
        self.completed_at = None;
        self.duration_ms = None;
        self.worker_id = None;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Reset execution for retry
    pub fn reset_for_retry(&mut self) {
        self.status = ExecutionStatus::Pending;
        self.updated_at = Utc::now();
    }

    /// Calculate execution duration
    fn calculate_duration(&mut self) {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            self.duration_ms = Some((completed - started).num_milliseconds() as i32);
        }
    }

    /// Check if the execution is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Completed
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::TimedOut
        )
    }

    /// Check if the execution is running or pending
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Pending | ExecutionStatus::Running | ExecutionStatus::Retrying
        )
    }

    /// Check if the execution was successful
    pub fn is_successful(&self) -> bool {
        matches!(self.status, ExecutionStatus::Completed)
    }

    /// Check if the execution failed
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Failed | ExecutionStatus::TimedOut
        )
    }

    /// Check if the execution can be retried
    pub fn can_retry(&self) -> bool {
        self.is_failed() && self.retry_count < self.max_retries
    }

    /// Get execution progress percentage (0-100)
    pub fn progress_percentage(&self) -> f64 {
        match self.status {
            ExecutionStatus::Pending => 0.0,
            ExecutionStatus::Running | ExecutionStatus::Retrying => 50.0,
            ExecutionStatus::Completed => 100.0,
            ExecutionStatus::Failed | ExecutionStatus::Cancelled | ExecutionStatus::TimedOut => 0.0,
        }
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    /// Add HTTP request information
    pub fn add_http_request(&mut self, request_info: serde_json::Value) {
        let mut requests = self
            .http_requests
            .take()
            .unwrap_or_else(|| serde_json::json!([]));

        if let Some(array) = requests.as_array_mut() {
            array.push(request_info);
        } else {
            requests = serde_json::json!([request_info]);
        }

        self.http_requests = Some(requests);
        self.updated_at = Utc::now();
    }

    /// Set recording path
    pub fn set_recording_path(&mut self, path: String) {
        self.recording_path = Some(path);
        self.updated_at = Utc::now();
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
            ExecutionStatus::TimedOut => write!(f, "timed_out"),
            ExecutionStatus::Retrying => write!(f, "retrying"),
        }
    }
}

impl std::str::FromStr for ExecutionStatus {
    type Err = crate::StorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ExecutionStatus::Pending),
            "running" => Ok(ExecutionStatus::Running),
            "completed" => Ok(ExecutionStatus::Completed),
            "failed" => Ok(ExecutionStatus::Failed),
            "cancelled" => Ok(ExecutionStatus::Cancelled),
            "timed_out" => Ok(ExecutionStatus::TimedOut),
            "retrying" => Ok(ExecutionStatus::Retrying),
            _ => Err(crate::StorageError::ValidationFailed(format!(
                "Invalid execution status: {}",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_creation() {
        let execution = Execution::new(1, serde_json::json!({"test": "data"}));

        assert_eq!(execution.task_id, 1);
        assert_eq!(execution.status, ExecutionStatus::Pending);
        assert_eq!(execution.retry_count, 0);
        assert!(execution.is_active());
        assert!(!execution.is_terminal());
    }

    #[test]
    fn test_execution_lifecycle() {
        let mut execution = Execution::new(1, serde_json::json!({"test": "data"}));

        // Start execution
        execution.start(Some("worker-1".to_string()));
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert_eq!(execution.worker_id, Some("worker-1".to_string()));
        assert!(execution.started_at.is_some());
        assert!(execution.is_active());

        // Complete execution
        execution.complete(serde_json::json!({"result": "success"}));
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert!(execution.output.is_some());
        assert!(execution.completed_at.is_some());
        assert!(execution.duration_ms.is_some());
        assert!(execution.is_terminal());
        assert!(execution.is_successful());
    }

    #[test]
    fn test_execution_failure_and_retry() {
        let mut execution = Execution::new(1, serde_json::json!({"test": "data"}));

        // Start and fail execution
        execution.start(Some("worker-1".to_string()));
        execution.fail(
            "Connection error".to_string(),
            Some(serde_json::json!({"code": "CONN_ERROR"})),
        );

        assert_eq!(execution.status, ExecutionStatus::Failed);
        assert!(execution.is_failed());
        assert!(execution.can_retry());

        // Retry execution
        assert!(execution.retry().is_ok());
        assert_eq!(execution.status, ExecutionStatus::Retrying);
        assert_eq!(execution.retry_count, 1);
        assert!(execution.error_message.is_none());

        // Reset for retry
        execution.reset_for_retry();
        assert_eq!(execution.status, ExecutionStatus::Pending);
    }

    #[test]
    fn test_execution_timeout() {
        let mut execution = Execution::new(1, serde_json::json!({"test": "data"}));

        execution.start(Some("worker-1".to_string()));
        execution.timeout();

        assert_eq!(execution.status, ExecutionStatus::TimedOut);
        assert!(execution.is_failed());
        assert!(execution.is_terminal());
        assert_eq!(
            execution.error_message,
            Some("Execution timed out".to_string())
        );
    }

    #[test]
    fn test_execution_max_retries() {
        let mut execution = Execution::new(1, serde_json::json!({"test": "data"}));
        execution.max_retries = 2;

        // Fail and retry twice
        execution.fail("Error 1".to_string(), None);
        assert!(execution.retry().is_ok());
        assert_eq!(execution.retry_count, 1);

        execution.reset_for_retry();
        execution.start(Some("worker-1".to_string()));
        execution.fail("Error 2".to_string(), None);
        assert!(execution.retry().is_ok());
        assert_eq!(execution.retry_count, 2);

        // Third retry should fail
        execution.reset_for_retry();
        execution.start(Some("worker-1".to_string()));
        execution.fail("Error 3".to_string(), None);
        assert!(execution.retry().is_err());
        assert!(!execution.can_retry());
    }

    #[test]
    fn test_execution_http_requests() {
        let mut execution = Execution::new(1, serde_json::json!({"test": "data"}));

        execution.add_http_request(serde_json::json!({
            "url": "https://api.example.com/users",
            "method": "GET",
            "status": 200
        }));

        execution.add_http_request(serde_json::json!({
            "url": "https://api.example.com/posts",
            "method": "POST",
            "status": 201
        }));

        let requests = execution.http_requests.unwrap();
        assert_eq!(requests.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_execution_status_conversion() {
        assert_eq!(
            "running".parse::<ExecutionStatus>().unwrap(),
            ExecutionStatus::Running
        );
        assert_eq!(
            "completed".parse::<ExecutionStatus>().unwrap(),
            ExecutionStatus::Completed
        );
        assert!("invalid_status".parse::<ExecutionStatus>().is_err());

        assert_eq!(ExecutionStatus::Running.to_string(), "running");
        assert_eq!(ExecutionStatus::TimedOut.to_string(), "timed_out");
    }
}
