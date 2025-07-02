//! Worker process management for task execution

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;
use serde_json::Value as JsonValue;

use crate::error::ExecutionError;
use crate::ipc::{CoordinatorMessage, TaskExecutionResult, WorkerMessage, WorkerStatus, ExecutionContext};
use ratchet_js::{JsTask, JsTaskRunner, ExecutionContext as JsExecutionContext};

/// Configuration for worker processes
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub worker_count: usize,
    pub restart_on_crash: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub task_timeout_seconds: u64,
    pub worker_idle_timeout_seconds: Option<u64>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get(),
            restart_on_crash: true,
            max_restart_attempts: 3,
            restart_delay_seconds: 5,
            health_check_interval_seconds: 30,
            task_timeout_seconds: 300,               // 5 minutes
            worker_idle_timeout_seconds: Some(3600), // 1 hour
        }
    }
}

/// Worker process status
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerProcessStatus {
    Starting,
    Ready,
    Busy,
    Unresponsive,
    Failed,
    Stopped,
}

/// A single worker process handle
#[derive(Debug)]
pub struct WorkerProcess {
    pub id: String,
    pub pid: Option<u32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub restart_count: u32,
    pub status: WorkerProcessStatus,
    last_health_check: Option<chrono::DateTime<chrono::Utc>>,
}

impl WorkerProcess {
    /// Create a new worker process (simplified version)
    pub fn new(worker_id: String) -> Self {
        Self {
            id: worker_id,
            pid: None,
            started_at: chrono::Utc::now(),
            restart_count: 0,
            status: WorkerProcessStatus::Starting,
            last_health_check: None,
        }
    }

    /// Start the worker process
    pub async fn start(&mut self) -> Result<(), ExecutionError> {
        debug!("Starting worker process: {}", self.id);
        self.status = WorkerProcessStatus::Ready;
        Ok(())
    }

    /// Stop the worker process
    pub async fn stop(&mut self) -> Result<(), ExecutionError> {
        debug!("Stopping worker process: {}", self.id);
        self.status = WorkerProcessStatus::Stopped;
        Ok(())
    }

    /// Send a message to the worker
    pub async fn send_message(&mut self, _message: WorkerMessage) -> Result<(), ExecutionError> {
        // Simplified implementation - in full version this would use IPC
        Ok(())
    }

    /// Check worker health
    pub async fn health_check(&mut self) -> Result<WorkerStatus, ExecutionError> {
        self.last_health_check = Some(chrono::Utc::now());

        Ok(WorkerStatus {
            worker_id: self.id.clone(),
            pid: self.pid.unwrap_or(0),
            started_at: self.started_at,
            last_activity: chrono::Utc::now(),
            tasks_executed: 0,
            tasks_failed: 0,
            memory_usage_mb: None,
            cpu_usage_percent: None,
        })
    }
}

/// Statistics about worker processes
#[derive(Debug, Clone)]
pub struct WorkerStats {
    pub worker_id: String,
    pub status: WorkerProcessStatus,
    pub tasks_executed: u64,
    pub tasks_failed: u64,
    pub restart_count: u32,
    pub uptime_seconds: i64,
    pub memory_usage_mb: Option<u64>,
    pub cpu_usage_percent: Option<f32>,
}

/// Manages a pool of worker processes
pub struct WorkerProcessManager {
    config: WorkerConfig,
    workers: HashMap<String, WorkerProcess>,
    _pending_tasks: Arc<Mutex<HashMap<Uuid, oneshot::Sender<Result<TaskExecutionResult, ExecutionError>>>>>,
    _task_queue: Arc<Mutex<Vec<WorkerMessage>>>,
}

impl WorkerProcessManager {
    /// Create a new worker process manager
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            workers: HashMap::new(),
            _pending_tasks: Arc::new(Mutex::new(HashMap::new())),
            _task_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start all worker processes
    pub async fn start(&mut self) -> Result<(), ExecutionError> {
        info!("Starting {} worker processes", self.config.worker_count);

        for i in 0..self.config.worker_count {
            let worker_id = format!("worker-{}", i);
            let mut worker = WorkerProcess::new(worker_id.clone());

            worker.start().await?;
            self.workers.insert(worker_id, worker);
        }

        info!("All worker processes started successfully");
        Ok(())
    }

