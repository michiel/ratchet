//! Unified task service interface
//!
//! This module provides a unified interface for task operations that abstracts
//! over different storage backends (database, registry, filesystem, etc.)

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::{DatabaseError, RegistryError};
use ratchet_api_types::{ListResponse, PaginationInput, UnifiedTask};

/// Unified task service that abstracts task storage location
#[async_trait]
pub trait TaskService: Send + Sync {
    /// Find a task by its ID (UUID)
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UnifiedTask>, TaskServiceError>;
    
    /// Find a task by its name (searches both database and registry)
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, TaskServiceError>;
    
    /// List all available tasks from all sources
    async fn list_tasks(
        &self,
        pagination: Option<PaginationInput>,
        filters: Option<TaskServiceFilters>,
    ) -> Result<ListResponse<UnifiedTask>, TaskServiceError>;
    
    /// Get task metadata (including source information)
    async fn get_task_metadata(&self, id: Uuid) -> Result<Option<TaskMetadata>, TaskServiceError>;
    
    /// Execute a task with the given input
    async fn execute_task(&self, id: Uuid, input: JsonValue) -> Result<JsonValue, TaskServiceError>;
    
    /// Check if a task exists
    async fn task_exists(&self, id: Uuid) -> Result<bool, TaskServiceError>;
    
    /// Get task source information
    async fn get_task_source(&self, id: Uuid) -> Result<Option<TaskSource>, TaskServiceError>;
}

/// Task metadata including source information
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub source: TaskSource,
    pub enabled: bool,
}

/// Information about where a task is stored
#[derive(Debug, Clone)]
pub enum TaskSource {
    /// Task is stored in the database
    Database,
    /// Task is from a registry source (Git, filesystem, etc.)
    Registry { source_name: String },
    /// Task is embedded in the application
    Embedded,
}

/// Filters for task listing
#[derive(Debug, Clone, Default)]
pub struct TaskServiceFilters {
    pub enabled_only: Option<bool>,
    pub source_type: Option<TaskSourceType>,
    pub name_contains: Option<String>,
}

/// Type of task source for filtering
#[derive(Debug, Clone)]
pub enum TaskSourceType {
    Database,
    Registry,
    Embedded,
    Any,
}

/// Unified error type for task service operations
#[derive(Debug, thiserror::Error)]
pub enum TaskServiceError {
    #[error("Task not found: {id}")]
    TaskNotFound { id: String },
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),
    
    #[error("Execution error: {message}")]
    Execution { message: String },
    
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<String> for TaskServiceError {
    fn from(message: String) -> Self {
        TaskServiceError::Internal { message }
    }
}