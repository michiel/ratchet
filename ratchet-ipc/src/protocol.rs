//! IPC protocol definitions and message types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;
use uuid::Uuid;

/// IPC protocol version for compatibility checking
pub const IPC_PROTOCOL_VERSION: u32 = 1;

/// Execution context passed to JavaScript tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub execution_id: String,   // Execution UUID as string
    pub job_id: Option<String>, // Job UUID as string (optional for direct executions)
    pub task_id: String,        // Task UUID as string
    pub task_version: String,   // Task version
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(
        execution_uuid: Uuid,
        job_uuid: Option<Uuid>,
        task_uuid: Uuid,
        task_version: String,
    ) -> Self {
        Self {
            execution_id: execution_uuid.to_string(),
            job_id: job_uuid.map(|uuid| uuid.to_string()),
            task_id: task_uuid.to_string(),
            task_version,
        }
    }
}

/// Messages sent from coordinator to worker processes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    /// Execute a task
    ExecuteTask {
        job_id: i32,
        task_id: i32,
        task_path: String,
        input_data: JsonValue,
        execution_context: ExecutionContext,
        correlation_id: Uuid,
    },

    /// Validate a task
    ValidateTask {
        task_path: String,
        correlation_id: Uuid,
    },

    /// Health check ping
    Ping { correlation_id: Uuid },

    /// Shutdown signal
    Shutdown,
}

/// Messages sent from worker processes to coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoordinatorMessage {
    /// Task execution result
    TaskResult {
        job_id: i32,
        correlation_id: Uuid,
        result: TaskExecutionResult,
    },

    /// Task validation result
    ValidationResult {
        correlation_id: Uuid,
        result: TaskValidationResult,
    },

    /// Health check response
    Pong {
        correlation_id: Uuid,
        worker_id: String,
        status: WorkerStatus,
    },

    /// Worker error
    Error {
        correlation_id: Option<Uuid>,
        error: WorkerError,
    },

    /// Worker ready for work
    Ready { worker_id: String },
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub success: bool,
    pub output: Option<JsonValue>,
    pub error_message: Option<String>,
    pub error_details: Option<JsonValue>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i32,
}

impl TaskExecutionResult {
    /// Create a successful result
    pub fn success(
        output: JsonValue,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Self {
        let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
        Self {
            success: true,
            output: Some(output),
            error_message: None,
            error_details: None,
            started_at,
            completed_at,
            duration_ms,
        }
    }

    /// Create a failed result
    pub fn failure(
        error: String,
        details: Option<JsonValue>,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Self {
        let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
        Self {
            success: false,
            output: None,
            error_message: Some(error),
            error_details: details,
            started_at,
            completed_at,
            duration_ms,
        }
    }
}

/// Task validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskValidationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub error_details: Option<JsonValue>,
}

impl TaskValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            error_message: None,
            error_details: None,
        }
    }

    /// Create an invalid result
    pub fn invalid(error: String, details: Option<JsonValue>) -> Self {
        Self {
            valid: false,
            error_message: Some(error),
            error_details: details,
        }
    }
}

/// Worker status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: String,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub tasks_executed: u64,
    pub tasks_failed: u64,
    pub memory_usage_mb: Option<u64>,
    pub cpu_usage_percent: Option<f32>,
}

