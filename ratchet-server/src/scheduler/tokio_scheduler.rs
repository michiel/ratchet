//! tokio-cron-scheduler implementation of the SchedulerService trait

use async_trait::async_trait;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::RepositoryBridge;
use ratchet_api_types::{ApiId, UnifiedSchedule};
use ratchet_interfaces::{RepositoryFactory, ScheduleStatus, SchedulerError, SchedulerService};

/// Configuration for the tokio-cron-scheduler service
#[derive(Debug, Clone)]
pub struct TokioCronSchedulerConfig {
    /// Maximum number of concurrent job executions
    pub max_concurrent_jobs: usize,
    /// Job timeout in seconds
    pub job_timeout_seconds: u64,
    /// Enable job notifications
    pub enable_notifications: bool,
}

impl Default for TokioCronSchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 100,
            job_timeout_seconds: 3600,
            enable_notifications: false,
        }
    }
}

/// tokio-cron-scheduler implementation of the SchedulerService
pub struct TokioCronSchedulerService {
    scheduler: Arc<Mutex<JobScheduler>>,
    repository_bridge: Arc<RepositoryBridge>,
    config: TokioCronSchedulerConfig,
    is_running: AtomicBool,
}

impl TokioCronSchedulerService {
    /// Create a new tokio-cron-scheduler service
    pub async fn new(
        repositories: Arc<dyn RepositoryFactory>,
        config: TokioCronSchedulerConfig,
    ) -> Result<Self, SchedulerError> {
        info!("Creating new tokio-cron-scheduler service");

        // Create repository bridge
        let repository_bridge = Arc::new(RepositoryBridge::new(repositories));

        // Create JobScheduler with default storage for now
        // TODO: In the future, we can implement a full custom storage adapter
        let scheduler = JobScheduler::new().await.map_err(|e| {
            error!("Failed to create JobScheduler: {}", e);
            SchedulerError::Internal(format!("Failed to create JobScheduler: {}", e))
        })?;

        info!("Successfully created tokio-cron-scheduler service");

        Ok(Self {
            scheduler: Arc::new(Mutex::new(scheduler)),
            repository_bridge,
            config,
            is_running: AtomicBool::new(false),
        })
    }

    /// Create a job execution handler for schedule execution
    fn create_job_execution_handler(&self, schedule_id: ApiId) -> impl Fn(Uuid) + Send + Sync + Clone {
        let bridge = self.repository_bridge.clone();

        move |_job_id: Uuid| {
            let bridge = bridge.clone();
            let schedule_id_for_exec = schedule_id.clone();
            let schedule_id_for_log = schedule_id.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::execute_scheduled_job(bridge, schedule_id_for_exec).await {
                    error!(
                        "Failed to execute scheduled job for schedule {}: {}",
                        schedule_id_for_log, e
                    );
                }
            });
        }
    }

    /// Execute a scheduled job by creating a job in the repository
    async fn execute_scheduled_job(bridge: Arc<RepositoryBridge>, schedule_id: ApiId) -> Result<(), SchedulerError> {
        let execution_time = Utc::now();

        debug!("Executing scheduled job for schedule: {}", schedule_id);

        // Create job through repository pattern
        let created_job = bridge
            .create_job_for_schedule(schedule_id.clone(), execution_time)
            .await?;

        // Update schedule execution metadata
        // Note: We don't have next_run info here, tokio-cron-scheduler handles that internally
        bridge
            .update_schedule_execution(schedule_id.clone(), execution_time, None)
            .await?;

        info!(
            "Successfully executed scheduled job for schedule {}, created_job_id={}",
            schedule_id, created_job.id
        );

        Ok(())
    }

    /// Load existing schedules from the repository and add them to the scheduler
    async fn load_existing_schedules(&self) -> Result<(), SchedulerError> {
        info!("Loading existing schedules from repository");

        let schedules = self.repository_bridge.load_all_schedules().await?;
        info!("Found {} existing schedules to load", schedules.len());

        for schedule in schedules {
            if !schedule.enabled {
                debug!("Skipping disabled schedule: {}", schedule.name);
                continue;
            }

            debug!(
                "Adding schedule to tokio-cron-scheduler: {} ({})",
                schedule.name, schedule.cron_expression
            );

            // Create job with our execution handler
            let schedule_id = schedule.id.clone();
            let cron_expression = schedule.cron_expression.clone();
            let execution_handler = self.create_job_execution_handler(schedule_id);

            let job = Job::new_async(cron_expression.as_str(), move |uuid, _| {
                execution_handler(uuid);
                Box::pin(async {})
            })
            .map_err(|e| {
                error!("Failed to create job for schedule {}: {}", schedule.name, e);
                SchedulerError::InvalidCron(format!("Invalid cron expression '{}': {}", cron_expression, e))
            })?;

            // Add job to scheduler
            let scheduler = self.scheduler.lock().await;
            scheduler.add(job).await.map_err(|e| {
                error!("Failed to add job to scheduler: {}", e);
                SchedulerError::Internal(format!("Failed to add job to scheduler: {}", e))
            })?;

            info!("Successfully added schedule to scheduler: {}", schedule.name);
        }

        Ok(())
    }
}

