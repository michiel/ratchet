# Phase 5C: Extract Registry System to ratchet-registry

## Overview

Extract the task registry system from ratchet-lib into a standalone `ratchet-registry` crate. The registry system is responsible for task discovery, loading, validation, and synchronization between filesystem/HTTP sources and the database.

## Current Registry System Analysis

Based on analysis of `ratchet-lib/src/registry/`, the registry system includes:

### Core Components
- **Registry Service** (`registry.rs`) - Main registry management and task discovery
- **Task Loaders** (`loaders/`) - Filesystem and HTTP-based task loading
- **Service Layer** (`service.rs`) - High-level registry operations 
- **File Watcher** (`watcher.rs`) - Automatic task reloading on filesystem changes

### Key Features
- Task discovery from filesystem and HTTP endpoints
- Automatic validation of task schemas and metadata
- Database synchronization with conflict resolution
- Hot reloading via filesystem watchers
- Task versioning and availability tracking
- Caching for performance optimization

## Phase 5C Implementation Plan

### 5C.1: Create ratchet-registry Crate Structure

```bash
# Create the new crate
cargo new --lib ratchet-registry

# Directory structure to create:
ratchet-registry/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── config.rs
│   ├── registry.rs          # Core registry management
│   ├── service.rs           # High-level operations
│   ├── watcher.rs           # File system watching
│   ├── loaders/
│   │   ├── mod.rs
│   │   ├── filesystem.rs    # Filesystem task loading
│   │   ├── http.rs          # HTTP endpoint loading
│   │   └── validation.rs    # Task validation logic
│   ├── sync/
│   │   ├── mod.rs
│   │   ├── database.rs      # Database synchronization
│   │   └── conflict.rs      # Conflict resolution
│   └── cache/
│       ├── mod.rs
│       └── memory.rs        # In-memory caching
└── tests/
    ├── integration_test.rs
    └── fixtures/
        └── sample_tasks/
```

### 5C.2: Dependencies and Configuration

**Cargo.toml dependencies:**
```toml
[dependencies]
# Core dependencies
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true, features = ["fs", "time"] }
chrono = { workspace = true }
uuid = { workspace = true }

# Ratchet dependencies
ratchet-core = { path = "../ratchet-core" }
ratchet-storage = { path = "../ratchet-storage" }
ratchet-http = { path = "../ratchet-http" }
ratchet-config = { path = "../ratchet-config" }
ratchet-caching = { path = "../ratchet-caching" }

# Registry-specific dependencies
notify = { version = "6.1", features = ["serde"] }  # File watching
walkdir = "2.4"                                     # Directory traversal
zip = "0.6"                                         # ZIP file handling
jsonschema = { workspace = true }                   # Schema validation
regex = "1.10"                                      # Pattern matching
url = { workspace = true }                          # URL handling

[features]
default = ["filesystem", "http", "watcher"]
filesystem = ["dep:walkdir", "dep:zip"]
http = ["dep:reqwest"]
watcher = ["dep:notify"]
validation = ["dep:jsonschema"]
```

### 5C.3: Core Types and Traits

**Registry Configuration:**
```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub sources: Vec<TaskSource>,
    pub sync_interval: Duration,
    pub enable_auto_sync: bool,
    pub enable_validation: bool,
    pub cache_config: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskSource {
    #[serde(rename = "filesystem")]
    Filesystem { 
        path: String,
        recursive: bool,
        watch: bool,
    },
    #[serde(rename = "http")]
    Http { 
        url: String,
        auth: Option<HttpAuth>,
        polling_interval: Duration,
    },
}
```

**Core Registry Trait:**
```rust
// src/registry.rs
#[async_trait]
pub trait TaskRegistry: Send + Sync {
    async fn discover_tasks(&self) -> Result<Vec<DiscoveredTask>, RegistryError>;
    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition, RegistryError>;
    async fn validate_task(&self, task: &TaskDefinition) -> Result<ValidationResult, RegistryError>;
    async fn sync_with_database(&self) -> Result<SyncResult, RegistryError>;
    async fn get_task_versions(&self, name: &str) -> Result<Vec<String>, RegistryError>;
    async fn watch_for_changes(&self) -> Result<tokio::sync::mpsc::Receiver<RegistryEvent>, RegistryError>;
}
```

