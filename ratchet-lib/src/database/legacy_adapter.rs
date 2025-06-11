//! Legacy database adapter for backwards compatibility
//!
//! This module provides a transitional adapter that implements the legacy ratchet-lib
//! database interface using the modern ratchet-storage backend. This allows existing
//! code to continue working while migration to the modern interface proceeds.

use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use ratchet_storage::seaorm::repositories::RepositoryFactory as ModernRepositoryFactory;
use ratchet_api_types::{
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule,
    ApiId, PaginationInput, ListResponse,
    ExecutionStatus, JobStatus, JobPriority
};

/// Legacy database adapter that wraps modern implementation
/// 
/// This adapter provides the legacy ratchet-lib database interface while
/// delegating to the modern ratchet-storage implementation under the hood.
/// This enables gradual migration without breaking existing code.
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_storage crate directly instead. Will be removed in 0.5.0"
)]
pub struct LegacyDatabaseAdapter {
    /// Modern repository factory implementation
    modern_impl: Arc<ModernRepositoryFactory>,
}

impl LegacyDatabaseAdapter {
    /// Create a new legacy database adapter
    /// 
    /// # Arguments
    /// * `modern_impl` - The modern repository factory to wrap
    /// 
    /// # Example
    /// ```rust
    /// use ratchet_storage::seaorm::repositories::RepositoryFactory;
    /// use ratchet_lib::database::legacy_adapter::LegacyDatabaseAdapter;
    /// use std::sync::Arc;
    /// 
    /// let modern_factory = Arc::new(RepositoryFactory::new(connection));
    /// let legacy_adapter = LegacyDatabaseAdapter::new(modern_factory);
    /// ```
    pub fn new(modern_impl: Arc<ModernRepositoryFactory>) -> Self {
        eprintln!("⚠️  WARNING: Using deprecated LegacyDatabaseAdapter. Please migrate to ratchet_storage crate.");
        eprintln!("    See migration guide: https://docs.rs/ratchet-storage/latest/ratchet_storage/migration/");
        
        Self { modern_impl }
    }

    /// Get a reference to the underlying modern implementation
    /// 
    /// This can be used to gradually migrate code by accessing modern
    /// repository methods directly while maintaining legacy adapter for
    /// other parts of the system.
    pub fn modern_repository_factory(&self) -> &ModernRepositoryFactory {
        &self.modern_impl
    }
}

/// Legacy repository factory trait
/// 
/// Provides the old ratchet-lib interface for repository access.
/// New code should use `ratchet_interfaces::RepositoryFactory` instead.
#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_interfaces::RepositoryFactory instead. Will be removed in 0.5.0"
)]
#[async_trait]
pub trait LegacyRepositoryFactory: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get task repository
    async fn task_repository(&self) -> Result<Box<dyn LegacyTaskRepository>, Self::Error>;
    
    /// Get execution repository
    async fn execution_repository(&self) -> Result<Box<dyn LegacyExecutionRepository>, Self::Error>;
    
    /// Get job repository  
    async fn job_repository(&self) -> Result<Box<dyn LegacyJobRepository>, Self::Error>;
    
    /// Get schedule repository
    async fn schedule_repository(&self) -> Result<Box<dyn LegacyScheduleRepository>, Self::Error>;
    
    /// Health check
    async fn health_check(&self) -> Result<(), Self::Error>;
}

/// Legacy task repository trait
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_interfaces::TaskRepository instead. Will be removed in 0.5.0"  
)]
#[async_trait]
pub trait LegacyTaskRepository: Send + Sync {
    /// Find all tasks
    async fn find_all(&self) -> Result<Vec<LegacyTask>>;
    
    /// Find task by ID
    async fn find_by_id(&self, id: i32) -> Result<Option<LegacyTask>>;
    
    /// Find enabled tasks
    async fn find_enabled(&self) -> Result<Vec<LegacyTask>>;
    
    /// Create new task
    async fn create(&self, task: LegacyTask) -> Result<LegacyTask>;
    
    /// Update existing task
    async fn update(&self, task: LegacyTask) -> Result<LegacyTask>;
    
    /// Delete task
    async fn delete(&self, id: i32) -> Result<()>;
}

/// Legacy execution repository trait
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_interfaces::ExecutionRepository instead. Will be removed in 0.5.0"
)]
#[async_trait]
pub trait LegacyExecutionRepository: Send + Sync {
    /// Find executions by task ID
    async fn find_by_task_id(&self, task_id: i32) -> Result<Vec<LegacyExecution>>;
    
    /// Create new execution
    async fn create(&self, execution: LegacyExecution) -> Result<LegacyExecution>;
    
    /// Update execution status
    async fn update_status(&self, id: i32, status: String) -> Result<()>;
}

/// Legacy job repository trait
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_interfaces::JobRepository instead. Will be removed in 0.5.0"
)]
#[async_trait]
pub trait LegacyJobRepository: Send + Sync {
    /// Find queued jobs
    async fn find_queued(&self) -> Result<Vec<LegacyJob>>;
    
    /// Create new job
    async fn create(&self, job: LegacyJob) -> Result<LegacyJob>;
    
    /// Update job status
    async fn update_status(&self, id: i32, status: String) -> Result<()>;
}

/// Legacy schedule repository trait
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_interfaces::ScheduleRepository instead. Will be removed in 0.5.0"
)]
#[async_trait]
pub trait LegacyScheduleRepository: Send + Sync {
    /// Find active schedules
    async fn find_active(&self) -> Result<Vec<LegacySchedule>>;
    
