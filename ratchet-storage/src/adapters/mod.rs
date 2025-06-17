//! Adapter layer for repository pattern unification
//!
//! This module provides adapters that implement the abstract repository interfaces
//! from ratchet-interfaces using the concrete SeaORM implementations from ratchet-storage.
//! This enables a unified repository pattern across the entire codebase.

use std::sync::Arc;
use async_trait::async_trait;
use ratchet_interfaces::{
    Repository, RepositoryFactory,
    TaskRepository as InterfaceTaskRepository, TaskFilters,
    ExecutionRepository as InterfaceExecutionRepository, ExecutionFilters,
    JobRepository as InterfaceJobRepository, JobFilters,
    ScheduleRepository as InterfaceScheduleRepository, ScheduleFilters,
    DatabaseError
};
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus, JobPriority
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unified repository factory that implements ratchet-interfaces traits
/// using ratchet-storage SeaORM implementations
pub struct UnifiedRepositoryFactory {
    /// SeaORM repository factory
    seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>,
    
    /// Cached repository adapters
    task_adapter: Arc<UnifiedTaskRepository>,
    execution_adapter: Arc<UnifiedExecutionRepository>,
    job_adapter: Arc<UnifiedJobRepository>,
    schedule_adapter: Arc<UnifiedScheduleRepository>,
}

impl UnifiedRepositoryFactory {
    /// Create a new unified repository factory
    pub fn new(seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>) -> Self {
        let task_adapter = Arc::new(UnifiedTaskRepository::new(seaorm_factory.clone()));
        let execution_adapter = Arc::new(UnifiedExecutionRepository::new(seaorm_factory.clone()));
        let job_adapter = Arc::new(UnifiedJobRepository::new(seaorm_factory.clone()));
        let schedule_adapter = Arc::new(UnifiedScheduleRepository::new(seaorm_factory.clone()));
        
        Self {
            seaorm_factory,
            task_adapter,
            execution_adapter,
            job_adapter,
            schedule_adapter,
        }
    }
    
    /// Get access to the underlying SeaORM factory for advanced operations
    pub fn seaorm_factory(&self) -> &crate::seaorm::repositories::RepositoryFactory {
        &self.seaorm_factory
    }
}

#[async_trait]
impl RepositoryFactory for UnifiedRepositoryFactory {
    fn task_repository(&self) -> &dyn InterfaceTaskRepository {
        self.task_adapter.as_ref()
    }
    
    fn execution_repository(&self) -> &dyn InterfaceExecutionRepository {
        self.execution_adapter.as_ref()
    }
    
    fn job_repository(&self) -> &dyn InterfaceJobRepository {
        self.job_adapter.as_ref()
    }
    
