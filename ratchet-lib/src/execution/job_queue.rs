use crate::database::{
    entities::{Job, JobPriority, JobStatus},
    repositories::RepositoryFactory,
    DatabaseError,
};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Job queue errors
#[derive(Error, Debug)]
pub enum JobQueueError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("Job not found: {0}")]
    JobNotFound(i32),

    #[error("Invalid job state: {0}")]
    InvalidState(String),

    #[error("Queue is full")]
    QueueFull,
}

/// Job queue interface
#[async_trait(?Send)]
pub trait JobQueue {
    /// Add a job to the queue
    async fn enqueue_job(
        &self,
        task_id: i32,
        input_data: JsonValue,
        priority: JobPriority,
    ) -> Result<i32, JobQueueError>;

    /// Add a scheduled job to the queue
    async fn enqueue_scheduled_job(
        &self,
        task_id: i32,
        schedule_id: i32,
        input_data: JsonValue,
        process_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<i32, JobQueueError>;

    /// Get next jobs ready for processing
    async fn dequeue_jobs(&self, limit: u32) -> Result<Vec<Job>, JobQueueError>;

    /// Get queue statistics
    async fn get_stats(&self) -> Result<JobQueueStats, JobQueueError>;

    /// Cancel a job
    async fn cancel_job(&self, job_id: i32) -> Result<(), JobQueueError>;

    /// Retry a failed job
    async fn retry_job(&self, job_id: i32) -> Result<(), JobQueueError>;
}

/// Job queue statistics
#[derive(Debug, Clone)]
pub struct JobQueueStats {
    pub total_jobs: u64,
    pub queued_jobs: u64,
    pub processing_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub retrying_jobs: u64,
}

/// Database-backed job queue manager
#[derive(Clone)]
pub struct JobQueueManager {
    repositories: RepositoryFactory,
    config: JobQueueConfig,
}

/// Job queue configuration
#[derive(Debug, Clone)]
pub struct JobQueueConfig {
    /// Maximum number of jobs to dequeue at once
    pub max_dequeue_batch_size: u32,

    /// Maximum number of jobs allowed in queue (0 = unlimited)
    pub max_queue_size: u64,

    /// Default retry delay in seconds
    pub default_retry_delay: i32,

    /// Default maximum retries
    pub default_max_retries: i32,
}

impl Default for JobQueueConfig {
    fn default() -> Self {
        Self {
            max_dequeue_batch_size: 10,
            max_queue_size: 10000,
            default_retry_delay: 60,
            default_max_retries: 3,
        }
    }
}

impl JobQueueManager {
    /// Create a new job queue manager
    pub fn new(repositories: RepositoryFactory, config: JobQueueConfig) -> Self {
        Self {
            repositories,
            config,
        }
    }

    /// Create with default configuration
    pub fn with_default_config(repositories: RepositoryFactory) -> Self {
        Self::new(repositories, JobQueueConfig::default())
    }

    /// Check if queue is full
    async fn is_queue_full(&self) -> Result<bool, JobQueueError> {
        if self.config.max_queue_size == 0 {
            return Ok(false); // Unlimited
        }

        let stats = self.repositories.job_repo.get_queue_stats().await?;
        Ok(stats.total >= self.config.max_queue_size)
    }

    /// Send-compatible enqueue job method for GraphQL resolvers
    pub async fn enqueue_job_send(
        &self,
        task_id: i32,
        input_data: JsonValue,
        priority: JobPriority,
    ) -> Result<i32, JobQueueError> {
        // Direct implementation to avoid ?Send trait issues
        // Check queue capacity
        if self.config.max_queue_size > 0 {
            let stats = self.repositories.job_repo.get_queue_stats().await?;
            if stats.total >= self.config.max_queue_size {
                return Err(JobQueueError::QueueFull);
            }
        }

        // Create job
        let mut job = Job::new(task_id, input_data, priority);
        job.max_retries = self.config.default_max_retries;
        job.retry_delay_seconds = self.config.default_retry_delay;

        let created_job = self.repositories.job_repo.create(job).await?;

        info!(
            "Enqueued job {} for task {} with priority {:?}",
            created_job.id, task_id, priority
        );

        Ok(created_job.id)
    }
}

#[async_trait(?Send)]
impl JobQueue for JobQueueManager {
    async fn enqueue_job(
        &self,
        task_id: i32,
        input_data: JsonValue,
        priority: JobPriority,
    ) -> Result<i32, JobQueueError> {
        // Check queue capacity
        if self.is_queue_full().await? {
            return Err(JobQueueError::QueueFull);
        }

        // Create job
        let mut job = Job::new(task_id, input_data, priority);
        job.max_retries = self.config.default_max_retries;
        job.retry_delay_seconds = self.config.default_retry_delay;

        let created_job = self.repositories.job_repo.create(job).await?;

        info!(
            "Enqueued job {} for task {} with priority {:?}",
            created_job.id, task_id, priority
        );

        Ok(created_job.id)
    }

