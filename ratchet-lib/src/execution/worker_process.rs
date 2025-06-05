use std::collections::HashMap;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::execution::ipc::{
    CoordinatorMessage, IpcError, MessageEnvelope, TaskExecutionResult, TaskValidationResult,
    WorkerMessage, WorkerStatus,
};

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

/// A single worker process handle
#[derive(Debug)]
pub struct WorkerProcess {
    pub id: String,
    pub pid: Option<u32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub restart_count: u32,
    pub status: WorkerProcessStatus,
    child: Option<Child>,
    stdin_tx: Option<mpsc::UnboundedSender<WorkerMessage>>,
    last_health_check: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerProcessStatus {
    Starting,
    Ready,
    Busy,
    Unresponsive,
    Failed,
    Stopped,
}

impl WorkerProcess {
    /// Spawn a new worker process
    pub async fn spawn(
        worker_id: String,
        _config: &WorkerConfig,
        message_tx: mpsc::UnboundedSender<WorkerToManagerMessage>,
    ) -> Result<Self, WorkerProcessError> {
        debug!("Spawning worker process: {}", worker_id);

        // Get current executable path
        let current_exe = std::env::current_exe().map_err(|e| {
            WorkerProcessError::SpawnError(format!("Failed to get current exe: {}", e))
        })?;

        let mut cmd = Command::new(&current_exe);
        cmd.arg("--worker")
            .arg("--worker-id")
            .arg(&worker_id)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| {
            WorkerProcessError::SpawnError(format!("Failed to spawn worker: {}", e))
        })?;

        let pid = child.id();

        // Set up communication channels
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| WorkerProcessError::SpawnError("Failed to get stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WorkerProcessError::SpawnError("Failed to get stdout".to_string()))?;

        let (stdin_tx, stdin_rx) = mpsc::unbounded_channel();

        // Spawn stdin writer task
        let worker_id_clone = worker_id.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::stdin_writer_task(worker_id_clone, stdin, stdin_rx).await {
                error!("Worker stdin writer failed: {}", e);
            }
        });

