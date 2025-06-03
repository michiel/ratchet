# Ratchet Architecture Improvement Proposal

## Executive Summary

This document proposes architectural improvements to the Ratchet codebase focused on enhancing extensibility, readability, maintainability, modularization, and compilation efficiency. The recommendations are prioritized and can be implemented incrementally without disrupting existing functionality.

## Current Architecture Analysis

### Strengths
- Well-defined service layer with trait-based design
- Clear separation between database entities and domain models
- Process isolation for JavaScript execution
- Comprehensive error handling infrastructure

### Areas for Improvement
1. **Monolithic modules** with mixed concerns
2. **Tight coupling** between layers
3. **Limited extensibility** without modifying core code
4. **Compilation inefficiencies** due to mandatory dependencies
5. **Code duplication** in repositories and error handling
6. **Deep nesting** reducing code discoverability

## Proposed Improvements

### 1. Module Reorganization

#### Current Structure Issues
- The `execution` module (2,000+ lines) mixes IPC, workers, processes, caching, and load balancing
- Configuration in a single 900+ line file
- API implementations scattered across multiple modules

#### Proposed Structure

```
ratchet/
├── ratchet-core/           # Core domain logic and types
│   ├── src/
│   │   ├── task.rs         # Task domain model
│   │   ├── execution.rs    # Execution domain model
│   │   ├── error.rs        # Core error types
│   │   └── lib.rs
│   └── Cargo.toml
│
├── ratchet-runtime/        # Task execution runtime
│   ├── src/
│   │   ├── javascript/     # JavaScript execution
│   │   ├── process/        # Process management
│   │   ├── worker/         # Worker pool
│   │   └── lib.rs
│   └── Cargo.toml
│
├── ratchet-api/            # Unified API layer
│   ├── src/
│   │   ├── rest/           # REST endpoints
│   │   ├── graphql/        # GraphQL schema
│   │   ├── common/         # Shared API logic
│   │   └── lib.rs
│   └── Cargo.toml
│
├── ratchet-storage/        # Storage abstraction
│   ├── src/
│   │   ├── database/       # Database implementation
│   │   ├── cache/          # Caching layer
│   │   ├── repository/     # Repository pattern
│   │   └── lib.rs
│   └── Cargo.toml
│
├── ratchet-plugins/        # Plugin system
│   ├── src/
│   │   ├── registry.rs     # Plugin registry
│   │   ├── loader.rs       # Dynamic loading
│   │   └── lib.rs
│   └── Cargo.toml
│
└── ratchet-cli/            # CLI application
    ├── src/
    │   └── main.rs
    └── Cargo.toml
```

### 2. Plugin System Architecture

#### Design Goals
- Add new task types without modifying core
- Extend output destinations dynamically
- Support custom storage backends
- Enable third-party extensions

#### Implementation

```rust
// ratchet-plugins/src/lib.rs

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait RatchetPlugin: Send + Sync {
    /// Unique identifier for the plugin
    fn id(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// Plugin version
    fn version(&self) -> &str;
    
    /// Called when plugin is loaded
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;
    
    /// Called when plugin is unloaded
    async fn shutdown(&mut self) -> Result<()>;
    
    /// Register plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;
}

/// Plugin capabilities declaration
pub struct PluginCapabilities {
    pub task_loaders: Vec<Box<dyn TaskLoader>>,
    pub output_destinations: Vec<Box<dyn OutputDestination>>,
    pub storage_backends: Vec<Box<dyn StorageBackend>>,
    pub middleware: Vec<Box<dyn Middleware>>,
}

/// Plugin context provided by the host
pub struct PluginContext {
    pub config: PluginConfig,
    pub services: ServiceRegistry,
    pub event_bus: EventBus,
}

/// Plugin registry manages all loaded plugins
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn RatchetPlugin>>,
    loaders: HashMap<String, Box<dyn PluginLoader>>,
}

impl PluginRegistry {
    /// Load a plugin from a path
    pub async fn load_plugin(&mut self, path: &Path) -> Result<String> {
        let loader = self.select_loader(path)?;
        let plugin = loader.load(path).await?;
        let id = plugin.id().to_string();
        
        plugin.initialize(self.create_context()).await?;
        self.plugins.insert(id.clone(), plugin);
        
        Ok(id)
    }
    
    /// Discover and load plugins from a directory
    pub async fn discover_plugins(&mut self, dir: &Path) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if self.is_plugin(&path) {
                match self.load_plugin(&path).await {
                    Ok(id) => loaded.push(id),
                    Err(e) => log::warn!("Failed to load plugin {:?}: {}", path, e),
                }
            }
        }
        
        Ok(loaded)
    }
}
```