    async fn enqueue_scheduled_job(
        &self,
        task_id: i32,
        schedule_id: i32,
        input_data: JsonValue,
        process_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<i32, JobQueueError> {
        // Check queue capacity
        if self.is_queue_full().await? {
            return Err(JobQueueError::QueueFull);
        }

        // Create scheduled job
        let mut job = Job::new_scheduled(task_id, schedule_id, input_data, process_at);
        job.max_retries = self.config.default_max_retries;
        job.retry_delay_seconds = self.config.default_retry_delay;

        let created_job = self.repositories.job_repo.create(job).await?;

        info!(
            "Enqueued scheduled job {} for task {} from schedule {} to process at {}",
            created_job.id, task_id, schedule_id, process_at
        );

        Ok(created_job.id)
    }

    async fn dequeue_jobs(&self, limit: u32) -> Result<Vec<Job>, JobQueueError> {
        let batch_limit = std::cmp::min(limit, self.config.max_dequeue_batch_size) as u64;

        let jobs = self
            .repositories
            .job_repo
            .find_ready_for_processing(batch_limit)
            .await?;

        debug!("Dequeued {} jobs ready for processing", jobs.len());

        Ok(jobs)
    }

    async fn get_stats(&self) -> Result<JobQueueStats, JobQueueError> {
        let db_stats = self.repositories.job_repo.get_queue_stats().await?;

        Ok(JobQueueStats {
            total_jobs: db_stats.total,
            queued_jobs: db_stats.queued,
            processing_jobs: db_stats.processing,
            completed_jobs: db_stats.completed,
            failed_jobs: db_stats.failed,
            retrying_jobs: db_stats.retrying,
        })
    }

    async fn cancel_job(&self, job_id: i32) -> Result<(), JobQueueError> {
        let job = self.repositories.job_repo.find_by_id(job_id).await?;
        let job = job.ok_or_else(|| JobQueueError::JobNotFound(job_id))?;

        // Can only cancel queued or retrying jobs
        match job.status {
            JobStatus::Queued | JobStatus::Retrying => {
                self.repositories
                    .job_repo
                    .update_status(job_id, JobStatus::Cancelled)
                    .await?;

                info!("Cancelled job {}", job_id);
                Ok(())
            }
            _ => Err(JobQueueError::InvalidState(format!(
                "Cannot cancel job {} with status {:?}",
                job_id, job.status
            ))),
        }
    }

    async fn retry_job(&self, job_id: i32) -> Result<(), JobQueueError> {
        let job = self.repositories.job_repo.find_by_id(job_id).await?;
        let job = job.ok_or_else(|| JobQueueError::JobNotFound(job_id))?;

        // Can only retry failed jobs
        match job.status {
            JobStatus::Failed => {
                // Reset job for retry
                let mut updated_job = job;
                updated_job.status = JobStatus::Queued;
                updated_job.retry_count = 0;
                updated_job.error_message = None;
                updated_job.error_details = None;
                updated_job.process_at = None;
                updated_job.started_at = None;
                updated_job.completed_at = None;

                self.repositories.job_repo.update(updated_job).await?;

                info!("Reset job {} for retry", job_id);
                Ok(())
            }
            _ => Err(JobQueueError::InvalidState(format!(
                "Cannot retry job {} with status {:?}",
                job_id, job.status
            ))),
        }
    }
}

/// Job queue manager with in-memory caching for high-performance scenarios
pub struct CachedJobQueueManager {
    inner: JobQueueManager,
    cache: Arc<RwLock<Vec<Job>>>, // Simple in-memory cache for ready jobs
}

impl CachedJobQueueManager {
    /// Create a new cached job queue manager
    pub fn new(repositories: RepositoryFactory, config: JobQueueConfig) -> Self {
        Self {
            inner: JobQueueManager::new(repositories, config),
            cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Refresh the cache with ready jobs
    pub async fn refresh_cache(&self) -> Result<usize, JobQueueError> {
        let jobs = self.inner.dequeue_jobs(50).await?; // Fetch up to 50 jobs
        let count = jobs.len();

        let mut cache = self.cache.write().await;
        *cache = jobs;

        debug!("Refreshed job cache with {} jobs", count);
        Ok(count)
    }

    /// Get jobs from cache, refresh if empty
    pub async fn get_cached_jobs(&self, limit: u32) -> Result<Vec<Job>, JobQueueError> {
        {
            let cache = self.cache.read().await;
            if !cache.is_empty() {
                let take_count = std::cmp::min(limit as usize, cache.len());
                return Ok(cache.iter().take(take_count).cloned().collect());
            }
        }

        // Cache is empty, refresh it
        self.refresh_cache().await?;

        let cache = self.cache.read().await;
        let take_count = std::cmp::min(limit as usize, cache.len());
        Ok(cache.iter().take(take_count).cloned().collect())
    }
}

#[async_trait(?Send)]
impl JobQueue for CachedJobQueueManager {
    async fn enqueue_job(
        &self,
        task_id: i32,
        input_data: JsonValue,
        priority: JobPriority,
    ) -> Result<i32, JobQueueError> {
        let result = self
            .inner
            .enqueue_job(task_id, input_data, priority)
            .await?;

        // Optionally refresh cache when new jobs are added
        if priority == JobPriority::Urgent || priority == JobPriority::High {
            let _ = self.refresh_cache().await; // Don't fail on cache refresh errors
        }

        Ok(result)
    }

    async fn enqueue_scheduled_job(
        &self,
        task_id: i32,
        schedule_id: i32,
        input_data: JsonValue,
        process_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<i32, JobQueueError> {
        self.inner
            .enqueue_scheduled_job(task_id, schedule_id, input_data, process_at)
            .await
    }

    async fn dequeue_jobs(&self, limit: u32) -> Result<Vec<Job>, JobQueueError> {
        self.get_cached_jobs(limit).await
    }

    async fn get_stats(&self) -> Result<JobQueueStats, JobQueueError> {
        self.inner.get_stats().await
    }

    async fn cancel_job(&self, job_id: i32) -> Result<(), JobQueueError> {
        let result = self.inner.cancel_job(job_id).await?;
        let _ = self.refresh_cache().await; // Refresh cache after cancellation
        Ok(result)
    }

    async fn retry_job(&self, job_id: i32) -> Result<(), JobQueueError> {
        let result = self.inner.retry_job(job_id).await?;
        let _ = self.refresh_cache().await; // Refresh cache after retry
        Ok(result)
    }
}
