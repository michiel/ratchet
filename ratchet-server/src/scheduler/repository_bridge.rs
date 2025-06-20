//! Repository bridge for scheduler to repository layer communication

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{debug, info};

use ratchet_api_types::{ApiId, JobPriority, JobStatus, UnifiedJob, UnifiedSchedule};
use ratchet_interfaces::RepositoryFactory;
use ratchet_interfaces::SchedulerError;

/// Bridge between scheduler and repository layer
/// This ensures the scheduler only accesses data through repository interfaces
pub struct RepositoryBridge {
    repositories: Arc<dyn RepositoryFactory>,
}

impl RepositoryBridge {
    /// Create a new repository bridge
    pub fn new(repositories: Arc<dyn RepositoryFactory>) -> Self {
        Self { repositories }
    }

    /// Load all enabled schedules from the repository
    pub async fn load_all_schedules(&self) -> Result<Vec<UnifiedSchedule>, SchedulerError> {
        debug!("Loading all schedules from repository");

        let schedules = self
            .repositories
            .schedule_repository()
            .find_enabled()
            .await
            .map_err(|e| SchedulerError::Repository(e.to_string()))?;

        info!("Loaded {} schedules from repository", schedules.len());
        Ok(schedules)
    }

    /// Create a job for a scheduled execution
    pub async fn create_job_for_schedule(
        &self,
        schedule_id: ApiId,
        execution_time: DateTime<Utc>,
    ) -> Result<UnifiedJob, SchedulerError> {
        debug!("Creating job for schedule {}", schedule_id);

        // First, get the schedule to determine the task
        let schedule = self
            .repositories
            .schedule_repository()
            .find_by_id(schedule_id.as_i32().unwrap_or(0))
            .await
            .map_err(|e| SchedulerError::Repository(e.to_string()))?
            .ok_or(SchedulerError::ScheduleNotFound(schedule_id))?;

        // Extract values before moving schedule
        let task_id = schedule.task_id.clone();
        let schedule_name = schedule.name.clone();

        // Create a job for this scheduled execution
        let job = UnifiedJob {
            id: ApiId::from_uuid(uuid::Uuid::new_v4()),
            task_id,
            priority: JobPriority::Normal,
            status: JobStatus::Queued,
            retry_count: 0,
            max_retries: 3,
            queued_at: execution_time,
            scheduled_for: Some(execution_time),
            error_message: None,
            output_destinations: schedule.output_destinations.clone(),
        };

        // Store the job through the repository
        let created_job = self
            .repositories
            .job_repository()
            .create(job)
            .await
            .map_err(|e| SchedulerError::Repository(format!("Failed to create job: {}", e)))?;

        info!(
            "Created job {} for schedule {} (task {})",
            created_job.id, schedule_name, created_job.task_id
        );

        Ok(created_job)
    }

    /// Update schedule execution metadata
    pub async fn update_schedule_execution(
        &self,
        schedule_id: ApiId,
        last_run: DateTime<Utc>,
        next_run: Option<DateTime<Utc>>,
    ) -> Result<(), SchedulerError> {
        let schedule_id_clone = schedule_id.clone();
        debug!("Updating schedule {} execution metadata", schedule_id);

        // Get the current schedule
        let mut schedule = self
            .repositories
            .schedule_repository()
            .find_by_id(schedule_id.as_i32().unwrap_or(0))
            .await
            .map_err(|e| SchedulerError::Repository(e.to_string()))?
            .ok_or(SchedulerError::ScheduleNotFound(schedule_id))?;

        // Update execution metadata
        schedule.last_run = Some(last_run);
        schedule.next_run = next_run;
        schedule.updated_at = Utc::now();

        // Save the updated schedule
        self.repositories
            .schedule_repository()
            .update(schedule)
            .await
            .map_err(|e| SchedulerError::Repository(format!("Failed to update schedule: {}", e)))?;

        debug!("Updated schedule {} execution metadata", schedule_id_clone);
        Ok(())
    }

    /// Find a schedule by ID
    pub async fn find_schedule(&self, schedule_id: ApiId) -> Result<Option<UnifiedSchedule>, SchedulerError> {
        self.repositories
            .schedule_repository()
            .find_by_id(schedule_id.as_i32().unwrap_or(0))
            .await
            .map_err(|e| SchedulerError::Repository(e.to_string()))
    }

    /// Create a new schedule
    pub async fn create_schedule(&self, schedule: UnifiedSchedule) -> Result<UnifiedSchedule, SchedulerError> {
        self.repositories
            .schedule_repository()
            .create(schedule)
            .await
            .map_err(|e| SchedulerError::Repository(format!("Failed to create schedule: {}", e)))
    }

    /// Update an existing schedule
    pub async fn update_schedule(&self, schedule: UnifiedSchedule) -> Result<UnifiedSchedule, SchedulerError> {
        self.repositories
            .schedule_repository()
            .update(schedule)
            .await
            .map_err(|e| SchedulerError::Repository(format!("Failed to update schedule: {}", e)))
    }

    /// Delete a schedule
    pub async fn delete_schedule(&self, schedule_id: ApiId) -> Result<(), SchedulerError> {
        self.repositories
            .schedule_repository()
            .delete(schedule_id.as_i32().unwrap_or(0))
            .await
            .map_err(|e| SchedulerError::Repository(format!("Failed to delete schedule: {}", e)))
    }
}
