use crate::database::{
    entities::{executions, Execution, ExecutionActiveModel, ExecutionStatus, Executions},
    DatabaseConnection, DatabaseError,
};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Filters for execution queries
#[derive(Debug, Clone, Default)]
pub struct ExecutionFilters {
    pub task_id: Option<i32>,
    pub status: Option<ExecutionStatus>,
    pub queued_after: Option<DateTime<Utc>>,
    pub completed_after: Option<DateTime<Utc>>,
}

/// Pagination settings for execution queries  
#[derive(Debug, Clone)]
pub struct ExecutionPagination {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub order_by: Option<executions::Column>,
    pub order_desc: Option<bool>,
}

/// Repository for execution-related database operations
#[derive(Clone)]
pub struct ExecutionRepository {
    db: DatabaseConnection,
}

impl ExecutionRepository {
    /// Create a new execution repository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new execution
    pub async fn create(&self, execution: Execution) -> Result<Execution, DatabaseError> {
        let active_model = ExecutionActiveModel {
            uuid: Set(execution.uuid),
            task_id: Set(execution.task_id),
            input: Set(execution.input),
            output: Set(execution.output),
            status: Set(execution.status),
            error_message: Set(execution.error_message),
            error_details: Set(execution.error_details),
            queued_at: Set(execution.queued_at),
            started_at: Set(execution.started_at),
            completed_at: Set(execution.completed_at),
            duration_ms: Set(execution.duration_ms),
            http_requests: Set(execution.http_requests),
            recording_path: Set(execution.recording_path),
            ..Default::default()
        };

        let result = active_model.insert(self.db.get_connection()).await?;
        Ok(result)
    }