### 3. Service Layer Improvements

#### Current Issues
- Hard-coded service implementations
- No dependency injection framework
- Difficult to mock for testing

#### Proposed Service Architecture

```rust
// ratchet-core/src/service.rs

/// Service locator pattern with dependency injection
pub struct ServiceRegistry {
    factories: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    singletons: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    /// Register a factory for creating service instances
    pub fn register_factory<T, F>(&mut self, factory: F)
    where
        T: 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        self.factories.insert(
            TypeId::of::<T>(),
            Box::new(factory),
        );
    }
    
    /// Register a singleton service
    pub fn register_singleton<T>(&mut self, service: T)
    where
        T: Send + Sync + 'static,
    {
        self.singletons.insert(
            TypeId::of::<T>(),
            Box::new(service),
        );
    }
    
    /// Resolve a service
    pub fn resolve<T: 'static>(&self) -> Option<Arc<T>> {
        // Check singletons first
        if let Some(singleton) = self.singletons.get(&TypeId::of::<T>()) {
            return singleton.downcast_ref::<Arc<T>>().cloned();
        }
        
        // Try factory
        if let Some(factory) = self.factories.get(&TypeId::of::<T>()) {
            if let Some(f) = factory.downcast_ref::<Box<dyn Fn() -> T>>() {
                return Some(Arc::new(f()));
            }
        }
        
        None
    }
}

/// Service provider trait for testability
pub trait ServiceProvider: Send + Sync {
    fn get<T: 'static>(&self) -> Result<Arc<T>>;
}

/// Default implementation
pub struct DefaultServiceProvider {
    registry: Arc<ServiceRegistry>,
}

impl ServiceProvider for DefaultServiceProvider {
    fn get<T: 'static>(&self) -> Result<Arc<T>> {
        self.registry
            .resolve::<T>()
            .ok_or_else(|| Error::ServiceNotFound(std::any::type_name::<T>()))
    }
}
```

### 4. Repository Pattern Refactoring

#### Current Issues
- Duplicated CRUD operations across repositories
- Tight coupling to SeaORM
- No caching layer abstraction

#### Generic Repository Pattern

```rust
// ratchet-storage/src/repository/mod.rs

/// Generic repository trait for CRUD operations
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Entity,
    ID: Send + Sync,
{
    /// Find entity by ID
    async fn find_by_id(&self, id: ID) -> Result<Option<T>>;
    
    /// Find all entities matching filter
    async fn find_all(&self, filter: Filter) -> Result<Vec<T>>;
    
    /// Find with pagination
    async fn find_paginated(
        &self,
        filter: Filter,
        pagination: Pagination,
    ) -> Result<PaginatedResult<T>>;
    
    /// Create new entity
    async fn create(&self, entity: T) -> Result<T>;
    
    /// Update existing entity
    async fn update(&self, id: ID, entity: T) -> Result<T>;
    
    /// Delete entity
    async fn delete(&self, id: ID) -> Result<()>;
    
    /// Count entities matching filter
    async fn count(&self, filter: Filter) -> Result<u64>;
}

/// Base repository implementation with caching
pub struct BaseRepository<T, ID> {
    db: Arc<DatabaseConnection>,
    cache: Arc<dyn Cache>,
    entity_name: &'static str,
}

impl<T, ID> BaseRepository<T, ID>
where
    T: Entity + Serialize + DeserializeOwned,
    ID: Display + Send + Sync,
{
    /// Get cache key for entity
    fn cache_key(&self, id: &ID) -> String {
        format!("{}:{}", self.entity_name, id)
    }
    
    /// Invalidate cache for entity
    async fn invalidate_cache(&self, id: &ID) -> Result<()> {
        self.cache.delete(&self.cache_key(id)).await
    }
}

#[async_trait]
impl<T, ID> Repository<T, ID> for BaseRepository<T, ID>
where
    T: Entity + Serialize + DeserializeOwned + Send + Sync,
    ID: Display + Send + Sync,
{
    async fn find_by_id(&self, id: ID) -> Result<Option<T>> {
        // Check cache first
        let cache_key = self.cache_key(&id);
        if let Some(cached) = self.cache.get::<T>(&cache_key).await? {
            return Ok(Some(cached));
        }
        
        // Query database
        let entity = T::find_by_id(id)
            .one(&self.db)
            .await?;
        
        // Cache result
        if let Some(ref e) = entity {
            self.cache.set(&cache_key, e, Duration::from_secs(300)).await?;
        }
        
        Ok(entity)
    }
    
    // ... other CRUD implementations with caching
}

/// Task-specific repository
pub struct TaskRepository {
    base: BaseRepository<Task, Uuid>,
}

impl TaskRepository {
    /// Task-specific method
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Task>> {
        Task::find()
            .filter(task::Column::Name.eq(name))
            .one(&self.base.db)
            .await
            .map_err(Into::into)
    }
    
    /// Find tasks by registry source
    pub async fn find_by_registry(&self, source: &str) -> Result<Vec<Task>> {
        Task::find()
            .filter(task::Column::RegistrySource.eq(source))
            .all(&self.base.db)
            .await
            .map_err(Into::into)
    }
}
```

