//! Service implementations and dependency injection setup

use std::sync::Arc;
use anyhow::Result;

use ratchet_interfaces::{
    RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator
};
use ratchet_rest_api::context::TasksContext;
// use ratchet_graphql_api::context::GraphQLContext; // Temporarily disabled

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

    // Create GraphQL context from service container (temporarily disabled)
    // pub fn graphql_context(&self) -> GraphQLContext {
    //     GraphQLContext {
    //         repositories: self.repositories.clone(),
    //         registry: self.registry.clone(),
    //         registry_manager: self.registry_manager.clone(),
    //         validator: self.validator.clone(),
    //     }
    // }
}

/// Create repository factory from configuration
async fn create_repository_factory(_config: &ServerConfig) -> Result<Arc<dyn RepositoryFactory>> {
    // For now, use the legacy implementation as a bridge
    // This would be replaced with a proper implementation based on ratchet-storage
    
    // Placeholder: In a real implementation, this would:
    // 1. Set up database connection pool
    // 2. Run migrations if enabled
    // 3. Create repository implementations
    // 4. Return repository factory
    
    todo!("Implement repository factory creation - bridge to ratchet-lib for now")
}

/// Create task registry from configuration
async fn create_task_registry(_config: &ServerConfig) -> Result<Arc<dyn TaskRegistry>> {
    // Placeholder: Would create filesystem/HTTP registry implementations
    todo!("Implement task registry creation - bridge to ratchet-lib for now")
}

/// Create registry manager from configuration
async fn create_registry_manager(_config: &ServerConfig) -> Result<Arc<dyn RegistryManager>> {
    // Placeholder: Would create registry manager with sync capabilities
    todo!("Implement registry manager creation - bridge to ratchet-lib for now")
}

/// Create task validator from configuration
async fn create_task_validator(_config: &ServerConfig) -> Result<Arc<dyn TaskValidator>> {
    // Placeholder: Would create task validator with schema validation
    todo!("Implement task validator creation - bridge to ratchet-lib for now")
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
            subscriber.init();
        } else {
            subscriber.init();
        }
    } else {
        subscriber.init();
    }

    tracing::info!("Logging initialized");
    Ok(())
}