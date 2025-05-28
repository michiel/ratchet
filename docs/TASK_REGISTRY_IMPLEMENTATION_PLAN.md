# Task Registry Implementation Plan

## Overview
Implement a task registry system that can load tasks from HTTP endpoints or filesystem locations (directories, zip files, or collections). The registry will manage task versions and provide GraphQL API access.

## Architecture

### 1. Core Components

#### TaskSource Enum
```rust
enum TaskSource {
    Filesystem { path: PathBuf },
    Http { url: String }, // Stub for now
}
```

#### TaskRegistry Structure
```rust
struct TaskRegistry {
    // Map of task_id -> Map of version -> Task
    tasks: HashMap<Uuid, HashMap<String, Arc<Task>>>,
    sources: Vec<TaskSource>,
}
```

#### TaskLoader Trait
```rust
trait TaskLoader {
    async fn load_tasks(&self, source: &TaskSource) -> Result<Vec<Task>>;
}
```

### 2. Implementation Phases

#### Phase 1: Core Registry Infrastructure
- Create `registry.rs` module in `ratchet-lib/src/`
- Implement `TaskRegistry` with basic CRUD operations
- Add version conflict detection with warning logs
- Implement thread-safe access with `Arc<RwLock<TaskRegistry>>`

#### Phase 2: Filesystem Loader
- Implement `FilesystemTaskLoader` in `registry/loaders/filesystem.rs`
- Support loading from:
  - Single task directory
  - Single task ZIP file
  - Directory containing multiple tasks (dirs/zips)
- Recursive task discovery with proper error handling

#### Phase 3: HTTP Loader Stub
- Create `HttpTaskLoader` stub in `registry/loaders/http.rs`
- Return `NotImplemented` error for now
- Define interface for future implementation

#### Phase 4: Registry Service Integration
- Add `RegistryService` to services module
- Integrate with `RatchetEngine`
- Add registry initialization on startup
- Configure sources from config file

#### Phase 5: GraphQL API
- Add registry queries to GraphQL schema:
  ```graphql
  type Query {
    tasks: [Task!]!
    task(id: ID!, version: String): Task
    taskVersions(id: ID!): [String!]!
  }
  
  type Task {
    id: ID!
    version: String!
    label: String!
    description: String!
    availableVersions: [String!]!
  }
  ```

### 3. Configuration Format

```yaml
# example-config.yaml
registry:
  sources:
    - type: filesystem
      path: ./sample/js-tasks
    - type: filesystem
      path: /opt/ratchet/tasks
    - type: http
      url: https://registry.example.com/tasks  # Stub
```

### 4. File Structure

```
ratchet-lib/src/
├── registry/
│   ├── mod.rs
│   ├── registry.rs      # Core TaskRegistry implementation
│   ├── service.rs       # RegistryService trait and implementation
│   └── loaders/
│       ├── mod.rs
│       ├── filesystem.rs # FilesystemTaskLoader
│       └── http.rs      # HttpTaskLoader (stub)
```

### 5. Key Features

1. **Version Management**
   - Store multiple versions of same task
   - Warn on duplicate version attempts
   - Default to latest version when not specified

2. **Lazy Loading**
   - Load task metadata eagerly
   - Load task content on demand

3. **Error Handling**
   - Continue loading on individual task errors
   - Collect and report all errors at end
   - Log warnings for duplicate versions

4. **Thread Safety**
   - Use `Arc<RwLock<TaskRegistry>>` for concurrent access
   - Immutable task references once loaded

### 6. Implementation Order

1. Create registry module structure
2. Implement core `TaskRegistry` with version management
3. Implement `FilesystemTaskLoader` with all file type support
4. Create HTTP loader stub
5. Add configuration parsing for registry sources
6. Integrate registry into services and engine
7. Add GraphQL schema and resolvers
8. Update example config with registry section

### 7. Testing Strategy

1. Unit tests for registry operations
2. Integration tests for filesystem loading
3. Tests for version conflict detection
4. GraphQL API tests
5. Configuration parsing tests

### 8. Migration Notes

- Existing task loading code remains unchanged
- Registry is additive - doesn't break existing functionality
- Tasks can still be loaded directly by path