//! Worker process implementation for task execution
//!
//! This module provides the worker process implementation that executes tasks
//! in isolated processes for thread safety and fault tolerance.

use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use ratchet_core::{
    task::{Task, TaskBuilder},
    RatchetError,
};
use ratchet_ipc::{
    CoordinatorMessage, IpcTransport, MessageEnvelope, StdioTransport, TaskExecutionResult,
    TaskValidationResult, WorkerError, WorkerMessage, WorkerStatus,
};

/// Worker process main entry point
pub async fn worker_main(worker_id: String) -> Result<(), WorkerError> {
    info!("Starting worker process: {}", worker_id);

    // Initialize worker
    let mut worker = Worker::new(worker_id.clone()).await?;

    // Send ready signal
    worker.send_ready().await?;

    // Main worker loop
    worker.run().await?;

    info!("Worker process {} shutting down", worker_id);
    Ok(())
}

/// Worker process implementation
pub struct Worker {
    worker_id: String,
    transport: StdioTransport,
    status: Arc<RwLock<WorkerStatus>>,
    task_cache: HashMap<String, Task>,
}

impl Worker {
    /// Create a new worker instance
    pub async fn new(worker_id: String) -> Result<Self, WorkerError> {
        let status = Arc::new(RwLock::new(WorkerStatus {
            worker_id: worker_id.clone(),
            pid: std::process::id(),
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            tasks_executed: 0,
            tasks_failed: 0,
            memory_usage_mb: None,
            cpu_usage_percent: None,
        }));

        Ok(Self {
            worker_id,
            transport: StdioTransport::new(),
            status,
            task_cache: HashMap::new(),
        })
    }

    /// Send ready signal to coordinator
    pub async fn send_ready(&mut self) -> Result<(), WorkerError> {
        let message = CoordinatorMessage::Ready {
            worker_id: self.worker_id.clone(),
        };

        self.send_message(message).await
    }