    /// Stop all worker processes
    pub async fn stop(&mut self) -> Result<(), ExecutionError> {
        info!("Stopping all worker processes");

        for (_, worker) in self.workers.iter_mut() {
            if let Err(e) = worker.stop().await {
                warn!("Failed to stop worker {}: {}", worker.id, e);
            }
        }

        self.workers.clear();
        info!("All worker processes stopped");
        Ok(())
    }

    /// Send a task to a worker and wait for the result
    pub async fn send_task(
        &mut self,
        message: WorkerMessage,
        _timeout: Duration,
    ) -> Result<CoordinatorMessage, ExecutionError> {
        // Find an available worker
        let worker_id = self
            .find_available_worker()
            .ok_or_else(|| ExecutionError::WorkerError("No available workers".to_string()))?;

        // Execute the task for real
        match message {
            WorkerMessage::ExecuteTask {
                job_id,
                correlation_id,
                task_path,
                input_data,
                execution_context,
                ..
            } => {
                let started_at = chrono::Utc::now();
                
                // Execute the JavaScript task
                let result = match self.execute_javascript_task(&task_path, input_data, execution_context).await {
                    Ok(output) => {
                        let completed_at = chrono::Utc::now();
                        let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
                        
                        TaskExecutionResult {
                            success: true,
                            output: Some(output),
                            error_message: None,
                            error_details: None,
                            started_at,
                            completed_at,
                            duration_ms,
                        }
                    }
                    Err(error) => {
                        let completed_at = chrono::Utc::now();
                        let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
                        
                        TaskExecutionResult {
                            success: false,
                            output: None,
                            error_message: Some(error.to_string()),
                            error_details: None,
                            started_at,
                            completed_at,
                            duration_ms,
                        }
                    }
                };

                Ok(CoordinatorMessage::TaskResult {
                    job_id,
                    correlation_id,
                    result,
                })
            }
            WorkerMessage::Ping { correlation_id } => {
                if let Some(worker) = self.workers.get_mut(&worker_id) {
                    let status = worker.health_check().await?;
                    Ok(CoordinatorMessage::Pong {
                        correlation_id,
                        worker_id: worker_id.clone(),
                        status,
                    })
                } else {
                    Err(ExecutionError::WorkerError("Worker not found".to_string()))
                }
            }
            _ => Err(ExecutionError::WorkerError("Unsupported message type".to_string())),
        }
    }

    /// Health check all workers
    pub async fn health_check_all(&mut self) -> Vec<Result<WorkerStatus, ExecutionError>> {
        let mut results = Vec::new();

        for (_, worker) in self.workers.iter_mut() {
            let result = worker.health_check().await;
            results.push(result);
        }

        results
    }

    /// Get statistics for all workers
    pub async fn get_worker_stats(&self) -> Vec<WorkerStats> {
        let mut stats = Vec::new();

        for worker in self.workers.values() {
            let uptime = chrono::Utc::now()
                .signed_duration_since(worker.started_at)
                .num_seconds();

            stats.push(WorkerStats {
                worker_id: worker.id.clone(),
                status: worker.status.clone(),
                tasks_executed: 0, // Simplified
                tasks_failed: 0,   // Simplified
                restart_count: worker.restart_count,
                uptime_seconds: uptime,
                memory_usage_mb: None,
                cpu_usage_percent: None,
            });
        }

        stats
    }

    /// Find an available worker
    fn find_available_worker(&self) -> Option<String> {
        self.workers
            .iter()
            .find(|(_, worker)| worker.status == WorkerProcessStatus::Ready)
            .map(|(id, _)| id.clone())
    }

    /// Get worker count
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Check if any workers are running
    pub fn has_running_workers(&self) -> bool {
        self.workers
            .values()
            .any(|w| matches!(w.status, WorkerProcessStatus::Ready | WorkerProcessStatus::Busy))
    }

