use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Execution context passed to JavaScript tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub execution_id: String,   // Execution UUID as string
    pub job_id: Option<String>, // Job UUID as string (optional for direct executions)
    pub task_id: String,        // Task UUID as string
    pub task_version: String,   // Task version
}

impl ExecutionContext {
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

    /// Task progress update
    TaskProgress {
        job_id: i32,
        correlation_id: Uuid,
        progress: TaskProgressUpdate,
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

/// Task validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskValidationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub error_details: Option<JsonValue>,
}

/// Task progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressUpdate {
    /// Progress value (0.0 to 1.0)
    pub progress: f32,

    /// Current step description
    pub step: Option<String>,

    /// Step number (current step)
    pub step_number: Option<u32>,

    /// Total steps
    pub total_steps: Option<u32>,

    /// Custom status message
    pub message: Option<String>,

    /// Progress data
    pub data: Option<JsonValue>,

    /// Timestamp of progress update
    pub timestamp: DateTime<Utc>,
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

/// Worker error types
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    MessageParseError(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            WorkerError::MessageParseError(error) => {
                write!(f, "Message parse error: {}", error)
            }
        }
    }
}

/// IPC protocol version for compatibility checking
pub const IPC_PROTOCOL_VERSION: u32 = 1;

/// Message envelope for all IPC communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub protocol_version: u32,
    pub timestamp: DateTime<Utc>,
    pub message: T,
}

impl<T> MessageEnvelope<T> {
    pub fn new(message: T) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION,
            timestamp: Utc::now(),
            message,
        }
    }
}

/// IPC transport trait for different communication mechanisms
#[async_trait::async_trait]
pub trait IpcTransport {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Send a message to the other end
    async fn send<T: Serialize + Send + Sync>(
        &mut self,
        message: &MessageEnvelope<T>,
    ) -> Result<(), Self::Error>;

    /// Receive a message from the other end
    async fn receive<T: for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Result<MessageEnvelope<T>, Self::Error>;

    /// Close the transport
    async fn close(&mut self) -> Result<(), Self::Error>;
}

/// Stdin/Stdout IPC transport for process communication
pub struct StdioTransport {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
            stdout: tokio::io::stdout(),
        }
    }
}

#[async_trait::async_trait]
impl IpcTransport for StdioTransport {
    type Error = IpcError;

    async fn send<T: Serialize + Send + Sync>(
        &mut self,
        message: &MessageEnvelope<T>,
    ) -> Result<(), Self::Error> {
        use tokio::io::AsyncWriteExt;

        let json = serde_json::to_string(message)
            .map_err(|e| IpcError::SerializationError(e.to_string()))?;

        // Send with length prefix and newline delimiter
        let message_with_newline = format!("{}\n", json);
        self.stdout
            .write_all(message_with_newline.as_bytes())
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        self.stdout
            .flush()
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        Ok(())
    }

    async fn receive<T: for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Result<MessageEnvelope<T>, Self::Error> {
        use tokio::io::AsyncBufReadExt;

        let mut reader = tokio::io::BufReader::new(&mut self.stdin);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| IpcError::IoError(e.to_string()))?;

        if line.is_empty() {
            return Err(IpcError::ConnectionClosed);
        }

        // Remove newline
        line.truncate(line.trim_end().len());

        serde_json::from_str(&line).map_err(|e| IpcError::DeserializationError(e.to_string()))
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        // Stdin/stdout don't need explicit closing
        Ok(())
    }
}

/// IPC error types
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch { expected: u32, actual: u32 },

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Worker error: {0:?}")]
    WorkerError(WorkerError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let execution_context = ExecutionContext::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            "1.0.0".to_string(),
        );

        let message = WorkerMessage::ExecuteTask {
            job_id: 123,
            task_id: 456,
            task_path: "/path/to/task".to_string(),
            input_data: serde_json::json!({"key": "value"}),
            execution_context,
            correlation_id: Uuid::new_v4(),
        };

        let envelope = MessageEnvelope::new(message);
        let json = serde_json::to_string(&envelope).unwrap();
        let _deserialized: MessageEnvelope<WorkerMessage> = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_coordinator_message_serialization() {
        let result = TaskExecutionResult {
            success: true,
            output: Some(serde_json::json!({"result": "success"})),
            error_message: None,
            error_details: None,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            duration_ms: 1000,
        };

        let message = CoordinatorMessage::TaskResult {
            job_id: 123,
            correlation_id: Uuid::new_v4(),
            result,
        };

        let envelope = MessageEnvelope::new(message);
        let json = serde_json::to_string(&envelope).unwrap();
        let _deserialized: MessageEnvelope<CoordinatorMessage> =
            serde_json::from_str(&json).unwrap();
    }
}
