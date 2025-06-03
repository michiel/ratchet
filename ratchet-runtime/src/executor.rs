//! Task execution engine
//! 
//! This module provides the high-level task execution engine that coordinates
//! between the runtime components and task execution.

use std::sync::Arc;
use tokio::sync::RwLock;
use log::{debug, info, warn};
use uuid::Uuid;

use ratchet_core::{task::Task, RatchetError, Result};
use ratchet_ipc::WorkerMessage;
use crate::process::{WorkerProcessManager, WorkerConfig, WorkerProcessError};

/// Task executor trait
#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task with the given input data
    async fn execute_task(
        &self,
        task: &Task,
        input_data: serde_json::Value,
        context: Option<ratchet_ipc::ExecutionContext>,
    ) -> Result<serde_json::Value>;

    /// Validate a task without executing it
    async fn validate_task(&self, task: &Task) -> Result<bool>;

    /// Get execution statistics
    async fn get_stats(&self) -> ExecutionStats;
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: u64,
    pub active_workers: u32,
    pub available_workers: u32,
}

/// High-level execution engine
pub struct ExecutionEngine {
    worker_manager: Arc<RwLock<WorkerProcessManager>>,
    stats: Arc<RwLock<ExecutionStats>>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub async fn new(config: WorkerConfig) -> Result<Self> {
        let worker_manager = WorkerProcessManager::new(config);
        
        let engine = Self {
            worker_manager: Arc::new(RwLock::new(worker_manager)),
            stats: Arc::new(RwLock::new(ExecutionStats {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                average_execution_time_ms: 0,
                active_workers: 0,
                available_workers: 0,
            })),
        };

        // Start worker processes
        engine.start_workers().await?;

        Ok(engine)
    }

    /// Start all worker processes
    pub async fn start_workers(&self) -> Result<()> {
        let mut manager = self.worker_manager.write().await;
        manager.start().await
            .map_err(|e| RatchetError::ExecutionError(format!("Failed to start workers: {}", e)))?;
        
        info!("Execution engine started successfully");
        Ok(())
    }

    /// Stop all worker processes
    pub async fn stop_workers(&self) -> Result<()> {
        let mut manager = self.worker_manager.write().await;
        manager.stop().await
            .map_err(|e| RatchetError::ExecutionError(format!("Failed to stop workers: {}", e)))?;
        
        info!("Execution engine stopped successfully");
        Ok(())
    }

    /// Get available worker count
    pub async fn available_worker_count(&self) -> u32 {
        let manager = self.worker_manager.read().await;
        let stats = manager.get_worker_stats().await;
        stats.iter().filter(|(_, status)| {
            matches!(status, crate::process::WorkerProcessStatus::Ready)
        }).count() as u32
    }

    /// Update execution statistics
    async fn update_stats(&self, execution_time_ms: u64, success: bool) {
        let mut stats = self.stats.write().await;
        stats.total_executions += 1;
        
        if success {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }

        // Update average execution time (simple moving average)
        if stats.total_executions == 1 {
            stats.average_execution_time_ms = execution_time_ms;
        } else {
            stats.average_execution_time_ms = (stats.average_execution_time_ms + execution_time_ms) / 2;
        }

        stats.available_workers = self.available_worker_count().await;
    }
}

#[async_trait::async_trait]
impl TaskExecutor for ExecutionEngine {
    async fn execute_task(
        &self,
        task: &Task,
        input_data: serde_json::Value,
        context: Option<ratchet_ipc::ExecutionContext>,
    ) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        debug!("Executing task: {} with input: {:?}", task.metadata.id, input_data);

        // Get an available worker
        let mut manager = self.worker_manager.write().await;
        let _worker_id = manager.get_available_worker().await
            .ok_or_else(|| RatchetError::ExecutionError("No available workers".to_string()))?;

        // Create execution context if not provided
        let exec_context = context.unwrap_or_else(|| {
            ratchet_ipc::ExecutionContext::new(
                Uuid::new_v4(),
                None,
                Uuid::new_v4(),
                "1.0.0".to_string(),
            )
        });

        // Create task execution message
        let correlation_id = Uuid::new_v4();
        let message = WorkerMessage::ExecuteTask {
            job_id: 0, // TODO: Add proper job tracking
            task_id: 0, // TODO: Add proper task ID tracking
            task_path: task.metadata.name.clone(), // Using task name as path for now
            input_data,
            execution_context: exec_context,
            correlation_id,
        };

