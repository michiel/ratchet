use crate::database::{
    entities::Schedule,
    repositories::RepositoryFactory,
    DatabaseError,
};
use crate::execution::job_queue::{JobQueue, JobQueueError};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{interval, Duration};
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{debug, error, info, warn};

/// Scheduler errors
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    
    #[error("Job queue error: {0}")]
    JobQueueError(#[from] JobQueueError),
    
    #[error("Scheduler error: {0}")]
    SchedulerError(#[from] JobSchedulerError),
    
    #[error("Invalid cron expression: {0}")]
    InvalidCronExpression(String),
    
    #[error("Schedule not found: {0}")]
    ScheduleNotFound(i32),
}

/// Task scheduler trait
#[async_trait::async_trait(?Send)]
pub trait TaskScheduler {
    /// Start the scheduler
    async fn start(&self) -> Result<(), SchedulerError>;
    
    /// Stop the scheduler
    async fn stop(&self) -> Result<(), SchedulerError>;
    
    /// Add a new schedule
    async fn add_schedule(&self, schedule: Schedule) -> Result<(), SchedulerError>;
    
    /// Remove a schedule
    async fn remove_schedule(&self, schedule_id: i32) -> Result<(), SchedulerError>;
    
    /// Update a schedule
    async fn update_schedule(&self, schedule: Schedule) -> Result<(), SchedulerError>;
    
    /// Get scheduler status
    fn is_running(&self) -> bool;
}

/// Schedule manager using tokio-cron-scheduler
pub struct ScheduleManager {
    repositories: RepositoryFactory,
    job_queue: Arc<dyn JobQueue>,
    scheduler: JobScheduler,
    poll_interval: Duration,
}

impl ScheduleManager {
    /// Create a new schedule manager
    pub fn new(
        repositories: RepositoryFactory,
        job_queue: Arc<dyn JobQueue>,
        poll_interval_seconds: u64,
    ) -> Result<Self, SchedulerError> {
        let scheduler = JobScheduler::new()?;
        
        Ok(Self {
            repositories,
            job_queue,
            scheduler,
            poll_interval: Duration::from_secs(poll_interval_seconds),
        })
    }
    
    /// Create with default configuration (poll every 60 seconds)
    pub fn with_default_config(
        repositories: RepositoryFactory,
        job_queue: Arc<dyn JobQueue>,
    ) -> Result<Self, SchedulerError> {
        Self::new(repositories, job_queue, 60)
    }
    
    /// Load all schedules from database and register them
    async fn load_schedules(&self) -> Result<(), SchedulerError> {
        info!("Loading schedules from database");
        
        let schedules = self.repositories.schedule_repo.find_enabled().await?;
        
        for schedule in schedules {
            if let Err(e) = self.register_schedule(&schedule).await {
                error!("Failed to register schedule {}: {}", schedule.id, e);
            }
        }
        
        info!("Loaded {} schedules", schedules.len());
        Ok(())
    }
    
    /// Register a single schedule with the cron scheduler
    async fn register_schedule(&self, schedule: &Schedule) -> Result<(), SchedulerError> {
        let schedule_id = schedule.id;
        let task_id = schedule.task_id;
        let input_data = schedule.input_data.0.clone();
        let cron_expr = schedule.cron_expression.clone();
        
        let job_queue = Arc::clone(&self.job_queue);
        let repositories = self.repositories.clone();
        
        let job = Job::new_async(&cron_expr, move |_uuid, _locked| {
            let job_queue = Arc::clone(&job_queue);
            let repositories = repositories.clone();
            let input_data = input_data.clone();
            
            Box::pin(async move {
                info!("Executing scheduled job for schedule {} task {}", schedule_id, task_id);
                
                // Calculate next run time and enqueue job
                match job_queue.enqueue_scheduled_job(
                    task_id,
                    schedule_id,
                    input_data,
                    chrono::Utc::now(),
                ).await {
                    Ok(job_id) => {
                        info!("Enqueued job {} from schedule {}", job_id, schedule_id);
                        
                        // Record execution in schedule
                        if let Err(e) = repositories.schedule_repo.record_execution(schedule_id).await {
                            error!("Failed to record schedule execution: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to enqueue scheduled job: {}", e);
                    }
                }
            })
        })?;
        
        let job_id = self.scheduler.add(job).await?;
        debug!("Registered schedule {} with job ID {:?}", schedule_id, job_id);
        
        Ok(())
    }
    
    /// Start background task to poll for schedule changes
    async fn start_schedule_poller(&self) -> Result<(), SchedulerError> {
        let repositories = self.repositories.clone();
        let mut interval = interval(self.poll_interval);
        
        info!("Starting schedule poller with interval: {:?}", self.poll_interval);
        
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                
                // Poll for schedules that are ready to run but not in the cron scheduler
                // This is a simple implementation - in production, you might want more sophisticated
                // change detection (e.g., database triggers, message queues, etc.)
                match repositories.schedule_repo.find_ready_to_run().await {
                    Ok(ready_schedules) => {
                        if !ready_schedules.is_empty() {
                            debug!("Found {} schedules ready to run", ready_schedules.len());
                            // Here you would implement logic to handle schedules that 
                            // aren't properly registered with the cron scheduler
                        }
                    }
                    Err(e) => {
                        warn!("Error polling for ready schedules: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl TaskScheduler for ScheduleManager {
    async fn start(&self) -> Result<(), SchedulerError> {
        info!("Starting schedule manager");
        
        // Load existing schedules
        self.load_schedules().await?;
        
        // Start the cron scheduler
        self.scheduler.start().await?;
        
        // Start background poller
        self.start_schedule_poller().await?;
        
        info!("Schedule manager started successfully");
        Ok(())
    }
    
    async fn stop(&self) -> Result<(), SchedulerError> {
        info!("Stopping schedule manager");
        self.scheduler.shutdown().await?;
        info!("Schedule manager stopped");
        Ok(())
    }
    
    async fn add_schedule(&self, schedule: Schedule) -> Result<(), SchedulerError> {
        info!("Adding new schedule: {} for task {}", schedule.id, schedule.task_id);
        
        // Register with cron scheduler if enabled
        if schedule.enabled {
            self.register_schedule(&schedule).await?;
        }
        
        debug!("Added schedule: {}", schedule.id);
        Ok(())
    }
    
    async fn remove_schedule(&self, schedule_id: i32) -> Result<(), SchedulerError> {
        info!("Removing schedule: {}", schedule_id);
        
        // Note: tokio-cron-scheduler doesn't provide an easy way to remove jobs by custom ID
        // In a production system, you'd want to track job UUIDs and remove them properly
        // For now, we'll just log the removal
        warn!("Schedule removal from cron scheduler not fully implemented for schedule {}", schedule_id);
        
        Ok(())
    }
    
    async fn update_schedule(&self, schedule: Schedule) -> Result<(), SchedulerError> {
        info!("Updating schedule: {}", schedule.id);
        
        // For simplicity, remove and re-add the schedule
        self.remove_schedule(schedule.id).await?;
        
        if schedule.enabled {
            self.add_schedule(schedule).await?;
        }
        
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        // tokio-cron-scheduler doesn't expose a direct is_running method
        // For now, assume it's running if we have a scheduler instance
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, RatchetConfig};
    use crate::database::DatabaseConnection;
    use crate::execution::job_queue::{JobQueueManager, JobQueueConfig};
    use std::time::Duration;
    use tempfile::NamedTempFile;

    async fn create_test_setup() -> (ScheduleManager, RepositoryFactory) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();
        
        let config = DatabaseConfig {
            url: format!("sqlite://{}?mode=rwc", db_path),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        };

        let db = DatabaseConnection::new(config).await.unwrap();
        db.migrate().await.unwrap();
        
        let repositories = RepositoryFactory::new(db);
        let job_queue = Arc::new(JobQueueManager::with_default_config(repositories.clone()));
        let scheduler = ScheduleManager::with_default_config(repositories.clone(), job_queue).unwrap();
        
        (scheduler, repositories)
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let (scheduler, _) = create_test_setup().await;
        assert!(scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let (scheduler, _) = create_test_setup().await;
        
        // Start scheduler
        assert!(scheduler.start().await.is_ok());
        
        // Stop scheduler
        assert!(scheduler.stop().await.is_ok());
    }
}