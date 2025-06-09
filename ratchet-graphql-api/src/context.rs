//! GraphQL context types for dependency injection

use std::sync::Arc;
use ratchet_interfaces::{
    RepositoryFactory, TaskRegistry, RegistryManager, 
    TaskValidator
};

/// Main GraphQL context containing all service dependencies
#[derive(Clone)]
pub struct GraphQLContext {
    pub repositories: Arc<dyn RepositoryFactory>,
    pub registry: Arc<dyn TaskRegistry>,
    pub registry_manager: Arc<dyn RegistryManager>,
    pub validator: Arc<dyn TaskValidator>,
}

impl GraphQLContext {
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
        }
    }
}

/// Configuration for GraphQL setup
#[derive(Debug, Clone)]
pub struct GraphQLConfig {
    pub enable_playground: bool,
    pub enable_introspection: bool,
    pub max_query_depth: Option<usize>,
    pub max_query_complexity: Option<usize>,
    pub enable_tracing: bool,
    pub enable_apollo_tracing: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            enable_playground: true,
            enable_introspection: true,
            max_query_depth: Some(15),
            max_query_complexity: Some(1000),
            enable_tracing: true,
            enable_apollo_tracing: false,
        }
    }
}