    /// Create new schedule
    async fn create(&self, schedule: LegacySchedule) -> Result<LegacySchedule>;
    
    /// Update schedule
    async fn update(&self, schedule: LegacySchedule) -> Result<LegacySchedule>;
}

// Legacy entity types for backward compatibility

/// Legacy task entity
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_api_types::UnifiedTask instead. Will be removed in 0.5.0"
)]
#[derive(Debug, Clone)]
pub struct LegacyTask {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub path: String,
    pub metadata: serde_json::Value,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

/// Legacy execution entity
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_api_types::UnifiedExecution instead. Will be removed in 0.5.0"
)]
#[derive(Debug, Clone)]
pub struct LegacyExecution {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Legacy job entity
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_api_types::UnifiedJob instead. Will be removed in 0.5.0"
)]
#[derive(Debug, Clone)]
pub struct LegacyJob {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: String,
    pub priority: String,
    pub error_message: Option<String>,
    pub max_retries: i32,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Legacy schedule entity
#[deprecated(
    since = "0.4.0",
    note = "Use ratchet_api_types::UnifiedSchedule instead. Will be removed in 0.5.0"
)]
#[derive(Debug, Clone)]
pub struct LegacySchedule {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub name: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub input: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

// Conversion functions between legacy and modern types

impl From<UnifiedTask> for LegacyTask {
    fn from(task: UnifiedTask) -> Self {
        Self {
            id: task.id.as_i32().unwrap_or(0),
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            path: String::new(), // Not available in UnifiedTask
            metadata: task.metadata.unwrap_or_default(),
            input_schema: task.input_schema.unwrap_or_default(),
            output_schema: task.output_schema.unwrap_or_default(),
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
        }
    }
}

impl From<LegacyTask> for UnifiedTask {
    fn from(task: LegacyTask) -> Self {
        let version = task.version.clone();
        Self {
            id: ApiId::from_i32(task.id),
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            enabled: task.enabled,
            registry_source: false, // Assume not from registry
            available_versions: vec![version],
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
            in_sync: true, // Assume in sync
            input_schema: Some(task.input_schema),
            output_schema: Some(task.output_schema),
            metadata: Some(task.metadata),
        }
    }
}

impl From<UnifiedExecution> for LegacyExecution {
    fn from(execution: UnifiedExecution) -> Self {
        Self {
            id: execution.id.as_i32().unwrap_or(0),
            uuid: execution.uuid,
            task_id: execution.task_id.as_i32().unwrap_or(0),
            input: execution.input.unwrap_or_default(),
            output: execution.output,
            status: execution.status.to_string(),
            error_message: execution.error_message,
            created_at: execution.created_at,
            updated_at: execution.updated_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
        }
    }
}

impl From<LegacyExecution> for UnifiedExecution {
    fn from(execution: LegacyExecution) -> Self {
        let status = match execution.status.to_lowercase().as_str() {
            "pending" => ExecutionStatus::Pending,
            "running" => ExecutionStatus::Running,
            "completed" => ExecutionStatus::Completed,
            "failed" => ExecutionStatus::Failed,
            "cancelled" => ExecutionStatus::Cancelled,
            _ => ExecutionStatus::Pending, // Default fallback
        };

        Self {
            id: ApiId::from_i32(execution.id),
            uuid: execution.uuid,
            task_id: ApiId::from_i32(execution.task_id),
            status,
            input: Some(execution.input),
            output: execution.output,
            error_message: execution.error_message,
            created_at: execution.created_at,
            updated_at: execution.updated_at,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
        }
    }
}

impl From<UnifiedJob> for LegacyJob {
    fn from(job: UnifiedJob) -> Self {
        Self {
            id: job.id.as_i32().unwrap_or(0),
            uuid: job.uuid,
            task_id: job.task_id.as_i32().unwrap_or(0),
            input: job.input.unwrap_or_default(),
            output: job.output,
            status: job.status.to_string(),
            priority: job.priority.to_string(),
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
}

impl From<UnifiedSchedule> for LegacySchedule {
    fn from(schedule: UnifiedSchedule) -> Self {
        Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_conversion() {
        let unified_task = UnifiedTask {
            id: ApiId::from_i32(1),
            uuid: Uuid::new_v4(),
            name: "test-task".to_string(),
            description: Some("A test task".to_string()),
            version: "1.0.0".to_string(),
            enabled: true,
            registry_source: false,
            available_versions: vec!["1.0.0".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            validated_at: None,
            in_sync: true,
            input_schema: Some(serde_json::json!({"type": "object"})),
            output_schema: Some(serde_json::json!({"type": "object"})),
            metadata: Some(serde_json::json!({"author": "test"})),
        };

        // Convert to legacy and back
        let legacy_task = LegacyTask::from(unified_task.clone());
        let converted_back = UnifiedTask::from(legacy_task);

        assert_eq!(unified_task.name, converted_back.name);
        assert_eq!(unified_task.version, converted_back.version);
        assert_eq!(unified_task.enabled, converted_back.enabled);
    }

    #[test]
    fn test_execution_status_conversion() {
        let execution = LegacyExecution {
            id: 1,
            uuid: Uuid::new_v4(),
            task_id: 1,
            input: serde_json::json!({}),
            output: None,
            status: "running".to_string(),
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
        };

        let unified_execution = UnifiedExecution::from(execution);
        assert_eq!(unified_execution.status, ExecutionStatus::Running);
    }
}