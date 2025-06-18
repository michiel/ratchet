//! Background scheduler service for executing scheduled tasks

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn, error, debug};
use chrono::{DateTime, Utc};

use ratchet_interfaces::{RepositoryFactory, TaskRegistry};
use ratchet_api_types::{ApiId, UnifiedJob, JobStatus, JobPriority};
use ratchet_storage::seaorm::repositories::RepositoryFactory as SeaOrmRepositoryFactory;
use ratchet_output::OutputDeliveryManager;

/// Configuration for the scheduler service
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Interval between schedule polls (default: 30 seconds)
    pub poll_interval: Duration,
    /// Maximum number of concurrent task executions (default: 10)
    pub max_concurrent_executions: usize,
    /// Whether the scheduler is enabled (default: true)
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(30),
            max_concurrent_executions: 10,
            enabled: true,
        }
    }
}

/// Background scheduler service that executes scheduled tasks
pub struct SchedulerService {
    config: SchedulerConfig,
    repositories: Arc<dyn RepositoryFactory>,
    seaorm_factory: Arc<SeaOrmRepositoryFactory>,
    registry: Arc<dyn TaskRegistry>,
    output_manager: Arc<OutputDeliveryManager>,
}

impl SchedulerService {
    /// Create a new scheduler service
    pub fn new(
        config: SchedulerConfig,
        repositories: Arc<dyn RepositoryFactory>,
        seaorm_factory: Arc<SeaOrmRepositoryFactory>,
        registry: Arc<dyn TaskRegistry>,
        output_manager: Arc<OutputDeliveryManager>,
    ) -> Self {
        Self {
            config,
            repositories,
            seaorm_factory,
            registry,
            output_manager,
        }
    }

    /// Start the scheduler background task
    pub async fn start(self: Arc<Self>) -> Result<()> {
        if !self.config.enabled {
            info!("Scheduler service is disabled");
            return Ok(());
        }

        info!(
            "Starting scheduler service with poll interval: {:?}",
            self.config.poll_interval
        );

        let mut interval_timer = interval(self.config.poll_interval);
        interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.poll_and_execute_schedules().await {
                error!("Error polling schedules: {}", e);
            }
        }
    }

    /// Poll for ready schedules and execute them
    async fn poll_and_execute_schedules(&self) -> Result<()> {
        debug!("Polling for ready schedules");

        let schedule_repo = self.seaorm_factory.schedule_repository();
        
        // Find schedules ready to run
        let ready_schedules = match schedule_repo.find_ready_to_run().await {
            Ok(schedules) => schedules,
            Err(e) => {
                error!("Failed to query ready schedules: {}", e);
                return Ok(());
            }
        };

        if ready_schedules.is_empty() {
            debug!("No schedules ready to run");
            return Ok(());
        }

        info!("Found {} schedules ready to run", ready_schedules.len());

        for schedule in ready_schedules {
            let schedule_id = schedule.id;
            let schedule_name = schedule.name.clone();
            if let Err(e) = self.execute_schedule(schedule, &schedule_repo).await {
                error!("Failed to execute schedule {} ({}): {}", schedule_id, schedule_name, e);
            }
        }

        Ok(())
    }

    /// Execute a specific schedule
    async fn execute_schedule(
        &self, 
        schedule: ratchet_storage::seaorm::entities::Schedule,
        schedule_repo: &ratchet_storage::seaorm::repositories::schedule_repository::ScheduleRepository
    ) -> Result<()> {
        info!(
            "Executing schedule: id={}, name={}, task_id={}",
            schedule.id, schedule.name, schedule.task_id
        );

        // Calculate next run time
        let next_run = self.calculate_next_run(&schedule.cron_expression)?;
        
        if let Err(e) = schedule_repo.record_execution(schedule.id).await {
            warn!("Failed to record schedule execution: {}", e);
        }

        if let Err(e) = schedule_repo.update_next_run(schedule.id, next_run).await {
            warn!("Failed to update schedule next run time: {}", e);
        }

        // Create a simplified job for the task execution  
        let job = UnifiedJob {
            id: ApiId::from_uuid(uuid::Uuid::new_v4()), // Generate new job ID
            task_id: ApiId::from_i32(schedule.task_id),
            priority: JobPriority::Normal,
            status: JobStatus::Queued,
            retry_count: 0,
            max_retries: 3,
            queued_at: Utc::now(),
            scheduled_for: Some(Utc::now()),
            error_message: None,
            output_destinations: None,
        };

        // Store the job
        match self.repositories.job_repository().create(job).await {
            Ok(created_job) => {
                info!("Created job {} for schedule {}", created_job.id, schedule.name);
            },
            Err(e) => {
                error!("Failed to create job for schedule {}: {}", schedule.name, e);
            }
        }

        Ok(())
    }


    /// Calculate the next run time for a cron expression
    fn calculate_next_run(&self, cron_expression: &str) -> Result<Option<DateTime<Utc>>> {
        use cron::Schedule;
        use std::str::FromStr;

        let schedule = Schedule::from_str(cron_expression)
            .map_err(|e| anyhow::anyhow!("Invalid cron expression '{}': {}", cron_expression, e))?;

        let next = schedule.upcoming(Utc).next();
        Ok(next)
    }
}