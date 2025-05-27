use crate::execution::{
    executor::{TaskExecutor, ExecutionError, ExecutionResult},
    job_queue::{JobQueue, JobQueueError},
};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Worker pool errors
#[derive(Error, Debug)]
pub enum WorkerPoolError {
    #[error("Worker error: {0}")]
    WorkerError(String),
    
    #[error("Execution error: {0}")]
    ExecutionError(#[from] ExecutionError),
    
    #[error("Job queue error: {0}")]
    JobQueueError(#[from] JobQueueError),
    
    #[error("Worker pool is stopped")]
    PoolStopped,
    
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),
}

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of worker threads
    pub worker_count: usize,
    
    /// Polling interval for new jobs (in seconds)
    pub poll_interval_seconds: u64,
    
    /// Maximum number of jobs a worker can process concurrently
    pub max_concurrent_jobs: usize,
    
    /// Worker timeout for job execution (in seconds)
    pub job_timeout_seconds: u64,
    
    /// Batch size for dequeuing jobs
    pub dequeue_batch_size: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            poll_interval_seconds: 5,
            max_concurrent_jobs: 1, // Start with 1 job per worker for simplicity
            job_timeout_seconds: 300, // 5 minutes
            dequeue_batch_size: 10,
        }
    }
}

/// Worker status
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Stopped,
    Error,
}

/// Worker statistics
#[derive(Debug, Clone)]
pub struct WorkerStats {
    pub id: String,
    pub status: WorkerStatus,
    pub jobs_processed: u64,
    pub jobs_failed: u64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

/// Message types for worker communication
#[derive(Debug)]
enum WorkerMessage {
    Stop,
    GetStats(oneshot::Sender<WorkerStats>),
}

/// Individual worker
pub struct Worker {
    id: String,
    status: Arc<RwLock<WorkerStatus>>,
    stats: Arc<RwLock<WorkerStats>>,
    message_rx: mpsc::UnboundedReceiver<WorkerMessage>,
    config: WorkerConfig,
}

impl Worker {
    /// Create a new worker
    fn new(
        id: String,
        config: WorkerConfig,
        message_rx: mpsc::UnboundedReceiver<WorkerMessage>,
    ) -> Self {
        let stats = WorkerStats {
            id: id.clone(),
            status: WorkerStatus::Idle,
            jobs_processed: 0,
            jobs_failed: 0,
            started_at: chrono::Utc::now(),
            last_activity: None,
        };
        
        Self {
            id,
            status: Arc::new(RwLock::new(WorkerStatus::Idle)),
            stats: Arc::new(RwLock::new(stats)),
            message_rx,
            config,
        }
    }
    
    /// Run the worker loop
    async fn run(
        mut self,
        executor: Arc<dyn TaskExecutor>,
        job_queue: Arc<dyn JobQueue>,
    ) {
        info!("Worker {} starting", self.id);
        
        let mut poll_interval = interval(Duration::from_secs(self.config.poll_interval_seconds));
        
        loop {
            tokio::select! {
                // Handle control messages
                msg = self.message_rx.recv() => {
                    match msg {
                        Some(WorkerMessage::Stop) => {
                            info!("Worker {} received stop signal", self.id);
                            break;
                        }
                        Some(WorkerMessage::GetStats(sender)) => {
                            let stats = self.stats.read().await.clone();
                            let _ = sender.send(stats);
                        }
                        None => {
                            warn!("Worker {} message channel closed", self.id);
                            break;
                        }
                    }
                }
                
                // Poll for jobs
                _ = poll_interval.tick() => {
                    if let Err(e) = self.process_jobs(&executor, &job_queue).await {
                        error!("Worker {} error processing jobs: {}", self.id, e);
                        
                        // Update status to error
                        *self.status.write().await = WorkerStatus::Error;
                        
                        // Sleep a bit before trying again
                        sleep(Duration::from_secs(10)).await;
                        
                        // Reset status to idle
                        *self.status.write().await = WorkerStatus::Idle;
                    }
                }
            }
        }
        
        *self.status.write().await = WorkerStatus::Stopped;
        info!("Worker {} stopped", self.id);
    }
    
