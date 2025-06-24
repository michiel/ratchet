//! Unified task service implementation
//!
//! This module provides a concrete implementation of the TaskService trait that
//! combines access to both database-stored tasks and registry-sourced tasks.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratchet_api_types::{ApiId, ListResponse, PaginationInput, UnifiedTask};
use ratchet_interfaces::{
    RepositoryFactory, TaskFilters, TaskRegistry, TaskService, TaskServiceError, TaskServiceFilters,
    TaskServiceMetadata, TaskSource, TaskSourceType,
};

/// Unified task service implementation
pub struct UnifiedTaskService {
    /// Database repositories for persisted tasks
    repositories: Arc<dyn RepositoryFactory>,
    /// Registry for dynamic task discovery
    registry: Arc<dyn TaskRegistry>,
}

impl UnifiedTaskService {
    /// Create a new unified task service
    pub fn new(
        repositories: Arc<dyn RepositoryFactory>,
        registry: Arc<dyn TaskRegistry>,
    ) -> Self {
        Self {
            repositories,
            registry,
        }
    }
    
    /// Generate a consistent UUID from a task name
    fn generate_task_uuid(name: &str) -> Uuid {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert hash to UUID bytes
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&hash.to_be_bytes());
        bytes[8..16].copy_from_slice(&hash.to_le_bytes());
        
        Uuid::from_bytes(bytes)
    }
}

#[async_trait]
impl TaskService for UnifiedTaskService {
    /// Find a task by its ID (searches both database and registry)
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UnifiedTask>, TaskServiceError> {
        debug!("Looking for task by ID: {}", id);
        
        // First try database
        let task_repo = self.repositories.task_repository();
        if let Ok(Some(task)) = task_repo.find_by_uuid(id).await {
            debug!("Found task {} in database", id);
            return Ok(Some(task));
        }
        
        // Then try registry
        match self.registry.discover_tasks().await {
            Ok(discovered_tasks) => {
                for task_meta in discovered_tasks {
                    // Generate a consistent UUID from the task name
                    let task_uuid = Self::generate_task_uuid(&task_meta.name);
                    
                    if task_uuid == id {
                        debug!("Found task {} in registry", id);
                        // Convert registry task to UnifiedTask
                        let unified_task = UnifiedTask {
                            id: ApiId::from_uuid(task_uuid),
                            uuid: task_uuid,
                            name: task_meta.name.clone(),
                            description: task_meta.description.clone(),
                            version: task_meta.version.clone(),
                            enabled: true, // Registry tasks are enabled by default
                            registry_source: true,
                            available_versions: vec![task_meta.version],
                            created_at: chrono::Utc::now(), // Use current time as fallback
                            updated_at: chrono::Utc::now(), // Use current time as fallback
                            validated_at: None,
                            in_sync: true, // Registry tasks are always in sync
                            input_schema: task_meta.input_schema,
                            output_schema: task_meta.output_schema,
                            metadata: task_meta.metadata,
                        };
                        return Ok(Some(unified_task));
                    }
                }
                debug!("Task {} not found in registry", id);
            }
            Err(e) => {
                warn!("Failed to search registry for task {}: {}", id, e);
            }
        }
        
        Ok(None)
    }
    
    /// Find a task by its name (searches both database and registry)
    async fn find_by_name(&self, name: &str) -> Result<Option<UnifiedTask>, TaskServiceError> {
        debug!("Looking for task by name: {}", name);
        
        // First try database
        let task_repo = self.repositories.task_repository();
        if let Ok(Some(task)) = task_repo.find_by_name(name).await {
            debug!("Found task '{}' in database", name);
            return Ok(Some(task));
        }
        
        // Then try registry
        match self.registry.discover_tasks().await {
            Ok(discovered_tasks) => {
                for task_meta in discovered_tasks {
                    if task_meta.name == name {
                        debug!("Found task '{}' in registry", name);
                        // Generate UUID from task name for consistency
                        let task_uuid = Self::generate_task_uuid(&task_meta.name);
                        
                        // Convert registry task to UnifiedTask
                        let unified_task = UnifiedTask {
                            id: ApiId::from_uuid(task_uuid),
                            uuid: task_uuid,
                            name: task_meta.name.clone(),
                            description: task_meta.description.clone(),
                            version: task_meta.version.clone(),
                            enabled: true, // Registry tasks are enabled by default
                            registry_source: true,
                            available_versions: vec![task_meta.version],
                            created_at: chrono::Utc::now(), // Use current time as fallback
                            updated_at: chrono::Utc::now(), // Use current time as fallback
                            validated_at: None,
                            in_sync: true, // Registry tasks are always in sync
                            input_schema: task_meta.input_schema,
                            output_schema: task_meta.output_schema,
                            metadata: task_meta.metadata,
                        };
                        return Ok(Some(unified_task));
                    }
                }
                debug!("Task '{}' not found in registry", name);
            }
            Err(e) => {
                warn!("Failed to search registry for task '{}': {}", name, e);
            }
        }
        
        Ok(None)
    }
    
