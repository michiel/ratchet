//! Bridge implementations for functional services
//!
//! This module provides adapters that bridge between ratchet-registry services
//! and the interfaces expected by ratchet-interfaces, enabling smooth integration
//! of task registry, management, and validation functionality.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;

use ratchet_interfaces::{
    database::RepositoryFactory,
    registry::{TaskRegistry, RegistryManager, TaskValidator},
};
use ratchet_api_types::{
    ApiId, UnifiedTask
};

// =============================================================================
// Registry Bridge Implementations
// =============================================================================

/// Bridge that adapts ratchet-registry's DefaultTaskRegistry to the interface expected by ratchet-interfaces
pub struct BridgeTaskRegistry {
    service: Arc<ratchet_registry::DefaultRegistryService>,
    repositories: Option<Arc<dyn RepositoryFactory>>,
}

// Import the RegistryService trait to access methods
use ratchet_registry::RegistryService;

impl BridgeTaskRegistry {
    pub async fn new(config: &crate::config::ServerConfig) -> anyhow::Result<Self> {
        // Create a filesystem source pointing to the sample tasks directory
        let sample_tasks_path = std::env::current_dir()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find parent directory"))?
            .join("sample")
            .join("js-tasks")
            .join("tasks");
            
        let filesystem_source = ratchet_registry::TaskSource::Filesystem {
            path: sample_tasks_path.to_string_lossy().to_string(),
            recursive: true,
            watch: false,
        };

        let registry_config = ratchet_registry::RegistryConfig {
            sources: vec![filesystem_source],
            sync_interval: std::time::Duration::from_secs(300),
            enable_auto_sync: false,
            enable_validation: true,
            cache_config: ratchet_registry::config::CacheConfig::default(),
        };

        let service = Arc::new(ratchet_registry::DefaultRegistryService::new(registry_config));
        
        // Discover and load tasks on startup
        match service.discover_all_tasks().await {
            Ok(discovered_tasks) => {
                tracing::info!("Successfully discovered {} tasks during registry initialization", discovered_tasks.len());
                for task in &discovered_tasks {
                    tracing::info!("Discovered task: {} v{}", task.metadata.name, task.metadata.version);
                }
                
                // We need to load the tasks into the internal registry
                let registry = service.registry().await;
                for discovered in discovered_tasks {
                    if let Err(e) = service.load_task(&discovered.task_ref).await {
                        tracing::warn!("Failed to load task {}: {}", discovered.metadata.name, e);
                        continue;
                    }
                    
                    // Try to load the full task definition and add it to the registry
                    match service.load_task(&discovered.task_ref).await {
                        Ok(task_def) => {
                            if let Err(e) = registry.add_task(task_def.clone()).await {
                                tracing::warn!("Failed to add task {} to registry: {}", discovered.metadata.name, e);
                            } else {
                                tracing::info!("Successfully added task {} to registry", discovered.metadata.name);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load task definition for {}: {}", discovered.metadata.name, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to discover tasks during registry initialization: {}", e);
            }
        }

        Ok(Self { service, repositories: None })
    }
    
    /// Set the repository factory for database synchronization
    pub fn set_repositories(&mut self, repositories: Arc<dyn RepositoryFactory>) {
        self.repositories = Some(repositories);
    }
    
    /// Sync discovered tasks to the database
    pub async fn sync_tasks_to_database(&self) -> anyhow::Result<()> {
        if let Some(repositories) = &self.repositories {
            let registry = self.service.registry().await;
            let tasks = registry.list_tasks().await.map_err(convert_registry_error)?;
            
            let task_repo = repositories.task_repository();
            
            for task in tasks {
                // Convert registry task to storage task
                let unified_task = convert_task_definition_to_unified(&task);
                
                // Check if task already exists in database
                if let Ok(Some(_existing)) = task_repo.find_by_uuid(task.metadata.uuid).await {
                    tracing::debug!("Task {} already exists in database, skipping", task.metadata.name);
                    continue;
                }
                
                // Create new task in database
                match task_repo.create(unified_task).await {
                    Ok(_) => {
                        tracing::info!("Successfully synced task {} to database", task.metadata.name);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to sync task {} to database: {:?}", task.metadata.name, e);
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ratchet_interfaces::TaskRegistry for BridgeTaskRegistry {
    async fn discover_tasks(&self) -> Result<Vec<ratchet_interfaces::TaskMetadata>, ratchet_interfaces::RegistryError> {
        let discovered_tasks = self.service.discover_all_tasks().await
            .map_err(convert_registry_error)?;
        
        let mut metadata_list = Vec::new();
        for discovered in discovered_tasks {
            let metadata = convert_task_metadata(&discovered.metadata);
            metadata_list.push(metadata);
        }
        
        Ok(metadata_list)
    }
    
    async fn get_task_metadata(&self, name: &str) -> Result<ratchet_interfaces::TaskMetadata, ratchet_interfaces::RegistryError> {
        let registry = self.service.registry().await;
        let tasks = registry.list_tasks().await
            .map_err(convert_registry_error)?;
        
        for task in tasks {
            if task.metadata.name == name {
                return Ok(convert_task_metadata(&task.metadata));
            }
        }
        
        Err(ratchet_interfaces::RegistryError::TaskNotFound { name: name.to_string() })
    }
    
    async fn load_task_content(&self, name: &str) -> Result<String, ratchet_interfaces::RegistryError> {
        let registry = self.service.registry().await;
        let tasks = registry.list_tasks().await
            .map_err(convert_registry_error)?;
        
        for task in tasks {
            if task.metadata.name == name {
                return Ok(task.script.clone());
            }
        }
        
        Err(ratchet_interfaces::RegistryError::TaskNotFound { name: name.to_string() })
    }
    
    async fn task_exists(&self, name: &str) -> Result<bool, ratchet_interfaces::RegistryError> {
        let registry = self.service.registry().await;
        let tasks = registry.list_tasks().await
            .map_err(convert_registry_error)?;
        
        Ok(tasks.iter().any(|task| task.metadata.name == name))
    }
    
    fn registry_id(&self) -> &str {
        "default-bridge-registry"
    }
    
    async fn health_check(&self) -> Result<(), ratchet_interfaces::RegistryError> {
        // Just verify that we can list tasks
        let _ = self.service.discover_all_tasks().await
            .map_err(convert_registry_error)?;
        Ok(())
    }
}

/// Bridge that adapts ratchet-registry to provide registry manager functionality
pub struct BridgeRegistryManager {
    registries: Vec<Arc<BridgeTaskRegistry>>,
}

impl BridgeRegistryManager {
    pub async fn new(config: &crate::config::ServerConfig) -> anyhow::Result<Self> {
        let primary_registry = Arc::new(BridgeTaskRegistry::new(config).await?);
        Ok(Self {
            registries: vec![primary_registry],
        })
    }
}

#[async_trait]
impl ratchet_interfaces::RegistryManager for BridgeRegistryManager {
    async fn add_registry(&self, _registry: Box<dyn ratchet_interfaces::TaskRegistry>) -> Result<(), ratchet_interfaces::RegistryError> {
        // For now, we only support a single registry
        Ok(())
    }
    
    async fn remove_registry(&self, _registry_id: &str) -> Result<(), ratchet_interfaces::RegistryError> {
        // For now, we only support a single registry
        Ok(())
    }
    
    async fn list_registries(&self) -> Vec<&str> {
        vec!["default-bridge-registry"]
    }
    
    async fn discover_all_tasks(&self) -> Result<Vec<(String, ratchet_interfaces::TaskMetadata)>, ratchet_interfaces::RegistryError> {
        let mut all_tasks = Vec::new();
        
        for registry in &self.registries {
            let tasks = registry.discover_tasks().await?;
            for task in tasks {
                all_tasks.push((registry.registry_id().to_string(), task));
            }
        }
        
        Ok(all_tasks)
    }
    
    async fn find_task(&self, name: &str) -> Result<(String, ratchet_interfaces::TaskMetadata), ratchet_interfaces::RegistryError> {
        for registry in &self.registries {
            if let Ok(metadata) = registry.get_task_metadata(name).await {
                return Ok((registry.registry_id().to_string(), metadata));
            }
        }
        
        Err(ratchet_interfaces::RegistryError::TaskNotFound { name: name.to_string() })
    }
    
    async fn load_task(&self, name: &str) -> Result<String, ratchet_interfaces::RegistryError> {
        for registry in &self.registries {
            if let Ok(content) = registry.load_task_content(name).await {
                return Ok(content);
            }
        }
        
        Err(ratchet_interfaces::RegistryError::TaskNotFound { name: name.to_string() })
    }
    
    async fn sync_with_database(&self) -> Result<ratchet_interfaces::SyncResult, ratchet_interfaces::RegistryError> {
        // For now, return empty sync result
        Ok(ratchet_interfaces::SyncResult {
            added: vec![],
            updated: vec![],
            removed: vec![],
            errors: vec![],
        })
    }
}

/// Basic task validator implementation
pub struct BridgeTaskValidator;

impl BridgeTaskValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ratchet_interfaces::TaskValidator for BridgeTaskValidator {
    async fn validate_metadata(&self, _metadata: &ratchet_interfaces::TaskMetadata) -> Result<ratchet_interfaces::ValidationResult, ratchet_interfaces::RegistryError> {
        // Basic validation - all tasks are considered valid for now
        Ok(ratchet_interfaces::ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_content(&self, _content: &str, _metadata: &ratchet_interfaces::TaskMetadata) -> Result<ratchet_interfaces::ValidationResult, ratchet_interfaces::RegistryError> {
        // Basic validation - all content is considered valid for now
        Ok(ratchet_interfaces::ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_input(&self, _input: &serde_json::Value, _metadata: &ratchet_interfaces::TaskMetadata) -> Result<ratchet_interfaces::ValidationResult, ratchet_interfaces::RegistryError> {
        // Basic validation - all input is considered valid for now
        Ok(ratchet_interfaces::ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
}

// =============================================================================
// Helper conversion functions
// =============================================================================

fn convert_registry_error(err: ratchet_registry::RegistryError) -> ratchet_interfaces::RegistryError {
    match err {
        ratchet_registry::RegistryError::TaskNotFound(name) => ratchet_interfaces::RegistryError::TaskNotFound { name },
        ratchet_registry::RegistryError::ValidationError(msg) => ratchet_interfaces::RegistryError::InvalidFormat { message: msg },
        ratchet_registry::RegistryError::Io(e) => ratchet_interfaces::RegistryError::Io { message: e.to_string() },
        ratchet_registry::RegistryError::Configuration(msg) => ratchet_interfaces::RegistryError::InvalidFormat { message: msg },
        ratchet_registry::RegistryError::NotImplemented(msg) => ratchet_interfaces::RegistryError::Unavailable { message: msg },
        ratchet_registry::RegistryError::LoadError(msg) => ratchet_interfaces::RegistryError::InvalidFormat { message: msg },
        ratchet_registry::RegistryError::SyncError(msg) => ratchet_interfaces::RegistryError::Unavailable { message: msg },
        ratchet_registry::RegistryError::WatcherError(msg) => ratchet_interfaces::RegistryError::Unavailable { message: msg },
        ratchet_registry::RegistryError::Http(e) => ratchet_interfaces::RegistryError::Network { message: e.to_string() },
        ratchet_registry::RegistryError::Json(e) => ratchet_interfaces::RegistryError::InvalidFormat { message: e.to_string() },
        ratchet_registry::RegistryError::Storage(e) => ratchet_interfaces::RegistryError::Unavailable { message: e.to_string() },
        ratchet_registry::RegistryError::Core(e) => ratchet_interfaces::RegistryError::Unavailable { message: e.to_string() },
        ratchet_registry::RegistryError::TaskJoin(e) => ratchet_interfaces::RegistryError::Unavailable { message: e.to_string() },
        ratchet_registry::RegistryError::Other(msg) => ratchet_interfaces::RegistryError::Unavailable { message: msg },
        ratchet_registry::RegistryError::GitError(msg) => ratchet_interfaces::RegistryError::Unavailable { message: msg },
        ratchet_registry::RegistryError::Git(e) => ratchet_interfaces::RegistryError::Unavailable { message: e.to_string() },
    }
}

fn convert_task_metadata(metadata: &ratchet_registry::TaskMetadata) -> ratchet_interfaces::TaskMetadata {
    ratchet_interfaces::TaskMetadata {
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        description: metadata.description.clone(),
        input_schema: None, // TODO: Extract from task definition if available
        output_schema: None, // TODO: Extract from task definition if available
        metadata: None, // TODO: Convert additional metadata if needed
    }
}

fn convert_task_definition_to_unified(task_def: &ratchet_registry::TaskDefinition) -> UnifiedTask {
    use ratchet_api_types::{ApiId, UnifiedTask};
    
    UnifiedTask {
        id: ApiId::from_i32(0), // Will be auto-generated by database
        uuid: task_def.metadata.uuid,
        name: task_def.metadata.name.clone(),
        description: task_def.metadata.description.clone(),
        version: task_def.metadata.version.clone(),
        enabled: true,
        registry_source: true,
        available_versions: vec![task_def.metadata.version.clone()],
        created_at: task_def.metadata.created_at,
        updated_at: task_def.metadata.updated_at,
        validated_at: Some(chrono::Utc::now()),
        in_sync: true,
        input_schema: task_def.input_schema.clone(),
        output_schema: task_def.output_schema.clone(),
        metadata: Some(serde_json::json!({
            "source": task_def.reference.source,
            "script_length": task_def.script.len(),
            "dependencies": task_def.dependencies,
            "environment": task_def.environment
        })),
    }
}