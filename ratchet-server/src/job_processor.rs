//! Job processor service for processing queued jobs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use async_trait::async_trait;
use tokio::time::sleep;
use tracing::{info, debug, error, warn};

use ratchet_interfaces::{RepositoryFactory, DatabaseError};
use ratchet_api_types::{ExecutionStatus, UnifiedExecution, ApiId};

/// Configuration for the job processor service
#[derive(Debug, Clone)]
pub struct JobProcessorConfig {
    /// Poll interval for checking new jobs (in seconds)
    pub poll_interval_seconds: u64,
    /// Maximum number of jobs to process per batch
    pub batch_size: u64,
    /// Enable automatic job processing
    pub enabled: bool,
}

impl Default for JobProcessorConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 5,
            batch_size: 10,
            enabled: true,
        }
    }
}

/// Job processor service that polls for queued jobs and creates executions
pub struct JobProcessorService {
    repositories: Arc<dyn RepositoryFactory>,
    config: JobProcessorConfig,
    is_running: AtomicBool,
}

impl JobProcessorService {
    /// Create a new job processor service
    pub fn new(
        repositories: Arc<dyn RepositoryFactory>,
        config: JobProcessorConfig,
    ) -> Self {
        Self {
            repositories,
            config,
            is_running: AtomicBool::new(false),
        }
    }

    /// Start the job processor service
    pub async fn start(&self) -> Result<(), DatabaseError> {
        if !self.config.enabled {
            info!("Job processor service is disabled");
            return Ok(());
        }

        if self.is_running.load(Ordering::Relaxed) {
            warn!("Job processor service is already running");
            return Ok(());
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!("Starting job processor service with {} second poll interval", 
              self.config.poll_interval_seconds);

        // Main processing loop
        while self.is_running.load(Ordering::Relaxed) {
            if let Err(e) = self.process_batch().await {
                error!("Error processing job batch: {}", e);
            }

            // Sleep between polls
            sleep(Duration::from_secs(self.config.poll_interval_seconds)).await;
        }

        info!("Job processor service stopped");
        Ok(())
    }

    /// Stop the job processor service
    pub async fn stop(&self) {
        info!("Stopping job processor service");
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Process a batch of ready jobs
    async fn process_batch(&self) -> Result<(), DatabaseError> {
        debug!("Checking for ready jobs to process");

        // Get ready jobs from the repository
        let jobs = self.repositories.job_repository()
            .find_ready_for_processing(self.config.batch_size)
            .await?;

        if jobs.is_empty() {
            debug!("No jobs ready for processing");
            return Ok(());
        }

        info!("Found {} jobs ready for processing", jobs.len());

        // Process each job
        for job in jobs {
            let job_id_copy = job.id.clone();
            if let Err(e) = self.process_job(&job.id).await {
                error!("Failed to process job {}: {}", job_id_copy, e);
                
                // Mark job as failed
                if let Err(mark_err) = self.repositories.job_repository()
                    .mark_failed(job.id, e.to_string(), None).await {
                    error!("Failed to mark job {} as failed: {}", job_id_copy, mark_err);
                }
            }
        }

        Ok(())
    }

    /// Process a single job by creating an execution and marking it as completed
    /// For now, this is a simplified implementation that doesn't actually execute tasks
    async fn process_job(&self, job_id: &ApiId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing job {}", job_id);

        // Get the job details
        let job = self.repositories.job_repository()
            .find_by_id(job_id.as_i32().ok_or("Invalid job ID")?)
            .await?
            .ok_or("Job not found")?;

        // Create an execution for this job
        let execution = UnifiedExecution {
            id: ApiId::from_uuid(uuid::Uuid::new_v4()),
            uuid: uuid::Uuid::new_v4(),
            task_id: job.task_id.clone(),
            status: ExecutionStatus::Pending,
            input: serde_json::json!({}), // TODO: Get input from job metadata
            output: None,
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            http_requests: None,
            recording_path: None,
            progress: None,
            can_retry: false,
            can_cancel: false,
        };

        // Create the execution in the repository
        let created_execution = self.repositories.execution_repository()
            .create(execution)
            .await?;

        // Store IDs before they get moved
        let execution_id = created_execution.id.clone();
        let job_id_for_processing = job.id.clone();
        
        // Mark job as processing and link to execution
        self.repositories.job_repository()
            .mark_processing(job_id_for_processing, execution_id.clone())
            .await?;

        info!("Created execution {} for job {}", execution_id, job_id);

        // For now, we'll simulate task execution with a simple success
        // In a full implementation, this would delegate to a task executor
        // TODO: Integrate with actual task execution system
        
        // Mark execution as started
        self.repositories.execution_repository()
            .mark_started(execution_id.clone())
            .await
            .map_err(|e| error!("Failed to mark execution {} as started: {}", execution_id, e))
            .ok();

        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(100)).await;

        // For heartbeat tasks, create a simple success response
        let output = if job.task_id.to_string().contains("heartbeat") {
            serde_json::json!({
                "status": "success",
                "message": "Heartbeat completed successfully",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "system_healthy": true
            })
        } else {
            serde_json::json!({
                "status": "success", 
                "message": "Job completed successfully",
                "job_id": job_id.to_string()
            })
        };

        // Mark execution as completed
        if let Err(e) = self.repositories.execution_repository()
            .mark_completed(
                execution_id.clone(), 
                output, 
                Some(100) // 100ms duration
            ).await {
            error!("Failed to mark execution {} as completed: {}", execution_id, e);
        }

        // Mark job as completed
        if let Err(e) = self.repositories.job_repository()
            .mark_completed(job.id.clone()).await {
            error!("Failed to mark job {} as completed: {}", job_id, e);
        }

        info!("Successfully processed job {} with execution {}", job_id, execution_id);
        Ok(())
    }
}

/// Job processor service trait for dependency injection
#[async_trait]
pub trait JobProcessor: Send + Sync {
    /// Start the job processor
    async fn start(&self) -> Result<(), DatabaseError>;
    
    /// Stop the job processor
    async fn stop(&self);
    
    /// Check if running
    fn is_running(&self) -> bool;
}

#[async_trait]
impl JobProcessor for JobProcessorService {
    async fn start(&self) -> Result<(), DatabaseError> {
        JobProcessorService::start(self).await
    }

    async fn stop(&self) {
        JobProcessorService::stop(self).await
    }

    fn is_running(&self) -> bool {
        JobProcessorService::is_running(self)
    }
}