        // Spawn stdout reader task
        let worker_id_clone = worker_id.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::stdout_reader_task(worker_id_clone, stdout, message_tx).await {
                error!("Worker stdout reader failed: {}", e);
            }
        });

        Ok(Self {
            id: worker_id,
            pid,
            started_at: chrono::Utc::now(),
            restart_count: 0,
            status: WorkerProcessStatus::Starting,
            child: Some(child),
            stdin_tx: Some(stdin_tx),
            last_health_check: None,
        })
    }

    /// Send a message to the worker
    pub async fn send_message(&mut self, message: WorkerMessage) -> Result<(), WorkerProcessError> {
        if let Some(stdin_tx) = &self.stdin_tx {
            stdin_tx.send(message).map_err(|e| {
                WorkerProcessError::CommunicationError(format!("Failed to send message: {}", e))
            })?;
            Ok(())
        } else {
            Err(WorkerProcessError::WorkerNotRunning)
        }
    }

    /// Execute a task on this worker
    pub async fn execute_task(
        &mut self,
        job_id: i32,
        task_id: i32,
        task_path: String,
        input_data: serde_json::Value,
    ) -> Result<TaskExecutionResult, WorkerProcessError> {
        let correlation_id = Uuid::new_v4();

        // Create a placeholder execution context for testing
        let execution_context = crate::execution::ipc::ExecutionContext::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            "1.0.0".to_string(),
        );
        
        let message = WorkerMessage::ExecuteTask {
            job_id,
            task_id,
            task_path,
            input_data,
            execution_context,
            correlation_id,
        };

        self.status = WorkerProcessStatus::Busy;
        self.send_message(message).await?;

        // TODO: Wait for response with timeout
        // For now, return a placeholder
        Ok(TaskExecutionResult {
            success: true,
            output: Some(serde_json::json!({"placeholder": true})),
            error_message: None,
            error_details: None,
            started_at: chrono::Utc::now(),
            completed_at: chrono::Utc::now(),
            duration_ms: 100,
        })
    }

    /// Perform health check on the worker
    pub async fn health_check(&mut self) -> Result<WorkerStatus, WorkerProcessError> {
        let correlation_id = Uuid::new_v4();

        let message = WorkerMessage::Ping { correlation_id };
        self.send_message(message).await?;

        self.last_health_check = Some(chrono::Utc::now());

        // TODO: Wait for pong response with timeout
        // For now, return a placeholder
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

    /// Stop the worker process gracefully
    pub async fn stop(&mut self) -> Result<(), WorkerProcessError> {
        debug!("Stopping worker process: {}", self.id);

        // Check if worker is already stopped
        if self.status == WorkerProcessStatus::Stopped || self.child.is_none() {
            debug!("Worker {} already stopped", self.id);
            return Ok(());
        }

        // Send shutdown message (ignore errors as worker may have already terminated)
        let _ = self.send_message(WorkerMessage::Shutdown).await;

        // Close stdin to signal shutdown
        self.stdin_tx = None;

        // Wait for graceful shutdown with timeout
        if let Some(mut child) = self.child.take() {
            // Use a shorter timeout and check if process is still alive
            let shutdown_timeout = Duration::from_millis(500);

            match tokio::time::timeout(shutdown_timeout, child.wait()).await {
                Ok(Ok(_exit_status)) => {
                    debug!("Worker {} terminated gracefully", self.id);
                }
                Ok(Err(e)) => {
                    debug!("Worker {} wait failed: {}", self.id, e);
                }
                Err(_) => {
                    // Timeout - force kill
                    debug!(
                        "Worker {} didn't respond to shutdown, force killing",
                        self.id
                    );
                    if let Err(e) = child.kill().await {
                        debug!("Failed to kill worker process {}: {}", self.id, e);
                    }
                }
            }
        }

        self.status = WorkerProcessStatus::Stopped;

        Ok(())
    }

    /// Check if the worker is available for work
    pub fn is_available(&self) -> bool {
        matches!(self.status, WorkerProcessStatus::Ready)
    }

    /// Stdin writer task
    async fn stdin_writer_task(
        worker_id: String,
        mut stdin: tokio::process::ChildStdin,
        mut rx: mpsc::UnboundedReceiver<WorkerMessage>,
    ) -> Result<(), WorkerProcessError> {
        use tokio::io::AsyncWriteExt;

        while let Some(message) = rx.recv().await {
            let envelope = MessageEnvelope::new(message);
            let json = serde_json::to_string(&envelope)
                .map_err(|e| WorkerProcessError::CommunicationError(e.to_string()))?;

            let line = format!("{}\n", json);

            if let Err(e) = stdin.write_all(line.as_bytes()).await {
                // During shutdown, broken pipe errors are expected - don't log as error
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    debug!(
                        "Worker {} stdin closed (worker likely terminated)",
                        worker_id
                    );
                } else {
                    error!("Failed to write to worker {} stdin: {}", worker_id, e);
                }
                break;
            }

            if let Err(e) = stdin.flush().await {
                // During shutdown, broken pipe errors are expected - don't log as error
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    debug!(
                        "Worker {} stdin closed during flush (worker likely terminated)",
                        worker_id
                    );
                } else {
                    error!("Failed to flush worker {} stdin: {}", worker_id, e);
                }
                break;
            }
        }

        Ok(())
    }

    /// Stdout reader task
    async fn stdout_reader_task(
        worker_id: String,
        stdout: tokio::process::ChildStdout,
        message_tx: mpsc::UnboundedSender<WorkerToManagerMessage>,
    ) -> Result<(), WorkerProcessError> {
        use tokio::io::AsyncBufReadExt;

        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    debug!("Worker {} stdout closed", worker_id);
                    break;
                }
                Ok(_) => {
                    // Remove newline
                    line.truncate(line.trim_end().len());

                    match serde_json::from_str::<MessageEnvelope<CoordinatorMessage>>(&line) {
                        Ok(envelope) => {
                            debug!(
                                "Received message from worker {}: {:?}",
                                worker_id, envelope.message
                            );
                            // Send message to manager
                            let worker_msg = WorkerToManagerMessage {
                                worker_id: worker_id.clone(),
                                message: envelope.message,
                            };
                            if let Err(e) = message_tx.send(worker_msg) {
                                error!("Failed to send message to manager: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse message from worker {}: {} - line: {}",
                                worker_id, e, line
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from worker {} stdout: {}", worker_id, e);
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Message from worker to manager
#[derive(Debug)]
pub struct WorkerToManagerMessage {
    pub worker_id: String,
    pub message: CoordinatorMessage,
}

/// Worker process manager
pub struct WorkerProcessManager {
    config: WorkerConfig,
    workers: Arc<Mutex<Vec<WorkerProcess>>>,
    pending_tasks: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TaskExecutionResult>>>>,
    _pending_validations: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TaskValidationResult>>>>,
    _pending_health_checks: Arc<Mutex<HashMap<Uuid, oneshot::Sender<WorkerStatus>>>>,
    message_tx: mpsc::UnboundedSender<WorkerToManagerMessage>,
}

impl WorkerProcessManager {
    /// Create a new worker process manager
    pub fn new(config: WorkerConfig) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let workers = Arc::new(Mutex::new(Vec::new()));
        let pending_tasks = Arc::new(Mutex::new(HashMap::new()));
        let _pending_validations = Arc::new(Mutex::new(HashMap::new()));
        let _pending_health_checks = Arc::new(Mutex::new(HashMap::new()));
        
        // Start message processing task
        let workers_clone = workers.clone();
        let pending_tasks_clone = pending_tasks.clone();
        let pending_validations_clone = _pending_validations.clone();
        let pending_health_checks_clone = _pending_health_checks.clone();
        tokio::spawn(async move {
            Self::process_worker_messages(
                workers_clone, 
                pending_tasks_clone,
                pending_validations_clone,
                pending_health_checks_clone,
                message_rx
            ).await;
        });
        
        Self {
            config,
            workers,
            pending_tasks,
            _pending_validations,
            _pending_health_checks,
            message_tx,
        }
    }

    /// Start all worker processes
    pub async fn start(&mut self) -> Result<(), WorkerProcessError> {
        info!("Starting {} worker processes", self.config.worker_count);

        for i in 0..self.config.worker_count {
            let worker_id = format!("worker-{}", i);

            match WorkerProcess::spawn(worker_id.clone(), &self.config, self.message_tx.clone()).await {
                Ok(worker) => {
                    let mut workers = self.workers.lock().await;
                    workers.push(worker);
                    debug!("Successfully started worker: {}", worker_id);
                }
                Err(e) => {
                    error!("Failed to start worker {}: {}", worker_id, e);
                    return Err(e);
                }
            }
        }

        info!("All worker processes started successfully");
        Ok(())
    }

    /// Stop all worker processes
    pub async fn stop(&mut self) -> Result<(), WorkerProcessError> {
        info!("Stopping all worker processes");

        let mut workers = self.workers.lock().await;
        // Stop all workers sequentially but with improved error handling
        for worker in workers.iter_mut() {
            match worker.stop().await {
                Ok(_) => debug!("Successfully stopped worker: {}", worker.id),
                Err(e) => debug!("Worker {} stop completed with: {}", worker.id, e),
            }
        }

        workers.clear();
        info!("All worker processes stopped");
        Ok(())
    }

    /// Process messages from workers
    async fn process_worker_messages(
        workers: Arc<Mutex<Vec<WorkerProcess>>>,
        pending_tasks: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TaskExecutionResult>>>>,
        pending_validations: Arc<Mutex<HashMap<Uuid, oneshot::Sender<TaskValidationResult>>>>,
        pending_health_checks: Arc<Mutex<HashMap<Uuid, oneshot::Sender<WorkerStatus>>>>,
        mut message_rx: mpsc::UnboundedReceiver<WorkerToManagerMessage>,
    ) {
        while let Some(worker_msg) = message_rx.recv().await {
            debug!("Processing message from worker {}: {:?}", worker_msg.worker_id, worker_msg.message);
            
            // Update worker status
            if let Ok(mut workers) = workers.try_lock() {
                if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_msg.worker_id) {
                    match &worker_msg.message {
                        CoordinatorMessage::Ready { .. } => {
                            debug!("Worker {} is ready", worker.id);
                            worker.status = WorkerProcessStatus::Ready;
                        }
                        CoordinatorMessage::TaskResult { .. } => {
                            debug!("Worker {} completed task", worker.id);
                            worker.status = WorkerProcessStatus::Ready;
                        }
                        CoordinatorMessage::TaskProgress { .. } => {
                            debug!("Worker {} sent progress update", worker.id);
                            // Progress updates don't change worker status
                        }
                        CoordinatorMessage::Error { .. } => {
                            debug!("Worker {} reported error", worker.id);
                            worker.status = WorkerProcessStatus::Ready;
                        }
                        _ => {
                            debug!("Unhandled message from worker {}: {:?}", worker.id, worker_msg.message);
                        }
                    }
                }
            }

            // Handle pending task responses
            match worker_msg.message {
                CoordinatorMessage::TaskResult { correlation_id, result, .. } => {
                    if let Ok(mut pending) = pending_tasks.try_lock() {
                        if let Some(sender) = pending.remove(&correlation_id) {
                            debug!("Resolving pending task with correlation_id: {}", correlation_id);
                            if sender.send(result).is_err() {
                                warn!("Failed to send task result - receiver may have been dropped");
                            }
                        } else {
                            warn!("Received TaskResult for unknown correlation_id: {}", correlation_id);
                        }
                    }
                }
                CoordinatorMessage::TaskProgress { correlation_id, progress, .. } => {
                    debug!("Received progress update for correlation_id: {} - {:?}", correlation_id, progress);
                    // TODO: Forward progress updates to MCP progress manager
                    // For now, just log the progress
                }
                CoordinatorMessage::ValidationResult { correlation_id, result } => {
                    if let Ok(mut pending) = pending_validations.try_lock() {
                        if let Some(sender) = pending.remove(&correlation_id) {
                            debug!("Resolving pending validation with correlation_id: {}", correlation_id);
                            if sender.send(result).is_err() {
                                warn!("Failed to send validation result - receiver may have been dropped");
                            }
                        }
                    }
                }
                CoordinatorMessage::Pong { correlation_id, status, .. } => {
                    if let Ok(mut pending) = pending_health_checks.try_lock() {
                        if let Some(sender) = pending.remove(&correlation_id) {
                            debug!("Resolving pending health check with correlation_id: {}", correlation_id);
                            if sender.send(status).is_err() {
                                warn!("Failed to send health check result - receiver may have been dropped");
                            }
                        }
                    }
                }
                CoordinatorMessage::Error { correlation_id, error } => {
                    if let Some(correlation_id) = correlation_id {
                        // Try to resolve as a task error
                        if let Ok(mut pending) = pending_tasks.try_lock() {
                            if let Some(sender) = pending.remove(&correlation_id) {
                                debug!("Resolving pending task with error for correlation_id: {}", correlation_id);
                                let error_result = TaskExecutionResult {
                                    success: false,
                                    output: None,
                                    error_message: Some(format!("Worker error: {:?}", error)),
                                    error_details: None,
                                    started_at: chrono::Utc::now(),
                                    completed_at: chrono::Utc::now(),
                                    duration_ms: 0,
                                };
                                if sender.send(error_result).is_err() {
                                    warn!("Failed to send error result - receiver may have been dropped");
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Already handled above
                }
            }
        }
        debug!("Worker message processing task ended");
    }

    /// Get an available worker for task execution
    pub async fn get_available_worker(&mut self) -> Option<String> {
        // Return worker ID instead of reference since we can't return a mutable reference to Arc<Mutex<>>
        let workers = self.workers.lock().await;
        workers.iter()
            .find(|w| w.is_available())
            .map(|w| w.id.clone())
    }

    /// Get worker statistics
    pub async fn get_worker_stats(&self) -> Vec<(String, WorkerProcessStatus)> {
        let workers = self.workers.lock().await;
        workers
            .iter()
            .map(|w| (w.id.clone(), w.status.clone()))
            .collect()
    }

    /// Perform health checks on all workers
    pub async fn health_check_all(&mut self) -> Vec<Result<WorkerStatus, WorkerProcessError>> {
        let mut results = Vec::new();

        let mut workers = self.workers.lock().await;
        for worker in workers.iter_mut() {
            results.push(worker.health_check().await);
        }

        results
    }

    /// Send a task to an available worker and wait for the result
    pub async fn send_task(
        &mut self,
        message: WorkerMessage,
        timeout_duration: Duration,
    ) -> Result<CoordinatorMessage, WorkerProcessError> {
        use tokio::time::timeout;

        // Extract correlation ID from message
        let correlation_id = match &message {
            WorkerMessage::ExecuteTask { correlation_id, .. } => *correlation_id,
            WorkerMessage::ValidateTask { correlation_id, .. } => *correlation_id,
            WorkerMessage::Ping { correlation_id } => *correlation_id,
            WorkerMessage::Shutdown => {
                return Err(WorkerProcessError::CommunicationError(
                    "Cannot send shutdown to specific worker".to_string(),
                ))
            }
        };

        // Find available worker ID
        let worker_id = {
            let workers = self.workers.lock().await;
            workers.iter()
                .find(|w| w.is_available())
                .map(|w| w.id.clone())
                .ok_or_else(|| {
                    WorkerProcessError::CommunicationError("No available workers".to_string())
                })?
        };

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Store the response channel based on message type
        match &message {
            WorkerMessage::ExecuteTask { .. } => {
                let mut pending = self.pending_tasks.lock().await;
                pending.insert(correlation_id, tx);
            }
            WorkerMessage::ValidateTask { .. } => {
                // For validation tasks, we need a different channel type
                // For now, let's handle this as a task execution
                let mut pending = self.pending_tasks.lock().await;
                pending.insert(correlation_id, tx);
            }
            WorkerMessage::Ping { .. } => {
                // For ping messages, we also use the task channel for simplicity
                let mut pending = self.pending_tasks.lock().await;
                pending.insert(correlation_id, tx);
            }
            _ => {}
        }

        // Send message to worker
        {
            let mut workers = self.workers.lock().await;
            if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
                worker.send_message(message).await?;
            } else {
                return Err(WorkerProcessError::CommunicationError("Worker not found".to_string()));
            }
        }

        // Wait for response with timeout
        match timeout(timeout_duration, rx).await {
            Ok(Ok(result)) => {
                // Convert TaskExecutionResult to CoordinatorMessage
                Ok(CoordinatorMessage::TaskResult {
                    job_id: 0, // We'll need to track this properly
                    correlation_id,
                    result,
                })
            }
            Ok(Err(_)) => Err(WorkerProcessError::CommunicationError(
                "Response channel closed".to_string(),
            )),
            Err(_) => {
                // Clean up pending task on timeout
                if let Ok(mut pending) = self.pending_tasks.try_lock() {
                    pending.remove(&correlation_id);
                }
                Err(WorkerProcessError::Timeout)
            }
        }
    }
}

/// Worker process errors
#[derive(Debug, thiserror::Error)]
pub enum WorkerProcessError {
    #[error("Failed to spawn worker process: {0}")]
    SpawnError(String),

    #[error("Worker communication error: {0}")]
    CommunicationError(String),

    #[error("Worker is not running")]
    WorkerNotRunning,

    #[error("Worker timeout")]
    Timeout,

    #[error("Worker process crashed")]
    WorkerCrashed,

    #[error("IPC error: {0}")]
    IpcError(#[from] IpcError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_config_default() {
        let config = WorkerConfig::default();
        assert!(config.worker_count > 0);
        assert!(config.restart_on_crash);
    }

    #[test]
    fn test_worker_process_status() {
        let worker = WorkerProcess {
            id: "test-worker".to_string(),
            pid: Some(12345),
            started_at: chrono::Utc::now(),
            restart_count: 0,
            status: WorkerProcessStatus::Ready,
            child: None,
            stdin_tx: None,
            last_health_check: None,
        };

        assert!(worker.is_available());
        assert_eq!(worker.id, "test-worker");
    }
}