    /// Main worker loop
    pub async fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            match self.receive_message().await {
                Ok(envelope) => {
                    self.update_last_activity().await;

                    match self.handle_message(envelope.message).await {
                        Ok(Some(response)) => {
                            if let Err(e) = self.send_message(response).await {
                                error!("Failed to send response: {}", e);
                            }
                        }
                        Ok(None) => {
                            // No response needed
                        }
                        Err(e) => {
                            error!("Error handling message: {:?}", e);
                            let error_msg = CoordinatorMessage::Error {
                                correlation_id: None,
                                error: e,
                            };
                            let _ = self.send_message(error_msg).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle incoming messages
    async fn handle_message(
        &mut self,
        message: WorkerMessage,
    ) -> Result<Option<CoordinatorMessage>, WorkerError> {
        match message {
            WorkerMessage::ExecuteTask {
                job_id,
                task_id,
                task_path,
                input_data,
                execution_context,
                correlation_id,
            } => {
                let result = self
                    .execute_task_impl(job_id, task_id, &task_path, input_data, execution_context)
                    .await;
                Ok(Some(CoordinatorMessage::TaskResult {
                    job_id,
                    correlation_id,
                    result,
                }))
            }

            WorkerMessage::ValidateTask {
                task_path,
                correlation_id,
            } => {
                let result = self.validate_task_impl(&task_path).await;
                Ok(Some(CoordinatorMessage::ValidationResult {
                    correlation_id,
                    result,
                }))
            }

            WorkerMessage::Ping { correlation_id } => {
                let status = self.get_current_status().await;
                Ok(Some(CoordinatorMessage::Pong {
                    correlation_id,
                    worker_id: self.worker_id.clone(),
                    status,
                }))
            }

            WorkerMessage::Shutdown => {
                info!("Received shutdown signal");
                std::process::exit(0);
            }
        }
    }

    /// Execute a task (simplified implementation)
    async fn execute_task_impl(
        &mut self,
        job_id: i32,
        _task_id: i32,
        task_path: &str,
        input_data: serde_json::Value,
        _execution_context: ratchet_ipc::ExecutionContext,
    ) -> TaskExecutionResult {
        let started_at = chrono::Utc::now();

        debug!("Executing task at path: {}", task_path);

        // Simplified task execution for now
        // TODO: Integrate with actual task execution engine
        match self.load_task_cached(task_path).await {
            Ok(_task) => {
                let completed_at = chrono::Utc::now();
                let duration_ms = (completed_at - started_at).num_milliseconds() as i32;

                self.increment_executed_tasks().await;

                TaskExecutionResult {
                    success: true,
                    output: Some(serde_json::json!({
                        "message": "Task executed successfully",
                        "input": input_data,
                        "job_id": job_id
                    })),
                    error_message: None,
                    error_details: None,
                    started_at,
                    completed_at,
                    duration_ms,
                }
            }
            Err(e) => {
                let completed_at = chrono::Utc::now();
                let duration_ms = (completed_at - started_at).num_milliseconds() as i32;

                self.increment_failed_tasks().await;

                TaskExecutionResult {
                    success: false,
                    output: None,
                    error_message: Some(format!("Failed to load task: {}", e)),
                    error_details: Some(serde_json::json!({
                        "job_id": job_id,
                        "task_path": task_path,
                        "error_type": "execution_error"
                    })),
                    started_at,
                    completed_at,
                    duration_ms,
                }
            }
        }
    }

    /// Validate a task
    async fn validate_task_impl(&mut self, task_path: &str) -> TaskValidationResult {
        debug!("Validating task at path: {}", task_path);

        match self.load_task_cached(task_path).await {
            Ok(_task) => {
                // TODO: Implement actual validation logic
                TaskValidationResult {
                    valid: true,
                    error_message: None,
                    error_details: None,
                }
            }
            Err(e) => TaskValidationResult {
                valid: false,
                error_message: Some(format!("Failed to load task: {}", e)),
                error_details: Some(serde_json::json!({
                    "task_path": task_path,
                    "error_type": "load_error"
                })),
            },
        }
    }

    /// Load task with caching (simplified)
    async fn load_task_cached(&mut self, task_path: &str) -> Result<Task, RatchetError> {
        if let Some(task) = self.task_cache.get(task_path) {
            return Ok(task.clone());
        }

        // TODO: Integrate with actual task loading
        // For now, create a mock task
        let task = TaskBuilder::new(format!("Task from {}", task_path), "1.0.0")
            .input_schema(serde_json::json!({"type": "object"}))
            .output_schema(serde_json::json!({"type": "object"}))
            .javascript_source("console.log('mock task');")
            .build()
            .map_err(|e| RatchetError::ExecutionError(format!("Failed to build task: {}", e)))?;

        self.task_cache.insert(task_path.to_string(), task.clone());
        Ok(task)
    }

    /// Send a message to the coordinator
    async fn send_message(&mut self, message: CoordinatorMessage) -> Result<(), WorkerError> {
        let envelope = MessageEnvelope::new(message);
        self.transport
            .send(&envelope)
            .await
            .map_err(|e| WorkerError::CommunicationError {
                error: e.to_string(),
            })
    }

    /// Receive a message from the coordinator
    async fn receive_message(&mut self) -> Result<MessageEnvelope<WorkerMessage>, WorkerError> {
        self.transport
            .receive()
            .await
            .map_err(|e| WorkerError::CommunicationError {
                error: e.to_string(),
            })
    }

    /// Update last activity timestamp
    async fn update_last_activity(&self) {
        let mut status = self.status.write().await;
        status.last_activity = chrono::Utc::now();
    }

    /// Increment executed tasks counter
    async fn increment_executed_tasks(&self) {
        let mut status = self.status.write().await;
        status.tasks_executed += 1;
    }

    /// Increment failed tasks counter
    async fn increment_failed_tasks(&self) {
        let mut status = self.status.write().await;
        status.tasks_failed += 1;
    }

    /// Get current worker status
    async fn get_current_status(&self) -> WorkerStatus {
        let status = self.status.read().await;
        status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_creation() {
        let worker_id = "test-worker".to_string();
        let result = Worker::new(worker_id).await;

        assert!(result.is_ok());
        let worker = result.unwrap();
        assert_eq!(worker.worker_id, "test-worker");
    }

    #[tokio::test]
    async fn test_worker_status() {
        let worker = Worker::new("test-worker".to_string()).await.unwrap();
        let status = worker.get_current_status().await;

        assert_eq!(status.worker_id, "test-worker");
        assert_eq!(status.tasks_executed, 0);
        assert_eq!(status.tasks_failed, 0);
    }

    #[tokio::test]
    async fn test_task_loading() {
        let mut worker = Worker::new("test-worker".to_string()).await.unwrap();

        // Test loading a task
        let result = worker.load_task_cached("test-task").await;
        assert!(result.is_ok());

        // Test cache hit
        let result2 = worker.load_task_cached("test-task").await;
        assert!(result2.is_ok());
    }
}
