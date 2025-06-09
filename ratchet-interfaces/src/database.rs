//! Database repository interfaces
//!
//! This module defines the core repository traits that enable dependency injection
//! and testing through interface segregation. These traits break circular dependencies
//! by providing clean contracts that both legacy and new implementations can satisfy.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ExecutionStatus, JobStatus, JobPriority
};
// ApiResult not needed in trait definitions - using DatabaseError instead
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Common database error type
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Constraint violation: {message}")]
    Constraint { message: String },
    
    #[error("Connection error: {message}")]
    Connection { message: String },
    
    #[error("Transaction error: {message}")]
    Transaction { message: String },
    
    #[error("Internal database error: {message}")]
    Internal { message: String },
}

/// Base repository trait with health check capability
#[async_trait]
pub trait Repository: Send + Sync {
    /// Check if the repository is healthy and can serve requests
    async fn health_check(&self) -> Result<(), DatabaseError>;
}

/// Generic CRUD repository trait
#[async_trait]
pub trait CrudRepository<T>: Repository {
    /// Create a new entity
    async fn create(&self, entity: T) -> Result<T, DatabaseError>;
    
    /// Find entity by integer ID
    async fn find_by_id(&self, id: i32) -> Result<Option<T>, DatabaseError>;
    
    /// Find entity by UUID
    async fn find_by_uuid(&self, uuid: Uuid) -> Result<Option<T>, DatabaseError>;
    
    /// Update an existing entity
    async fn update(&self, entity: T) -> Result<T, DatabaseError>;
    
    /// Delete entity by ID
    async fn delete(&self, id: i32) -> Result<(), DatabaseError>;
    
    /// Get total count of entities
    async fn count(&self) -> Result<u64, DatabaseError>;
}

/// Repository trait for entities that support filtering and pagination
#[async_trait]
pub trait FilteredRepository<T, F>: CrudRepository<T> {
    /// Find entities with filters and pagination
    async fn find_with_filters(
        &self, 
        filters: F, 
        pagination: PaginationInput
    ) -> Result<ListResponse<T>, DatabaseError>;
    
    /// Count entities matching filters
    async fn count_with_filters(&self, filters: F) -> Result<u64, DatabaseError>;
}

// =============================================================================
// Task Repository
// =============================================================================

/// Filter criteria for task queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilters {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,
    pub validated_after: Option<DateTime<Utc>>,
}

/// Task repository interface
#[async_trait]
pub trait TaskRepository: FilteredRepository<UnifiedTask, TaskFilters> {
    /// Find all enabled tasks
    async fn find_enabled(&self) -> Result<Vec<UnifiedTask>, DatabaseError>;
    
    /// Find task by name
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, DatabaseError>;
    
    /// Mark a task as validated
    async fn mark_validated(&self, id: ApiId) -> Result<(), DatabaseError>;
    
    /// Set task enabled status
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError>;
    
    /// Update task sync status
    async fn set_in_sync(&self, id: ApiId, in_sync: bool) -> Result<(), DatabaseError>;
}

// =============================================================================
// Execution Repository  
// =============================================================================

/// Filter criteria for execution queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFilters {
    pub task_id: Option<ApiId>,
    pub status: Option<ExecutionStatus>,
    pub queued_after: Option<DateTime<Utc>>,
    pub completed_after: Option<DateTime<Utc>>,
}

/// Execution repository interface
#[async_trait]
pub trait ExecutionRepository: FilteredRepository<UnifiedExecution, ExecutionFilters> {
    /// Find executions by task ID
    async fn find_by_task_id(&self, task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError>;
    
    /// Find executions by status
    async fn find_by_status(&self, status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError>;
    
    /// Mark execution as started
    async fn mark_started(&self, id: ApiId) -> Result<(), DatabaseError>;
    
    /// Mark execution as completed with output
    async fn mark_completed(
        &self, 
        id: ApiId, 
        output: serde_json::Value,
        duration_ms: Option<i32>
    ) -> Result<(), DatabaseError>;
    
    /// Mark execution as failed with error details
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error_message: String,
        error_details: Option<serde_json::Value>
    ) -> Result<(), DatabaseError>;
    
    /// Mark execution as cancelled
    async fn mark_cancelled(&self, id: ApiId) -> Result<(), DatabaseError>;
    
    /// Update execution progress
    async fn update_progress(&self, id: ApiId, progress: f32) -> Result<(), DatabaseError>;
}

// =============================================================================
// Job Repository
// =============================================================================

/// Filter criteria for job queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFilters {
    pub task_id: Option<ApiId>,
    pub status: Option<JobStatus>,
    pub priority: Option<JobPriority>,
    pub queued_after: Option<DateTime<Utc>>,
    pub scheduled_before: Option<DateTime<Utc>>,
}

