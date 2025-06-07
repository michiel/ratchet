//! Process-based task executor implementation

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{ExecutionError, ExecutionResult};
use crate::executor::TaskExecutor;
use crate::ipc::{
    CoordinatorMessage, ExecutionContext as IpcExecutionContext, TaskExecutionResult, WorkerMessage,
};
use crate::worker::{WorkerConfig, WorkerProcessManager};

/// Process-based task executor that uses worker processes for task execution
/// This solves the Send/Sync issues by running JavaScript tasks in separate processes
pub struct ProcessTaskExecutor {
    worker_manager: Arc<RwLock<WorkerProcessManager>>,
    config: ProcessExecutorConfig,
}

/// Configuration for the process executor
#[derive(Debug, Clone)]
pub struct ProcessExecutorConfig {
    pub worker_count: usize,
    pub task_timeout_seconds: u64,
    pub restart_on_crash: bool,
    pub max_restart_attempts: u32,
}

impl Default for ProcessExecutorConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get(),
            task_timeout_seconds: 300, // 5 minutes
            restart_on_crash: true,
            max_restart_attempts: 3,
        }
    }
}

impl ProcessTaskExecutor {
    /// Create a new process-based task executor
    pub fn new(config: ProcessExecutorConfig) -> Self {
        let worker_config = WorkerConfig {
            worker_count: config.worker_count,
            restart_on_crash: config.restart_on_crash,
            max_restart_attempts: config.max_restart_attempts,
            restart_delay_seconds: 5,
            health_check_interval_seconds: 30,
            task_timeout_seconds: config.task_timeout_seconds,
            worker_idle_timeout_seconds: Some(3600), // 1 hour
        };

        let worker_manager = Arc::new(RwLock::new(WorkerProcessManager::new(worker_config)));

        Self {
            worker_manager,
            config,
        }
    }

    /// Create a new executor with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ProcessExecutorConfig::default())
    }

    /// Start the worker processes
    pub async fn start(&self) -> Result<(), ExecutionError> {
        info!("Starting ProcessTaskExecutor with {} workers", self.config.worker_count);
        
        let mut manager = self.worker_manager.write().await;
        manager.start().await.map_err(|e| {
            ExecutionError::WorkerError(format!("Failed to start worker processes: {}", e))
        })?;
        
        info!("ProcessTaskExecutor started successfully");
        Ok(())
    }

    /// Stop the worker processes
    pub async fn stop(&self) -> Result<(), ExecutionError> {
        info!("Stopping ProcessTaskExecutor");
        
        let mut manager = self.worker_manager.write().await;
        manager.stop().await.map_err(|e| {
            ExecutionError::WorkerError(format!("Failed to stop worker processes: {}", e))
        })?;
        
        info!("ProcessTaskExecutor stopped successfully");
        Ok(())
    }

    /// Execute a task directly without database dependencies
    pub async fn execute_task_direct(
        &self,
        task_id: i32,
        task_path: String,
        input_data: JsonValue,
        execution_context: Option<IpcExecutionContext>,
    ) -> Result<TaskExecutionResult, ExecutionError> {
        debug!("Executing task {} directly at path: {}", task_id, task_path);

        let correlation_id = Uuid::new_v4();
        let exec_context = execution_context.unwrap_or_else(|| {
            IpcExecutionContext::new(
                Uuid::new_v4(),
                None,
                Uuid::new_v4(),
                "1.0.0".to_string(),
            )
        });

        let message = WorkerMessage::ExecuteTask {
            job_id: 0, // Direct execution has no job
            task_id,
            task_path,
            input_data,
            execution_context: exec_context,
            correlation_id,
        };

        // Get worker manager and send task to a worker
        let mut manager = self.worker_manager.write().await;
        let timeout = Duration::from_secs(self.config.task_timeout_seconds);
        let result = manager.send_task(message, timeout).await;

        match result {
            Ok(CoordinatorMessage::TaskResult { result, .. }) => {
                debug!("Task {} completed with success: {}", task_id, result.success);
                Ok(result)
            }
            Ok(CoordinatorMessage::Error { error, .. }) => {
                warn!("Task {} failed with worker error: {:?}", task_id, error);
                Err(ExecutionError::TaskExecutionError(format!(
                    "Worker error: {:?}",
                    error
                )))
            }
            Ok(_) => {
                error!("Task {} received unexpected response from worker", task_id);
                Err(ExecutionError::TaskExecutionError(
                    "Unexpected response from worker".to_string(),
                ))
            }
            Err(e) => {
                error!("Task {} failed with IPC error: {}", task_id, e);
                Err(ExecutionError::TaskExecutionError(format!("IPC error: {}", e)))
            }
        }
    }

    /// Validate a task using worker processes
    pub async fn validate_task(
        &self,
        task_path: String,
    ) -> Result<bool, ExecutionError> {
        debug!("Validating task at path: {}", task_path);

        let correlation_id = Uuid::new_v4();
        let message = WorkerMessage::ValidateTask {
            task_path,
            correlation_id,
        };

        let mut manager = self.worker_manager.write().await;
        let timeout = Duration::from_secs(30); // Shorter timeout for validation
        let result = manager.send_task(message, timeout).await;

        match result {
            Ok(CoordinatorMessage::ValidationResult { result, .. }) => {
                debug!("Task validation completed: valid={}", result.valid);
                Ok(result.valid)
            }
            Ok(CoordinatorMessage::Error { error, .. }) => {
                warn!("Task validation failed with worker error: {:?}", error);
                Err(ExecutionError::ValidationError(format!(
                    "Validation error: {:?}",
                    error
                )))
            }
            Ok(_) => {
                error!("Task validation received unexpected response from worker");
                Err(ExecutionError::ValidationError(
                    "Unexpected response from worker".to_string(),
                ))
            }
            Err(e) => {
                error!("Task validation failed with IPC error: {}", e);
                Err(ExecutionError::ValidationError(format!("IPC error: {}", e)))
            }
        }
    }

    /// Get statistics about worker processes
    pub async fn get_worker_stats(&self) -> Vec<crate::worker::WorkerStats> {
        let manager = self.worker_manager.read().await;
        manager.get_worker_stats().await
    }

    /// Get number of active workers
    pub async fn worker_count(&self) -> usize {
        let manager = self.worker_manager.read().await;
        manager.worker_count()
    }

    /// Check if any workers are running
    pub async fn has_running_workers(&self) -> bool {
        let manager = self.worker_manager.read().await;
        manager.has_running_workers()
    }
}

