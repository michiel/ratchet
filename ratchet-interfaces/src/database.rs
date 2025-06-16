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
    UnifiedUser, UnifiedSession, UnifiedApiKey,
    ExecutionStatus, JobStatus, JobPriority,
    pagination::ListInput
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
    
    /// Find entities with filters, pagination, and sorting
    async fn find_with_list_input(
        &self,
        filters: F,
        list_input: ratchet_api_types::pagination::ListInput,
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
    // Basic filters (existing)
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,
    pub validated_after: Option<DateTime<Utc>>,
    
    // Advanced string filtering
    pub name_exact: Option<String>,
    pub name_contains: Option<String>,
    pub name_starts_with: Option<String>,
    pub name_ends_with: Option<String>,
    
    // Version filtering
    pub version: Option<String>,
    pub version_in: Option<Vec<String>>,
    
    // Extended date filtering
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub validated_before: Option<DateTime<Utc>>,
    
    // ID filtering
    pub uuid: Option<String>,
    pub uuid_in: Option<Vec<String>>,
    pub id_in: Option<Vec<i32>>,
    
    // Advanced boolean filtering
    pub has_validation: Option<bool>,
    pub in_sync: Option<bool>,
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
    // Basic filters (existing)
    pub task_id: Option<ApiId>,
    pub status: Option<ExecutionStatus>,
    pub queued_after: Option<DateTime<Utc>>,
    pub completed_after: Option<DateTime<Utc>>,
    
    // Advanced ID filtering
    pub task_id_in: Option<Vec<ApiId>>,
    pub id_in: Option<Vec<ApiId>>,
    
    // Advanced status filtering
    pub status_in: Option<Vec<ExecutionStatus>>,
    pub status_not: Option<ExecutionStatus>,
    
    // Extended date filtering
    pub queued_before: Option<DateTime<Utc>>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub completed_before: Option<DateTime<Utc>>,
    
    // Duration filtering
    pub duration_min_ms: Option<i32>,
    pub duration_max_ms: Option<i32>,
    
    // Progress filtering
    pub progress_min: Option<f32>,
    pub progress_max: Option<f32>,
    pub has_progress: Option<bool>,
    
    // Error filtering
    pub has_error: Option<bool>,
    pub error_message_contains: Option<String>,
    
    // Advanced boolean filtering
    pub can_retry: Option<bool>,
    pub can_cancel: Option<bool>,
}

/// Execution repository interface
#[async_trait]
pub trait ExecutionRepository: FilteredRepository<UnifiedExecution, ExecutionFilters> {
    /// Find executions by task ID
    async fn find_by_task_id(&self, task_id: ApiId) -> Result<Vec<UnifiedExecution>, DatabaseError>;
    
    /// Find executions by status
    async fn find_by_status(&self, status: ExecutionStatus) -> Result<Vec<UnifiedExecution>, DatabaseError>;
    
    /// Update execution status
    async fn update_status(&self, id: ApiId, status: ExecutionStatus) -> Result<(), DatabaseError>;
    
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
    // Basic filters (existing)
    pub task_id: Option<ApiId>,
    pub status: Option<JobStatus>,
    pub priority: Option<JobPriority>,
    pub queued_after: Option<DateTime<Utc>>,
    pub scheduled_before: Option<DateTime<Utc>>,
    
    // Advanced ID filtering
    pub task_id_in: Option<Vec<ApiId>>,
    pub id_in: Option<Vec<ApiId>>,
    
    // Advanced status filtering
    pub status_in: Option<Vec<JobStatus>>,
    pub status_not: Option<JobStatus>,
    
    // Advanced priority filtering
    pub priority_in: Option<Vec<JobPriority>>,
    pub priority_min: Option<JobPriority>,
    
    // Extended date filtering
    pub queued_before: Option<DateTime<Utc>>,
    pub scheduled_after: Option<DateTime<Utc>>,
    
    // Retry filtering
    pub retry_count_min: Option<i32>,
    pub retry_count_max: Option<i32>,
    pub max_retries_min: Option<i32>,
    pub max_retries_max: Option<i32>,
    pub has_retries_remaining: Option<bool>,
    
    // Error filtering
    pub has_error: Option<bool>,
    pub error_message_contains: Option<String>,
    
    // Scheduling filtering
    pub is_scheduled: Option<bool>,
    pub due_now: Option<bool>, // scheduled_for <= now
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
    // Basic filters (existing)
    pub task_id: Option<ApiId>,
    pub enabled: Option<bool>,
    pub next_run_before: Option<DateTime<Utc>>,
    