    /// Process available jobs
    async fn process_jobs(
        &self,
        executor: &Arc<dyn TaskExecutor>,
        job_queue: &Arc<dyn JobQueue>,
    ) -> Result<(), WorkerPoolError> {
        // Get jobs from queue
        let jobs = job_queue.dequeue_jobs(self.config.dequeue_batch_size).await?;
        
        if jobs.is_empty() {
            // No jobs available, stay idle
            return Ok(());
        }
        
        debug!("Worker {} processing {} jobs", self.id, jobs.len());
        
        // Update status to busy
        *self.status.write().await = WorkerStatus::Busy;
        
        // Process jobs
        for job in jobs {
            debug!("Worker {} processing job {}", self.id, job.id);
            
            // Update last activity
            {
                let mut stats = self.stats.write().await;
                stats.last_activity = Some(chrono::Utc::now());
            }
            
            // Execute job with timeout
            let result = tokio::time::timeout(
                Duration::from_secs(self.config.job_timeout_seconds),
                executor.execute_job(job.id),
            ).await;
            
            // Handle result
            match result {
                Ok(Ok(execution_result)) => {
                    if execution_result.success {
                        debug!("Worker {} completed job {} successfully", self.id, job.id);
                        
                        let mut stats = self.stats.write().await;
                        stats.jobs_processed += 1;
                    } else {
                        warn!("Worker {} job {} failed: {:?}", self.id, job.id, execution_result.error);
                        
                        let mut stats = self.stats.write().await;
                        stats.jobs_failed += 1;
                    }
                }
                Ok(Err(e)) => {
                    error!("Worker {} job {} execution error: {}", self.id, job.id, e);
                    
                    let mut stats = self.stats.write().await;
                    stats.jobs_failed += 1;
                }
                Err(_) => {
                    error!("Worker {} job {} timed out after {}s", self.id, job.id, self.config.job_timeout_seconds);
                    
                    let mut stats = self.stats.write().await;
                    stats.jobs_failed += 1;
                }
            }
        }
        
        // Update status back to idle
        *self.status.write().await = WorkerStatus::Idle;
        
        Ok(())
    }
}

/// Worker pool for managing multiple workers
pub struct WorkerPool {
    config: WorkerConfig,
    workers: Vec<WorkerHandle>,
    executor: Arc<dyn TaskExecutor>,
    job_queue: Arc<dyn JobQueue>,
}

/// Handle for communicating with a worker
struct WorkerHandle {
    id: String,
    message_tx: mpsc::UnboundedSender<WorkerMessage>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(
        config: WorkerConfig,
        executor: Arc<dyn TaskExecutor>,
        job_queue: Arc<dyn JobQueue>,
    ) -> Self {
        Self {
            config,
            workers: Vec::new(),
            executor,
            job_queue,
        }
    }
    
    /// Start the worker pool
    pub async fn start(&mut self) -> Result<(), WorkerPoolError> {
        info!("Starting worker pool with {} workers", self.config.worker_count);
        
        for i in 0..self.config.worker_count {
            let worker_id = format!("worker-{}", i);
            let (message_tx, message_rx) = mpsc::unbounded_channel();
            
            let worker = Worker::new(worker_id.clone(), self.config.clone(), message_rx);
            
            let executor = Arc::clone(&self.executor);
            let job_queue = Arc::clone(&self.job_queue);
            
            let task_handle = tokio::spawn(async move {
                worker.run(executor, job_queue).await;
            });
            
            let handle = WorkerHandle {
                id: worker_id,
                message_tx,
                task_handle,
            };
            
            self.workers.push(handle);
        }
        
        info!("Worker pool started with {} workers", self.workers.len());
        Ok(())
    }
    
    /// Stop the worker pool
    pub async fn stop(&mut self) -> Result<(), WorkerPoolError> {
        info!("Stopping worker pool");
        
        // Send stop signals to all workers
        for worker in &self.workers {
            if let Err(e) = worker.message_tx.send(WorkerMessage::Stop) {
                warn!("Failed to send stop signal to worker {}: {}", worker.id, e);
            }
        }
        
        // Wait for all workers to stop
        let mut handles = Vec::new();
        for worker in self.workers.drain(..) {
            handles.push(worker.task_handle);
        }
        
        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Worker task failed to complete cleanly: {}", e);
            }
        }
        
