//! Context types for dependency injection in REST API handlers
//!
//! This module defines context structs that group related dependencies
//! for each REST API endpoint group. This enables clean dependency injection
//! and makes testing easier with mock implementations.

use ratchet_interfaces::{
    RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator,
};
use ratchet_mcp::server::task_dev_tools::TaskDevelopmentService;
use std::sync::Arc;

/// Context for task-related endpoints
/// 
/// Provides access to task registry, validation, and repository operations.
#[derive(Clone)]
pub struct TasksContext {
    /// Repository factory for database operations
    pub repositories: Arc<dyn RepositoryFactory>,
    /// Task registry for task discovery and loading
    pub registry: Arc<dyn TaskRegistry>,
    /// Registry manager for multi-registry operations
    pub registry_manager: Arc<dyn RegistryManager>,
    /// Task validator for content validation
    pub validator: Arc<dyn TaskValidator>,
    /// Optional MCP task development service for advanced task operations
    pub mcp_task_service: Option<Arc<TaskDevelopmentService>>,
}

impl TasksContext {
    pub fn new(
        repositories: Arc<dyn RepositoryFactory>,
        registry: Arc<dyn TaskRegistry>,
        registry_manager: Arc<dyn RegistryManager>,
        validator: Arc<dyn TaskValidator>,
    ) -> Self {
        Self {
            repositories,
            registry,
            registry_manager,
            validator,
            mcp_task_service: None,
        }
    }
    
    /// Create a new TasksContext with MCP task development service
    pub fn with_mcp_service(
        repositories: Arc<dyn RepositoryFactory>,
        registry: Arc<dyn TaskRegistry>,
        registry_manager: Arc<dyn RegistryManager>,
        validator: Arc<dyn TaskValidator>,
        mcp_task_service: Arc<TaskDevelopmentService>,
    ) -> Self {
        Self {
            repositories,
            registry,
            registry_manager,
            validator,
            mcp_task_service: Some(mcp_task_service),
        }
    }
}

/// Context for execution-related endpoints
/// 
/// Provides access to execution tracking and job queue management.
#[derive(Clone)]
pub struct ExecutionsContext {
    /// Repository factory for database operations
    pub repositories: Arc<dyn RepositoryFactory>,
}

impl ExecutionsContext {
    pub fn new(repositories: Arc<dyn RepositoryFactory>) -> Self {
        Self { repositories }
    }
}

/// Context for job-related endpoints
/// 
/// Provides access to job queue operations and scheduling.
#[derive(Clone)]
pub struct JobsContext {
    /// Repository factory for database operations
    pub repositories: Arc<dyn RepositoryFactory>,
}

impl JobsContext {
    pub fn new(repositories: Arc<dyn RepositoryFactory>) -> Self {
        Self { repositories }
    }
}

/// Context for schedule-related endpoints
/// 
/// Provides access to scheduling operations and cron management.
#[derive(Clone)]
pub struct SchedulesContext {
    /// Repository factory for database operations
    pub repositories: Arc<dyn RepositoryFactory>,
}

impl SchedulesContext {
    pub fn new(repositories: Arc<dyn RepositoryFactory>) -> Self {
        Self { repositories }
    }
}

/// Context for worker-related endpoints
/// 
/// Provides access to worker status and monitoring.
#[derive(Clone)]
pub struct WorkersContext {
    // Currently no specific dependencies for workers
    // Future: Add worker pool manager, metrics collector, etc.
}

impl WorkersContext {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WorkersContext {
    fn default() -> Self {
        Self::new()
    }
}