use crate::database::DatabaseError;
use crate::services::ServiceError;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use thiserror::Error;

/// Task execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    
    #[error("Service error: {0}")]
    ServiceError(#[from] ServiceError),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Job not found: {0}")]
    JobNotFound(i32),
    
    #[error("Task execution error: {0}")]
    TaskExecutionError(String),
    
    #[error("Invalid execution state: {0}")]
    InvalidState(String),
}

/// Execution context containing metadata about the execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: i32,
    pub job_id: Option<i32>,
    pub task_id: i32,
    pub input_data: JsonValue,
    pub worker_id: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
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

/// Task executor trait for different execution strategies
#[async_trait(?Send)]
pub trait TaskExecutor {
    /// Execute a task with given input
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError>;
    
    /// Execute a job from the job queue
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError>;
    
    /// Check if executor is healthy
    async fn health_check(&self) -> Result<(), ExecutionError>;
}