        info!("Worker pool stopped");
        Ok(())
    }
    
    /// Get statistics for all workers
    pub async fn get_stats(&self) -> Result<Vec<WorkerStats>, WorkerPoolError> {
        let mut stats = Vec::new();
        
        for worker in &self.workers {
            let (tx, rx) = oneshot::channel();
            
            if let Err(_) = worker.message_tx.send(WorkerMessage::GetStats(tx)) {
                // Worker might be stopped, skip it
                continue;
            }
            
            if let Ok(worker_stats) = rx.await {
                stats.push(worker_stats);
            }
        }
        
        Ok(stats)
    }
    
    /// Get pool summary statistics
    pub async fn get_pool_stats(&self) -> Result<PoolStats, WorkerPoolError> {
        let worker_stats = self.get_stats().await?;
        
        let total_workers = worker_stats.len();
        let idle_workers = worker_stats.iter().filter(|s| s.status == WorkerStatus::Idle).count();
        let busy_workers = worker_stats.iter().filter(|s| s.status == WorkerStatus::Busy).count();
        let error_workers = worker_stats.iter().filter(|s| s.status == WorkerStatus::Error).count();
        
        let total_jobs_processed = worker_stats.iter().map(|s| s.jobs_processed).sum();
        let total_jobs_failed = worker_stats.iter().map(|s| s.jobs_failed).sum();
        
        Ok(PoolStats {
            total_workers,
            idle_workers,
            busy_workers,
            error_workers,
            total_jobs_processed,
            total_jobs_failed,
        })
    }
    
    /// Check if pool is healthy
    pub async fn health_check(&self) -> Result<(), WorkerPoolError> {
        let stats = self.get_pool_stats().await?;
        
        if stats.error_workers > stats.total_workers / 2 {
            return Err(WorkerPoolError::WorkerError(
                format!("Too many workers in error state: {}/{}", stats.error_workers, stats.total_workers)
            ));
        }
        
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub error_workers: usize,
    pub total_jobs_processed: u64,
    pub total_jobs_failed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, RatchetConfig};
    use crate::database::{DatabaseConnection, repositories::RepositoryFactory};
    use crate::execution::{
        executor::DatabaseTaskExecutor,
        job_queue::JobQueueManager,
    };
    use crate::services::RatchetEngine;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    async fn create_test_setup() -> (WorkerPool, Arc<dyn JobQueue>) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();
        
        let mut config = RatchetConfig::default();
        config.server = Some(crate::config::ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            database: DatabaseConfig {
                url: format!("sqlite://{}?mode=rwc", db_path),
                max_connections: 5,
                connection_timeout: Duration::from_secs(10),
            },
            auth: None,
        });

        let db = DatabaseConnection::new(config.server.as_ref().unwrap().database.clone()).await.unwrap();
        db.migrate().await.unwrap();
        
        let repositories = RepositoryFactory::new(db);
        let job_queue = Arc::new(JobQueueManager::with_default_config(repositories.clone()));
        let engine = RatchetEngine::new(config).unwrap();
        let executor = Arc::new(DatabaseTaskExecutor::new(engine, repositories));
        
        let worker_config = WorkerConfig {
            worker_count: 2,
            poll_interval_seconds: 1,
            ..Default::default()
        };
        
        let pool = WorkerPool::new(worker_config, executor, Arc::clone(&job_queue));
        
        (pool, job_queue)
    }

    #[tokio::test]
    async fn test_worker_pool_creation() {
        let (pool, _) = create_test_setup().await;
        assert_eq!(pool.workers.len(), 0); // Workers not started yet
    }

    #[tokio::test]
    async fn test_worker_pool_start_stop() {
        let (mut pool, _) = create_test_setup().await;
        
        // Start pool
        assert!(pool.start().await.is_ok());
        assert_eq!(pool.workers.len(), 2);
        
        // Stop pool
        assert!(pool.stop().await.is_ok());
        assert_eq!(pool.workers.len(), 0);
    }

    #[tokio::test]
    async fn test_worker_pool_stats() {
        let (mut pool, _) = create_test_setup().await;
        
        pool.start().await.unwrap();
        
        // Give workers a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let stats = pool.get_pool_stats().await.unwrap();
        assert_eq!(stats.total_workers, 2);
        
        pool.stop().await.unwrap();
    }
}