    /// Find execution by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Execution>, DatabaseError> {
        let execution = Executions::find_by_id(id)
            .one(self.db.get_connection())
            .await?;
        Ok(execution)
    }

    /// Find execution by UUID
    pub async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<Execution>, DatabaseError> {
        let execution = Executions::find()
            .filter(executions::Column::Uuid.eq(uuid.to_string()))
            .one(self.db.get_connection())
            .await?;
        Ok(execution)
    }

    /// Find executions by task ID
    pub async fn find_by_task_id(&self, task_id: i32) -> Result<Vec<Execution>, DatabaseError> {
        let executions = Executions::find()
            .filter(executions::Column::TaskId.eq(task_id))
            .order_by(executions::Column::QueuedAt, Order::Desc)
            .all(self.db.get_connection())
            .await?;
        Ok(executions)
    }

    /// Find executions by status
    pub async fn find_by_status(
        &self,
        status: ExecutionStatus,
    ) -> Result<Vec<Execution>, DatabaseError> {
        let executions = Executions::find()
            .filter(executions::Column::Status.eq(status))
            .order_by(executions::Column::QueuedAt, Order::Desc)
            .all(self.db.get_connection())
            .await?;
        Ok(executions)
    }

    /// Find recent executions (limit)
    pub async fn find_recent(&self, limit: u64) -> Result<Vec<Execution>, DatabaseError> {
        let executions = Executions::find()
            .order_by(executions::Column::QueuedAt, Order::Desc)
            .limit(limit)
            .all(self.db.get_connection())
            .await?;
        Ok(executions)
    }

    /// Find running executions
    pub async fn find_running(&self) -> Result<Vec<Execution>, DatabaseError> {
        self.find_by_status(ExecutionStatus::Running).await
    }

    /// Find pending executions
    pub async fn find_pending(&self) -> Result<Vec<Execution>, DatabaseError> {
        self.find_by_status(ExecutionStatus::Pending).await
    }

    /// Update execution
    pub async fn update(&self, execution: Execution) -> Result<Execution, DatabaseError> {
        let active_model: ExecutionActiveModel = execution.into();
        let updated_execution = active_model.update(self.db.get_connection()).await?;
        Ok(updated_execution)
    }

    /// Update execution status
    pub async fn update_status(
        &self,
        id: i32,
        status: ExecutionStatus,
    ) -> Result<(), DatabaseError> {
        let mut active_model = ExecutionActiveModel {
            id: Set(id),
            status: Set(status),
            ..Default::default()
        };

        // Set timestamps based on status
        match status {
            ExecutionStatus::Running => {
                active_model.started_at = Set(Some(chrono::Utc::now()));
            }
            ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled => {
                active_model.completed_at = Set(Some(chrono::Utc::now()));
            }
            _ => {}
        }

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Mark execution as started
    pub async fn mark_started(&self, id: i32) -> Result<(), DatabaseError> {
        self.update_status(id, ExecutionStatus::Running).await
    }

    /// Mark execution as completed with output and optional duration
    pub async fn mark_completed(
        &self,
        id: i32,
        output: serde_json::Value,
        duration_ms: Option<i32>,
    ) -> Result<(), DatabaseError> {
        let completed_at = chrono::Utc::now();
        
        // If duration not provided, calculate from started_at
        let final_duration = if duration_ms.is_none() {
            if let Ok(Some(execution)) = self.find_by_id(id).await {
                execution.started_at.map(|started_at| (completed_at - started_at).num_milliseconds() as i32)
            } else {
                None
            }
        } else {
            duration_ms
        };
        
        let active_model = ExecutionActiveModel {
            id: Set(id),
            status: Set(ExecutionStatus::Completed),
            output: Set(Some(output)),
            completed_at: Set(Some(completed_at)),
            duration_ms: Set(final_duration),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Mark execution as failed with error
    pub async fn mark_failed(
        &self,
        id: i32,
        error: String,
        details: Option<serde_json::Value>,
    ) -> Result<(), DatabaseError> {
        let active_model = ExecutionActiveModel {
            id: Set(id),
            status: Set(ExecutionStatus::Failed),
            error_message: Set(Some(error)),
            error_details: Set(details),
            completed_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        active_model.update(self.db.get_connection()).await?;
        Ok(())
    }

    /// Delete execution
    pub async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        Executions::delete_by_id(id)
            .exec(self.db.get_connection())
            .await?;
        Ok(())
    }

    /// Count executions
    pub async fn count(&self) -> Result<u64, DatabaseError> {
        let count = Executions::find().count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Count executions by status
    pub async fn count_by_status(&self, status: ExecutionStatus) -> Result<u64, DatabaseError> {
        let count = Executions::find()
            .filter(executions::Column::Status.eq(status))
            .count(self.db.get_connection())
            .await?;
        Ok(count)
    }

    /// Count executions by task
    pub async fn count_by_task(&self, task_id: i32) -> Result<u64, DatabaseError> {
        let count = Executions::find()
            .filter(executions::Column::TaskId.eq(task_id))
            .count(self.db.get_connection())
            .await?;
        Ok(count)
    }

    /// Update execution progress (for progress tracking)
    pub async fn update_progress(&self, id: i32, progress: f32) -> Result<(), DatabaseError> {
        // Note: Progress tracking would require adding a progress column to the execution table
        // For now, we'll store progress in the output JSON as a temporary solution
        if let Ok(Some(execution)) = self.find_by_id(id).await {
            let mut output = execution.output.unwrap_or_else(|| serde_json::json!({}));
            if let Some(output_obj) = output.as_object_mut() {
                output_obj.insert("progress".to_string(), serde_json::Value::Number(
                    serde_json::Number::from_f64(progress as f64).unwrap_or(serde_json::Number::from(0))
                ));
            }
            
            let active_model = ExecutionActiveModel {
                id: Set(id),
                output: Set(Some(output)),
                ..Default::default()
            };
            
            active_model.update(self.db.get_connection()).await?;
        }
        Ok(())
    }

    /// Find executions with advanced filtering
    pub async fn find_with_filters(
        &self,
        filters: ExecutionFilters,
        pagination: ExecutionPagination,
    ) -> Result<Vec<Execution>, DatabaseError> {
        let mut query = Executions::find();

        // Apply filters
        if let Some(task_id) = filters.task_id {
            query = query.filter(executions::Column::TaskId.eq(task_id));
        }
        
        if let Some(status) = filters.status {
            query = query.filter(executions::Column::Status.eq(status));
        }
        
        if let Some(queued_after) = filters.queued_after {
            query = query.filter(executions::Column::QueuedAt.gte(queued_after));
        }
        
        if let Some(completed_after) = filters.completed_after {
            query = query.filter(executions::Column::CompletedAt.gte(Some(completed_after)));
        }

        // Apply pagination
        if let Some(limit) = pagination.limit {
            query = query.limit(limit);
        }
        
        if let Some(offset) = pagination.offset {
            query = query.offset(offset);
        }

        // Apply ordering
        query = query.order_by(
            pagination.order_by.unwrap_or(executions::Column::QueuedAt),
            if pagination.order_desc.unwrap_or(true) { Order::Desc } else { Order::Asc }
        );

        let executions = query.all(self.db.get_connection()).await?;
        Ok(executions)
    }

    /// Count executions with filters
    pub async fn count_with_filters(&self, filters: ExecutionFilters) -> Result<u64, DatabaseError> {
        let mut query = Executions::find();

        // Apply same filters as find_with_filters
        if let Some(task_id) = filters.task_id {
            query = query.filter(executions::Column::TaskId.eq(task_id));
        }
        
        if let Some(status) = filters.status {
            query = query.filter(executions::Column::Status.eq(status));
        }
        
        if let Some(queued_after) = filters.queued_after {
            query = query.filter(executions::Column::QueuedAt.gte(queued_after));
        }
        
        if let Some(completed_after) = filters.completed_after {
            query = query.filter(executions::Column::CompletedAt.gte(Some(completed_after)));
        }

        let count = query.count(self.db.get_connection()).await?;
        Ok(count)
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> Result<ExecutionStats, DatabaseError> {
        let total = self.count().await?;
        let pending = self.count_by_status(ExecutionStatus::Pending).await?;
        let running = self.count_by_status(ExecutionStatus::Running).await?;
        let completed = self.count_by_status(ExecutionStatus::Completed).await?;
        let failed = self.count_by_status(ExecutionStatus::Failed).await?;

        Ok(ExecutionStats {
            total,
            pending,
            running,
            completed,
            failed,
        })
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total: u64,
    pub pending: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
}

#[async_trait(?Send)]
impl super::Repository for ExecutionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simple health check - try to count executions
        self.count().await?;
        Ok(())
    }
}
