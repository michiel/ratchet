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
use ratchet_storage::seaorm::repositories::{
    Repository as StorageRepository, 
    task_repository, execution_repository, job_repository, schedule_repository
};
use ratchet_storage::seaorm::entities;
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

/// Bridge execution repository
pub struct BridgeExecutionRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::ExecutionRepository>,
}

impl BridgeExecutionRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::ExecutionRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeExecutionRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.storage_repo.health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
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
        filters: ExecutionFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedExecution>, DatabaseError> {
        let storage_filters = convert_execution_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.storage_repo.find_with_filters(storage_filters, storage_pagination).await {
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
    
    async fn count_with_filters(&self, filters: ExecutionFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_execution_filters_to_storage(filters);
        
        match self.storage_repo.count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
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
    
    async fn update_status(&self, id: ApiId, status: ExecutionStatus) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let storage_status = convert_execution_status_to_storage(status);
        match self.storage_repo.update_status(i32_id, storage_status).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update_status(&self, id: ApiId, status: ExecutionStatus) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let storage_status = convert_execution_status_to_storage(status);
        match self.storage_repo.update_status(i32_id, storage_status).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

/// Bridge job repository
pub struct BridgeJobRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::JobRepository>,
}

impl BridgeJobRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::JobRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeJobRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.storage_repo.health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
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
    
    async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedJob>, DatabaseError> {
        match self.storage_repo.find_by_uuid(uuid).await {
            Ok(Some(job)) => Ok(Some(convert_storage_job_to_unified(job))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
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
        filters: JobFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedJob>, DatabaseError> {
        let storage_filters = convert_job_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.storage_repo.find_with_filters(storage_filters, storage_pagination).await {
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
    
    async fn count_with_filters(&self, filters: JobFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_job_filters_to_storage(filters);
        
        match self.storage_repo.count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl JobRepository for BridgeJobRepository {
    async fn find_queued(&self) -> Result<Vec<UnifiedJob>, DatabaseError> {
        match self.storage_repo.find_queued().await {
            Ok(jobs) => Ok(jobs.into_iter().map(convert_storage_job_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_by_priority(&self, priority: JobPriority) -> Result<Vec<UnifiedJob>, DatabaseError> {
        let storage_priority = convert_job_priority_to_storage(priority);
        match self.storage_repo.find_by_priority(storage_priority).await {
            Ok(jobs) => Ok(jobs.into_iter().map(convert_storage_job_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn update_status(&self, id: ApiId, status: JobStatus) -> Result<(), DatabaseError> {
        let i32_id = id.as_i32().unwrap_or(0);
        let storage_status = convert_job_status_to_storage(status);
        match self.storage_repo.update_status(i32_id, storage_status).await {
            Ok(()) => Ok(()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

/// Bridge schedule repository
pub struct BridgeScheduleRepository {
    storage_repo: Arc<ratchet_storage::seaorm::repositories::ScheduleRepository>,
}

impl BridgeScheduleRepository {
    pub fn new(storage_repo: Arc<ratchet_storage::seaorm::repositories::ScheduleRepository>) -> Self {
        Self { storage_repo }
    }
}

#[async_trait]
impl Repository for BridgeScheduleRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        self.storage_repo.health_check_send().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
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
    
    async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_by_uuid(uuid).await {
            Ok(Some(schedule)) => Ok(Some(convert_storage_schedule_to_unified(schedule))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_storage_error(e)),
        }
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
        filters: ScheduleFilters, 
        pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedSchedule>, DatabaseError> {
        let storage_filters = convert_schedule_filters_to_storage(filters);
        let storage_pagination = convert_pagination_to_storage(pagination.clone());
        
        match self.storage_repo.find_with_filters(storage_filters, storage_pagination).await {
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
    
    async fn count_with_filters(&self, filters: ScheduleFilters) -> Result<u64, DatabaseError> {
        let storage_filters = convert_schedule_filters_to_storage(filters);
        
        match self.storage_repo.count_with_filters(storage_filters).await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
}

#[async_trait]
impl ScheduleRepository for BridgeScheduleRepository {
    async fn find_active(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_active().await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
    }
    
    async fn find_due(&self, before: DateTime<Utc>) -> Result<Vec<UnifiedSchedule>, DatabaseError> {
        match self.storage_repo.find_due(before).await {
            Ok(schedules) => Ok(schedules.into_iter().map(convert_storage_schedule_to_unified).collect()),
            Err(e) => Err(convert_storage_error(e)),
        }
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
        input: execution.input.unwrap_or_default(),
        output: execution.output,
        error_message: execution.error_message,
        error_details: None, // Not available in unified execution
        queued_at: execution.created_at, // Use created_at as queued_at
        started_at: execution.started_at,
        completed_at: execution.completed_at,
        duration_ms: None, // Not available in unified execution
        http_requests: None, // Not available in unified execution
        recording_path: None, // Not available in unified execution
    }
}

/// Convert storage execution to unified execution
fn convert_storage_execution_to_unified(execution: ratchet_storage::seaorm::entities::executions::Model) -> UnifiedExecution {
    UnifiedExecution {
        id: ApiId::from_i32(execution.id),
        uuid: execution.uuid,
        task_id: ApiId::from_i32(execution.task_id),
        status: convert_storage_execution_status_to_unified(execution.status),
        input: Some(execution.input),
        output: execution.output,
        error_message: execution.error_message,
        created_at: execution.queued_at, // Use queued_at as created_at
        updated_at: execution.completed_at.unwrap_or(execution.queued_at), // Use completed_at or queued_at
        started_at: execution.started_at,
        completed_at: execution.completed_at,
    }
}

/// Convert unified job to storage job
fn convert_unified_job_to_storage(job: UnifiedJob) -> ratchet_storage::seaorm::entities::Job {
    ratchet_storage::seaorm::entities::Job {
        id: job.id.as_i32().unwrap_or(0),
        uuid: job.uuid,
        task_id: job.task_id.as_i32().unwrap_or(0),
        status: convert_job_status_to_storage(job.status),
        priority: convert_job_priority_to_storage(job.priority),
        input: job.input.unwrap_or_default(),
        output: job.output,
        error_message: job.error_message,
        max_retries: job.max_retries.unwrap_or(0),
        retry_count: job.retry_count.unwrap_or(0),
        created_at: job.created_at,
        updated_at: job.updated_at,
        scheduled_at: job.scheduled_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
    }
}

/// Convert storage job to unified job
fn convert_storage_job_to_unified(job: ratchet_storage::seaorm::entities::Job) -> UnifiedJob {
    UnifiedJob {
        id: ApiId::from_i32(job.id),
        uuid: job.uuid,
        task_id: ApiId::from_i32(job.task_id),
        status: convert_storage_job_status_to_unified(job.status),
        priority: convert_storage_job_priority_to_unified(job.priority),
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

/// Convert unified schedule to storage schedule
fn convert_unified_schedule_to_storage(schedule: UnifiedSchedule) -> ratchet_storage::seaorm::entities::Schedule {
    ratchet_storage::seaorm::entities::Schedule {
        id: schedule.id.as_i32().unwrap_or(0),
        uuid: schedule.uuid,
        task_id: schedule.task_id.as_i32().unwrap_or(0),
        name: schedule.name,
        cron_expression: schedule.cron_expression,
        enabled: schedule.enabled,
        input: schedule.input.unwrap_or_default(),
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
        last_run_at: schedule.last_run_at,
        next_run_at: schedule.next_run_at,
    }
}

/// Convert storage schedule to unified schedule
fn convert_storage_schedule_to_unified(schedule: ratchet_storage::seaorm::entities::Schedule) -> UnifiedSchedule {
    UnifiedSchedule {
        id: ApiId::from_i32(schedule.id),
        uuid: schedule.uuid,
        task_id: ApiId::from_i32(schedule.task_id),
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
        JobPriority::Critical => ratchet_storage::seaorm::entities::JobPriority::High, // Map to closest equivalent
    }
}

/// Convert storage job priority to unified job priority
fn convert_storage_job_priority_to_unified(priority: ratchet_storage::seaorm::entities::JobPriority) -> JobPriority {
    match priority {
        ratchet_storage::seaorm::entities::JobPriority::Low => JobPriority::Low,
        ratchet_storage::seaorm::entities::JobPriority::Normal => JobPriority::Normal,
        ratchet_storage::seaorm::entities::JobPriority::High => JobPriority::High,
    }
}

// Filter conversion functions

/// Convert interface ExecutionFilters to storage ExecutionFilters
fn convert_execution_filters_to_storage(filters: ExecutionFilters) -> ratchet_storage::seaorm::repositories::execution_repository::ExecutionFilters {
    ratchet_storage::seaorm::repositories::execution_repository::ExecutionFilters {
        task_id: filters.task_id.map(|id| id.as_i32().unwrap_or(0)),
        status: filters.status.map(convert_execution_status_to_storage),
        queued_after: filters.queued_after,
        completed_after: filters.completed_after,
    }
}

/// Convert interface JobFilters to storage JobFilters
fn convert_job_filters_to_storage(filters: JobFilters) -> ratchet_storage::seaorm::repositories::job_repository::JobFilters {
    ratchet_storage::seaorm::repositories::job_repository::JobFilters {
        task_id: filters.task_id.map(|id| id.as_i32().unwrap_or(0)),
        status: filters.status.map(convert_job_status_to_storage),
        priority: filters.priority.map(convert_job_priority_to_storage),
        queued_after: filters.queued_after,
        scheduled_before: filters.scheduled_before,
    }
}

/// Convert interface ScheduleFilters to storage ScheduleFilters
fn convert_schedule_filters_to_storage(filters: ScheduleFilters) -> ratchet_storage::seaorm::repositories::schedule_repository::ScheduleFilters {
    ratchet_storage::seaorm::repositories::schedule_repository::ScheduleFilters {
        task_id: filters.task_id.map(|id| id.as_i32().unwrap_or(0)),
        enabled: filters.enabled,
        next_run_before: filters.next_run_before,
    }
}