    // Advanced ID filtering
    pub task_id_in: Option<Vec<ApiId>>,
    pub id_in: Option<Vec<ApiId>>,
    
    // Name filtering
    pub name_contains: Option<String>,
    pub name_exact: Option<String>,
    pub name_starts_with: Option<String>,
    pub name_ends_with: Option<String>,
    
    // Cron expression filtering
    pub cron_expression_contains: Option<String>,
    pub cron_expression_exact: Option<String>,
    
    // Schedule timing filtering
    pub next_run_after: Option<DateTime<Utc>>,
    pub last_run_after: Option<DateTime<Utc>>,
    pub last_run_before: Option<DateTime<Utc>>,
    
    // Date range filtering
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    
    // Advanced filtering
    pub has_next_run: Option<bool>,
    pub has_last_run: Option<bool>,
    pub is_due: Option<bool>, // next_run <= now
    pub overdue: Option<bool>, // next_run < now and enabled
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
// Authentication Repositories
// =============================================================================

/// Filter criteria for user queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFilters {
    pub username: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub email_verified: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// User repository interface
#[async_trait]
pub trait UserRepository: FilteredRepository<UnifiedUser, UserFilters> {
    /// Find user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<UnifiedUser>, DatabaseError>;
    
    /// Find user by email
    async fn find_by_email(&self, email: &str) -> Result<Option<UnifiedUser>, DatabaseError>;
    
    /// Create user with password hash
    async fn create_user(&self, username: &str, email: &str, password_hash: &str, role: &str) -> Result<UnifiedUser, DatabaseError>;
    
    /// Update password hash
    async fn update_password(&self, user_id: ApiId, password_hash: &str) -> Result<(), DatabaseError>;
    
    /// Update last login time
    async fn update_last_login(&self, user_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Set user active status
    async fn set_active(&self, user_id: ApiId, is_active: bool) -> Result<(), DatabaseError>;
    
    /// Verify email
    async fn verify_email(&self, user_id: ApiId) -> Result<(), DatabaseError>;
}

/// Session repository interface  
#[async_trait]
pub trait SessionRepository: CrudRepository<UnifiedSession> {
    /// Create a new session
    async fn create_session(&self, user_id: ApiId, session_id: &str, jwt_id: &str, expires_at: DateTime<Utc>) -> Result<UnifiedSession, DatabaseError>;
    
    /// Find session by session ID
    async fn find_by_session_id(&self, session_id: &str) -> Result<Option<UnifiedSession>, DatabaseError>;
    
    /// Find sessions by user ID
    async fn find_by_user_id(&self, user_id: ApiId) -> Result<Vec<UnifiedSession>, DatabaseError>;
    
    /// Invalidate session
    async fn invalidate_session(&self, session_id: &str) -> Result<(), DatabaseError>;
    
    /// Invalidate all user sessions
    async fn invalidate_user_sessions(&self, user_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Update last used time
    async fn update_last_used(&self, session_id: &str) -> Result<(), DatabaseError>;
    
    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) -> Result<u64, DatabaseError>;
}

/// API key repository interface
#[async_trait]
pub trait ApiKeyRepository: CrudRepository<UnifiedApiKey> {
    /// Find API key by hash
    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<UnifiedApiKey>, DatabaseError>;
    
    /// Find API keys by user ID
    async fn find_by_user_id(&self, user_id: ApiId) -> Result<Vec<UnifiedApiKey>, DatabaseError>;
    
    /// Create API key
    async fn create_api_key(&self, user_id: ApiId, name: &str, key_hash: &str, key_prefix: &str, permissions: &str) -> Result<UnifiedApiKey, DatabaseError>;
    
    /// Update last used time
    async fn update_last_used(&self, api_key_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Increment usage count
    async fn increment_usage(&self, api_key_id: ApiId) -> Result<(), DatabaseError>;
    
    /// Set API key active status
    async fn set_active(&self, api_key_id: ApiId, is_active: bool) -> Result<(), DatabaseError>;
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
    
    /// Get user repository instance
    fn user_repository(&self) -> &dyn UserRepository;
    
    /// Get session repository instance
    fn session_repository(&self) -> &dyn SessionRepository;
    
    /// Get API key repository instance
    fn api_key_repository(&self) -> &dyn ApiKeyRepository;
    
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