### 5C.4: Migration Steps

#### Step 1: Copy Core Registry Code
1. Copy `ratchet-lib/src/registry/` contents to `ratchet-registry/src/`
2. Update imports to use workspace dependencies
3. Replace ratchet-lib database calls with ratchet-storage
4. Extract configuration from ratchet-config domains

#### Step 2: Enhance Task Loading
```rust
// src/loaders/filesystem.rs
pub struct FilesystemLoader {
    base_path: PathBuf,
    recursive: bool,
    cache: Arc<dyn Cache<String, TaskDefinition>>,
}

impl FilesystemLoader {
    pub async fn discover_tasks(&self) -> Result<Vec<TaskReference>, LoaderError> {
        // Scan filesystem for task.json, metadata.json files
        // Support both individual files and ZIP archives
        // Validate directory structure
    }
    
    pub async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition, LoaderError> {
        // Load main.js, input.schema.json, output.schema.json, metadata.json
        // Handle ZIP archive extraction
        // Apply caching with TTL
    }
}
```

#### Step 3: HTTP Task Loading
```rust
// src/loaders/http.rs
pub struct HttpLoader {
    client: Arc<ratchet_http::Client>,
    base_url: Url,
    auth: Option<HttpAuth>,
    cache: Arc<dyn Cache<String, TaskDefinition>>,
}

impl HttpLoader {
    pub async fn discover_tasks(&self) -> Result<Vec<TaskReference>, LoaderError> {
        // GET /tasks endpoint for task discovery
        // Support pagination for large registries
        // Handle authentication
    }
    
    pub async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition, LoaderError> {
        // GET /tasks/{name}/{version} for task content
        // Support content-encoding (gzip, etc.)
        // Implement retry logic with exponential backoff
    }
}
```

#### Step 4: Database Synchronization
```rust
// src/sync/database.rs
pub struct DatabaseSync {
    task_repo: Arc<dyn ratchet_storage::TaskRepository>,
    conflict_resolver: ConflictResolver,
}

impl DatabaseSync {
    pub async fn sync_discovered_tasks(&self, tasks: Vec<DiscoveredTask>) -> Result<SyncResult, SyncError> {
        // Compare with existing database tasks
        // Handle version conflicts (newer, older, same)
        // Update task metadata and availability
        // Preserve execution history
    }
    
    pub async fn cleanup_removed_tasks(&self, active_tasks: &[TaskReference]) -> Result<(), SyncError> {
        // Mark tasks as unavailable if no longer in registry
        // Preserve historical data
        // Handle graceful deprecation
    }
}
```

#### Step 5: File System Watching
```rust
// src/watcher.rs
pub struct RegistryWatcher {
    watchers: Vec<RecommendedWatcher>,
    event_sender: mpsc::Sender<RegistryEvent>,
}

impl RegistryWatcher {
    pub async fn watch_sources(&mut self, sources: &[TaskSource]) -> Result<(), WatcherError> {
        // Set up filesystem watchers for each source
        // Handle create, modify, delete events
        // Debounce rapid changes
        // Filter relevant file types (.js, .json, .zip)
    }
    
    pub async fn handle_event(&self, event: notify::Event) -> Result<(), WatcherError> {
        // Convert notify events to RegistryEvent
        // Trigger task reloading
        // Update database synchronization
    }
}
```

### 5C.5: Integration with ratchet-lib

#### Update ratchet-lib Dependencies
```toml
# ratchet-lib/Cargo.toml
[dependencies]
ratchet-registry = { path = "../ratchet-registry" }
```

#### Create Compatibility Layer
```rust
// ratchet-lib/src/lib.rs
// Registry functionality moved to ratchet-registry crate
pub mod registry {
    pub use ratchet_registry::*;
    
    // Re-export for backward compatibility
    pub mod service {
        pub use ratchet_registry::service::*;
    }
    
    pub mod loaders {
        pub use ratchet_registry::loaders::*;
    }
}
```

