//! Bridge implementations from legacy ratchet-lib to new interfaces
//!
//! This module provides adapters that wrap the existing ratchet-lib repositories
//! to satisfy the new interface traits, enabling smooth migration from monolithic
//! to modular architecture.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};

use ratchet_interfaces::{
    database::{
        RepositoryFactory, TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository,
        TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
        DatabaseError, Repository, CrudRepository, FilteredRepository
    }
};
use ratchet_storage::seaorm::repositories::{
    TaskRepository as StorageTaskRepository,
    ExecutionRepository as StorageExecutionRepository, 
    JobRepository as StorageJobRepository,
    ScheduleRepository as StorageScheduleRepository
};
use ratchet_storage::seaorm::entities;
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus, JobPriority
};
// ratchet_lib removed - using modern modular components

/// Bridge factory that wraps the ratchet-storage RepositoryFactory
pub struct BridgeRepositoryFactory {
    storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>,
    task_repository: BridgeTaskRepository,
    execution_repository: BridgeExecutionRepository,
    job_repository: BridgeJobRepository,
    schedule_repository: BridgeScheduleRepository,
}

impl BridgeRepositoryFactory {
    pub fn new(storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>) -> Self {
        let task_repository = BridgeTaskRepository::new(Arc::new(storage_factory.task_repository()));
        let execution_repository = BridgeExecutionRepository::new(Arc::new(storage_factory.execution_repository()));
        let job_repository = BridgeJobRepository::new(Arc::new(storage_factory.job_repository()));
        let schedule_repository = BridgeScheduleRepository::new(Arc::new(storage_factory.schedule_repository()));
        
        Self { 
            storage_factory,
            task_repository,
            execution_repository,
            job_repository,
            schedule_repository,
        }
    }
}

#[async_trait]
impl RepositoryFactory for BridgeRepositoryFactory {
    fn task_repository(&self) -> &dyn TaskRepository {
        &self.task_repository
    }
    
    fn execution_repository(&self) -> &dyn ExecutionRepository {
        &self.execution_repository
    }
    
    fn job_repository(&self) -> &dyn JobRepository {
        &self.job_repository
    }
    
