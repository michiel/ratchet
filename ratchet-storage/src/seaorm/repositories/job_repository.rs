use crate::database::{
    entities::{jobs, Job, JobActiveModel, JobStatus, JobPriority, Jobs},
    DatabaseConnection, DatabaseError,
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use chrono::{DateTime, Utc};

/// Filters for job queries
#[derive(Debug, Clone, Default)]
pub struct JobFilters {
    pub task_id: Option<i32>,
    pub status: Option<JobStatus>,
    pub priority: Option<JobPriority>,
    pub queued_after: Option<DateTime<Utc>>,
    pub scheduled_after: Option<DateTime<Utc>>,
}

/// Pagination settings for job queries
#[derive(Debug, Clone)]
pub struct JobPagination {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub order_by: Option<jobs::Column>,
    pub order_desc: Option<bool>,
}

/// Repository for job-related database operations
#[derive(Clone)]
pub struct JobRepository {
    db: DatabaseConnection,
}

impl JobRepository {
    /// Create a new job repository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new job
    pub async fn create(&self, job: Job) -> Result<Job, DatabaseError> {
        let active_model = JobActiveModel {
            uuid: Set(job.uuid),
            task_id: Set(job.task_id),
            execution_id: Set(job.execution_id),
            schedule_id: Set(job.schedule_id),
            priority: Set(job.priority),
            status: Set(job.status),
            input_data: Set(job.input_data),
            retry_count: Set(job.retry_count),
            max_retries: Set(job.max_retries),
            retry_delay_seconds: Set(job.retry_delay_seconds),
            error_message: Set(job.error_message),
            error_details: Set(job.error_details),
            queued_at: Set(job.queued_at),
            process_at: Set(job.process_at),
            started_at: Set(job.started_at),
            completed_at: Set(job.completed_at),
            metadata: Set(job.metadata),
            output_destinations: Set(job.output_destinations),
            ..Default::default()
        };

        let result = active_model.insert(self.db.get_connection()).await?;
        Ok(result)
    }

    /// Find job by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Job>, DatabaseError> {
        let job = Jobs::find_by_id(id).one(self.db.get_connection()).await?;
        Ok(job)
    }

    /// Find job by UUID
    pub async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<Job>, DatabaseError> {
        let job = Jobs::find()
            .filter(jobs::Column::Uuid.eq(uuid))
            .one(self.db.get_connection())
            .await?;
        Ok(job)
    }

    /// Find jobs ready for processing (prioritized queue)
    pub async fn find_ready_for_processing(&self, limit: u64) -> Result<Vec<Job>, DatabaseError> {
        let now = chrono::Utc::now();
        let jobs = Jobs::find()
            .filter(jobs::Column::Status.is_in(vec![JobStatus::Queued, JobStatus::Retrying]))
            .filter(
                jobs::Column::ProcessAt
                    .is_null()
                    .or(jobs::Column::ProcessAt.lte(now)),
            )
            .order_by(jobs::Column::Priority, Order::Desc) // Higher priority first
            .order_by(jobs::Column::QueuedAt, Order::Asc) // FIFO within same priority
            .limit(limit)
            .all(self.db.get_connection())
            .await?;
        Ok(jobs)
    }

    /// Find jobs by status
    pub async fn find_by_status(&self, status: JobStatus) -> Result<Vec<Job>, DatabaseError> {
        let jobs = Jobs::find()
            .filter(jobs::Column::Status.eq(status))
            .order_by(jobs::Column::QueuedAt, Order::Desc)
            .all(self.db.get_connection())
            .await?;
        Ok(jobs)
    }

    /// Find jobs by task ID
    pub async fn find_by_task_id(&self, task_id: i32) -> Result<Vec<Job>, DatabaseError> {
        let jobs = Jobs::find()
            .filter(jobs::Column::TaskId.eq(task_id))
            .order_by(jobs::Column::QueuedAt, Order::Desc)
            .all(self.db.get_connection())
            .await?;
        Ok(jobs)
    }

    /// Update job
    pub async fn update(&self, job: Job) -> Result<Job, DatabaseError> {
        let active_model: JobActiveModel = job.into();
        let updated_job = active_model.update(self.db.get_connection()).await?;
        Ok(updated_job)
    }

    /// Update job status
    pub async fn update_status(&self, id: i32, status: JobStatus) -> Result<(), DatabaseError> {
        let mut active_model = JobActiveModel {
            id: Set(id),
            status: Set(status),
            ..Default::default()
        };

        // Set timestamps based on status
        match status {
            JobStatus::Processing => {
                active_model.started_at = Set(Some(chrono::Utc::now()));
            }
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
                active_model.completed_at = Set(Some(chrono::Utc::now()));
            }
            _ => {}
        }

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Mark job as processing with execution ID
    pub async fn mark_processing(&self, id: i32, execution_id: i32) -> Result<(), DatabaseError> {
        let active_model = JobActiveModel {
            id: Set(id),
            status: Set(JobStatus::Processing),
            execution_id: Set(Some(execution_id)),
            started_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Mark job as completed
    pub async fn mark_completed(&self, id: i32) -> Result<(), DatabaseError> {
        self.update_status(id, JobStatus::Completed).await
    }

    /// Mark job as failed and increment retry count
    pub async fn mark_failed(
        &self,
        id: i32,
        error: String,
        details: Option<serde_json::Value>,
    ) -> Result<bool, DatabaseError> {
        // Get current job to check retry logic
        let job = self.find_by_id(id).await?;
        if let Some(mut job) = job {
            let will_retry = job.fail(error, details);

            let active_model = JobActiveModel {
                id: Set(id),
                status: Set(job.status),
                retry_count: Set(job.retry_count),
                error_message: Set(job.error_message),
                error_details: Set(job.error_details),
                process_at: Set(job.process_at),
                completed_at: Set(job.completed_at),
                ..Default::default()
            };

            active_model.update(self.db.get_connection()).await?;
            Ok(will_retry)
        } else {
            Ok(false)
        }
    }

    /// Delete job
    pub async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        Jobs::delete_by_id(id)
            .exec(self.db.get_connection())
            .await?;
        Ok(())
    }

    /// Count jobs
    pub async fn count(&self) -> Result<u64, DatabaseError> {
        let count = Jobs::find().count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Count jobs by status
    pub async fn count_by_status(&self, status: JobStatus) -> Result<u64, DatabaseError> {
        let count = Jobs::find()
            .filter(jobs::Column::Status.eq(status))
            .count(self.db.get_connection())
            .await?;
        Ok(count)
    }

    /// Cancel a job
    pub async fn cancel(&self, id: i32) -> Result<(), DatabaseError> {
        // Only allow cancelling jobs that are queued, retrying, or processing
        if let Some(job) = self.find_by_id(id).await? {
            match job.status {
                JobStatus::Queued | JobStatus::Retrying | JobStatus::Processing => {
                    self.update_status(id, JobStatus::Cancelled).await
                }
                _ => {
                    // Job already completed, failed, or cancelled
                    Ok(())
                }
            }
        } else {
            Ok(()) // Job doesn't exist, consider it "cancelled"
        }
    }

    /// Schedule a job to retry at a specific time
    pub async fn schedule_retry(&self, id: i32, retry_at: DateTime<Utc>) -> Result<(), DatabaseError> {
        let active_model = JobActiveModel {
            id: Set(id),
            status: Set(JobStatus::Retrying),
            process_at: Set(Some(retry_at)),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Find jobs by priority
    pub async fn find_by_priority(&self, priority: JobPriority) -> Result<Vec<Job>, DatabaseError> {
        let jobs = Jobs::find()
            .filter(jobs::Column::Priority.eq(priority))
            .order_by(jobs::Column::QueuedAt, Order::Asc)
            .all(self.db.get_connection())
            .await?;
        Ok(jobs)
    }

    /// Find jobs with advanced filtering
    pub async fn find_with_filters(
        &self,
        filters: JobFilters,
        pagination: JobPagination,
    ) -> Result<Vec<Job>, DatabaseError> {
        let mut query = Jobs::find();

        // Apply filters
        if let Some(task_id) = filters.task_id {
            query = query.filter(jobs::Column::TaskId.eq(task_id));
        }
        
        if let Some(status) = filters.status {
            query = query.filter(jobs::Column::Status.eq(status));
        }
        
        if let Some(priority) = filters.priority {
            query = query.filter(jobs::Column::Priority.eq(priority));
        }
        
        if let Some(queued_after) = filters.queued_after {
            query = query.filter(jobs::Column::QueuedAt.gte(queued_after));
        }
        
        if let Some(scheduled_after) = filters.scheduled_after {
            query = query.filter(jobs::Column::ProcessAt.gte(Some(scheduled_after)));
        }

        // Apply pagination
        if let Some(limit) = pagination.limit {
            query = query.limit(limit);
        }
        
        if let Some(offset) = pagination.offset {
            query = query.offset(offset);
        }

        // Apply ordering (default to priority + queued_at)
        query = query.order_by(
            pagination.order_by.unwrap_or(jobs::Column::Priority),
            if pagination.order_desc.unwrap_or(true) { Order::Desc } else { Order::Asc }
        );
        
        // Secondary order by queued time for jobs with same priority
        if pagination.order_by.is_none() {
            query = query.order_by(jobs::Column::QueuedAt, Order::Asc);
        }

        let jobs = query.all(self.db.get_connection()).await?;
        Ok(jobs)
    }

    /// Count jobs with filters
    pub async fn count_with_filters(&self, filters: JobFilters) -> Result<u64, DatabaseError> {
        let mut query = Jobs::find();

        // Apply same filters as find_with_filters
        if let Some(task_id) = filters.task_id {
            query = query.filter(jobs::Column::TaskId.eq(task_id));
        }
        
        if let Some(status) = filters.status {
            query = query.filter(jobs::Column::Status.eq(status));
        }
        
        if let Some(priority) = filters.priority {
            query = query.filter(jobs::Column::Priority.eq(priority));
        }
        
        if let Some(queued_after) = filters.queued_after {
            query = query.filter(jobs::Column::QueuedAt.gte(queued_after));
        }
        
        if let Some(scheduled_after) = filters.scheduled_after {
            query = query.filter(jobs::Column::ProcessAt.gte(Some(scheduled_after)));
        }

        let count = query.count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Get job queue statistics
    pub async fn get_queue_stats(&self) -> Result<JobQueueStats, DatabaseError> {
        let total = self.count().await?;
        let queued = self.count_by_status(JobStatus::Queued).await?;
        let processing = self.count_by_status(JobStatus::Processing).await?;
        let completed = self.count_by_status(JobStatus::Completed).await?;
        let failed = self.count_by_status(JobStatus::Failed).await?;
        let retrying = self.count_by_status(JobStatus::Retrying).await?;

        Ok(JobQueueStats {
            total,
            queued,
            processing,
            completed,
            failed,
            retrying,
        })
    }
}

/// Job queue statistics
#[derive(Debug, Clone)]
pub struct JobQueueStats {
    pub total: u64,
    pub queued: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub retrying: u64,
}

#[async_trait(?Send)]
impl super::Repository for JobRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.count().await?;
        Ok(())
    }
}
