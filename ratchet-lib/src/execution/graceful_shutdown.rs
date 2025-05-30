use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tracing::{info, warn, error};

/// Shutdown signal types with escalating urgency
#[derive(Debug, Clone, Copy)]
pub enum ShutdownSignal {
    /// Graceful shutdown - allow current tasks to complete
    Graceful,
    /// Urgent shutdown - stop accepting new tasks, finish current ones quickly
    Urgent,
    /// Forced shutdown - terminate immediately
    Forced,
}

/// Graceful shutdown coordinator
pub struct ShutdownCoordinator {
    sender: broadcast::Sender<ShutdownSignal>,
    is_shutting_down: Arc<RwLock<bool>>,
    active_tasks: Arc<RwLock<u32>>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(16);
        
        Self {
            sender,
            is_shutting_down: Arc::new(RwLock::new(false)),
            active_tasks: Arc::new(RwLock::new(0)),
        }
    }

    /// Subscribe to shutdown signals
    pub fn subscribe(&self) -> broadcast::Receiver<ShutdownSignal> {
        self.sender.subscribe()
    }

    /// Check if shutdown is in progress
    pub async fn is_shutting_down(&self) -> bool {
        *self.is_shutting_down.read().await
    }

    /// Increment active task counter
    pub async fn task_started(&self) {
        let mut count = self.active_tasks.write().await;
        *count += 1;
    }

    /// Decrement active task counter
    pub async fn task_completed(&self) {
        let mut count = self.active_tasks.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Get current active task count
    pub async fn active_task_count(&self) -> u32 {
        *self.active_tasks.read().await
    }

    /// Initiate graceful shutdown with escalating urgency
    pub async fn shutdown(&self) -> Result<(), ShutdownError> {
        {
            let mut shutting_down = self.is_shutting_down.write().await;
            *shutting_down = true;
        }

        info!("Starting graceful shutdown");

        // Phase 1: Graceful shutdown (30 seconds)
        self.sender.send(ShutdownSignal::Graceful)
            .map_err(|_| ShutdownError::BroadcastError)?;

        if self.wait_for_tasks(Duration::from_secs(30)).await {
            info!("Graceful shutdown completed successfully");
            return Ok(());
        }

        // Phase 2: Urgent shutdown (10 seconds)
        warn!("Graceful shutdown timeout, escalating to urgent shutdown");
        self.sender.send(ShutdownSignal::Urgent)
            .map_err(|_| ShutdownError::BroadcastError)?;

        if self.wait_for_tasks(Duration::from_secs(10)).await {
            info!("Urgent shutdown completed");
            return Ok(());
        }

        // Phase 3: Forced shutdown
        error!("Urgent shutdown timeout, forcing shutdown");
        self.sender.send(ShutdownSignal::Forced)
            .map_err(|_| ShutdownError::BroadcastError)?;

        // Give a brief moment for forced shutdown to take effect
        tokio::time::sleep(Duration::from_millis(500)).await;

        let remaining_tasks = self.active_task_count().await;
        if remaining_tasks > 0 {
            warn!("Forced shutdown completed with {} tasks still active", remaining_tasks);
        } else {
            info!("Forced shutdown completed successfully");
        }

        Ok(())
    }

    /// Wait for all tasks to complete within the given timeout
    async fn wait_for_tasks(&self, timeout_duration: Duration) -> bool {
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout_duration {
            let active = self.active_task_count().await;
            if active == 0 {
                return true;
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        false
    }
}

/// Task that can be gracefully shut down
#[async_trait::async_trait]
pub trait GracefulTask {
    /// Handle shutdown signal
    async fn handle_shutdown(&mut self, signal: ShutdownSignal) -> Result<(), ShutdownError>;

    /// Check if task should continue running
    async fn should_continue(&self) -> bool;
}

/// Shutdown-aware task wrapper
pub struct ShutdownAwareTask<T> {
    task: T,
    shutdown_rx: broadcast::Receiver<ShutdownSignal>,
    coordinator: Arc<ShutdownCoordinator>,
    should_continue: bool,
}

impl<T> ShutdownAwareTask<T>
where
    T: GracefulTask + Send + Sync,
{
    pub fn new(task: T, coordinator: Arc<ShutdownCoordinator>) -> Self {
        let shutdown_rx = coordinator.subscribe();
        
        Self {
            task,
            shutdown_rx,
            coordinator,
            should_continue: true,
        }
    }

    /// Run the task with shutdown awareness
    pub async fn run<F, Fut, R>(mut self, task_fn: F) -> Result<R, ShutdownError>
    where
        F: FnOnce(T) -> Fut,
        Fut: std::future::Future<Output = Result<R, ShutdownError>>,
    {
        // Register task start
        self.coordinator.task_started().await;

        let result = tokio::select! {
            // Main task execution
            result = task_fn(self.task) => {
                result
            }
            
            // Shutdown signal handling
            signal = self.shutdown_rx.recv() => {
                match signal {
                    Ok(signal) => {
                        info!("Received shutdown signal: {:?}", signal);
                        self.task.handle_shutdown(signal).await?;
                        Err(ShutdownError::TaskCancelled)
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        Err(ShutdownError::ChannelClosed)
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Shutdown signal lagged, assuming forced shutdown");
                        self.task.handle_shutdown(ShutdownSignal::Forced).await?;
                        Err(ShutdownError::TaskCancelled)
                    }
                }
            }
        };

        // Register task completion
        self.coordinator.task_completed().await;

        result
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    #[error("Broadcast error")]
    BroadcastError,
    
    #[error("Task was cancelled during shutdown")]
    TaskCancelled,
    
    #[error("Shutdown channel closed")]
    ChannelClosed,
    
    #[error("Task execution error: {0}")]
    TaskError(String),
}

/// Process shutdown manager for external processes
pub struct ProcessShutdownManager;

impl ProcessShutdownManager {
    /// Shutdown a process gracefully with escalating signals
    pub async fn shutdown_process(
        mut child: tokio::process::Child,
    ) -> Result<(), ShutdownError> {
        // Phase 1: SIGTERM (graceful)
        if let Some(id) = child.id() {
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                
                if let Err(e) = signal::kill(Pid::from_raw(id as i32), Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to process {}: {}", id, e);
                }
            }
        }

        // Wait for graceful termination
        match timeout(Duration::from_secs(10), child.wait()).await {
            Ok(Ok(status)) => {
                info!("Process terminated gracefully with status: {:?}", status);
                return Ok(());
            }
            Ok(Err(e)) => {
                error!("Error waiting for process: {}", e);
            }
            Err(_) => {
                warn!("Process did not terminate gracefully within timeout");
            }
        }

        // Phase 2: SIGKILL (forced)
        if let Err(e) = child.kill().await {
            error!("Failed to kill process: {}", e);
            return Err(ShutdownError::TaskError(e.to_string()));
        }

        // Final wait
        match timeout(Duration::from_secs(5), child.wait()).await {
            Ok(Ok(status)) => {
                info!("Process killed successfully with status: {:?}", status);
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Error waiting for killed process: {}", e);
                Err(ShutdownError::TaskError(e.to_string()))
            }
            Err(_) => {
                error!("Process did not terminate even after SIGKILL");
                Err(ShutdownError::TaskError("Process unresponsive to SIGKILL".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestTask {
        should_shutdown: Arc<AtomicBool>,
    }

    impl TestTask {
        fn new() -> Self {
            Self {
                should_shutdown: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait::async_trait]
    impl GracefulTask for TestTask {
        async fn handle_shutdown(&mut self, _signal: ShutdownSignal) -> Result<(), ShutdownError> {
            self.should_shutdown.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn should_continue(&self) -> bool {
            !self.should_shutdown.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        
        // Test task counting
        coordinator.task_started().await;
        coordinator.task_started().await;
        assert_eq!(coordinator.active_task_count().await, 2);
        
        coordinator.task_completed().await;
        assert_eq!(coordinator.active_task_count().await, 1);
        
        coordinator.task_completed().await;
        assert_eq!(coordinator.active_task_count().await, 0);
    }

    #[tokio::test]
    async fn test_shutdown_aware_task() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let task = TestTask::new();
        let shutdown_task = ShutdownAwareTask::new(task, coordinator.clone());

        // Start a task that listens for shutdown
        let handle = tokio::spawn(async move {
            shutdown_task.run(|mut task| async move {
                while task.should_continue().await {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(())
            }).await
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Initiate shutdown
        let shutdown_handle = tokio::spawn(async move {
            coordinator.shutdown().await
        });

        // Both should complete
        let (task_result, shutdown_result) = tokio::join!(handle, shutdown_handle);
        
        assert!(task_result.is_ok());
        assert!(shutdown_result.is_ok());
    }
}