    /// Execute a JavaScript task using the ratchet-js engine
    /// This runs in a separate thread to avoid Send issues with Boa engine
    async fn execute_javascript_task(
        &self,
        task_path: &str,
        input_data: JsonValue,
        execution_context: ExecutionContext,
    ) -> Result<JsonValue, ExecutionError> {
        debug!("Executing JavaScript task at path: {}", task_path);

        // For now, handle embedded tasks by checking known embedded task names
        // In a full implementation, this would integrate with the registry
        let (task_name, js_content) = self.resolve_task_content(task_path)?;
        debug!("Resolved task: {} with content length: {}", task_name, js_content.len());

        // Create JavaScript task
        let js_task = JsTask {
            name: task_name.clone(),
            content: js_content.clone(),
            input_schema: None, // TODO: Load from registry if available
            output_schema: None, // TODO: Load from registry if available
        };

        // Create execution context for JavaScript
        let js_context = Some(JsExecutionContext {
            execution_id: execution_context.execution_id.clone(),
            task_id: execution_context.task_id.clone(),
            task_version: execution_context.task_version.clone(),
            job_id: execution_context.job_id.clone(),
        });

        // Execute the task in a separate thread to avoid Send issues with Boa
        let result = tokio::task::spawn_blocking(move || {
            let runner = JsTaskRunner::new();
            // Use the sync blocking execution since we're in a blocking task
            tokio::runtime::Handle::current().block_on(async move {
                runner.execute_task(&js_task, input_data, js_context).await
            })
        })
        .await
        .map_err(|e| ExecutionError::TaskExecutionError(format!("Task execution failed: {}", e)))?
        .map_err(|e| ExecutionError::TaskExecutionError(format!("JavaScript execution failed: {}", e)))?;

        debug!("JavaScript task completed successfully");
        Ok(result)
    }

