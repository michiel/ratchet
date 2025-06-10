//! Service implementations and dependency injection setup

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;

use ratchet_interfaces::{
    RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator,
    TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository,
    Repository, CrudRepository, FilteredRepository,
    TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters,
    DatabaseError, TaskMetadata, RegistryError, ValidationResult, SyncResult
};
use ratchet_api_types::{
    ApiId, PaginationInput, ListResponse,
    UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule
};
use uuid::Uuid;
use ratchet_rest_api::context::TasksContext;
use ratchet_graphql_api::context::GraphQLContext;

use crate::config::ServerConfig;

/// Service container holding all application services
#[derive(Clone)]
pub struct ServiceContainer {
    pub repositories: Arc<dyn RepositoryFactory>,
    pub registry: Arc<dyn TaskRegistry>,
    pub registry_manager: Arc<dyn RegistryManager>,
    pub validator: Arc<dyn TaskValidator>,
}

impl ServiceContainer {
    /// Create a new service container with real implementations
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        // For now, we'll use the legacy ratchet-lib implementations
        // In the future, these would be replaced with the new modular implementations
        
        // This is a bridge implementation during the migration
        let repositories = create_repository_factory(config).await?;
        let registry = create_task_registry(config).await?;
        let registry_manager = create_registry_manager(config).await?;
        let validator = create_task_validator(config).await?;

        Ok(Self {
            repositories,
            registry,
            registry_manager,
            validator,
        })
    }

    /// Create a test service container with mock implementations
    #[cfg(test)]
    pub fn new_test() -> Self {
        use std::sync::Arc;
        
        // Create mock implementations for testing
        // These would be defined in the testing modules of each interface crate
        todo!("Implement mock service container for tests")
    }

    /// Create REST API context from service container
    pub fn rest_context(&self) -> TasksContext {
        TasksContext {
            repositories: self.repositories.clone(),
            registry: self.registry.clone(),
            registry_manager: self.registry_manager.clone(),
            validator: self.validator.clone(),
        }
    }

    /// Create GraphQL context from service container
    pub fn graphql_context(&self) -> GraphQLContext {
        GraphQLContext::new(
            self.repositories.clone(),
            self.registry.clone(),
            self.registry_manager.clone(),
            self.validator.clone(),
        )
    }
}

/// Create repository factory from configuration
async fn create_repository_factory(config: &ServerConfig) -> Result<Arc<dyn RepositoryFactory>> {
    // Create storage database connection
    let storage_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        connection_timeout: std::time::Duration::from_secs(config.database.connection_timeout_seconds),
    };
    
    let db_connection = ratchet_storage::seaorm::connection::DatabaseConnection::new(storage_config).await?;
    let storage_factory = ratchet_storage::seaorm::repositories::RepositoryFactory::new(db_connection);
    Ok(Arc::new(crate::bridges::BridgeRepositoryFactory::new(Arc::new(storage_factory))))
}

/// Create task registry from configuration
async fn create_task_registry(_config: &ServerConfig) -> Result<Arc<dyn TaskRegistry>> {
    // Create a stub task registry for now
    Ok(Arc::new(StubTaskRegistry::new()))
}

/// Create registry manager from configuration
async fn create_registry_manager(_config: &ServerConfig) -> Result<Arc<dyn RegistryManager>> {
    // Create a stub registry manager for now
    Ok(Arc::new(StubRegistryManager::new()))
}

/// Create task validator from configuration
async fn create_task_validator(_config: &ServerConfig) -> Result<Arc<dyn TaskValidator>> {
    // Create a stub task validator for now
    Ok(Arc::new(StubTaskValidator::new()))
}

/// Initialize logging system
pub async fn init_logging(config: &ServerConfig) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let subscriber = tracing_subscriber::registry();

    // Add console layer
    let subscriber = subscriber.with(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
    );

    // Add file layer if enabled
    if config.logging.enable_file_logging {
        if let Some(file_path) = &config.logging.file_path {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;
            
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(file)
                .with_ansi(false);
                
            let subscriber = subscriber.with(file_layer);
            // Use try_init to avoid panic if global subscriber already set
            if let Err(_) = subscriber.try_init() {
                tracing::debug!("Global tracing subscriber already initialized, skipping");
            }
        } else {
            // Use try_init to avoid panic if global subscriber already set
            if let Err(_) = subscriber.try_init() {
                tracing::debug!("Global tracing subscriber already initialized, skipping");
            }
        }
    } else {
        // Use try_init to avoid panic if global subscriber already set
        if let Err(_) = subscriber.try_init() {
            tracing::debug!("Global tracing subscriber already initialized, skipping");
        }
    }

    tracing::info!("Logging initialized");
    Ok(())
}

// =============================================================================
// Stub Implementations (Temporary for migration phase)
// =============================================================================

/// Stub repository factory that returns empty results
pub struct StubRepositoryFactory;

impl StubRepositoryFactory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RepositoryFactory for StubRepositoryFactory {
    fn task_repository(&self) -> &dyn TaskRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn execution_repository(&self) -> &dyn ExecutionRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn job_repository(&self) -> &dyn JobRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    fn schedule_repository(&self) -> &dyn ScheduleRepository {
        unimplemented!("Repository factory stubs not implemented yet")
    }
    
    async fn health_check(&self) -> Result<(), DatabaseError> {
        Ok(())
    }
}

/// Stub task registry
pub struct StubTaskRegistry;

impl StubTaskRegistry {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskRegistry for StubTaskRegistry {
    async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>, RegistryError> {
        Ok(vec![])
    }
    
    async fn get_task_metadata(&self, _name: &str) -> Result<TaskMetadata, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn load_task_content(&self, _name: &str) -> Result<String, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn task_exists(&self, _name: &str) -> Result<bool, RegistryError> {
        Ok(false)
    }
    
    fn registry_id(&self) -> &str {
        "stub-registry"
    }
    
    async fn health_check(&self) -> Result<(), RegistryError> {
        Ok(())
    }
}

/// Stub registry manager
pub struct StubRegistryManager;

impl StubRegistryManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RegistryManager for StubRegistryManager {
    async fn add_registry(&self, _registry: Box<dyn TaskRegistry>) -> Result<(), RegistryError> {
        Ok(())
    }
    
    async fn remove_registry(&self, _registry_id: &str) -> Result<(), RegistryError> {
        Ok(())
    }
    
    async fn list_registries(&self) -> Vec<&str> {
        vec![]
    }
    
    async fn discover_all_tasks(&self) -> Result<Vec<(String, TaskMetadata)>, RegistryError> {
        Ok(vec![])
    }
    
    async fn find_task(&self, _name: &str) -> Result<(String, TaskMetadata), RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn load_task(&self, _name: &str) -> Result<String, RegistryError> {
        Err(RegistryError::TaskNotFound { name: "stub".to_string() })
    }
    
    async fn sync_with_database(&self) -> Result<SyncResult, RegistryError> {
        Ok(SyncResult {
            added: vec![],
            updated: vec![],
            removed: vec![],
            errors: vec![],
        })
    }
}

/// Stub task validator
pub struct StubTaskValidator;

impl StubTaskValidator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskValidator for StubTaskValidator {
    async fn validate_metadata(&self, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_content(&self, _content: &str, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
    
    async fn validate_input(&self, _input: &serde_json::Value, _metadata: &TaskMetadata) -> Result<ValidationResult, RegistryError> {
        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }
}