    fn schedule_repository(&self) -> &dyn InterfaceScheduleRepository {
        self.schedule_adapter.as_ref()
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.seaorm_factory.task_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

/// Unified task repository adapter
pub struct UnifiedTaskRepository {
    seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>,
}

impl UnifiedTaskRepository {
    pub fn new(seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>) -> Self {
        Self { seaorm_factory }
    }
}

#[async_trait]
impl Repository for UnifiedTaskRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.seaorm_factory.task_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedTask> for UnifiedTaskRepository {
    async fn create(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.seaorm_factory.task_repository().create(storage_task).await {
            Ok(created_task) => Ok(convert_storage_task_to_unified(created_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.seaorm_factory.task_repository().find_by_id(id).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.seaorm_factory.task_repository().find_by_uuid(uuid).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.seaorm_factory.task_repository().update(storage_task).await {
            Ok(updated_task) => Ok(convert_storage_task_to_unified(updated_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.seaorm_factory.task_repository().delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.seaorm_factory.task_repository().count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedTask, TaskFilters> for UnifiedTaskRepository {
    async fn find_with_filters(
        &self, 
        filters: TaskFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // Convert interface filters to storage filters
        let storage_filters = convert_task_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.seaorm_factory.task_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(tasks) => {
                let unified_tasks: Vec<UnifiedTask> = tasks.into_iter()
                    .map(convert_storage_task_to_unified)
                    .collect();
                    
                Ok(ListResponse {
                    items: unified_tasks,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0, // Would need separate count query
                        has_next: false, // Would need to calculate
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1, // Would need to calculate
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: TaskFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // Convert interface filters to storage filters
        let storage_filters = convert_task_filters_to_storage(filters);
        let storage_pagination = convert_list_input_to_storage_pagination(list_input.clone());
        
        match self.seaorm_factory.task_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(tasks) => {
                let unified_tasks: Vec<UnifiedTask> = tasks.into_iter()
                    .map(convert_storage_task_to_unified)
                    .collect();
                    
                let pagination = list_input.get_pagination();
                Ok(ListResponse {
                    items: unified_tasks,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0, // Would need separate count query
                        has_next: false, // Would need to calculate
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1, // Would need to calculate
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count_with_filters(&self, filters: TaskFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_task_filters_to_storage(filters);
        
        match self.seaorm_factory.task_repository().count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl InterfaceTaskRepository for UnifiedTaskRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError> {
        match self.seaorm_factory.task_repository().find_enabled().await {
            Ok(tasks) => Ok(tasks.into_iter().map(convert_storage_task_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.seaorm_factory.task_repository().find_by_name(name).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_validated(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.task_repository().mark_validated(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.task_repository().set_enabled(i32_id, enabled).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn set_in_sync(&self, _id: ApiId, _in_sync: bool) -> Result<(), DatabaseError> {
        // Not implemented in SeaORM layer yet
        Err(DatabaseError::Internal { 
            message: "Task sync updates not yet implemented in SeaORM layer".to_string() 
        })
    }
}

/// Unified execution repository adapter
pub struct UnifiedExecutionRepository {
    seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>,
}

impl UnifiedExecutionRepository {
    pub fn new(seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>) -> Self {
        Self { seaorm_factory }
    }
}

#[async_trait]
impl Repository for UnifiedExecutionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.seaorm_factory.execution_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedExecution> for UnifiedExecutionRepository {
    async fn create(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        let storage_execution = convert_unified_execution_to_storage(entity);
        
        match self.seaorm_factory.execution_repository().create(storage_execution).await {
            Ok(created_execution) => Ok(convert_storage_execution_to_unified(created_execution)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedExecution>, DatabaseError> {
        match self.seaorm_factory.execution_repository().find_by_id(id).await {
            Ok(Some(execution)) => Ok(Some(convert_storage_execution_to_unified(execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedExecution>, DatabaseError> {
        match self.seaorm_factory.execution_repository().find_by_uuid(uuid).await {
            Ok(Some(execution)) => Ok(Some(convert_storage_execution_to_unified(execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        let storage_execution = convert_unified_execution_to_storage(entity);
        
        match self.seaorm_factory.execution_repository().update(storage_execution).await {
            Ok(updated_execution) => Ok(convert_storage_execution_to_unified(updated_execution)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.seaorm_factory.execution_repository().delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.seaorm_factory.execution_repository().count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedExecution, ExecutionFilters> for UnifiedExecutionRepository {
    async fn find_with_filters(
        &self, 
        filters: ExecutionFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        let storage_filters = convert_execution_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.seaorm_factory.execution_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(executions) => {
                let unified_executions: Vec<UnifiedExecution> = executions.into_iter()
                    .map(convert_storage_execution_to_unified)
                    .collect();
                    
                Ok(ListResponse {
                    items: unified_executions,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: ExecutionFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        let storage_filters = convert_execution_filters_to_storage(filters);
        let storage_pagination = convert_list_input_to_execution_pagination(list_input.clone());
        
        match self.seaorm_factory.execution_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(executions) => {
                let unified_executions: Vec<UnifiedExecution> = executions.into_iter()
                    .map(convert_storage_execution_to_unified)
                    .collect();
                    
                let pagination = list_input.get_pagination();
                Ok(ListResponse {
                    items: unified_executions,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count_with_filters(&self, filters: ExecutionFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_execution_filters_to_storage(filters);
        
        match self.seaorm_factory.execution_repository().count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl InterfaceExecutionRepository for UnifiedExecutionRepository {
    async fn find_by_task_id(&self, task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        let i32_id = task_id.as_i32().unwrap_or(0);
        match self.seaorm_factory.execution_repository().find_by_task_id(i32_id).await {
            Ok(executions) => Ok(executions.into_iter().map(convert_storage_execution_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_status(&self, status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        // This method might not exist in SeaORM layer - need to implement differently
        Err(DatabaseError::Internal { 
            message: "find_by_status not implemented in SeaORM layer".to_string()
        })
    }
    
    async fn mark_started(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.execution_repository().mark_started(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_completed(
        &self, 
        id: ApiId, 
        output: serde_json::Value,
        duration_ms: Option<i32>
    ) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.execution_repository().mark_completed(i32_id, output, duration_ms).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error_message: String,
        error_details: Option<serde_json::Value>
    ) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.execution_repository().mark_failed(i32_id, error_message, error_details).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_cancelled(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.execution_repository().mark_cancelled(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update_progress(&self, _id: ApiId, _progress: f32) -> Result<(), DatabaseError> {
        // Progress tracking not implemented in SeaORM layer yet
        Err(DatabaseError::Internal { 
            message: "Progress tracking not implemented in SeaORM layer".to_string()
        })
    }
}

/// Unified job repository adapter
pub struct UnifiedJobRepository {
    seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>,
}

impl UnifiedJobRepository {
    pub fn new(seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>) -> Self {
        Self { seaorm_factory }
    }
}

#[async_trait]
impl Repository for UnifiedJobRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.seaorm_factory.job_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedJob> for UnifiedJobRepository {
    async fn create(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        
        match self.seaorm_factory.job_repository().create(storage_job).await {
            Ok(created_job) => Ok(convert_storage_job_to_unified(created_job)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedJob>, DatabaseError> {
        match self.seaorm_factory.job_repository().find_by_id(id).await {
            Ok(Some(job)) => Ok(Some(convert_storage_job_to_unified(job))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedJob>, DatabaseError> {
        match self.seaorm_factory.job_repository().find_by_uuid(uuid).await {
            Ok(Some(job)) => Ok(Some(convert_storage_job_to_unified(job))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        
        match self.seaorm_factory.job_repository().update(storage_job).await {
            Ok(updated_job) => Ok(convert_storage_job_to_unified(updated_job)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.seaorm_factory.job_repository().delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.seaorm_factory.job_repository().count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedJob, JobFilters> for UnifiedJobRepository {
    async fn find_with_filters(
        &self, 
        filters: JobFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        let storage_filters = convert_job_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.seaorm_factory.job_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(jobs) => {
                let unified_jobs: Vec<UnifiedJob> = jobs.into_iter()
                    .map(convert_storage_job_to_unified)
                    .collect();
                    
                Ok(ListResponse {
                    items: unified_jobs,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: JobFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        let storage_filters = convert_job_filters_to_storage(filters);
        let storage_pagination = convert_list_input_to_job_pagination(list_input.clone());
        
        match self.seaorm_factory.job_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(jobs) => {
                let unified_jobs: Vec<UnifiedJob> = jobs.into_iter()
                    .map(convert_storage_job_to_unified)
                    .collect();
                    
                let pagination = list_input.get_pagination();
                Ok(ListResponse {
                    items: unified_jobs,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count_with_filters(&self, filters: JobFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_job_filters_to_storage(filters);
        
        match self.seaorm_factory.job_repository().count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl InterfaceJobRepository for UnifiedJobRepository {
    async fn find_ready_for_processing(&self, limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError> {
        match self.seaorm_factory.job_repository().find_ready_for_processing(limit).await {
            Ok(jobs) => Ok(jobs.into_iter().map(convert_storage_job_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError> {
        let storage_status = convert_job_status_to_storage(status);
        match self.seaorm_factory.job_repository().find_by_status(storage_status).await {
            Ok(jobs) => Ok(jobs.into_iter().map(convert_storage_job_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_processing(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let i32_execution_id = execution_id.as_i32().unwrap_or(0);
        match self.seaorm_factory.job_repository().mark_processing(i32_id, i32_execution_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_completed(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.job_repository().mark_completed(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error: String, 
        details: Option<serde_json::Value>
    ) -> Result<bool, DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.job_repository().mark_failed(i32_id, error, details).await {
            Ok(can_retry) => Ok(can_retry),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn schedule_retry(&self, id: ApiId, retry_at: DateTime<Utc>) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.job_repository().schedule_retry(i32_id, retry_at).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn cancel(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.job_repository().cancel(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

/// Unified schedule repository adapter
pub struct UnifiedScheduleRepository {
    seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>,
}

impl UnifiedScheduleRepository {
    pub fn new(seaorm_factory: Arc<crate::seaorm::repositories::RepositoryFactory>) -> Self {
        Self { seaorm_factory }
    }
}

#[async_trait]
impl Repository for UnifiedScheduleRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.seaorm_factory.schedule_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedSchedule> for UnifiedScheduleRepository {
    async fn create(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.seaorm_factory.schedule_repository().create(storage_schedule).await {
            Ok(created_schedule) => Ok(convert_storage_schedule_to_unified(created_schedule)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        match self.seaorm_factory.schedule_repository().find_by_id(id).await {
            Ok(Some(schedule)) => Ok(Some(convert_storage_schedule_to_unified(schedule))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        match self.seaorm_factory.schedule_repository().find_by_uuid(uuid).await {
            Ok(Some(schedule)) => Ok(Some(convert_storage_schedule_to_unified(schedule))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.seaorm_factory.schedule_repository().update(storage_schedule).await {
            Ok(updated_schedule) => Ok(convert_storage_schedule_to_unified(updated_schedule)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.seaorm_factory.schedule_repository().delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.seaorm_factory.schedule_repository().count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedSchedule, ScheduleFilters> for UnifiedScheduleRepository {
    async fn find_with_filters(
        &self, 
        filters: ScheduleFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        let storage_filters = convert_schedule_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.seaorm_factory.schedule_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(schedules) => {
                let unified_schedules: Vec<UnifiedSchedule> = schedules.into_iter()
                    .map(convert_storage_schedule_to_unified)
                    .collect();
                    
                Ok(ListResponse {
                    items: unified_schedules,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_with_list_input(
        &self,
        filters: ScheduleFilters,
        list_input: ratchet_api_types::pagination::ListInput,
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        let storage_filters = convert_schedule_filters_to_storage(filters);
        let storage_pagination = convert_list_input_to_schedule_pagination(list_input.clone());
        
        match self.seaorm_factory.schedule_repository().find_with_filters(storage_filters, storage_pagination).await {
            Ok(schedules) => {
                let unified_schedules: Vec<UnifiedSchedule> = schedules.into_iter()
                    .map(convert_storage_schedule_to_unified)
                    .collect();
                    
                let pagination = list_input.get_pagination();
                Ok(ListResponse {
                    items: unified_schedules,
                    meta: ratchet_api_types::pagination::PaginationMeta {
                        page: pagination.page.unwrap_or(1),
                        limit: pagination.limit.unwrap_or(20),
                        offset: pagination.offset.unwrap_or(0),
                        total: 0,
                        has_next: false,
                        has_previous: pagination.offset.unwrap_or(0) > 0,
                        total_pages: 1,
                    },
                })
            },
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count_with_filters(&self, filters: ScheduleFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_schedule_filters_to_storage(filters);
        
        match self.seaorm_factory.schedule_repository().count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl InterfaceScheduleRepository for UnifiedScheduleRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.seaorm_factory.schedule_repository().find_enabled().await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.seaorm_factory.schedule_repository().find_ready_to_run().await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn record_execution(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let i32_execution_id = execution_id.as_i32().unwrap_or(0);
        match self.seaorm_factory.schedule_repository().record_execution(i32_id, i32_execution_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update_next_run(&self, id: ApiId, next_run: DateTime<Utc>) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.schedule_repository().update_next_run(i32_id, next_run).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.seaorm_factory.schedule_repository().set_enabled(i32_id, enabled).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

// Conversion helper functions - these would need to be implemented based on actual entity structures

/// Convert storage error to interface DatabaseError
fn convert_storage_error(err: crate::seaorm::connection::DatabaseError) -> DatabaseError {
    DatabaseError::Internal { message: err.to_string() }
}

/// Convert unified task to storage task
fn convert_unified_task_to_storage(task: UnifiedTask) -> crate::seaorm::entities::tasks::Model {
    crate::seaorm::entities::tasks::Model {
        id: task.id.as_i32().unwrap_or(0),
        uuid: task.uuid,
        name: task.name,
        description: task.description,
        version: task.version,
        path: String::new(), // Would need to be provided or inferred
        metadata: task.metadata.unwrap_or_default(),
        input_schema: task.input_schema.unwrap_or_default(),
        output_schema: task.output_schema.unwrap_or_default(),
        enabled: task.enabled,
        created_at: task.created_at,
        updated_at: task.updated_at,
        validated_at: task.validated_at,
    }
}

/// Convert storage task to unified task
fn convert_storage_task_to_unified(task: crate::seaorm::entities::tasks::Model) -> UnifiedTask {
    UnifiedTask {
        id: ApiId::from_i32(task.id),
        uuid: task.uuid,
        name: task.name,
        description: task.description,
        version: task.version.clone(),
        enabled: task.enabled,
        registry_source: false, // Default value, could be inferred from metadata
        available_versions: vec![task.version], // Default, could expand based on registry
        created_at: task.created_at,
        updated_at: task.updated_at,
        validated_at: task.validated_at,
        in_sync: true, // Default value
        input_schema: Some(task.input_schema),
        output_schema: Some(task.output_schema),
        metadata: Some(task.metadata),
    }
}

// Similar conversion functions would be needed for Execution, Job, Schedule...
// Simplified implementations for now to avoid compilation errors

fn convert_unified_execution_to_storage(_execution: UnifiedExecution) -> crate::seaorm::entities::executions::Model {
    todo!("Implement execution conversion")
}

fn convert_storage_execution_to_unified(_execution: crate::seaorm::entities::executions::Model) -> UnifiedExecution {
    todo!("Implement execution conversion")
}

fn convert_unified_job_to_storage(_job: UnifiedJob) -> crate::seaorm::entities::jobs::Model {
    todo!("Implement job conversion")
}

fn convert_storage_job_to_unified(_job: crate::seaorm::entities::jobs::Model) -> UnifiedJob {
    todo!("Implement job conversion")
}

fn convert_unified_schedule_to_storage(schedule: UnifiedSchedule) -> crate::seaorm::entities::schedules::Model {
    use crate::seaorm::entities::schedules::Model;
    use uuid::Uuid;
    
    Model {
        id: schedule.id.as_i32().unwrap_or(0), // For creation, this will be set by the database
        uuid: Uuid::new_v4(), // Generate a new UUID for storage
        task_id: schedule.task_id.as_i32().unwrap_or(0),
        name: schedule.name,
        cron_expression: schedule.cron_expression,
        input_data: serde_json::Value::Object(serde_json::Map::new()).into(), // Default empty object
        enabled: schedule.enabled,
        next_run_at: schedule.next_run,
        last_run_at: schedule.last_run,
        execution_count: 0, // Start with 0 executions
        max_executions: None, // Unlimited by default
        metadata: None, // No metadata by default
        output_destinations: None, // No output destinations by default
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    }
}

fn convert_storage_schedule_to_unified(schedule: crate::seaorm::entities::schedules::Model) -> UnifiedSchedule {
    UnifiedSchedule {
        id: ApiId::new(schedule.id),
        task_id: ApiId::new(schedule.task_id),
        name: schedule.name,
        description: None, // This field doesn't exist in storage model, could be extracted from metadata
        cron_expression: schedule.cron_expression,
        enabled: schedule.enabled,
        next_run: schedule.next_run_at,
        last_run: schedule.last_run_at,
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    }
}

fn convert_task_filters_to_storage(filters: TaskFilters) -> crate::seaorm::repositories::task_repository::TaskFilters {
    crate::seaorm::repositories::task_repository::TaskFilters {
        name: filters.name,
        enabled: filters.enabled,
        has_validation: None, // Map from validated_after if needed
        version: None, // Could be extended from interface if needed
    }
}

fn convert_execution_filters_to_storage(_filters: ExecutionFilters) -> crate::seaorm::repositories::execution_repository::ExecutionFilters {
    todo!("Implement filter conversion")
}

fn convert_job_filters_to_storage(_filters: JobFilters) -> crate::seaorm::repositories::job_repository::JobFilters {
    todo!("Implement filter conversion")
}

fn convert_schedule_filters_to_storage(filters: ScheduleFilters) -> crate::seaorm::repositories::schedule_repository::ScheduleFilters {
    crate::seaorm::repositories::schedule_repository::ScheduleFilters {
        name: filters.name,
        enabled: filters.enabled,
        task_id: filters.task_id.map(|id| id.as_i32().unwrap_or(0)),
    }
}

fn convert_pagination_to_storage(pagination: PaginationInput) -> crate::seaorm::repositories::task_repository::Pagination {
    crate::seaorm::repositories::task_repository::Pagination {
        limit: pagination.limit.map(|l| l as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by: None, // TODO: Add sorting support - needs separate parameter
        order_desc: None,
    }
}

fn convert_list_input_to_storage_pagination(list_input: ratchet_api_types::pagination::ListInput) -> crate::seaorm::repositories::task_repository::Pagination {
    let pagination = list_input.get_pagination();
    let sort = list_input.sort;
    
    let (order_by, order_desc) = if let Some(sort_input) = sort {
        let order_desc = match sort_input.direction {
            Some(ratchet_api_types::pagination::SortDirection::Desc) => Some(true),
            _ => Some(false),
        };
        (Some(sort_input.field), order_desc)
    } else {
        (None, None)
    };
    
    crate::seaorm::repositories::task_repository::Pagination {
        limit: pagination.limit.map(|l| l as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by,
        order_desc,
    }
}

fn convert_list_input_to_execution_pagination(list_input: ratchet_api_types::pagination::ListInput) -> crate::seaorm::repositories::execution_repository::ExecutionPagination {
    let pagination = list_input.get_pagination();
    let sort = list_input.sort;
    
    let (order_by, order_desc) = if let Some(sort_input) = sort {
        use crate::seaorm::repositories::execution_repository::executions;
        let order_desc = match sort_input.direction {
            Some(ratchet_api_types::pagination::SortDirection::Desc) => Some(true),
            _ => Some(false),
        };
        let order_by = match sort_input.field.as_str() {
            "id" => Some(executions::Column::Id),
            "created_at" => Some(executions::Column::CreatedAt),
            "updated_at" => Some(executions::Column::UpdatedAt),
            "queued_at" => Some(executions::Column::QueuedAt),
            "started_at" => Some(executions::Column::StartedAt),
            "completed_at" => Some(executions::Column::CompletedAt),
            _ => Some(executions::Column::Id), // Default fallback
        };
        (order_by, order_desc)
    } else {
        (None, None)
    };
    
    crate::seaorm::repositories::execution_repository::ExecutionPagination {
        limit: pagination.limit.map(|l| l as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by,
        order_desc,
    }
}

fn convert_list_input_to_job_pagination(list_input: ratchet_api_types::pagination::ListInput) -> crate::seaorm::repositories::job_repository::JobPagination {
    let pagination = list_input.get_pagination();
    let sort = list_input.sort;
    
    let (order_by, order_desc) = if let Some(sort_input) = sort {
        use crate::seaorm::repositories::job_repository::jobs;
        let order_desc = match sort_input.direction {
            Some(ratchet_api_types::pagination::SortDirection::Desc) => Some(true),
            _ => Some(false),
        };
        let order_by = match sort_input.field.as_str() {
            "id" => Some(jobs::Column::Id),
            "created_at" => Some(jobs::Column::CreatedAt),
            "updated_at" => Some(jobs::Column::UpdatedAt),
            "queued_at" => Some(jobs::Column::QueuedAt),
            "scheduled_for" => Some(jobs::Column::ScheduledFor),
            "priority" => Some(jobs::Column::Priority),
            _ => Some(jobs::Column::Id), // Default fallback
        };
        (order_by, order_desc)
    } else {
        (None, None)
    };
    
    crate::seaorm::repositories::job_repository::JobPagination {
        limit: pagination.limit.map(|l| l as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by,
        order_desc,
    }
}

// Schedule repository uses task pagination for now since it doesn't have its own pagination type
fn convert_list_input_to_schedule_pagination(list_input: ratchet_api_types::pagination::ListInput) -> crate::seaorm::repositories::task_repository::Pagination {
    convert_list_input_to_storage_pagination(list_input)
}

fn convert_job_status_to_storage(_status: JobStatus) -> crate::seaorm::entities::jobs::JobStatus {
    todo!("Implement job status conversion")
}