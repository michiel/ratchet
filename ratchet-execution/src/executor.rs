//! Core executor traits and context types

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::error::{ExecutionError, ExecutionResult};
use crate::ipc::ExecutionContext as IpcExecutionContext;

/// Execution context containing metadata about the execution
#[derive(Debug, Clone)]
pub struct LocalExecutionContext {
    pub execution_id: i32,
    pub job_id: Option<i32>,
    pub task_id: i32,
    pub input_data: JsonValue,
    pub worker_id: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Task executor trait for different execution strategies
#[async_trait(?Send)]
pub trait TaskExecutor {
    /// Execute a task with given input
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<IpcExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError>;

    /// Execute a job from the job queue
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError>;

    /// Check if executor is healthy
    async fn health_check(&self) -> Result<(), ExecutionError>;
}