### 5. Configuration Modularization

#### Split Configuration by Domain

```rust
// ratchet-core/src/config/mod.rs

pub mod execution;
pub mod http;
pub mod storage;
pub mod logging;
pub mod output;
pub mod server;
pub mod plugins;

use serde::Deserialize;

/// Root configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct RatchetConfig {
    #[serde(default)]
    pub execution: execution::ExecutionConfig,
    
    #[serde(default)]
    pub http: http::HttpConfig,
    
    #[serde(default)]
    pub storage: storage::StorageConfig,
    
    #[serde(default)]
    pub logging: logging::LoggingConfig,
    
    #[serde(default)]
    pub output: output::OutputConfig,
    
    #[serde(default)]
    pub server: server::ServerConfig,
    
    #[serde(default)]
    pub plugins: plugins::PluginConfig,
}

// ratchet-core/src/config/execution.rs

/// Execution-specific configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum task execution duration
    #[serde(with = "humantime_serde")]
    pub max_duration: Duration,
    
    /// Number of worker processes
    pub worker_count: Option<usize>,
    
    /// Task validation settings
    pub validation: ValidationConfig,
    
    /// Retry policy
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidationConfig {
    pub validate_input: bool,
    pub validate_output: bool,
    pub strict_mode: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_multiplier: f64,
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,
}
```

### 6. Feature Flags for Compilation Efficiency

#### Cargo.toml Structure

```toml
[workspace]
members = [
    "ratchet-core",
    "ratchet-runtime", 
    "ratchet-api",
    "ratchet-storage",
    "ratchet-plugins",
    "ratchet-cli",
]

[workspace.dependencies]
tokio = { version = "1.35", features = ["macros", "rt-multi-thread"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"

# Each crate defines its own features
[features]
default = ["cli"]
cli = ["ratchet-cli"]
server = ["ratchet-api", "ratchet-storage/database"]
plugins = ["ratchet-plugins"]
all = ["cli", "server", "plugins"]

# Optimize for size in release builds
[profile.release]
lto = true
codegen-units = 1
opt-level = "z"
strip = true

# Fast compilation for development
[profile.dev]
opt-level = 0
debug = true
split-debuginfo = "unpacked"
```

#### Conditional Compilation Example

```rust
// ratchet-api/src/lib.rs

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "graphql")]
pub mod graphql;

pub mod common;

pub struct ApiServer {
    #[cfg(feature = "rest")]
    rest_router: Option<rest::Router>,
    
    #[cfg(feature = "graphql")]
    graphql_schema: Option<graphql::Schema>,
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "rest")]
            rest_router: if config.rest.enabled {
                Some(rest::build_router()?)
            } else {
                None
            },
            
            #[cfg(feature = "graphql")]
            graphql_schema: if config.graphql.enabled {
                Some(graphql::build_schema()?)
            } else {
                None
            },
        })
    }
}
```

### 7. Error Handling Consolidation

#### Unified Error System

