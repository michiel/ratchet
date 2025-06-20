//! Execution interface definitions
//!
//! Provides the core task execution interfaces that allow different
//! execution engines to be used interchangeably.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Duration;

/// Core task execution interface
///
/// This trait abstracts the execution of tasks, allowing different
/// execution engines (process-based, in-memory, distributed) to be
/// used interchangeably throughout the Ratchet system.
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute a task with given input
    ///
    /// # Arguments
    /// * `task_id` - Unique identifier for the task
    /// * `input` - JSON input data for the task
    /// * `context` - Optional execution context with timeout and metadata
    ///
    /// # Returns
    /// Result containing execution output and metadata, or an error
    async fn execute_task(
        &self,
        task_id: &str,
        input: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, Self::Error>;

    /// Check if executor is healthy and ready to execute tasks
    async fn health_check(&self) -> Result<(), Self::Error>;

    /// Get executor metrics (optional)
    fn metrics(&self) -> ExecutorMetrics {
        ExecutorMetrics::default()
    }

    /// Graceful shutdown of the executor
    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Execution context for task runs
///
/// Provides additional configuration and metadata for task execution.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Maximum time allowed for task execution
    pub timeout: Option<Duration>,
    /// Whether to enable detailed execution tracing
    pub trace_enabled: bool,
    /// Custom metadata for the execution
    pub metadata: HashMap<String, String>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(300)), // 5 minutes default
            trace_enabled: false,
            metadata: HashMap::new(),
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Enable execution tracing
    pub fn with_tracing(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    /// Add metadata to the execution context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Task execution result
///
/// Contains the output and metadata from a completed task execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// JSON output produced by the task
    pub output: JsonValue,
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
    /// Log messages generated during execution
    pub logs: Vec<String>,
    /// Optional detailed execution trace
    pub trace: Option<JsonValue>,
    /// Exit status information
    pub status: ExecutionStatus,
}

/// Execution status information
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    /// Task completed successfully
    Success,
    /// Task failed with an error
    Failed { error_message: String },
    /// Task was cancelled or timed out
    Cancelled { reason: String },
}

impl ExecutionStatus {
    /// Check if the execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }

    /// Check if the execution failed
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionStatus::Failed { .. })
    }

    /// Check if the execution was cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self, ExecutionStatus::Cancelled { .. })
    }
}

/// Execution engine metrics
///
/// Provides insights into the performance and health of the execution engine.
#[derive(Debug, Clone, Default)]
pub struct ExecutorMetrics {
    /// Total number of tasks executed
    pub tasks_executed: u64,
    /// Number of tasks that failed
    pub tasks_failed: u64,
    /// Number of tasks currently running
    pub tasks_running: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Number of active workers/processes
    pub active_workers: u32,
    /// Memory usage in bytes (if available)
    pub memory_usage_bytes: Option<u64>,
}

impl ExecutorMetrics {
    /// Calculate the task success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.tasks_executed == 0 {
            1.0
        } else {
            let successful = self.tasks_executed - self.tasks_failed;
            successful as f64 / self.tasks_executed as f64
        }
    }

    /// Calculate the task failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_execution_context_builder() {
        let context = ExecutionContext::new()
            .with_timeout(Duration::from_secs(60))
            .with_tracing()
            .with_metadata("user_id", "123")
            .with_metadata("session_id", "abc");

        assert_eq!(context.timeout, Some(Duration::from_secs(60)));
        assert!(context.trace_enabled);
        assert_eq!(context.metadata["user_id"], "123");
        assert_eq!(context.metadata["session_id"], "abc");
    }

    #[test]
    fn test_execution_status() {
        let success = ExecutionStatus::Success;
        assert!(success.is_success());
        assert!(!success.is_failed());
        assert!(!success.is_cancelled());

        let failed = ExecutionStatus::Failed {
            error_message: "Task failed".to_string(),
        };
        assert!(!failed.is_success());
        assert!(failed.is_failed());
        assert!(!failed.is_cancelled());

        let cancelled = ExecutionStatus::Cancelled {
            reason: "Timeout".to_string(),
        };
        assert!(!cancelled.is_success());
        assert!(!cancelled.is_failed());
        assert!(cancelled.is_cancelled());
    }

    #[test]
    fn test_executor_metrics() {
        let metrics = ExecutorMetrics {
            tasks_executed: 100,
            tasks_failed: 3,
            tasks_running: 5,
            average_execution_time_ms: 250.0,
            active_workers: 4,
            memory_usage_bytes: Some(1024 * 1024),
        };

        assert!((metrics.success_rate() - 0.97).abs() < 0.0001);
        assert!((metrics.failure_rate() - 0.03).abs() < 0.0001);
    }

    #[test]
    fn test_execution_result_creation() {
        let result = ExecutionResult {
            output: json!({"result": "success", "value": 42}),
            execution_time_ms: 1500,
            logs: vec!["Starting task".to_string(), "Task completed".to_string()],
            trace: Some(json!({"steps": ["init", "execute", "cleanup"]})),
            status: ExecutionStatus::Success,
        };

        assert!(result.status.is_success());
        assert_eq!(result.execution_time_ms, 1500);
        assert_eq!(result.logs.len(), 2);
        assert!(result.trace.is_some());
    }
}