#### Update Service Integration
```rust
// Update services that use registry
// ratchet-lib/src/services.rs
use ratchet_registry::{TaskRegistry, RegistryConfig};

impl RatchetEngine {
    pub async fn with_registry(mut self, config: RegistryConfig) -> Result<Self, ServiceError> {
        let registry = ratchet_registry::Registry::new(config).await?;
        self.registry = Some(Arc::new(registry));
        Ok(self)
    }
}
```

### 5C.6: Testing Strategy

#### Unit Tests
```rust
// tests/unit/
mod loaders {
    mod filesystem_test;
    mod http_test;
    mod validation_test;
}

mod sync {
    mod database_test;
    mod conflict_test;
}

mod registry_test;
mod watcher_test;
```

#### Integration Tests
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_full_registry_workflow() {
    // Set up test filesystem with sample tasks
    // Configure registry with multiple sources
    // Test discovery, loading, validation, sync
    // Verify database state after sync
}

#[tokio::test]
async fn test_conflict_resolution() {
    // Create conflicting task versions
    // Test various conflict scenarios
    // Verify resolution strategy
}

#[tokio::test]
async fn test_hot_reloading() {
    // Set up file watcher
    // Modify tasks in filesystem
    // Verify automatic reloading
    // Check database synchronization
}
```

#### Test Fixtures
```
tests/fixtures/
├── sample_tasks/
│   ├── simple-task/
│   │   ├── main.js
│   │   ├── metadata.json
│   │   ├── input.schema.json
│   │   └── output.schema.json
│   ├── complex-task.zip
│   └── invalid-task/
└── mock_http_registry/
    ├── tasks_endpoint.json
    └── task_definitions/
```

### 5C.7: Migration Validation

#### Verification Steps
1. **Functionality Preservation**: All existing registry features work
2. **Performance**: No regression in task loading/discovery performance
3. **API Compatibility**: Existing code using registry continues to work
4. **Configuration**: Registry config migrates cleanly from ratchet-config
5. **Database**: Task synchronization maintains data integrity
6. **Monitoring**: Registry operations properly logged and traced

#### Migration Checklist
- [ ] ratchet-registry crate created with full functionality
- [ ] All registry modules migrated from ratchet-lib
- [ ] Database integration updated to use ratchet-storage
- [ ] HTTP client updated to use ratchet-http
- [ ] Configuration updated to use ratchet-config
- [ ] File watching and hot reloading working
- [ ] Task validation with jsonschema integration
- [ ] Conflict resolution for task versions
- [ ] Caching layer for performance
- [ ] ratchet-lib updated to use ratchet-registry
- [ ] Backward compatibility maintained through re-exports
- [ ] All tests passing (unit and integration)
- [ ] Performance benchmarks equivalent to original
- [ ] Documentation updated

### 5C.8: Post-Migration Cleanup

#### Remove Registry Code from ratchet-lib
```rust
// Remove these files after successful migration:
// ratchet-lib/src/registry/registry.rs
// ratchet-lib/src/registry/service.rs
// ratchet-lib/src/registry/watcher.rs
// ratchet-lib/src/registry/loaders/
```

#### Update Documentation
- Update README.md with new crate structure
- Add ratchet-registry documentation
- Update migration guide
- Add examples for registry usage

### 5C.9: Future Enhancements

With ratchet-registry as a standalone crate, future enhancements become easier:

1. **Plugin System**: Custom task loaders
2. **Distributed Registry**: Multi-node task discovery
3. **Task Marketplace**: Public/private task sharing
4. **Versioning**: Semantic versioning support
5. **Security**: Task signing and verification
6. **Performance**: Distributed caching
7. **Monitoring**: Registry health metrics

## Success Criteria

Phase 5C is complete when:
- ✅ ratchet-registry crate fully functional and independent
- ✅ All registry functionality migrated from ratchet-lib
- ✅ Backward compatibility maintained through re-exports
- ✅ Database integration uses ratchet-storage
- ✅ HTTP operations use ratchet-http
- ✅ Configuration uses ratchet-config
- ✅ All tests passing with equivalent performance
- ✅ Registry code removed from ratchet-lib
- ✅ Clean workspace build with no circular dependencies

This completes the major ratchet-lib decomposition, leaving only core business logic and API implementations in the original crate.