        // Send task to worker and wait for result
        let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes
        let result = manager.send_task(message, timeout_duration).await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                if let ratchet_ipc::CoordinatorMessage::TaskResult { result, .. } = response {
                    self.update_stats(execution_time, result.success).await;
                    
                    if result.success {
                        info!("Task {} executed successfully in {}ms", task.metadata.id, execution_time);
                        result.output.ok_or_else(|| 
                            RatchetError::ExecutionError("Task succeeded but returned no output".to_string())
                        )
                    } else {
                        let error_msg = result.error_message
                            .unwrap_or_else(|| "Unknown execution error".to_string());
                        warn!("Task {} failed: {}", task.metadata.id, error_msg);
                        Err(RatchetError::ExecutionError(error_msg))
                    }
                } else {
                    self.update_stats(execution_time, false).await;
                    Err(RatchetError::ExecutionError("Unexpected response from worker".to_string()))
                }
            }
            Err(WorkerProcessError::Timeout) => {
                self.update_stats(execution_time, false).await;
                Err(RatchetError::ExecutionError("Task execution timed out".to_string()))
            }
            Err(e) => {
                self.update_stats(execution_time, false).await;
                Err(RatchetError::ExecutionError(format!("Worker communication error: {}", e)))
            }
        }
    }

    async fn validate_task(&self, task: &Task) -> Result<bool> {
        debug!("Validating task: {}", task.metadata.id);

        // For now, we'll just check basic task properties
        // TODO: Implement actual task validation by sending to worker
        
        if task.metadata.name.is_empty() {
            return Ok(false);
        }

        // TODO: Add more comprehensive validation
        Ok(true)
    }

    async fn get_stats(&self) -> ExecutionStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

/// Simple in-memory task executor for testing
pub struct InMemoryTaskExecutor {
    stats: Arc<RwLock<ExecutionStats>>,
}

impl InMemoryTaskExecutor {
    /// Create a new in-memory task executor
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ExecutionStats {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                average_execution_time_ms: 0,
                active_workers: 1,
                available_workers: 1,
            })),
        }
    }
}

impl Default for InMemoryTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TaskExecutor for InMemoryTaskExecutor {
    async fn execute_task(
        &self,
        task: &Task,
        input_data: serde_json::Value,
        _context: Option<ratchet_ipc::ExecutionContext>,
    ) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        debug!("In-memory execution of task: {} with input: {:?}", task.metadata.id, input_data);

        // Simulate task execution
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_executions += 1;
            stats.successful_executions += 1;
            
            if stats.total_executions == 1 {
                stats.average_execution_time_ms = execution_time;
            } else {
                stats.average_execution_time_ms = (stats.average_execution_time_ms + execution_time) / 2;
            }
        }

        // Return mock result
        Ok(serde_json::json!({
            "success": true,
            "message": "Task executed successfully in memory",
            "task_id": task.metadata.id.to_string(),
            "execution_time_ms": execution_time,
            "input_echo": input_data
        }))
    }

    async fn validate_task(&self, task: &Task) -> Result<bool> {
        debug!("In-memory validation of task: {}", task.metadata.id);
        
        // Simple validation
        Ok(!task.metadata.name.is_empty())
    }

    async fn get_stats(&self) -> ExecutionStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratchet_core::task::TaskBuilder;

    #[tokio::test]
    async fn test_in_memory_executor() {
        let executor = InMemoryTaskExecutor::new();
        
        let task = TaskBuilder::new("Test Task", "1.0.0")
            .input_schema(serde_json::json!({"type": "object"}))
            .output_schema(serde_json::json!({"type": "object"}))
            .javascript_source("console.log('test');")
            .build()
            .unwrap();
        
        let input = serde_json::json!({"test": "data"});
        let result = executor.execute_task(&task, input, None).await;
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["success"], true);
        assert!(output["task_id"].is_string());
    }

    #[tokio::test]
    async fn test_task_validation() {
        let executor = InMemoryTaskExecutor::new();
        
        let valid_task = TaskBuilder::new("Valid Task", "1.0.0")
            .input_schema(serde_json::json!({"type": "object"}))
            .output_schema(serde_json::json!({"type": "object"}))
            .javascript_source("console.log('valid');")
            .build()
            .unwrap();
        
        let result = executor.validate_task(&valid_task).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_executor_stats() {
        let executor = InMemoryTaskExecutor::new();
        
        let task = TaskBuilder::new("Stats Task", "1.0.0")
            .input_schema(serde_json::json!({"type": "object"}))
            .output_schema(serde_json::json!({"type": "object"}))
            .javascript_source("console.log('stats');")
            .build()
            .unwrap();
        
        // Execute a task
        let _ = executor.execute_task(&task, serde_json::json!({}), None).await;
        
        let stats = executor.get_stats().await;
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 0);
    }
}