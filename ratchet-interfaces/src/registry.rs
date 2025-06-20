//! Task registry interfaces
//!
//! This module defines interfaces for task discovery, loading, and registry management.
//! These traits enable different registry implementations (filesystem, HTTP, etc.)
//! while maintaining consistent interfaces.

use async_trait::async_trait;
// Note: Registry interfaces don't need ratchet_api_types imports currently
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Task metadata for registry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Registry error types
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Task not found: {name}")]
    TaskNotFound { name: String },

    #[error("Invalid task format: {message}")]
    InvalidFormat { message: String },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Permission denied: {message}")]
    Permission { message: String },

    #[error("Registry unavailable: {message}")]
    Unavailable { message: String },
}

/// Task registry interface for discovering and loading tasks
#[async_trait]
pub trait TaskRegistry: Send + Sync {
    /// Discover all available tasks in the registry
    async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>, RegistryError>;

    /// Get metadata for a specific task
    async fn get_task_metadata(&self, name: &str) -> Result<TaskMetadata, RegistryError>;

    /// Load task content (JavaScript code, etc.)
    async fn load_task_content(&self, name: &str) -> Result<String, RegistryError>;

    /// Check if a task exists in the registry
    async fn task_exists(&self, name: &str) -> Result<bool, RegistryError>;

    /// Get the registry source identifier
    fn registry_id(&self) -> &str;

    /// Check if the registry is available
    async fn health_check(&self) -> Result<(), RegistryError>;
}

/// Filesystem-specific registry operations
#[async_trait]
pub trait FilesystemRegistry: TaskRegistry {
    /// Get the base path of the registry
    fn base_path(&self) -> &Path;

    /// Watch for changes in the registry
    async fn watch_changes(
        &self,
    ) -> Result<impl std::future::Future<Output = Result<(), RegistryError>>, RegistryError>;
}

/// HTTP-based registry operations
#[async_trait]
pub trait HttpRegistry: TaskRegistry {
    /// Get the base URL of the registry
    fn base_url(&self) -> &str;

    /// Set authentication credentials
    async fn set_credentials(&self, credentials: HttpCredentials) -> Result<(), RegistryError>;

    /// Download and cache task content
    async fn download_task(&self, name: &str, cache_path: &Path) -> Result<(), RegistryError>;
}

/// HTTP authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpCredentials {
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, header: String },
}

/// Registry manager for handling multiple registries
#[async_trait]
pub trait RegistryManager: Send + Sync {
    /// Add a registry to the manager
    async fn add_registry(&self, registry: Box<dyn TaskRegistry>) -> Result<(), RegistryError>;

    /// Remove a registry by ID
    async fn remove_registry(&self, registry_id: &str) -> Result<(), RegistryError>;

    /// List all registered registries
    async fn list_registries(&self) -> Vec<&str>;

    /// Discover tasks across all registries
    async fn discover_all_tasks(&self) -> Result<Vec<(String, TaskMetadata)>, RegistryError>;

    /// Find a task in any registry
    async fn find_task(&self, name: &str) -> Result<(String, TaskMetadata), RegistryError>;

    /// Load task content from the appropriate registry
    async fn load_task(&self, name: &str) -> Result<String, RegistryError>;

    /// Sync registry contents with database
    async fn sync_with_database(&self) -> Result<SyncResult, RegistryError>;
}

/// Result of registry synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub errors: Vec<(String, String)>,
}

/// Task validation interface
#[async_trait]
pub trait TaskValidator: Send + Sync {
    /// Validate task metadata
    async fn validate_metadata(&self, metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError>;

    /// Validate task content (JavaScript syntax, etc.)
    async fn validate_content(&self, content: &str, metadata: &TaskMetadata)
        -> Result<ValidationResult, RegistryError>;

    /// Validate input against task schema
    async fn validate_input(
        &self,
        input: &serde_json::Value,
        metadata: &TaskMetadata,
    ) -> Result<ValidationResult, RegistryError>;
}

/// Task validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

/// Validation warning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}