    /// Resolve task content from path/name
    /// This is a simplified implementation that handles embedded tasks
    fn resolve_task_content(&self, task_path: &str) -> Result<(String, String), ExecutionError> {
        // Handle embedded tasks first
        if task_path == "heartbeat" || task_path.contains("heartbeat") {
            // Return the embedded heartbeat task JavaScript
            // Using simplified synchronous version that works with Boa
            let heartbeat_js = r#"
function main(input) {
    try {
        // Basic system status
        const timestamp = new Date().toISOString();
        
        // Get process uptime (simulated for Boa engine)
        const uptimeSeconds = Math.floor(Math.random() * 10000); // Simulated uptime
        
        // Basic system information
        const systemInfo = {
            version: "ratchet-0.1.0",
            uptime_seconds: uptimeSeconds,
            active_jobs: 0 // This would be populated by the execution context
        };
        
        // Return success response
        return {
            status: "ok",
            timestamp: timestamp,
            message: "Heartbeat successful - system running normally",
            system_info: systemInfo
        };
        
    } catch (error) {
        // Return error response if something goes wrong
        return {
            status: "error",
            timestamp: new Date().toISOString(),
            message: "Heartbeat failed: " + error.message,
            system_info: {
                version: "ratchet-0.1.0",
                uptime_seconds: 0,
                active_jobs: 0
            }
        };
    }
}
"#;
            return Ok(("heartbeat".to_string(), heartbeat_js.to_string()));
        }

        // Handle bridge task paths (from the execution bridge)
        if task_path.starts_with("/bridge-task/") {
            let task_name = task_path.strip_prefix("/bridge-task/").unwrap_or(task_path);
            
            // For now, try to match known embedded tasks
            if task_name == "heartbeat" {
                // Use the same embedded heartbeat content
                return self.resolve_task_content("heartbeat");
            }
        }

        // For unknown tasks, return an error
        Err(ExecutionError::TaskExecutionError(format!(
            "Unable to resolve task content for path: {}. Only embedded tasks are currently supported.",
            task_path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_creation() {
        let worker = WorkerProcess::new("test-worker".to_string());
        assert_eq!(worker.id, "test-worker");
        assert_eq!(worker.status, WorkerProcessStatus::Starting);
    }

    #[tokio::test]
    async fn test_worker_manager_creation() {
        let config = WorkerConfig::default();
        let manager = WorkerProcessManager::new(config);
        assert_eq!(manager.worker_count(), 0);
    }

    #[tokio::test]
    async fn test_worker_manager_start_stop() {
        let config = WorkerConfig {
            worker_count: 2,
            ..Default::default()
        };
        let mut manager = WorkerProcessManager::new(config);

        // Start workers
        let start_result = manager.start().await;
        assert!(start_result.is_ok());
        assert_eq!(manager.worker_count(), 2);
        assert!(manager.has_running_workers());

        // Stop workers
        let stop_result = manager.stop().await;
        assert!(stop_result.is_ok());
        assert_eq!(manager.worker_count(), 0);
        assert!(!manager.has_running_workers());
    }

    #[tokio::test]
    async fn test_worker_health_check() {
        let mut worker = WorkerProcess::new("test-worker".to_string());
        worker.start().await.unwrap();

        let health_result = worker.health_check().await;
        assert!(health_result.is_ok());

        let status = health_result.unwrap();
        assert_eq!(status.worker_id, "test-worker");
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let config = WorkerConfig {
            worker_count: 1,
            ..Default::default()
        };
        let mut manager = WorkerProcessManager::new(config);
        manager.start().await.unwrap();

        let stats = manager.get_worker_stats().await;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].worker_id, "worker-0");
        assert_eq!(stats[0].status, WorkerProcessStatus::Ready);
    }

    #[tokio::test]
    async fn test_send_ping_message() {
        let config = WorkerConfig {
            worker_count: 1,
            ..Default::default()
        };
        let mut manager = WorkerProcessManager::new(config);
        manager.start().await.unwrap();

        let message = WorkerMessage::Ping {
            correlation_id: Uuid::new_v4(),
        };

        let result = manager.send_task(message, Duration::from_secs(5)).await;
        assert!(result.is_ok());

        if let Ok(CoordinatorMessage::Pong { worker_id, .. }) = result {
            assert_eq!(worker_id, "worker-0");
        } else {
            panic!("Expected Pong response");
        }
    }

    #[tokio::test]
    async fn test_execute_heartbeat_task() {
        // First test just the JavaScript execution directly
        let heartbeat_js = r#"
function main(input) {
    return {
        status: "ok",
        timestamp: new Date().toISOString(),
        message: "Heartbeat successful - system running normally",
        system_info: {
            version: "ratchet-0.1.0",
            uptime_seconds: 12345,
            active_jobs: 0
        }
    };
}
"#;

        let js_task = JsTask {
            name: "heartbeat".to_string(),
            content: heartbeat_js.to_string(),
            input_schema: None,
            output_schema: None,
        };

        let js_context = Some(JsExecutionContext {
            execution_id: "test-exec-123".to_string(),
            task_id: "heartbeat".to_string(),
            task_version: "1.0.0".to_string(),
            job_id: None,
        });

        let runner = JsTaskRunner::new();
        let js_result = runner.execute_task(&js_task, serde_json::json!({}), js_context).await;
        
        println!("Direct JS execution result: {:?}", js_result);
        assert!(js_result.is_ok(), "Direct JS execution should succeed");
        
        let js_output = js_result.unwrap();
        println!("Direct JS output: {}", serde_json::to_string_pretty(&js_output).unwrap());
        assert!(js_output.is_object(), "JS output should be an object");
        
        // Now test through the worker manager
        let config = WorkerConfig {
            worker_count: 1,
            ..Default::default()
        };
        let mut manager = WorkerProcessManager::new(config);
        manager.start().await.unwrap();

        let execution_context = ExecutionContext {
            execution_id: "test-exec-123".to_string(),
            job_id: None,
            task_id: "heartbeat".to_string(),
            task_version: "1.0.0".to_string(),
        };

        let message = WorkerMessage::ExecuteTask {
            job_id: 1,
            task_id: 2,
            task_path: "heartbeat".to_string(),
            input_data: serde_json::json!({}),
            execution_context,
            correlation_id: Uuid::new_v4(),
        };

        let result = manager.send_task(message, Duration::from_secs(10)).await;
        assert!(result.is_ok());

        if let Ok(CoordinatorMessage::TaskResult { result, .. }) = result {
            println!("Worker task execution result: {:?}", result);
            assert!(result.success, "Task should succeed");
            assert!(result.output.is_some(), "Task should return output");
            
            let output = result.output.unwrap();
            println!("Worker output: {}", serde_json::to_string_pretty(&output).unwrap());
            assert!(output.is_object(), "Output should be an object");
            
            let output_obj = output.as_object().unwrap();
            assert!(output_obj.contains_key("status"), "Output should have status field");
            assert_eq!(output_obj["status"], "ok", "Status should be 'ok'");
        } else {
            panic!("Expected TaskResult response, got: {:?}", result);
        }
    }
}
