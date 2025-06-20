//! Error types for task execution

use serde_json::Value as JsonValue;
use thiserror::Error;

/// Task execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Job not found: {0}")]
    JobNotFound(i32),

    #[error("Task execution error: {0}")]
    TaskExecutionError(String),

    #[error("Task validation error: {0}")]
    ValidationError(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Invalid execution state: {0}")]
    InvalidState(String),

    #[error("IPC error: {0}")]
    IpcError(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

// Convert from storage errors
impl From<ratchet_storage::error::StorageError> for ExecutionError {
    fn from(err: ratchet_storage::error::StorageError) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

// Convert from config errors
impl From<ratchet_config::error::ConfigError> for ExecutionError {
    fn from(err: ratchet_config::error::ConfigError) -> Self {
        Self::ConfigurationError(err.to_string())
    }
}

// Convert from IPC errors
impl From<ratchet_ipc::error::IpcError> for ExecutionError {
    fn from(err: ratchet_ipc::error::IpcError) -> Self {
        Self::IpcError(err.to_string())
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub execution_id: i32,
    pub success: bool,
    pub output: Option<JsonValue>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub http_requests: Option<JsonValue>,
}
