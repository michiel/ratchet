//! Execution domain model and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

use crate::task::TaskId;
use crate::types::Priority;

/// Unique identifier for an execution (newtype pattern for type safety)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub Uuid);

impl ExecutionId {
    /// Create a new random execution ID
    pub fn new() -> Self {
        ExecutionId(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ExecutionId {
    fn from(uuid: Uuid) -> Self {
        ExecutionId(uuid)
    }
}

impl From<ExecutionId> for Uuid {
    fn from(id: ExecutionId) -> Self {
        id.0
    }
}

/// Unique identifier for a job (newtype pattern for type safety)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub Uuid);

impl JobId {
    /// Create a new random job ID
    pub fn new() -> Self {
        JobId(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution is queued and waiting to start
    Pending,
    /// Execution is currently running
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed with an error
    Failed,
    /// Execution was cancelled
    Cancelled,
    /// Execution timed out
    TimedOut,
}

impl ExecutionStatus {
    /// Check if the execution is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Completed
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::TimedOut
        )
    }

    /// Check if the execution is still active
    pub fn is_active(&self) -> bool {
        matches!(self, ExecutionStatus::Running)
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::TimedOut => "timed_out",
        }
    }
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Context information for an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub execution_id: ExecutionId,

    /// Optional job ID if part of a job
    pub job_id: Option<JobId>,

    /// Task ID being executed
    pub task_id: TaskId,

    /// Task version being executed
    pub task_version: String,

    /// Priority of the execution
    pub priority: Priority,

    /// Optional trace ID for distributed tracing
    pub trace_id: Option<String>,

    /// Optional parent span ID for distributed tracing
    pub parent_span_id: Option<String>,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Complete execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    /// Execution context
    pub context: ExecutionContext,

    /// Current status
    pub status: ExecutionStatus,

    /// Input data provided to the task
    pub input_data: serde_json::Value,

    /// Output data from the task (if completed)
    pub output_data: Option<serde_json::Value>,

    /// Error information (if failed)
    pub error: Option<ExecutionError>,

    /// When the execution was created
    pub created_at: DateTime<Utc>,

    /// When the execution started running
    pub started_at: Option<DateTime<Utc>>,

    /// When the execution completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Execution duration
    pub duration: Option<Duration>,

    /// Number of retry attempts
    pub retry_count: u32,

    /// Worker that executed this task
    pub worker_id: Option<String>,

    /// Output destinations for results
    pub output_destinations: Vec<OutputDestination>,
}

/// Error information for failed executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    /// Error code or type
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Detailed error description
    pub details: Option<String>,

    /// Stack trace if available
    pub stack_trace: Option<String>,

    /// Whether this error is retryable
    pub retryable: bool,
}

/// Output destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputDestination {
    /// Write to filesystem
    Filesystem {
        path: String,
        format: OutputFormat,
        permissions: Option<String>,
    },

    /// Send to webhook
    Webhook {
        url: String,
        method: crate::types::HttpMethod,
        headers: std::collections::HashMap<String, String>,
        timeout_seconds: u32,
    },

    /// Store in database
    Database {
        table: String,
        columns: std::collections::HashMap<String, String>,
    },

    /// Upload to S3
    S3 {
        bucket: String,
        key: String,
        region: Option<String>,
    },
}

/// Output format for destinations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
    JsonCompact,
    Yaml,
    Csv,
    Raw,
}

impl Execution {
    /// Create a new pending execution
    pub fn new(task_id: TaskId, task_version: String, input_data: serde_json::Value, priority: Priority) -> Self {
        let context = ExecutionContext {
            execution_id: ExecutionId::new(),
            job_id: None,
            task_id,
            task_version,
            priority,
            trace_id: None,
            parent_span_id: None,
            metadata: std::collections::HashMap::new(),
        };

        Self {
            context,
            status: ExecutionStatus::Pending,
            input_data,
            output_data: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            duration: None,
            retry_count: 0,
            worker_id: None,
            output_destinations: Vec::new(),
        }
    }