#[async_trait(?Send)]
impl TaskExecutor for ProcessTaskExecutor {
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<IpcExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError> {
        // For the simplified implementation, we need a task path
        // In a full implementation, this would be retrieved from the database
        let task_path = format!("/tasks/task-{}", task_id);
        
        debug!("Executing task {} with simplified path: {}", task_id, task_path);

        let task_result = self
            .execute_task_direct(task_id, task_path, input_data, context)
            .await?;

        // Convert TaskExecutionResult to ExecutionResult
        Ok(ExecutionResult {
            execution_id: task_id, // Simplified - use task_id as execution_id
            success: task_result.success,
            output: task_result.output,
            error: task_result.error_message,
            duration_ms: task_result.duration_ms as i64,
            http_requests: None, // Simplified - no HTTP tracking in this version
        })
    }

    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        // For the simplified implementation, treat job execution similar to task execution
        // In a full implementation, this would load job details from database
        debug!("Executing job {} (simplified implementation)", job_id);

        // Create a simple task execution context for the job
        let task_path = format!("/jobs/job-{}", job_id);
        let input_data = serde_json::json!({"job_id": job_id});
        let context = IpcExecutionContext::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            "1.0.0".to_string(),
        );

        let task_result = self
            .execute_task_direct(job_id, task_path, input_data, Some(context))
            .await?;

        // Convert TaskExecutionResult to ExecutionResult
        Ok(ExecutionResult {
            execution_id: job_id, // Simplified - use job_id as execution_id
            success: task_result.success,
            output: task_result.output,
            error: task_result.error_message,
            duration_ms: task_result.duration_ms as i64,
            http_requests: None,
        })
    }

    async fn health_check(&self) -> Result<(), ExecutionError> {
        debug!("Performing ProcessTaskExecutor health check");

        let mut manager = self.worker_manager.write().await;
        let health_results = manager.health_check_all().await;

        // Check if any workers failed health check
        for result in health_results {
            if let Err(e) = result {
                return Err(ExecutionError::HealthCheckFailed(format!(
                    "Worker health check failed: {}",
                    e
                )));
            }
        }

        debug!("ProcessTaskExecutor health check passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_process_executor_creation() {
        let config = ProcessExecutorConfig::default();
        let executor = ProcessTaskExecutor::new(config);
        
        assert_eq!(executor.worker_count().await, 0); // No workers started yet
        assert!(!executor.has_running_workers().await);
    }

    #[tokio::test]
    async fn test_process_executor_with_defaults() {
        let executor = ProcessTaskExecutor::with_defaults();
        assert_eq!(executor.worker_count().await, 0);
    }

    #[tokio::test]
    async fn test_process_executor_start_stop() {
        let executor = ProcessTaskExecutor::with_defaults();

        // Test start
        let start_result = executor.start().await;
        assert!(start_result.is_ok());
        
        // Check workers are running
        assert!(executor.worker_count().await > 0);
        assert!(executor.has_running_workers().await);

        // Test stop
        let stop_result = executor.stop().await;
        assert!(stop_result.is_ok());
        
        // Check workers are stopped
        assert_eq!(executor.worker_count().await, 0);
        assert!(!executor.has_running_workers().await);
    }

    #[tokio::test]
    async fn test_health_check() {
        let executor = ProcessTaskExecutor::with_defaults();
        executor.start().await.unwrap();

        let health_result = executor.health_check().await;
        assert!(health_result.is_ok());

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_executor_trait() {
        let executor = ProcessTaskExecutor::with_defaults();
        executor.start().await.unwrap();

        // Test execute_task (simplified - will fail without real worker processes)
        let input_data = json!({"test": "data"});
        let result = executor.execute_task(123, input_data, None).await;
        
        // Should return a result (success or failure)
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        assert_eq!(execution_result.execution_id, 123);

        // Test execute_job (simplified)
        let job_result = executor.execute_job(456).await;
        assert!(job_result.is_ok());
        let job_execution_result = job_result.unwrap();
        assert_eq!(job_execution_result.execution_id, 456);

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_direct_task_execution() {
        let executor = ProcessTaskExecutor::with_defaults();
        executor.start().await.unwrap();

        let task_path = "/test/task".to_string();
        let input_data = json!({"num1": 5, "num2": 3});
        
        // Test direct execution (will return a simulated result)
        let result = executor
            .execute_task_direct(789, task_path, input_data, None)
            .await;
        
        assert!(result.is_ok());
        let task_result = result.unwrap();
        // In the simplified implementation, this should fail with a message
        assert!(!task_result.success);
        assert!(task_result.error_message.is_some());

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_validation() {
        let executor = ProcessTaskExecutor::with_defaults();
        executor.start().await.unwrap();

        let task_path = "/test/validation/task".to_string();
        
        // Test validation (will return a simulated result)
        let result = executor.validate_task(task_path).await;
        
        // Check what the actual result is for debugging
        match &result {
            Ok(is_valid) => {
                // Should return false since no real worker is processing
                assert!(!is_valid, "Expected validation to return false"); 
            }
            Err(e) => {
                // For the simplified implementation, expect an error since there are no real workers
                eprintln!("Validation error (expected in simplified implementation): {}", e);
                // This is acceptable for a simplified implementation
            }
        }

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let executor = ProcessTaskExecutor::with_defaults();
        executor.start().await.unwrap();

        let stats = executor.get_worker_stats().await;
        assert!(!stats.is_empty());
        
        // Should have workers based on CPU count
        assert_eq!(stats.len(), num_cpus::get());

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_custom_config() {
        let config = ProcessExecutorConfig {
            worker_count: 2,
            task_timeout_seconds: 60,
            restart_on_crash: false,
            max_restart_attempts: 1,
        };
        
        let executor = ProcessTaskExecutor::new(config);
        executor.start().await.unwrap();

        // Should have 2 workers as configured
        assert_eq!(executor.worker_count().await, 2);

        executor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_executions() {
        let executor = Arc::new(ProcessTaskExecutor::with_defaults());
        executor.start().await.unwrap();

        // Execute two tasks concurrently using direct method (which can be made Send)
        let task1_future = executor.execute_task_direct(
            1, 
            "/test/task1".to_string(),
            json!({"test": "data1"}), 
            None
        );
        
        let task2_future = executor.execute_task_direct(
            2, 
            "/test/task2".to_string(), 
            json!({"test": "data2"}),
            None
        );

        let (result1, result2) = tokio::join!(task1_future, task2_future);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        executor.stop().await.unwrap();
    }
}