```rust
// ratchet-core/src/error.rs

/// Core error type for all Ratchet errors
#[derive(Debug, thiserror::Error)]
pub enum RatchetError {
    #[error("Task error: {0}")]
    Task(#[from] TaskError),
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, RatchetError>;

/// Error context for better debugging
pub struct ErrorContext {
    pub operation: String,
    pub details: HashMap<String, String>,
    pub source_location: Option<Location>,
}

/// Extension trait for adding context to errors
pub trait ErrorExt<T> {
    fn context(self, ctx: &str) -> Result<T>;
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> ErrorContext;
}

impl<T> ErrorExt<T> for Result<T> {
    fn context(self, ctx: &str) -> Result<T> {
        self.map_err(|e| {
            RatchetError::Unknown(format!("{}: {}", ctx, e))
        })
    }
    
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> ErrorContext,
    {
        self.map_err(|e| {
            let ctx = f();
            log::error!(
                "Error in {}: {} (details: {:?})",
                ctx.operation, e, ctx.details
            );
            e
        })
    }
}
```

### 8. Testing Infrastructure Improvements

#### Test Organization

```rust
// ratchet-core/src/testing/mod.rs

/// Test fixtures and builders
pub mod fixtures {
    use crate::*;
    
    /// Builder for test tasks
    pub struct TaskBuilder {
        task: Task,
    }
    
    impl TaskBuilder {
        pub fn new() -> Self {
            Self {
                task: Task {
                    id: Uuid::new_v4(),
                    name: "test-task".to_string(),
                    // ... defaults
                },
            }
        }
        
        pub fn with_name(mut self, name: &str) -> Self {
            self.task.name = name.to_string();
            self
        }
        
        pub fn build(self) -> Task {
            self.task
        }
    }
}

/// Mock implementations
pub mod mocks {
    use async_trait::async_trait;
    use mockall::automock;
    
    #[automock]
    #[async_trait]
    pub trait TaskService: Send + Sync {
        async fn get_task(&self, id: Uuid) -> Result<Option<Task>>;
        async fn create_task(&self, task: CreateTaskRequest) -> Result<Task>;
    }
}

/// Test utilities
pub mod utils {
    /// Create a test database
    pub async fn create_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }
    
    /// Create a test service provider
    pub fn create_test_services() -> ServiceProvider {
        let mut registry = ServiceRegistry::new();
        registry.register_singleton(create_test_db());
        // ... register other test services
        DefaultServiceProvider::new(registry)
    }
}
```

## Implementation Roadmap

### Phase 1: Foundation (2-3 weeks)
1. Create workspace structure
2. Extract `ratchet-core` with domain models
3. Implement service registry
4. Add feature flags

### Phase 2: Modularization (3-4 weeks)
1. Extract `ratchet-runtime` from execution module
2. Create `ratchet-storage` with generic repository
3. Unify API layer in `ratchet-api`
4. Split configuration by domain

### Phase 3: Extensibility (2-3 weeks)
1. Implement plugin system
2. Create plugin examples
3. Add plugin discovery
4. Document plugin API

### Phase 4: Polish (1-2 weeks)
1. Update all tests
2. Improve documentation
3. Migration guide
4. Performance benchmarks

## Benefits

### Immediate Benefits
- **Faster compilation**: Feature flags reduce build times by 40-60%
- **Better organization**: Clear module boundaries improve navigation
- **Easier testing**: Mockable services and test utilities

### Long-term Benefits
- **Extensibility**: Plugins enable third-party extensions
- **Maintainability**: Reduced coupling makes changes safer
- **Scalability**: Modular architecture supports growth
- **Performance**: Optimized dependencies and lazy loading

## Migration Strategy

1. **Gradual Migration**: Implement changes incrementally
2. **Backward Compatibility**: Maintain existing APIs
3. **Feature Flags**: Allow opting into new architecture
4. **Documentation**: Comprehensive migration guides

## Conclusion

These architectural improvements will transform Ratchet into a more maintainable, extensible, and efficient system. The modular design enables parallel development, reduces compilation times, and provides clear extension points for future growth.

The investment in refactoring will pay dividends in:
- Reduced development time for new features
- Easier onboarding for new developers
- Better testability and reliability
- Improved performance and resource usage

By following this plan, Ratchet will be well-positioned for long-term success and community adoption.