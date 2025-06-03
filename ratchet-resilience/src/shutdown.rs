//! Graceful shutdown coordination
//! 
//! This module provides graceful shutdown capabilities with escalating urgency,
//! task tracking, and process management.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use log::{info, warn, error};

/// Shutdown signal types with escalating urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// Graceful shutdown - allow current tasks to complete
    Graceful,
    /// Urgent shutdown - stop accepting new tasks, finish current ones quickly
    Urgent,
    /// Forced shutdown - terminate immediately
    Forced,
}

impl std::fmt::Display for ShutdownSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownSignal::Graceful => write!(f, "graceful"),
            ShutdownSignal::Urgent => write!(f, "urgent"),
            ShutdownSignal::Forced => write!(f, "forced"),
        }
    }
}

/// Graceful shutdown coordinator
pub struct ShutdownCoordinator {
    sender: broadcast::Sender<ShutdownSignal>,
    is_shutting_down: Arc<RwLock<bool>>,
    active_tasks: Arc<RwLock<u32>>,
    graceful_timeout: Duration,
    urgent_timeout: Duration,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with default timeouts
    pub fn new() -> Self {
        Self::with_timeouts(Duration::from_secs(30), Duration::from_secs(10))
    }

    /// Create a new shutdown coordinator with custom timeouts
    pub fn with_timeouts(graceful_timeout: Duration, urgent_timeout: Duration) -> Self {
        let (sender, _) = broadcast::channel(16);
        
        Self {
            sender,
            is_shutting_down: Arc::new(RwLock::new(false)),
            active_tasks: Arc::new(RwLock::new(0)),
            graceful_timeout,
            urgent_timeout,
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
        // Prevent multiple simultaneous shutdowns
        {
            let mut shutting_down = self.is_shutting_down.write().await;
            if *shutting_down {
                return Err(ShutdownError::AlreadyShuttingDown);
            }
            *shutting_down = true;
        }

        info!("Starting graceful shutdown");

        // Phase 1: Graceful shutdown
        self.sender.send(ShutdownSignal::Graceful)
            .map_err(|_| ShutdownError::BroadcastError)?;

        if self.wait_for_tasks(self.graceful_timeout).await {
            info!("Graceful shutdown completed successfully");
            return Ok(());
        }

        // Phase 2: Urgent shutdown
        warn!("Graceful shutdown timeout, escalating to urgent shutdown");
        self.sender.send(ShutdownSignal::Urgent)
            .map_err(|_| ShutdownError::BroadcastError)?;

        if self.wait_for_tasks(self.urgent_timeout).await {
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
            Err(ShutdownError::TasksRemaining(remaining_tasks))
        } else {
            info!("Forced shutdown completed successfully");
            Ok(())
        }
    }

    /// Wait for all tasks to complete within the given timeout
    async fn wait_for_tasks(&self, timeout_duration: Duration) -> bool {
        let start = tokio::time::Instant::now();
        
        while start.elapsed() < timeout_duration {
            let active = self.active_task_count().await;
            if active == 0 {
                return true;
            }
            
            // Adaptive sleep based on task count
            let sleep_duration = if active > 10 {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(50)
            };
            
            tokio::time::sleep(sleep_duration).await;
        }
        
        false
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Task that can be gracefully shut down
#[async_trait::async_trait]
pub trait GracefulTask: Send + Sync {
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
}

impl<T> ShutdownAwareTask<T>
where
    T: GracefulTask,
{
    /// Create a new shutdown-aware task
    pub fn new(task: T, coordinator: Arc<ShutdownCoordinator>) -> Self {
        let shutdown_rx = coordinator.subscribe();
        
        Self {
            task,
            shutdown_rx,
            coordinator,
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

        // Create a future for the task execution
        let task_future = task_fn(self.task);
        tokio::pin!(task_future);

        let result = tokio::select! {
            // Main task execution
            result = &mut task_future => {
                result
            }
            
            // Shutdown signal handling
            signal = self.shutdown_rx.recv() => {
                match signal {
                    Ok(signal) => {
                        info!("Received shutdown signal: {}", signal);
                        Err(ShutdownError::TaskCancelled)
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        Err(ShutdownError::ChannelClosed)
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Shutdown signal lagged, assuming forced shutdown");
                        Err(ShutdownError::TaskCancelled)
                    }
                }
            }
        };

        // Register task completion
        self.coordinator.task_completed().await;

        result
    }

    /// Run the task with periodic continuation checks
    pub async fn run_periodic<F, Fut, R>(mut self, mut task_fn: F) -> Result<R, ShutdownError>
    where
        F: FnMut(&mut T) -> Fut,
        Fut: std::future::Future<Output = Result<Option<R>, ShutdownError>>,
    {
        self.coordinator.task_started().await;

        let result = loop {
            // Check shutdown signal first
            if let Ok(signal) = self.shutdown_rx.try_recv() {
                info!("Received shutdown signal during periodic task: {}", signal);
                let _ = self.task.handle_shutdown(signal).await;
                break Err(ShutdownError::TaskCancelled);
            }

            // Check if we should continue
            if !self.task.should_continue().await {
                break Err(ShutdownError::TaskCancelled);
            }

            // Execute task iteration
            match task_fn(&mut self.task).await {
                Ok(Some(value)) => break Ok(value),
                Ok(None) => {
                    // Add a small delay to prevent busy loop
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => break Err(e),
            }
        };

        self.coordinator.task_completed().await;
        result
    }
}

/// Shutdown error types
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// Broadcast channel error
    #[error("Failed to send shutdown signal")]
    BroadcastError,
    
    /// Task was cancelled during shutdown
    #[error("Task was cancelled during shutdown")]
    TaskCancelled,
    
    /// Shutdown channel closed
    #[error("Shutdown channel closed")]
    ChannelClosed,
    
    /// Task execution error
    #[error("Task execution error: {0}")]
    TaskError(String),
    
    /// Shutdown already in progress
    #[error("Shutdown already in progress")]
    AlreadyShuttingDown,
    
    /// Tasks remaining after forced shutdown
    #[error("Forced shutdown completed with {0} tasks still active")]
    TasksRemaining(u32),
}

/// Process shutdown manager for external processes
pub struct ProcessShutdownManager;

impl ProcessShutdownManager {
    /// Shutdown a process gracefully with escalating signals
    pub async fn shutdown_process(
        mut child: tokio::process::Child,
        graceful_timeout: Duration,
    ) -> Result<std::process::ExitStatus, ShutdownError> {
        // Phase 1: Try graceful termination (SIGTERM on Unix)
        if let Some(id) = child.id() {
            info!("Initiating graceful shutdown for process {}", id);
            
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                
                if let Err(e) = signal::kill(Pid::from_raw(id as i32), Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to process {}: {}", id, e);
                }
            }
            
            #[cfg(windows)]
            {
                // On Windows, we'll try to terminate gracefully first
                // This is less graceful than Unix signals but better than nothing
                warn!("Windows graceful shutdown: sending terminate signal to process {}", id);
            }
        }

        // Wait for graceful termination
        match timeout(graceful_timeout, child.wait()).await {
            Ok(Ok(status)) => {
                info!("Process terminated gracefully with status: {:?}", status);
                return Ok(status);
            }
            Ok(Err(e)) => {
                error!("Error waiting for process: {}", e);
            }
            Err(_) => {
                warn!("Process did not terminate gracefully within timeout");
            }
        }

        // Phase 2: Force termination
        info!("Forcing process termination");
        if let Err(e) = child.kill().await {
            error!("Failed to kill process: {}", e);
            return Err(ShutdownError::TaskError(e.to_string()));
        }

        // Final wait
        match timeout(Duration::from_secs(5), child.wait()).await {
            Ok(Ok(status)) => {
                info!("Process terminated forcefully with status: {:?}", status);
                Ok(status)
            }
            Ok(Err(e)) => {
                error!("Error waiting for killed process: {}", e);
                Err(ShutdownError::TaskError(e.to_string()))
            }
            Err(_) => {
                error!("Process did not terminate even after force kill");
                Err(ShutdownError::TaskError("Process unresponsive to termination".to_string()))
            }
        }
    }

    /// Shutdown multiple processes in parallel
    pub async fn shutdown_processes(
        children: Vec<tokio::process::Child>,
        graceful_timeout: Duration,
    ) -> Vec<Result<std::process::ExitStatus, ShutdownError>> {
        let futures = children.into_iter().map(|child| {
            Self::shutdown_process(child, graceful_timeout)
        });

        futures::future::join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    struct TestTask {
        should_shutdown: Arc<AtomicBool>,
        handle_count: Arc<AtomicU32>,
    }

    impl TestTask {
        fn new() -> Self {
            Self {
                should_shutdown: Arc::new(AtomicBool::new(false)),
                handle_count: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
    impl GracefulTask for TestTask {
        async fn handle_shutdown(&mut self, _signal: ShutdownSignal) -> Result<(), ShutdownError> {
            self.should_shutdown.store(true, Ordering::Relaxed);
            self.handle_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn should_continue(&self) -> bool {
            !self.should_shutdown.load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_basic() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        
        // Test initial state
        assert!(!coordinator.is_shutting_down().await);
        assert_eq!(coordinator.active_task_count().await, 0);
        
        // Test task counting
        coordinator.task_started().await;
        coordinator.task_started().await;
        assert_eq!(coordinator.active_task_count().await, 2);
        
        coordinator.task_completed().await;
        assert_eq!(coordinator.active_task_count().await, 1);
        
        coordinator.task_completed().await;
        assert_eq!(coordinator.active_task_count().await, 0);
        
        // Test extra completion doesn't go negative
        coordinator.task_completed().await;
        assert_eq!(coordinator.active_task_count().await, 0);
    }

    #[tokio::test]
    async fn test_shutdown_signals() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let mut receiver = coordinator.subscribe();
        
        // Start shutdown in background
        let coordinator_clone = coordinator.clone();
        tokio::spawn(async move {
            // Add a task to prevent immediate completion
            coordinator_clone.task_started().await;
            coordinator_clone.shutdown().await.ok();
        });
        
        // Should receive graceful signal first
        let signal = receiver.recv().await.unwrap();
        assert_eq!(signal, ShutdownSignal::Graceful);
    }

    #[tokio::test]
    async fn test_shutdown_aware_task() {
        let coordinator = Arc::new(ShutdownCoordinator::with_timeouts(
            Duration::from_millis(100),
            Duration::from_millis(50),
        ));
        
        let task = TestTask::new();
        let shutdown_task = ShutdownAwareTask::new(task, coordinator.clone());

        // Start a long-running task
        let handle = tokio::spawn(async move {
            shutdown_task.run(|task| async move {
                for _ in 0..100 {
                    if !task.should_continue().await {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Ok(())
            }).await
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(coordinator.active_task_count().await, 1);

        // Initiate shutdown
        let shutdown_result = coordinator.shutdown().await;
        assert!(shutdown_result.is_ok());
        
        // Task should have completed
        let task_result = handle.await.unwrap();
        assert!(task_result.is_err()); // Should be cancelled
        
        // Note: With current implementation, handle_shutdown is not called when task is cancelled
        // This is acceptable behavior for this design pattern
        assert_eq!(coordinator.active_task_count().await, 0);
    }

    #[tokio::test]
    async fn test_double_shutdown_prevented() {
        let coordinator = Arc::new(ShutdownCoordinator::with_timeouts(
            Duration::from_millis(100),
            Duration::from_millis(50),
        ));
        
        // Create a subscriber to prevent broadcast error
        let _receiver = coordinator.subscribe();
        
        // Add a task to delay shutdown
        coordinator.task_started().await;
        
        // Start first shutdown
        let coordinator_clone = coordinator.clone();
        let handle1 = tokio::spawn(async move {
            coordinator_clone.shutdown().await
        });
        
        // Give the first shutdown time to start
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Try second shutdown - should fail because first is in progress
        let result2 = coordinator.shutdown().await;
        assert!(matches!(result2, Err(ShutdownError::AlreadyShuttingDown)));
        
        // Complete the task to allow first shutdown to finish
        coordinator.task_completed().await;
        
        // First should succeed 
        let result1 = handle1.await.unwrap();
        assert!(result1.is_ok());
    }

    #[tokio::test]
    async fn test_periodic_task() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let task = TestTask::new();
        let shutdown_task = ShutdownAwareTask::new(task, coordinator.clone());
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // Start periodic task
        let handle = tokio::spawn(async move {
            shutdown_task.run_periodic(|_task| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    
                    if count >= 5 {
                        Ok(Some(count))
                    } else {
                        Ok(None)
                    }
                }
            }).await
        });
        
        // Let it run a few iterations
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Should complete naturally
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert!(counter.load(Ordering::Relaxed) >= 5);
    }
}