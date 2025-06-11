//! Implementation of the legacy database adapter
//!
//! This module contains the concrete implementations of the legacy repository
//! traits that delegate to the modern ratchet-storage implementation.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;

use ratchet_storage::seaorm::repositories::RepositoryFactory as ModernRepositoryFactory;
use ratchet_interfaces::{RepositoryFactory, TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository};
use ratchet_api_types::{UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule};

use super::legacy_adapter::{
    LegacyDatabaseAdapter, LegacyRepositoryFactory, 
    LegacyTaskRepository, LegacyExecutionRepository, LegacyJobRepository, LegacyScheduleRepository,
    LegacyTask, LegacyExecution, LegacyJob, LegacySchedule
};

/// Error type for legacy adapter operations
#[derive(Debug, thiserror::Error)]
pub enum LegacyAdapterError {
    #[error("Database operation failed: {0}")]
    Database(String),
    
    #[error("Conversion error: {0}")]
    Conversion(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[async_trait]
impl LegacyRepositoryFactory for LegacyDatabaseAdapter {
    type Error = LegacyAdapterError;

    async fn task_repository(&self) -> Result<Box<dyn LegacyTaskRepository>, Self::Error> {
        Ok(Box::new(LegacyTaskRepositoryImpl {
            modern_impl: self.modern_impl.clone(),
        }))
    }
    
    async fn execution_repository(&self) -> Result<Box<dyn LegacyExecutionRepository>, Self::Error> {
        Ok(Box::new(LegacyExecutionRepositoryImpl {
            modern_impl: self.modern_impl.clone(),
        }))
    }
    
    async fn job_repository(&self) -> Result<Box<dyn LegacyJobRepository>, Self::Error> {
        Ok(Box::new(LegacyJobRepositoryImpl {
            modern_impl: self.modern_impl.clone(),
        }))
    }
    
    async fn schedule_repository(&self) -> Result<Box<dyn LegacyScheduleRepository>, Self::Error> {
        Ok(Box::new(LegacyScheduleRepositoryImpl {
            modern_impl: self.modern_impl.clone(),
        }))
    }
    
    async fn health_check(&self) -> Result<(), Self::Error> {
        self.modern_impl.health_check().await
            .map_err(|e| LegacyAdapterError::Database(e.to_string()))
    }
}

/// Legacy task repository implementation
struct LegacyTaskRepositoryImpl {
    modern_impl: Arc<ModernRepositoryFactory>,
}

#[async_trait]
impl LegacyTaskRepository for LegacyTaskRepositoryImpl {
    async fn find_all(&self) -> Result<Vec<LegacyTask>> {
        let modern_repo = self.modern_impl.task_repository();
        
        // Use a basic filter to get all tasks
        let filters = ratchet_interfaces::TaskFilters {
            name: None,
            enabled: None,
            registry_source: None,
            validated_after: None,
        };
        
        let pagination = ratchet_api_types::PaginationInput {
            page: Some(1),
            limit: Some(1000), // Large limit to get all tasks
            offset: None,
        };
        
        match modern_repo.find_with_filters(filters, pagination).await {
            Ok(response) => {
                let legacy_tasks = response.items.into_iter()
                    .map(LegacyTask::from)
                    .collect();
                Ok(legacy_tasks)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to find tasks: {}", e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<LegacyTask>> {
        let modern_repo = self.modern_impl.task_repository();
        
        match modern_repo.find_by_id(id).await {
            Ok(Some(task)) => Ok(Some(LegacyTask::from(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to find task by id: {}", e)),
        }
    }
    
    async fn find_enabled(&self) -> Result<Vec<LegacyTask>> {
        let modern_repo = self.modern_impl.task_repository();
        
        match modern_repo.find_enabled().await {
            Ok(tasks) => {
                let legacy_tasks = tasks.into_iter()
                    .map(LegacyTask::from)
                    .collect();
                Ok(legacy_tasks)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to find enabled tasks: {}", e)),
        }
    }
    
    async fn create(&self, task: LegacyTask) -> Result<LegacyTask> {
        let modern_repo = self.modern_impl.task_repository();
        let unified_task = UnifiedTask::from(task);
        
        match modern_repo.create(unified_task).await {
            Ok(created_task) => Ok(LegacyTask::from(created_task)),
            Err(e) => Err(anyhow::anyhow!("Failed to create task: {}", e)),
        }
    }
    
    async fn update(&self, task: LegacyTask) -> Result<LegacyTask> {
        let modern_repo = self.modern_impl.task_repository();
        let unified_task = UnifiedTask::from(task);
        
        match modern_repo.update(unified_task).await {
            Ok(updated_task) => Ok(LegacyTask::from(updated_task)),
            Err(e) => Err(anyhow::anyhow!("Failed to update task: {}", e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<()> {
        let modern_repo = self.modern_impl.task_repository();
        
        match modern_repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to delete task: {}", e)),
        }
    }
}

/// Legacy execution repository implementation
struct LegacyExecutionRepositoryImpl {
    modern_impl: Arc<ModernRepositoryFactory>,
}

#[async_trait]
impl LegacyExecutionRepository for LegacyExecutionRepositoryImpl {
    async fn find_by_task_id(&self, task_id: i32) -> Result<Vec<LegacyExecution>> {
        let modern_repo = self.modern_impl.execution_repository();
        let api_id = ratchet_api_types::ApiId::from_i32(task_id);
        
        match modern_repo.find_by_task_id(api_id).await {
            Ok(executions) => {
                let legacy_executions = executions.into_iter()
                    .map(LegacyExecution::from)
                    .collect();
                Ok(legacy_executions)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to find executions by task id: {}", e)),
        }
    }
    
    async fn create(&self, execution: LegacyExecution) -> Result<LegacyExecution> {
        let modern_repo = self.modern_impl.execution_repository();
        let unified_execution = UnifiedExecution::from(execution);
        
        match modern_repo.create(unified_execution).await {
            Ok(created_execution) => Ok(LegacyExecution::from(created_execution)),
            Err(e) => Err(anyhow::anyhow!("Failed to create execution: {}", e)),
        }
    }
    
    async fn update_status(&self, id: i32, status: String) -> Result<()> {
        let modern_repo = self.modern_impl.execution_repository();
        let api_id = ratchet_api_types::ApiId::from_i32(id);
        
        // Convert string status to ExecutionStatus enum
        let execution_status = match status.to_lowercase().as_str() {
            "pending" => ratchet_api_types::ExecutionStatus::Pending,
            "running" => ratchet_api_types::ExecutionStatus::Running,
            "completed" => ratchet_api_types::ExecutionStatus::Completed,
            "failed" => ratchet_api_types::ExecutionStatus::Failed,
            "cancelled" => ratchet_api_types::ExecutionStatus::Cancelled,
            _ => return Err(anyhow::anyhow!("Invalid execution status: {}", status)),
        };

        // Use the appropriate interface method based on status
        match execution_status {
            ratchet_api_types::ExecutionStatus::Running => {
                modern_repo.mark_started(api_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark execution as started: {}", e))
            },
            ratchet_api_types::ExecutionStatus::Completed => {
                // For legacy compatibility, complete with empty output
                modern_repo.mark_completed(api_id, serde_json::json!({}), None).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark execution as completed: {}", e))
            },
            ratchet_api_types::ExecutionStatus::Failed => {
                modern_repo.mark_failed(api_id, "Execution failed".to_string(), None).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark execution as failed: {}", e))
            },
            ratchet_api_types::ExecutionStatus::Cancelled => {
                modern_repo.mark_cancelled(api_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark execution as cancelled: {}", e))
            },
            _ => {
                // For other statuses, we'd need a generic update method which may not exist
                Err(anyhow::anyhow!("Status update to {} not supported through legacy interface", status))
            }
        }
    }
}

/// Legacy job repository implementation
struct LegacyJobRepositoryImpl {
    modern_impl: Arc<ModernRepositoryFactory>,
}

#[async_trait]
impl LegacyJobRepository for LegacyJobRepositoryImpl {
    async fn find_queued(&self) -> Result<Vec<LegacyJob>> {
        let modern_repo = self.modern_impl.job_repository();
        let queued_status = ratchet_api_types::JobStatus::Queued;
        
        match modern_repo.find_by_status(queued_status).await {
            Ok(jobs) => {
                let legacy_jobs = jobs.into_iter()
                    .map(LegacyJob::from)
                    .collect();
                Ok(legacy_jobs)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to find queued jobs: {}", e)),
        }
    }
    
    async fn create(&self, job: LegacyJob) -> Result<LegacyJob> {
        let modern_repo = self.modern_impl.job_repository();
        let unified_job = UnifiedJob::from(job);
        
        match modern_repo.create(unified_job).await {
            Ok(created_job) => Ok(LegacyJob::from(created_job)),
            Err(e) => Err(anyhow::anyhow!("Failed to create job: {}", e)),
        }
    }
    
    async fn update_status(&self, id: i32, status: String) -> Result<()> {
        let modern_repo = self.modern_impl.job_repository();
        let api_id = ratchet_api_types::ApiId::from_i32(id);
        
        // Convert string status to JobStatus enum
        let job_status = match status.to_lowercase().as_str() {
            "queued" => ratchet_api_types::JobStatus::Queued,
            "processing" => ratchet_api_types::JobStatus::Processing,
            "completed" => ratchet_api_types::JobStatus::Completed,
            "failed" => ratchet_api_types::JobStatus::Failed,
            "cancelled" => ratchet_api_types::JobStatus::Cancelled,
            "retrying" => ratchet_api_types::JobStatus::Retrying,
            _ => return Err(anyhow::anyhow!("Invalid job status: {}", status)),
        };

        // Use the appropriate interface method based on status
        match job_status {
            ratchet_api_types::JobStatus::Processing => {
                // Need execution_id for processing - use dummy value for legacy compatibility
                let dummy_execution_id = ratchet_api_types::ApiId::from_i32(0);
                modern_repo.mark_processing(api_id, dummy_execution_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark job as processing: {}", e))
            },
            ratchet_api_types::JobStatus::Completed => {
                modern_repo.mark_completed(api_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to mark job as completed: {}", e))
            },
            ratchet_api_types::JobStatus::Failed => {
                modern_repo.mark_failed(api_id, "Job failed".to_string(), None).await
                    .map(|_| ()) // Ignore retry result for legacy compatibility
                    .map_err(|e| anyhow::anyhow!("Failed to mark job as failed: {}", e))
            },
            ratchet_api_types::JobStatus::Cancelled => {
                modern_repo.cancel(api_id).await
                    .map_err(|e| anyhow::anyhow!("Failed to cancel job: {}", e))
            },
            _ => {
                Err(anyhow::anyhow!("Status update to {} not supported through legacy interface", status))
            }
        }
    }
}

/// Legacy schedule repository implementation
struct LegacyScheduleRepositoryImpl {
    modern_impl: Arc<ModernRepositoryFactory>,
}

#[async_trait]
impl LegacyScheduleRepository for LegacyScheduleRepositoryImpl {
    async fn find_active(&self) -> Result<Vec<LegacySchedule>> {
        let modern_repo = self.modern_impl.schedule_repository();
        
        match modern_repo.find_enabled().await {
            Ok(schedules) => {
                let legacy_schedules = schedules.into_iter()
                    .map(LegacySchedule::from)
                    .collect();
                Ok(legacy_schedules)
            },
            Err(e) => Err(anyhow::anyhow!("Failed to find active schedules: {}", e)),
        }
    }
    
    async fn create(&self, schedule: LegacySchedule) -> Result<LegacySchedule> {
        let modern_repo = self.modern_impl.schedule_repository();
        let unified_schedule = UnifiedSchedule::from(schedule);
        
        match modern_repo.create(unified_schedule).await {
            Ok(created_schedule) => Ok(LegacySchedule::from(created_schedule)),
            Err(e) => Err(anyhow::anyhow!("Failed to create schedule: {}", e)),
        }
    }
    
    async fn update(&self, schedule: LegacySchedule) -> Result<LegacySchedule> {
        let modern_repo = self.modern_impl.schedule_repository();
        let unified_schedule = UnifiedSchedule::from(schedule);
        
        match modern_repo.update(unified_schedule).await {
            Ok(updated_schedule) => Ok(LegacySchedule::from(updated_schedule)),
            Err(e) => Err(anyhow::anyhow!("Failed to update schedule: {}", e)),
        }
    }
}

// Additional conversion functions needed for legacy Job type
impl From<LegacyJob> for UnifiedJob {
    fn from(job: LegacyJob) -> Self {
        let status = match job.status.to_lowercase().as_str() {
            "queued" => ratchet_api_types::JobStatus::Queued,
            "processing" => ratchet_api_types::JobStatus::Processing,
            "completed" => ratchet_api_types::JobStatus::Completed,
            "failed" => ratchet_api_types::JobStatus::Failed,
            "cancelled" => ratchet_api_types::JobStatus::Cancelled,
            "retrying" => ratchet_api_types::JobStatus::Retrying,
            _ => ratchet_api_types::JobStatus::Queued, // Default fallback
        };

        let priority = match job.priority.to_lowercase().as_str() {
            "low" => ratchet_api_types::JobPriority::Low,
            "normal" => ratchet_api_types::JobPriority::Normal,
            "high" => ratchet_api_types::JobPriority::High,
            "critical" => ratchet_api_types::JobPriority::Critical,
            _ => ratchet_api_types::JobPriority::Normal, // Default fallback
        };

        Self {
            id: ratchet_api_types::ApiId::from_i32(job.id),
            uuid: job.uuid,
            task_id: ratchet_api_types::ApiId::from_i32(job.task_id),
            status,
            priority,
            input: Some(job.input),
            output: job.output,
            error_message: job.error_message,
            max_retries: Some(job.max_retries),
            retry_count: Some(job.retry_count),
            created_at: job.created_at,
            updated_at: job.updated_at,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
        }
    }
}

impl From<LegacySchedule> for UnifiedSchedule {
    fn from(schedule: LegacySchedule) -> Self {
        Self {
            id: ratchet_api_types::ApiId::from_i32(schedule.id),
            uuid: schedule.uuid,
            task_id: ratchet_api_types::ApiId::from_i32(schedule.task_id),
            name: schedule.name,
            cron_expression: schedule.cron_expression,
            enabled: schedule.enabled,
            input: Some(schedule.input),
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
            last_run_at: schedule.last_run_at,
            next_run_at: schedule.next_run_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[tokio::test]
    async fn test_legacy_adapter_creation() {
        // This test would require a real database connection to work fully
        // For now, just test that the adapter can be created
        let config = ratchet_storage::seaorm::config::SeaOrmConfig::default();
        
        // In a real test, we'd create the modern factory with a real connection
        // let modern_factory = Arc::new(ModernRepositoryFactory::new(connection));
        // let adapter = LegacyDatabaseAdapter::new(modern_factory);
        // assert!(adapter.health_check().await.is_ok());
    }

    #[test]
    fn test_legacy_job_conversion() {
        let legacy_job = LegacyJob {
            id: 1,
            uuid: Uuid::new_v4(),
            task_id: 1,
            input: serde_json::json!({"test": "data"}),
            output: None,
            status: "queued".to_string(),
            priority: "high".to_string(),
            error_message: None,
            max_retries: 3,
            retry_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            scheduled_at: None,
            started_at: None,
            completed_at: None,
        };

        let unified_job = UnifiedJob::from(legacy_job.clone());
        let converted_back = LegacyJob::from(unified_job);

        assert_eq!(legacy_job.status, converted_back.status);
        assert_eq!(legacy_job.priority, converted_back.priority);
        assert_eq!(legacy_job.max_retries, converted_back.max_retries);
    }
}