    /// List all available tasks from all sources
    async fn list_tasks(
        &self,
        pagination: Option<PaginationInput>,
        filters: Option<TaskServiceFilters>,
    ) -> Result<ListResponse<UnifiedTask>, TaskServiceError> {
        debug!("Listing tasks with filters: {:?}", filters);
        
        let mut all_tasks = Vec::new();
        let filters = filters.unwrap_or_default();
        
        // Get tasks from database if not filtering by source type
        if matches!(filters.source_type, None | Some(TaskSourceType::Database) | Some(TaskSourceType::Any)) {
            let task_repo = self.repositories.task_repository();
            // Create minimal filter for database tasks
            let db_filters = TaskFilters { 
                name: None,
                enabled: Some(true),
                registry_source: None,
                validated_after: None,
                name_exact: None,
                name_contains: None,
                name_starts_with: None,
                name_ends_with: None,
                version: None,
                version_in: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                validated_before: None,
                uuid: None,
                uuid_in: None,
                id_in: None,
                has_validation: None,
                in_sync: None,
            };
            match task_repo.find_with_filters(db_filters, pagination.clone().unwrap_or_default()).await {
                Ok(db_response) => {
                    debug!("Found {} tasks in database", db_response.items.len());
                    all_tasks.extend(db_response.items);
                }
                Err(e) => {
                    warn!("Failed to list database tasks: {}", e);
                }
            }
        }
        
        // Get tasks from registry if not filtering by source type
        if matches!(filters.source_type, None | Some(TaskSourceType::Registry) | Some(TaskSourceType::Any)) {
            match self.registry.discover_tasks().await {
                Ok(discovered_tasks) => {
                    debug!("Found {} tasks in registry", discovered_tasks.len());
                    for task_meta in discovered_tasks {
                        // Generate UUID from task name for consistency
                        let task_uuid = Self::generate_task_uuid(&task_meta.name);
                        
                        // Convert to UnifiedTask
                        let unified_task = UnifiedTask {
                            id: ApiId::from_uuid(task_uuid),
                            uuid: task_uuid,
                            name: task_meta.name.clone(),
                            description: task_meta.description.clone(),
                            version: task_meta.version.clone(),
                            enabled: true, // Registry tasks are enabled by default
                            registry_source: true,
                            available_versions: vec![task_meta.version],
                            created_at: chrono::Utc::now(), // Use current time as fallback
                            updated_at: chrono::Utc::now(), // Use current time as fallback
                            validated_at: None,
                            in_sync: true,
                            input_schema: task_meta.input_schema,
                            output_schema: task_meta.output_schema,
                            metadata: task_meta.metadata,
                        };
                        all_tasks.push(unified_task);
                    }
                }
                Err(e) => {
                    warn!("Failed to list registry tasks: {}", e);
                }
            }
        }
        
        // Apply filters
        if let Some(name_filter) = &filters.name_contains {
            all_tasks.retain(|task| task.name.contains(name_filter));
        }
        
        if let Some(enabled_only) = filters.enabled_only {
            if enabled_only {
                all_tasks.retain(|task| task.enabled);
            }
        }
        
        // Apply pagination (simplified implementation)
        let total_count = all_tasks.len();
        let (page, limit) = if let Some(ref pagination) = pagination {
            (pagination.page.unwrap_or(0) as usize, pagination.limit.unwrap_or(50) as usize)
        } else {
            (0, 50)
        };
        
        let start = page * limit;
        let end = std::cmp::min(start + limit, total_count);
        let items = if start < total_count {
            all_tasks[start..end].to_vec()
        } else {
            Vec::new()
        };
        
        // Create pagination input for metadata calculation
        let pagination_input = pagination.unwrap_or(PaginationInput {
            page: Some(page as u32 + 1), // Convert 0-based to 1-based
            limit: Some(limit as u32),
            offset: None,
        });
        
        Ok(ListResponse::new(items, &pagination_input, total_count as u64))
    }
    
    /// Get task metadata including source information
    async fn get_task_metadata(&self, id: Uuid) -> Result<Option<TaskServiceMetadata>, TaskServiceError> {
        // First check database
        let task_repo = self.repositories.task_repository();
        if let Ok(Some(task)) = task_repo.find_by_uuid(id).await {
            return Ok(Some(TaskServiceMetadata {
                id,
                name: task.name,
                version: task.version,
                description: task.description,
                source: TaskSource::Database,
                enabled: task.enabled,
            }));
        }
        
        // Then check registry
        match self.registry.discover_tasks().await {
            Ok(discovered_tasks) => {
                for task_meta in discovered_tasks {
                    // Generate UUID from task name for consistency
                    let task_uuid = Self::generate_task_uuid(&task_meta.name);
                    
                    if task_uuid == id {
                        return Ok(Some(TaskServiceMetadata {
                            id,
                            name: task_meta.name.clone(),
                            version: task_meta.version.clone(),
                            description: task_meta.description.clone(),
                            source: TaskSource::Registry { source_name: "registry".to_string() },
                            enabled: true, // Registry tasks are enabled by default
                        }));
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get task metadata from registry: {}", e);
            }
        }
        
        Ok(None)
    }
    
    /// Execute a task with the given input
    async fn execute_task(&self, id: Uuid, input: JsonValue) -> Result<JsonValue, TaskServiceError> {
        // This would delegate to the execution service
        // For now, return an error indicating this needs implementation
        Err(TaskServiceError::Configuration {
            message: "Task execution not yet implemented in unified service".to_string(),
        })
    }
    
    /// Check if a task exists
    async fn task_exists(&self, id: Uuid) -> Result<bool, TaskServiceError> {
        Ok(self.find_by_id(id).await?.is_some())
    }
    
    /// Get task source information
    async fn get_task_source(&self, id: Uuid) -> Result<Option<TaskSource>, TaskServiceError> {
        if let Some(metadata) = self.get_task_metadata(id).await? {
            Ok(Some(metadata.source))
        } else {
            Ok(None)
        }
    }
}