    fn schedule_repository(&self) -> &dyn ScheduleRepository {
        &self.schedule_repository
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Delegate to storage health check via task repository
        self.storage_factory.task_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

/// Bridge task repository
pub struct BridgeTaskRepository {
    storage_repo: Arc<StorageTaskRepository>,
}

impl BridgeTaskRepository {
    pub fn new(storage_repo: Arc<StorageTaskRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeTaskRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Use ratchet-storage health check
        self.storage_repo.health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedTask> for BridgeTaskRepository {
    async fn create(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        // Convert unified task to storage task
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.storage_repo.create(storage_task).await {
            Ok(created_task) => Ok(convert_storage_task_to_unified(created_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_uuid(uuid).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        // Convert unified task to storage task
        let storage_task = convert_unified_task_to_storage(entity);
        
        match self.storage_repo.update(storage_task).await {
            Ok(updated_task) => Ok(convert_storage_task_to_unified(updated_task)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.storage_repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.storage_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedTask, TaskFilters> for BridgeTaskRepository {
    async fn find_with_filters(
        &self, 
        filters: TaskFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // Convert interface filters to storage filters
        let storage_filters = convert_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.storage_repo.find_with_filters(storage_filters, storage_pagination).await {
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
    
    async fn count_with_filters(&self, filters: TaskFilters) -> Result<u64, DatabaseError> {
        // Convert interface filters to storage filters
        let storage_filters = convert_filters_to_storage(filters);
        
        match self.storage_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl TaskRepository for BridgeTaskRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_enabled().await {
            Ok(tasks) => Ok(tasks.into_iter().map(convert_storage_task_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.storage_repo.find_by_name(name).await {
            Ok(Some(task)) => Ok(Some(convert_storage_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_validated(&self, id: ApiId) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.storage_repo.mark_validated(i32_id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        match self.storage_repo.set_enabled(i32_id, enabled).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn set_in_sync(&self, _id: ApiId, _in_sync: bool) -> Result<(), DatabaseError> {
        // For now, return an error as task sync updates are not implemented in ratchet-storage yet
        Err(DatabaseError::Internal { 
            message: "Task sync updates not yet implemented in ratchet-storage".to_string() 
        })
    }
}

/// Bridge execution repository
pub struct BridgeExecutionRepository {
    storage_repo: Arc<StorageExecutionRepository>,
}

impl BridgeExecutionRepository {
    pub fn new(storage_repo: Arc<StorageExecutionRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeExecutionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simplified health check - just return OK for now
        Ok(())
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedExecution> for BridgeExecutionRepository {
    async fn create(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        let storage_execution = convert_unified_execution_to_storage(entity);
        
        match self.storage_repo.create(storage_execution).await {
            Ok(created_execution) => Ok(convert_storage_execution_to_unified(created_execution)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedExecution>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(execution)) => Ok(Some(convert_storage_execution_to_unified(execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedExecution>, DatabaseError> {
        match self.storage_repo.find_by_uuid(uuid).await {
            Ok(Some(execution)) => Ok(Some(convert_storage_execution_to_unified(execution))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update(&self, entity: UnifiedExecution) -> Result<UnifiedExecution, DatabaseError> {
        let storage_execution = convert_unified_execution_to_storage(entity);
        
        match self.storage_repo.update(storage_execution).await {
            Ok(updated_execution) => Ok(convert_storage_execution_to_unified(updated_execution)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.storage_repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.storage_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedExecution, ExecutionFilters> for BridgeExecutionRepository {
    async fn find_with_filters(
        &self, 
        _filters: ExecutionFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        // Stub implementation - return empty list for now
        Ok(ListResponse {
            items: Vec::new(),
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
    }
    
    async fn count_with_filters(&self, _filters: ExecutionFilters) -> Result<u64, DatabaseError> {
        // Stub implementation - return 0 for now
        Ok(0)
    }
}

#[async_trait]
impl ExecutionRepository for BridgeExecutionRepository {
    async fn find_by_task_id(&self, task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        let i32_id = task_id.as_i32().unwrap_or(0);
        match self.storage_repo.find_by_task_id(i32_id).await {
            Ok(executions) => Ok(executions.into_iter().map(convert_storage_execution_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_status(&self, status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError> {
        // For now, return empty list - would need to implement in storage layer
        Ok(Vec::new())
    }
    
    async fn update_status(&self, id: ApiId, status: ExecutionStatus) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let storage_status = convert_execution_status_to_storage(status);
        match self.storage_repo.update_status(i32_id, storage_status).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn mark_started(&self, id: ApiId) -> Result<(), DatabaseError> {
        self.update_status(id, ExecutionStatus::Running).await
    }
    
    async fn mark_completed(
        &self, 
        id: ApiId, 
        output: serde_json::Value,
        duration_ms: Option<i32>
    ) -> Result<(), DatabaseError> {
        // For now, just update status - would need more sophisticated implementation
        self.update_status(id, ExecutionStatus::Completed).await
    }
    
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error_message: String,
        error_details: Option<serde_json::Value>
    ) -> Result<(), DatabaseError> {
        // For now, just update status - would need more sophisticated implementation  
        self.update_status(id, ExecutionStatus::Failed).await
    }
    
    async fn mark_cancelled(&self, id: ApiId) -> Result<(), DatabaseError> {
        self.update_status(id, ExecutionStatus::Cancelled).await
    }
    
    async fn update_progress(&self, _id: ApiId, _progress: f32) -> Result<(), DatabaseError> {
        // Stub implementation - progress tracking not yet implemented in storage
        Ok(())
    }
}

/// Bridge job repository
pub struct BridgeJobRepository {
    storage_repo: Arc<StorageJobRepository>,
}

impl BridgeJobRepository {
    pub fn new(storage_repo: Arc<StorageJobRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeJobRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simplified health check - just return OK for now
        Ok(())
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedJob> for BridgeJobRepository {
    async fn create(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        
        match self.storage_repo.create(storage_job).await {
            Ok(created_job) => Ok(convert_storage_job_to_unified(created_job)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedJob>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(job)) => Ok(Some(convert_storage_job_to_unified(job))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, _uuid: uuid::Uuid) -> Result<Option<UnifiedJob>, DatabaseError> {
        // Stub implementation - storage layer doesn't have find_by_uuid yet
        Ok(None)
    }
    
    async fn update(&self, entity: UnifiedJob) -> Result<UnifiedJob, DatabaseError> {
        let storage_job = convert_unified_job_to_storage(entity);
        
        match self.storage_repo.update(storage_job).await {
            Ok(updated_job) => Ok(convert_storage_job_to_unified(updated_job)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.storage_repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.storage_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedJob, JobFilters> for BridgeJobRepository {
    async fn find_with_filters(
        &self, 
        _filters: JobFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        // Stub implementation - return empty list for now
        Ok(ListResponse {
            items: Vec::new(),
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
    }
    
    async fn count_with_filters(&self, _filters: JobFilters) -> Result<u64, DatabaseError> {
        // Stub implementation - return 0 for now
        Ok(0)
    }
}

#[async_trait]
impl JobRepository for BridgeJobRepository {
    async fn find_ready_for_processing(&self, limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError> {
        // Stub implementation - would need to implement priority-based querying in storage
        Ok(Vec::new())
    }
    
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError> {
        // Stub implementation - would need status filtering in storage layer
        Ok(Vec::new())
    }
    
    async fn mark_processing(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError> {
        // Stub implementation - would need to link job to execution in storage
        Ok(())
    }
    
    async fn mark_completed(&self, id: ApiId) -> Result<(), DatabaseError> {
        // Stub implementation - would need job status updates in storage
        Ok(())
    }
    
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error: String, 
        details: Option<serde_json::Value>
    ) -> Result<bool, DatabaseError> {
        // Stub implementation - would need retry logic in storage
        Ok(false) // Returns false meaning no retry
    }
    
    async fn schedule_retry(&self, id: ApiId, retry_at: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError> {
        // Stub implementation - would need retry scheduling in storage
        Ok(())
    }
    
    async fn cancel(&self, id: ApiId) -> Result<(), DatabaseError> {
        // Stub implementation - would need job cancellation in storage
        Ok(())
    }
}

/// Bridge schedule repository
pub struct BridgeScheduleRepository {
    storage_repo: Arc<StorageScheduleRepository>,
}

impl BridgeScheduleRepository {
    pub fn new(storage_repo: Arc<StorageScheduleRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeScheduleRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Simplified health check - just return OK for now
        Ok(())
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedSchedule> for BridgeScheduleRepository {
    async fn create(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.storage_repo.create(storage_schedule).await {
            Ok(created_schedule) => Ok(convert_storage_schedule_to_unified(created_schedule)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_by_id(id).await {
            Ok(Some(schedule)) => Ok(Some(convert_storage_schedule_to_unified(schedule))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, _uuid: uuid::Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        // Stub implementation - storage layer doesn't have find_by_uuid yet
        Ok(None)
    }
    
    async fn update(&self, entity: UnifiedSchedule) -> Result<UnifiedSchedule, DatabaseError> {
        let storage_schedule = convert_unified_schedule_to_storage(entity);
        
        match self.storage_repo.update(storage_schedule).await {
            Ok(updated_schedule) => Ok(convert_storage_schedule_to_unified(updated_schedule)),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn delete(&self, id: i32) -> Result<(), DatabaseError> {
        match self.storage_repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.storage_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedSchedule, ScheduleFilters> for BridgeScheduleRepository {
    async fn find_with_filters(
        &self, 
        _filters: ScheduleFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        
        // Stub implementation - return empty list for now
        Ok(ListResponse {
            items: Vec::new(),
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
    }
    
    async fn count_with_filters(&self, _filters: ScheduleFilters) -> Result<u64, DatabaseError> {
        // Stub implementation - return 0 for now
        Ok(0)
    }
}

#[async_trait]
impl ScheduleRepository for BridgeScheduleRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        // Stub implementation - would need enabled filtering in storage
        Ok(Vec::new())
    }
    
    async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        // Stub implementation - would need time-based filtering in storage
        Ok(Vec::new())
    }
    
    async fn record_execution(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError> {
        // Stub implementation - would need execution tracking in storage
        Ok(())
    }
    
    async fn update_next_run(&self, id: ApiId, next_run: chrono::DateTime<chrono::Utc>) -> Result<(), DatabaseError> {
        // Stub implementation - would need next run updates in storage
        Ok(())
    }
    
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError> {
        // Stub implementation - would need enabled updates in storage
        Ok(())
    }
}


/// Helper function to convert ratchet-storage errors to DatabaseError
fn convert_storage_error(err: ratchet_storage::seaorm::connection::DatabaseError) -> DatabaseError {
    DatabaseError::Internal { message: err.to_string() }
}

/// Helper function to convert storage entities to unified types
fn convert_storage_task_to_unified(task: ratchet_storage::seaorm::entities::Task) -> UnifiedTask {
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

/// Helper function to convert unified types to storage entities
fn convert_unified_task_to_storage(task: UnifiedTask) -> ratchet_storage::seaorm::entities::Task {
    ratchet_storage::seaorm::entities::Task {
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

/// Convert interface TaskFilters to storage TaskFilters
fn convert_filters_to_storage(filters: TaskFilters) -> ratchet_storage::seaorm::repositories::task_repository::TaskFilters {
    ratchet_storage::seaorm::repositories::task_repository::TaskFilters {
        name: filters.name,
        enabled: filters.enabled,
        has_validation: filters.validated_after.map(|_| true), // Convert validated_after to has_validation
        version: None, // Not supported in current interface
    }
}

/// Convert interface PaginationInput to storage Pagination
fn convert_pagination_to_storage(pagination: PaginationInput) -> ratchet_storage::seaorm::repositories::task_repository::Pagination {
    ratchet_storage::seaorm::repositories::task_repository::Pagination {
        limit: Some(pagination.get_limit() as u64),
        offset: Some(pagination.get_offset() as u64),
        order_by: None,
        order_desc: None,
    }
}

// Additional conversion functions for other entity types

/// Convert unified execution to storage execution
fn convert_unified_execution_to_storage(execution: UnifiedExecution) -> ratchet_storage::seaorm::entities::executions::Model {
    ratchet_storage::seaorm::entities::executions::Model {
        id: execution.id.as_i32().unwrap_or(0),
        uuid: execution.uuid,
        task_id: execution.task_id.as_i32().unwrap_or(0),
        status: convert_execution_status_to_storage(execution.status),
        input: execution.input,
        output: execution.output,
        error_message: execution.error_message,
        error_details: execution.error_details,
        queued_at: execution.queued_at,
        started_at: execution.started_at,
        completed_at: execution.completed_at,
        duration_ms: execution.duration_ms,
        http_requests: execution.http_requests,
        recording_path: execution.recording_path,
    }
}

/// Convert storage execution to unified execution
fn convert_storage_execution_to_unified(execution: ratchet_storage::seaorm::entities::executions::Model) -> UnifiedExecution {
    UnifiedExecution {
        id: ApiId::from_i32(execution.id),
        uuid: execution.uuid,
        task_id: ApiId::from_i32(execution.task_id),
        input: execution.input,
        output: execution.output,
        status: convert_storage_execution_status_to_unified(execution.status),
        error_message: execution.error_message,
        error_details: execution.error_details,
        queued_at: execution.queued_at,
        started_at: execution.started_at,
        completed_at: execution.completed_at,
        duration_ms: execution.duration_ms,
        http_requests: execution.http_requests,
        recording_path: execution.recording_path,
        // Computed fields
        can_retry: execution.status == ratchet_storage::seaorm::entities::executions::ExecutionStatus::Failed,
        can_cancel: execution.status == ratchet_storage::seaorm::entities::executions::ExecutionStatus::Running,
        progress: None, // Default to None
    }
}

/// Convert unified job to storage job
fn convert_unified_job_to_storage(job: UnifiedJob) -> ratchet_storage::seaorm::entities::Job {
    ratchet_storage::seaorm::entities::Job {
        id: job.id.as_i32().unwrap_or(0),
        uuid: uuid::Uuid::new_v4(), // Generate new UUID since UnifiedJob doesn't have one
        task_id: job.task_id.as_i32().unwrap_or(0),
        execution_id: None, // Default to None
        schedule_id: None, // Default to None
        priority: convert_job_priority_to_storage(job.priority),
        status: convert_job_status_to_storage(job.status),
        input_data: serde_json::json!({}), // Default empty input data
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        retry_delay_seconds: 60, // Default delay
        error_message: job.error_message,
        error_details: None, // Default to None
        queued_at: job.queued_at,
        process_at: job.scheduled_for, // Map scheduled_for to process_at
        started_at: None, // Default to None
        completed_at: None, // Default to None
        metadata: None, // Default to None
        output_destinations: None, // Default to None
    }
}

/// Convert storage job to unified job
fn convert_storage_job_to_unified(job: ratchet_storage::seaorm::entities::Job) -> UnifiedJob {
    UnifiedJob {
        id: ApiId::from_i32(job.id),
        task_id: ApiId::from_i32(job.task_id),
        priority: convert_storage_job_priority_to_unified(job.priority),
        status: convert_storage_job_status_to_unified(job.status),
        retry_count: job.retry_count,
        max_retries: job.max_retries,
        queued_at: job.queued_at,
        scheduled_for: job.process_at, // Map process_at to scheduled_for
        error_message: job.error_message,
        output_destinations: None, // Default to None, could be parsed from metadata if needed
    }
}

/// Convert unified schedule to storage schedule
fn convert_unified_schedule_to_storage(schedule: UnifiedSchedule) -> ratchet_storage::seaorm::entities::Schedule {
    ratchet_storage::seaorm::entities::Schedule {
        id: schedule.id.as_i32().unwrap_or(0),
        uuid: uuid::Uuid::new_v4(), // Generate new UUID since UnifiedSchedule doesn't have one
        task_id: schedule.task_id.as_i32().unwrap_or(0),
        name: schedule.name,
        cron_expression: schedule.cron_expression,
        input_data: serde_json::json!({}), // Default empty input data
        enabled: schedule.enabled,
        next_run_at: schedule.next_run,
        last_run_at: schedule.last_run,
        execution_count: 0, // Default execution count
        max_executions: None, // Default no limit
        metadata: None, // Default no metadata
        output_destinations: None, // Default no output destinations
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    }
}

/// Convert storage schedule to unified schedule
fn convert_storage_schedule_to_unified(schedule: ratchet_storage::seaorm::entities::Schedule) -> UnifiedSchedule {
    UnifiedSchedule {
        id: ApiId::from_i32(schedule.id),
        task_id: ApiId::from_i32(schedule.task_id),
        name: schedule.name,
        description: None, // UnifiedSchedule has description field, but storage doesn't
        cron_expression: schedule.cron_expression,
        enabled: schedule.enabled,
        next_run: schedule.next_run_at,
        last_run: schedule.last_run_at,
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    }
}

// Status and enum conversion functions

/// Convert unified execution status to storage execution status
fn convert_execution_status_to_storage(status: ExecutionStatus) -> ratchet_storage::seaorm::entities::ExecutionStatus {
    match status {
        ExecutionStatus::Pending => ratchet_storage::seaorm::entities::executions::ExecutionStatus::Pending,
        ExecutionStatus::Running => ratchet_storage::seaorm::entities::executions::ExecutionStatus::Running,
        ExecutionStatus::Completed => ratchet_storage::seaorm::entities::executions::ExecutionStatus::Completed,
        ExecutionStatus::Failed => ratchet_storage::seaorm::entities::executions::ExecutionStatus::Failed,
        ExecutionStatus::Cancelled => ratchet_storage::seaorm::entities::executions::ExecutionStatus::Cancelled,
    }
}

/// Convert storage execution status to unified execution status
fn convert_storage_execution_status_to_unified(status: ratchet_storage::seaorm::entities::ExecutionStatus) -> ExecutionStatus {
    match status {
        ratchet_storage::seaorm::entities::executions::ExecutionStatus::Pending => ExecutionStatus::Pending,
        ratchet_storage::seaorm::entities::executions::ExecutionStatus::Running => ExecutionStatus::Running,
        ratchet_storage::seaorm::entities::executions::ExecutionStatus::Completed => ExecutionStatus::Completed,
        ratchet_storage::seaorm::entities::executions::ExecutionStatus::Failed => ExecutionStatus::Failed,
        ratchet_storage::seaorm::entities::executions::ExecutionStatus::Cancelled => ExecutionStatus::Cancelled,
    }
}

/// Convert unified job status to storage job status
fn convert_job_status_to_storage(status: JobStatus) -> ratchet_storage::seaorm::entities::JobStatus {
    match status {
        JobStatus::Queued => ratchet_storage::seaorm::entities::JobStatus::Queued,
        JobStatus::Processing => ratchet_storage::seaorm::entities::JobStatus::Processing,
        JobStatus::Completed => ratchet_storage::seaorm::entities::JobStatus::Completed,
        JobStatus::Failed => ratchet_storage::seaorm::entities::JobStatus::Failed,
        JobStatus::Cancelled => ratchet_storage::seaorm::entities::JobStatus::Cancelled,
        JobStatus::Retrying => ratchet_storage::seaorm::entities::JobStatus::Retrying,
    }
}

/// Convert storage job status to unified job status
fn convert_storage_job_status_to_unified(status: ratchet_storage::seaorm::entities::JobStatus) -> JobStatus {
    match status {
        ratchet_storage::seaorm::entities::JobStatus::Queued => JobStatus::Queued,
        ratchet_storage::seaorm::entities::JobStatus::Processing => JobStatus::Processing,
        ratchet_storage::seaorm::entities::JobStatus::Completed => JobStatus::Completed,
        ratchet_storage::seaorm::entities::JobStatus::Failed => JobStatus::Failed,
        ratchet_storage::seaorm::entities::JobStatus::Cancelled => JobStatus::Cancelled,
        ratchet_storage::seaorm::entities::JobStatus::Retrying => JobStatus::Retrying,
    }
}

/// Convert unified job priority to storage job priority
fn convert_job_priority_to_storage(priority: JobPriority) -> ratchet_storage::seaorm::entities::JobPriority {
    match priority {
        JobPriority::Low => ratchet_storage::seaorm::entities::JobPriority::Low,
        JobPriority::Normal => ratchet_storage::seaorm::entities::JobPriority::Normal,
        JobPriority::High => ratchet_storage::seaorm::entities::JobPriority::High,
        JobPriority::Critical => ratchet_storage::seaorm::entities::JobPriority::Urgent, // Map Critical to Urgent
    }
}

/// Convert storage job priority to unified job priority
fn convert_storage_job_priority_to_unified(priority: ratchet_storage::seaorm::entities::JobPriority) -> JobPriority {
    match priority {
        ratchet_storage::seaorm::entities::JobPriority::Low => JobPriority::Low,
        ratchet_storage::seaorm::entities::JobPriority::Normal => JobPriority::Normal,
        ratchet_storage::seaorm::entities::JobPriority::High => JobPriority::High,
        ratchet_storage::seaorm::entities::JobPriority::Urgent => JobPriority::Critical, // Map Urgent to Critical
    }
}

// Filter conversion functions

// Filter conversion functions removed - storage layer doesn't have these filter types yet