/// Job repository interface
#[async_trait]
pub trait JobRepository: FilteredRepository<UnifiedJob, JobFilters> {
    /// Find jobs ready for processing (sorted by priority and queue time)
    async fn find_ready_for_processing(&self, limit: u64) -> Result<Vec<UnifiedJob>, DatabaseError>;
    
    /// Find jobs by status
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<UnifiedJob>, DatabaseError>;
    
    /// Mark job as processing and link to execution
    async fn mark_processing(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Mark job as completed
    async fn mark_completed(&self, id: ApiId) -> Result<(), DatabaseError>;
    
    /// Mark job as failed and increment retry count
    async fn mark_failed(
        &self, 
        id: ApiId, 
        error: String, 
        details: Option<serde_json::Value>
    ) -> Result<bool, DatabaseError>; // Returns true if can retry
    
    /// Schedule job for retry
    async fn schedule_retry(&self, id: ApiId, retry_at: DateTime<Utc>) -> Result<(), DatabaseError>;
    
    /// Cancel job
    async fn cancel(&self, id: ApiId) -> Result<(), DatabaseError>;
}

// =============================================================================
// Schedule Repository
// =============================================================================

/// Filter criteria for schedule queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleFilters {
    pub task_id: Option<ApiId>,
    pub enabled: Option<bool>,
    pub next_run_before: Option<DateTime<Utc>>,
}

/// Schedule repository interface
#[async_trait]
pub trait ScheduleRepository: FilteredRepository<UnifiedSchedule, ScheduleFilters> {
    /// Find all enabled schedules
    async fn find_enabled(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError>;
    
    /// Find schedules ready to run (next_run <= now)
    async fn find_ready_to_run(&self) -> Result<Vec<UnifiedSchedule>, DatabaseError>;
    
    /// Record execution for a schedule
    async fn record_execution(&self, id: ApiId, execution_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Update next run time
    async fn update_next_run(&self, id: ApiId, next_run: DateTime<Utc>) -> Result<(), DatabaseError>;
    
    /// Set schedule enabled status
    async fn set_enabled(&self, id: ApiId, enabled: bool) -> Result<(), DatabaseError>;
}

// =============================================================================
// Repository Factory
// =============================================================================

/// Factory trait for creating repository instances
#[async_trait]
pub trait RepositoryFactory: Send + Sync {
    /// Get task repository instance
    fn task_repository(&self) -> &dyn TaskRepository;
    
    /// Get execution repository instance
    fn execution_repository(&self) -> &dyn ExecutionRepository;
    
    /// Get job repository instance
    fn job_repository(&self) -> &dyn JobRepository;
    
    /// Get schedule repository instance
    fn schedule_repository(&self) -> &dyn ScheduleRepository;
    
    /// Check health of all repositories
    async fn health_check(&self) -> Result<(), DatabaseError>;
}

// =============================================================================
// Transaction Management
// =============================================================================

/// Transaction context for atomic operations
#[async_trait]
pub trait TransactionContext: Send + Sync {
    /// Commit the current transaction
    async fn commit(self) -> Result<(), DatabaseError>;
    
    /// Rollback the current transaction
    async fn rollback(self) -> Result<(), DatabaseError>;
}

/// Transaction manager for atomic operations across repositories
#[async_trait]
pub trait TransactionManager: Send + Sync {
    type Context: TransactionContext;
    
    /// Begin a new transaction
    async fn begin_transaction(&self) -> Result<Self::Context, DatabaseError>;
    
    /// Execute a closure within a transaction, automatically committing or rolling back
    async fn with_transaction<F, R>(&self, f: F) -> Result<R, DatabaseError>
    where
        F: FnOnce(&Self::Context) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, DatabaseError>> + Send>> + Send,
        R: Send;
}