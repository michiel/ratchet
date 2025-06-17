//! SQLite metadata storage adapter for tokio-cron-scheduler

use std::sync::Arc;
use async_trait::async_trait;
use tokio_cron_scheduler::{JobStoredData, MetaDataStorage, JobSchedulerError};
use uuid::Uuid;
use tracing::{debug, error, warn};
use chrono::{DateTime, Utc};

use ratchet_api_types::{UnifiedSchedule, ApiId};
use super::{RepositoryBridge, SchedulerError};

/// SQLite metadata storage implementation for tokio-cron-scheduler
/// This bridges tokio-cron-scheduler's storage interface with our repository layer
pub struct SqliteMetadataStore {
    repository_bridge: Arc<RepositoryBridge>,
}

impl SqliteMetadataStore {
    /// Create a new SQLite metadata store
    pub fn new(repository_bridge: Arc<RepositoryBridge>) -> Self {
        Self { repository_bridge }
    }

    /// Convert a UnifiedSchedule to JobStoredData for tokio-cron-scheduler
    fn convert_schedule_to_job_data(&self, schedule: UnifiedSchedule) -> JobStoredData {
        JobStoredData {
            id: schedule.id.as_uuid().unwrap_or_else(|| Uuid::new_v4()),
            schedule: schedule.cron_expression.clone(),
            timezone: None, // TODO: Add timezone support in future
            count: 0, // Will be tracked separately
            extra: serde_json::json!({
                "name": schedule.name,
                "description": schedule.description,
                "task_id": schedule.task_id,
                "enabled": schedule.enabled,
                "last_run": schedule.last_run,
                "next_run": schedule.next_run,
                "created_at": schedule.created_at,
                "updated_at": schedule.updated_at,
            }),
        }
    }

    /// Convert JobStoredData to UnifiedSchedule for our repository layer
    fn convert_job_data_to_schedule(&self, job_data: JobStoredData) -> Result<UnifiedSchedule, SchedulerError> {
        let extra = job_data.extra;
        
        Ok(UnifiedSchedule {
            id: ApiId::from_uuid(job_data.id),
            task_id: extra.get("task_id")
                .and_then(|v| v.as_str())
                .map(|s| ApiId::from_string(s))
                .ok_or_else(|| SchedulerError::Internal("Missing task_id in job data".to_string()))?,
            name: extra.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            description: extra.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            cron_expression: job_data.schedule,
            enabled: extra.get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            next_run: extra.get("next_run")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            last_run: extra.get("last_run")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            created_at: extra.get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| Utc::now()),
            updated_at: extra.get("updated_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| Utc::now()),
        })
    }
}

#[async_trait]
impl MetaDataStorage for SqliteMetadataStore {
    /// Add a new job to the metadata storage
    async fn add(&self, job: JobStoredData) -> Result<(), JobSchedulerError> {
        debug!("Adding job to SQLite storage: id={}", job.id);
        
        // Convert JobStoredData to UnifiedSchedule
        let schedule = self.convert_job_data_to_schedule(job)
            .map_err(|e| {
                error!("Failed to convert job data to schedule: {}", e);
                JobSchedulerError::CantAdd
            })?;

        // Store via repository bridge
        self.repository_bridge.create_schedule(schedule).await
            .map_err(|e| {
                error!("Failed to create schedule via repository: {}", e);
                JobSchedulerError::CantAdd
            })?;

        debug!("Successfully added job to SQLite storage");
        Ok(())
    }

    /// Delete a job from the metadata storage
    async fn delete(&self, id: &Uuid) -> Result<(), JobSchedulerError> {
        debug!("Deleting job from SQLite storage: id={}", id);
        
        let schedule_id = ApiId::from_uuid(*id);
        self.repository_bridge.delete_schedule(schedule_id).await
            .map_err(|e| {
                error!("Failed to delete schedule via repository: {}", e);
                JobSchedulerError::CantRemove
            })?;

        debug!("Successfully deleted job from SQLite storage");
        Ok(())
    }

    /// Get a job from the metadata storage
    async fn get(&self, id: &Uuid) -> Option<JobStoredData> {
        debug!("Getting job from SQLite storage: id={}", id);
        
        let schedule_id = ApiId::from_uuid(*id);
        match self.repository_bridge.find_schedule(schedule_id).await {
            Ok(Some(schedule)) => {
                debug!("Found schedule in storage");
                Some(self.convert_schedule_to_job_data(schedule))
            },
            Ok(None) => {
                debug!("Schedule not found in storage");
                None
            },
            Err(e) => {
                warn!("Failed to find schedule in storage: {}", e);
                None
            }
        }
    }

    /// List all jobs in the metadata storage
    async fn list(&self) -> Vec<JobStoredData> {
        debug!("Listing all jobs from SQLite storage");
        
        match self.repository_bridge.load_all_schedules().await {
            Ok(schedules) => {
                debug!("Found {} schedules in storage", schedules.len());
                schedules.into_iter()
                    .map(|schedule| self.convert_schedule_to_job_data(schedule))
                    .collect()
            },
            Err(e) => {
                error!("Failed to load schedules from storage: {}", e);
                vec![]
            }
        }
    }
}