#[async_trait]
impl SchedulerService for TokioCronSchedulerService {
    /// Start the scheduler service
    async fn start(&self) -> Result<(), SchedulerError> {
        if self.is_running.load(Ordering::Relaxed) {
            warn!("Scheduler is already running");
            return Ok(());
        }

        info!("Starting tokio-cron-scheduler service");

        // Start with a completely fresh scheduler instance to avoid any stale state
        // This approach avoids the "Error receiving Closed" messages that occur
        // when shutting down an existing scheduler
        {
            let mut scheduler_guard = self.scheduler.lock().await;

            // Check if the current scheduler is initialized
            if scheduler_guard.inited().await {
                info!("Replacing existing scheduler with fresh instance to avoid stale state");

                // Create a completely new scheduler instance
                let fresh_scheduler = JobScheduler::new().await.map_err(|e| {
                    error!("Failed to create fresh JobScheduler: {}", e);
                    SchedulerError::Internal(format!("Failed to create fresh JobScheduler: {}", e))
                })?;

                // Replace the old scheduler with the new one
                // The old scheduler will be dropped, cleaning up its resources naturally
                *scheduler_guard = fresh_scheduler;
                info!("Successfully replaced scheduler with fresh instance");
            } else {
                info!("Scheduler not yet initialized, using existing instance");
            }
        }

        // Load existing schedules from repository
        self.load_existing_schedules().await?;

        // Start the scheduler
        {
            let scheduler = self.scheduler.lock().await;
            scheduler.start().await.map_err(|e| {
                error!("Failed to start scheduler: {}", e);
                SchedulerError::Internal(format!("Failed to start scheduler: {}", e))
            })?;
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!("tokio-cron-scheduler service started successfully");

        Ok(())
    }

    /// Stop the scheduler service
    async fn stop(&self) -> Result<(), SchedulerError> {
        if !self.is_running.load(Ordering::Relaxed) {
            warn!("Scheduler is not running");
            return Ok(());
        }

        info!("Stopping tokio-cron-scheduler service");

        let mut scheduler = self.scheduler.lock().await;
        scheduler.shutdown().await.map_err(|e| {
            error!("Failed to stop scheduler: {}", e);
            SchedulerError::Internal(format!("Failed to stop scheduler: {}", e))
        })?;

        self.is_running.store(false, Ordering::Relaxed);
        info!("tokio-cron-scheduler service stopped successfully");

        Ok(())
    }

    /// Add a new schedule to the scheduler
    async fn add_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError> {
        info!(
            "Adding new schedule to scheduler: {} ({})",
            schedule.name, schedule.cron_expression
        );

        if !schedule.enabled {
            debug!(
                "Schedule is disabled, not adding to active scheduler: {}",
                schedule.name
            );
            return Ok(());
        }

        // Create job with our execution handler
        let schedule_id = schedule.id.clone();
        let cron_expression = schedule.cron_expression.clone();
        let execution_handler = self.create_job_execution_handler(schedule_id);

        let job = Job::new_async(cron_expression.as_str(), move |uuid, _| {
            execution_handler(uuid);
            Box::pin(async {})
        })
        .map_err(|e| {
            error!("Failed to create job for schedule {}: {}", schedule.name, e);
            SchedulerError::InvalidCron(format!("Invalid cron expression '{}': {}", cron_expression, e))
        })?;

        // Add job to scheduler
        let scheduler = self.scheduler.lock().await;
        scheduler.add(job).await.map_err(|e| {
            error!("Failed to add job to scheduler: {}", e);
            SchedulerError::Internal(format!("Failed to add job to scheduler: {}", e))
        })?;

        info!("Successfully added schedule to scheduler: {}", schedule.name);
        Ok(())
    }

    /// Remove a schedule from the scheduler
    async fn remove_schedule(&self, schedule_id: ApiId) -> Result<(), SchedulerError> {
        info!("Removing schedule from scheduler: {}", schedule_id);

        let job_uuid = schedule_id
            .as_uuid()
            .ok_or_else(|| SchedulerError::Internal(format!("Cannot convert schedule_id to UUID: {}", schedule_id)))?;

        let scheduler = self.scheduler.lock().await;
        scheduler.remove(&job_uuid).await.map_err(|e| {
            error!("Failed to remove job from scheduler: {}", e);
            SchedulerError::Internal(format!("Failed to remove job from scheduler: {}", e))
        })?;

        info!("Successfully removed schedule from scheduler: {}", schedule_id);
        Ok(())
    }

    /// Update an existing schedule
    async fn update_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError> {
        info!("Updating schedule in scheduler: {}", schedule.name);

        // For simplicity, we'll remove the old schedule and add the new one
        // This ensures the cron expression and other changes take effect
        let schedule_id = schedule.id.clone();
        self.remove_schedule(schedule_id).await?;
        self.add_schedule(schedule).await?;

        Ok(())
    }

    /// Get the status of a specific schedule
    async fn get_schedule_status(&self, schedule_id: ApiId) -> Result<ScheduleStatus, SchedulerError> {
        debug!("Getting schedule status: {}", schedule_id);

        let schedule_id_clone = schedule_id.clone();
        // Get schedule from repository
        let schedule = self
            .repository_bridge
            .find_schedule(schedule_id)
            .await?
            .ok_or(SchedulerError::ScheduleNotFound(schedule_id_clone))?;

        // TODO: Get more detailed status from tokio-cron-scheduler if available
        // For now, we'll return basic status from the repository
        let final_schedule_id = schedule.id.clone();
        Ok(ScheduleStatus {
            id: final_schedule_id,
            enabled: schedule.enabled,
            last_run: schedule.last_run,
            next_run: schedule.next_run,
            is_running: self.is_running.load(Ordering::Relaxed) && schedule.enabled,
            run_count: 0, // TODO: Track this separately if needed
        })
    }

    /// Check if the scheduler is running
    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Get the number of active schedules
    async fn schedule_count(&self) -> Result<usize, SchedulerError> {
        // Get count from repository since tokio-cron-scheduler doesn't expose this directly
        let schedules = self.repository_bridge.load_all_schedules().await?;
        Ok(schedules.len())
    }
}