    /// Mark execution as started
    pub fn start(&mut self, worker_id: String) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(Utc::now());
        self.worker_id = Some(worker_id);
    }

    /// Mark execution as completed successfully
    pub fn complete(&mut self, output_data: serde_json::Value) {
        self.status = ExecutionStatus::Completed;
        self.output_data = Some(output_data);
        self.completed_at = Some(Utc::now());
        self.update_duration();
    }

    /// Mark execution as failed
    pub fn fail(&mut self, error: ExecutionError) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
        self.update_duration();
    }

    /// Mark execution as cancelled
    pub fn cancel(&mut self) {
        self.status = ExecutionStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.update_duration();
    }

    /// Mark execution as timed out
    pub fn timeout(&mut self) {
        self.status = ExecutionStatus::TimedOut;
        self.completed_at = Some(Utc::now());
        self.update_duration();
    }

    /// Update the duration based on start and completion times
    fn update_duration(&mut self) {
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            self.duration = Some(end.signed_duration_since(start).to_std().unwrap_or_default());
        }
    }

    /// Get a display name for the execution
    pub fn display_name(&self) -> String {
        format!(
            "Execution {} for task {}",
            self.context.execution_id, self.context.task_id
        )
    }
}

/// Builder for constructing executions
pub struct ExecutionBuilder {
    task_id: TaskId,
    task_version: String,
    input_data: serde_json::Value,
    priority: Priority,
    job_id: Option<JobId>,
    trace_id: Option<String>,
    metadata: std::collections::HashMap<String, serde_json::Value>,
    output_destinations: Vec<OutputDestination>,
}

impl ExecutionBuilder {
    /// Create a new execution builder
    pub fn new(task_id: TaskId, task_version: impl Into<String>, input_data: serde_json::Value) -> Self {
        Self {
            task_id,
            task_version: task_version.into(),
            input_data,
            priority: Priority::default(),
            job_id: None,
            trace_id: None,
            metadata: std::collections::HashMap::new(),
            output_destinations: Vec::new(),
        }
    }

    /// Set the priority
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the job ID
    pub fn job_id(mut self, job_id: JobId) -> Self {
        self.job_id = Some(job_id);
        self
    }

    /// Set the trace ID
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add output destinations
    pub fn output_destinations(mut self, destinations: Vec<OutputDestination>) -> Self {
        self.output_destinations = destinations;
        self
    }

    /// Build the execution
    pub fn build(self) -> Execution {
        let mut execution = Execution::new(self.task_id, self.task_version, self.input_data, self.priority);

        execution.context.job_id = self.job_id;
        execution.context.trace_id = self.trace_id;
        execution.context.metadata = self.metadata;
        execution.output_destinations = self.output_destinations;

        execution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status() {
        assert!(ExecutionStatus::Completed.is_terminal());
        assert!(ExecutionStatus::Failed.is_terminal());
        assert!(ExecutionStatus::Cancelled.is_terminal());
        assert!(ExecutionStatus::TimedOut.is_terminal());
        assert!(!ExecutionStatus::Pending.is_terminal());
        assert!(!ExecutionStatus::Running.is_terminal());

        assert!(ExecutionStatus::Running.is_active());
        assert!(!ExecutionStatus::Pending.is_active());
        assert!(!ExecutionStatus::Completed.is_active());
    }

    #[test]
    fn test_execution_lifecycle() {
        let mut execution = Execution::new(
            TaskId::new(),
            "1.0.0".to_string(),
            serde_json::json!({"input": "data"}),
            Priority::Normal,
        );

        assert_eq!(execution.status, ExecutionStatus::Pending);
        assert!(execution.started_at.is_none());

        // Start execution
        execution.start("worker-1".to_string());
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert!(execution.started_at.is_some());
        assert_eq!(execution.worker_id.as_deref(), Some("worker-1"));

        // Complete execution
        execution.complete(serde_json::json!({"output": "data"}));
        assert_eq!(execution.status, ExecutionStatus::Completed);
        assert!(execution.completed_at.is_some());
        assert!(execution.duration.is_some());
        assert!(execution.output_data.is_some());
    }

    #[test]
    fn test_execution_builder() {
        let task_id = TaskId::new();
        let job_id = JobId::new();

        let execution = ExecutionBuilder::new(task_id, "2.0.0", serde_json::json!({"test": true}))
            .priority(Priority::High)
            .job_id(job_id)
            .trace_id("trace-123")
            .metadata("environment", serde_json::json!("production"))
            .build();

        assert_eq!(execution.context.task_id, task_id);
        assert_eq!(execution.context.task_version, "2.0.0");
        assert_eq!(execution.context.priority, Priority::High);
        assert_eq!(execution.context.job_id, Some(job_id));
        assert_eq!(execution.context.trace_id.as_deref(), Some("trace-123"));
        assert_eq!(
            execution.context.metadata.get("environment"),
            Some(&serde_json::json!("production"))
        );
    }
}