impl WorkerStatus {
    /// Create a new worker status
    pub fn new(worker_id: String, pid: u32) -> Self {
        let now = Utc::now();
        Self {
            worker_id,
            pid,
            started_at: now,
            last_activity: now,
            tasks_executed: 0,
            tasks_failed: 0,
            memory_usage_mb: None,
            cpu_usage_percent: None,
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Record task execution
    pub fn record_task_execution(&mut self, success: bool) {
        self.tasks_executed += 1;
        if !success {
            self.tasks_failed += 1;
        }
        self.update_activity();
    }
}

/// Worker error types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "error_type", rename_all = "snake_case")]
pub enum WorkerError {
    /// Task execution failed
    TaskExecutionFailed {
        job_id: i32,
        error: String,
        details: Option<JsonValue>,
    },

    /// Task validation failed
    TaskValidationFailed {
        task_path: String,
        error: String,
        details: Option<JsonValue>,
    },

    /// Worker initialization failed
    InitializationFailed { error: String },

    /// Communication error
    CommunicationError { error: String },

    /// Worker panic/crash
    WorkerPanic {
        error: String,
        backtrace: Option<String>,
    },

    /// Message parse error
    MessageParseError { error: String },
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerError::TaskExecutionFailed { job_id, error, .. } => {
                write!(f, "Task execution failed (job_id: {}): {}", job_id, error)
            }
            WorkerError::TaskValidationFailed {
                task_path, error, ..
            } => {
                write!(f, "Task validation failed ({}): {}", task_path, error)
            }
            WorkerError::InitializationFailed { error } => {
                write!(f, "Worker initialization failed: {}", error)
            }
            WorkerError::CommunicationError { error } => {
                write!(f, "Communication error: {}", error)
            }
            WorkerError::WorkerPanic { error, .. } => {
                write!(f, "Worker panic: {}", error)
            }
            WorkerError::MessageParseError { error } => {
                write!(f, "Message parse error: {}", error)
            }
        }
    }
}

impl std::error::Error for WorkerError {}

/// Message envelope for all IPC communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub protocol_version: u32,
    pub timestamp: DateTime<Utc>,
    pub message: T,
}

impl<T> MessageEnvelope<T> {
    /// Create a new message envelope
    pub fn new(message: T) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION,
            timestamp: Utc::now(),
            message,
        }
    }

    /// Check if protocol version is compatible
    pub fn is_compatible(&self) -> bool {
        self.protocol_version == IPC_PROTOCOL_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_creation() {
        let exec_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        let context = ExecutionContext::new(exec_id, Some(job_id), task_id, "1.0.0".to_string());

        assert_eq!(context.execution_id, exec_id.to_string());
        assert_eq!(context.job_id, Some(job_id.to_string()));
        assert_eq!(context.task_id, task_id.to_string());
        assert_eq!(context.task_version, "1.0.0");
    }

    #[test]
    fn test_task_execution_result() {
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(1500);

        let success_result =
            TaskExecutionResult::success(serde_json::json!({"result": "ok"}), start, end);

        assert!(success_result.success);
        assert_eq!(success_result.duration_ms, 1500);
        assert!(success_result.error_message.is_none());

        let failure_result = TaskExecutionResult::failure(
            "Something went wrong".to_string(),
            Some(serde_json::json!({"code": "ERR_001"})),
            start,
            end,
        );

        assert!(!failure_result.success);
        assert_eq!(failure_result.duration_ms, 1500);
        assert_eq!(
            failure_result.error_message.as_deref(),
            Some("Something went wrong")
        );
    }

    #[test]
    fn test_worker_status() {
        let mut status = WorkerStatus::new("worker-1".to_string(), 12345);

        assert_eq!(status.worker_id, "worker-1");
        assert_eq!(status.pid, 12345);
        assert_eq!(status.tasks_executed, 0);
        assert_eq!(status.tasks_failed, 0);

        status.record_task_execution(true);
        assert_eq!(status.tasks_executed, 1);
        assert_eq!(status.tasks_failed, 0);

        status.record_task_execution(false);
        assert_eq!(status.tasks_executed, 2);
        assert_eq!(status.tasks_failed, 1);
    }

    #[test]
    fn test_message_envelope() {
        let message = WorkerMessage::Ping {
            correlation_id: Uuid::new_v4(),
        };

        let envelope = MessageEnvelope::new(message);
        assert_eq!(envelope.protocol_version, IPC_PROTOCOL_VERSION);
        assert!(envelope.is_compatible());

        // Test serialization
        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: MessageEnvelope<WorkerMessage> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.protocol_version, envelope.protocol_version);
    }
}
