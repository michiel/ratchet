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
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus, JobPriority
};
use ratchet_lib;

/// Bridge factory that wraps the legacy RepositoryFactory
pub struct BridgeRepositoryFactory {
    legacy_factory: Arc<ratchet_lib::database::repositories::RepositoryFactory>,
    task_repository: BridgeTaskRepository,
}

impl BridgeRepositoryFactory {
    pub fn new(legacy_factory: Arc<ratchet_lib::database::repositories::RepositoryFactory>) -> Self {
        let task_repository = BridgeTaskRepository::new(Arc::new(legacy_factory.task_repository()));
        Self { 
            legacy_factory,
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
        // Delegate to legacy database ping
        self.legacy_factory.database().ping().await
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

/// Bridge task repository
pub struct BridgeTaskRepository {
    legacy_repo: Arc<ratchet_lib::database::repositories::TaskRepository>,
}

impl BridgeTaskRepository {
    pub fn new(legacy_repo: Arc<ratchet_lib::database::repositories::TaskRepository>) -> Self {
        Self { legacy_repo }
    }
}

#[async_trait]
impl Repository for BridgeTaskRepository {
    async fn health_check(&self) -> Result<(), DatabaseError> {
        // Use count() instead of health_check to avoid Send issues
        self.legacy_repo.count().await
            .map(|_| ())
            .map_err(|e| DatabaseError::Internal { message: e.to_string() })
    }
}

#[async_trait]
impl ratchet_interfaces::CrudRepository<UnifiedTask> for BridgeTaskRepository {
    async fn create(&self, _entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        // For now, return an error as task creation through bridge is not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task creation through bridge not yet implemented".to_string() 
        })
    }
    
    async fn find_by_id(&self, id: i32) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.legacy_repo.find_by_id(id).await {
            Ok(Some(task)) => Ok(Some(convert_legacy_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_legacy_error(e)),
        }
    }
    
    async fn find_by_uuid(&self, uuid: uuid::Uuid) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.legacy_repo.find_by_uuid(uuid).await {
            Ok(Some(task)) => Ok(Some(convert_legacy_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_legacy_error(e)),
        }
    }
    
    async fn update(&self, _entity: UnifiedTask) -> Result<UnifiedTask, DatabaseError> {
        // For now, return an error as task update through bridge is not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task update through bridge not yet implemented".to_string() 
        })
    }
    
    async fn delete(&self, _id: i32) -> Result<(), DatabaseError> {
        // For now, return an error as task deletion through bridge is not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task deletion through bridge not yet implemented".to_string() 
        })
    }
    
    async fn count(&self) -> Result<u64, DatabaseError> {
        match self.legacy_repo.count().await {
            Ok(count) => Ok(count),
            Err(e) => Err(convert_legacy_error(e)),
        }
    }
}

#[async_trait]
impl ratchet_interfaces::FilteredRepository<UnifiedTask, TaskFilters> for BridgeTaskRepository {
    async fn find_with_filters(
        &self, 
        _filters: TaskFilters, 
        _pagination: PaginationInput
    ) -> Result<ListResponse<UnifiedTask>, DatabaseError> {
        // For now, return an error as filtered queries through bridge are not yet implemented
        Err(DatabaseError::Internal { 
            message: "Filtered task queries through bridge not yet implemented".to_string() 
        })
    }
    
    async fn count_with_filters(&self, _filters: TaskFilters) -> Result<u64, DatabaseError> {
        // For now, return an error as filtered count through bridge is not yet implemented
        Err(DatabaseError::Internal { 
            message: "Filtered task count through bridge not yet implemented".to_string() 
        })
    }
}

#[async_trait]
impl TaskRepository for BridgeTaskRepository {
    async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError> {
        match self.legacy_repo.find_enabled().await {
            Ok(tasks) => Ok(tasks.into_iter().map(convert_legacy_task_to_unified).collect()),
            Err(e) => Err(convert_legacy_error(e)),
        }
    }
    
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError> {
        match self.legacy_repo.find_by_name(name).await {
            Ok(Some(task)) => Ok(Some(convert_legacy_task_to_unified(task))),
            Ok(None) => Ok(None),
            Err(e) => Err(convert_legacy_error(e)),
        }
    }
    
    async fn mark_validated(&self, _id: ApiId) -> Result<(), DatabaseError> {
        // For now, return an error as task validation updates through bridge are not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task validation updates through bridge not yet implemented".to_string() 
        })
    }
    
    async fn set_enabled(&self, _id: ApiId, _enabled: bool) -> Result<(), DatabaseError> {
        // For now, return an error as task enabled updates through bridge are not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task enabled updates through bridge not yet implemented".to_string() 
        })
    }
    
    async fn set_in_sync(&self, _id: ApiId, _in_sync: bool) -> Result<(), DatabaseError> {
        // For now, return an error as task sync updates through bridge are not yet implemented
        Err(DatabaseError::Internal { 
            message: "Task sync updates through bridge not yet implemented".to_string() 
        })
    }
}

// Similar bridge implementations would be created for:
// - BridgeExecutionRepository
// - BridgeJobRepository  
// - BridgeScheduleRepository

/// Helper function to convert ratchet-lib errors to DatabaseError
fn convert_legacy_error(err: impl std::error::Error) -> DatabaseError {
    DatabaseError::Internal { message: err.to_string() }
}

/// Helper function to convert legacy entities to unified types
fn convert_legacy_task_to_unified(task: ratchet_lib::database::entities::tasks::Model) -> UnifiedTask {
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

/// Helper function to convert unified types to legacy entities
fn convert_unified_task_to_legacy(task: UnifiedTask) -> ratchet_lib::database::entities::tasks::ActiveModel {
    use ratchet_lib::database::entities::tasks::ActiveModel;
    use sea_orm::Set;
    
    ActiveModel {
        id: Set(task.id.as_i32().unwrap_or(0)),
        uuid: Set(task.uuid),
        name: Set(task.name),
        description: Set(task.description),
        version: Set(task.version),
        path: Set(String::new()), // Would need to be provided or inferred
        metadata: Set(task.metadata.unwrap_or_default()),
        input_schema: Set(task.input_schema.unwrap_or_default()),
        output_schema: Set(task.output_schema.unwrap_or_default()),
        enabled: Set(task.enabled),
        created_at: Set(task.created_at),
        updated_at: Set(task.updated_at),
        validated_at: Set(task.validated_at),
    }
}

// TODO: Implement filter and pagination conversion functions
// These would need to properly map between the interface types and legacy types