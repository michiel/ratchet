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
    RepositoryFactory, TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository,
    TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
    DatabaseError, Repository
};
use ratchet_storage::seaorm::repositories::{Repository as StorageRepository, task_repository};
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus, JobPriority
};
use ratchet_lib;

/// Bridge factory that wraps the ratchet-storage RepositoryFactory
pub struct BridgeRepositoryFactory {
    storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>,
    task_repository: BridgeTaskRepository,
}

impl BridgeRepositoryFactory {
    pub fn new(storage_factory: Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>) -> Self {
        let task_repository = BridgeTaskRepository::new(Arc::new(storage_factory.task_repository()));
        Self { 
            storage_factory,
            task_repository,
        }
    }
}

#[async_trait]
impl RepositoryFactory for BridgeRepositoryFactory {
    fn task_repository(&self) -> &dyn TaskRepository {
        &self.task_repository
    }
    
    fn execution_repository(&self) -> &dyn ExecutionRepository {
        // For now, panic to indicate this is not yet implemented
        // In a full implementation, we'd store all bridge repositories as fields
        panic!("Execution repository bridge not yet implemented")
    }
    
    fn job_repository(&self) -> &dyn JobRepository {
        // For now, panic to indicate this is not yet implemented
        panic!("Job repository bridge not yet implemented")
    }
    
    fn schedule_repository(&self) -> &dyn ScheduleRepository {
        // For now, panic to indicate this is not yet implemented
        panic!("Schedule repository bridge not yet implemented")
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Delegate to storage health check via task repository
        self.storage_factory.task_repository().health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

/// Bridge task repository
pub struct BridgeTaskRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::TaskRepository>,
}

impl BridgeTaskRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::TaskRepository>) -> Self {
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
        
        match self.storage_repo.count_with_filters(storage_filters).await {
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

// Similar bridge implementations would be created for:
// - BridgeExecutionRepository
// - BridgeJobRepository  
// - BridgeScheduleRepository

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