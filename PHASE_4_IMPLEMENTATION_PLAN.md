# Phase 4 Implementation Plan: Extract Server Components

## Overview

Phase 4 focuses on extracting server components from ratchet-lib into modular crates, following the successful CLI migration in Phase 3. This phase will reduce server component coupling and create reusable API building blocks.

## Implementation Strategy

### Phase 4A: Foundation - Extract API Types (Immediate Priority)

**Goal**: Create `ratchet-api-types` crate with unified types used across REST and GraphQL APIs.

**New Crate**: `ratchet-api-types/`
```
ratchet-api-types/
├── Cargo.toml
└── src/
    ├── lib.rs          # Main exports and re-exports
    ├── ids.rs          # ApiId and identifier utilities
    ├── domain.rs       # Unified domain objects (Task, Execution, Job)
    ├── enums.rs        # Status enums and other domain enums
    ├── pagination.rs   # Pagination types and utilities
    ├── errors.rs       # API error types and conversions
    └── conversions.rs  # Type conversion utilities
```

**Extraction Targets from ratchet-lib**:
- `src/api/types.rs` → Multiple domain-specific modules
- `src/api/errors.rs` → `errors.rs`
- `src/api/pagination.rs` → `pagination.rs` 
- `src/api/conversions.rs` → `conversions.rs`

**Dependencies**:
- `serde` and `serde_json` for serialization
- `uuid` for identifier handling
- `chrono` for datetime types
- `async-graphql` for GraphQL type attributes

**Benefits**:
- Shared types between REST and GraphQL APIs
- Foundation for future API crate extractions
- Reduced duplication and improved consistency

### Phase 4B: Interfaces - Create Repository Traits (High Priority)

**Goal**: Define clean interfaces to break database coupling.

**New Crate**: `ratchet-interfaces/`
```
ratchet-interfaces/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── repositories.rs  # Repository trait definitions
    ├── execution.rs     # Task execution traits
    ├── registry.rs      # Task registry traits
    └── services.rs      # Service layer traits
```

**Key Traits to Define**:
```rust
// repositories.rs
pub trait TaskRepository: Send + Sync {
    async fn get_task(&self, id: &str) -> Result<UnifiedTask>;
    async fn list_tasks(&self, pagination: PaginationInput) -> Result<ListResponse<UnifiedTask>>;
    // ... other methods
}

pub trait ExecutionRepository: Send + Sync {
    async fn get_execution(&self, id: &str) -> Result<UnifiedExecution>;
    async fn create_execution(&self, execution: CreateExecutionRequest) -> Result<UnifiedExecution>;
    // ... other methods
}

// execution.rs
pub trait TaskExecutor: Send + Sync {
    async fn execute_task(&self, task_id: &str, input: serde_json::Value) -> Result<ExecutionResult>;
    async fn validate_task(&self, task_id: &str) -> Result<ValidationResult>;
}

// registry.rs  
pub trait TaskRegistry: Send + Sync {
    async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>>;
    async fn get_task_metadata(&self, id: &str) -> Result<TaskMetadata>;
}
```

**Benefits**:
- Breaks circular dependencies through interface segregation
- Enables dependency injection for testing
- Allows different implementations (database, in-memory, etc.)

### Phase 4C: Web Infrastructure - Extract Middleware (Medium Priority)

**Goal**: Create reusable web middleware and utilities.

**New Crate**: `ratchet-web/`
```
ratchet-web/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── middleware/
    │   ├── mod.rs
    │   ├── cors.rs         # CORS middleware
    │   ├── error_handler.rs # Error response formatting
    │   ├── rate_limit.rs   # Rate limiting
    │   ├── request_id.rs   # Request ID tracing
    │   └── pagination.rs   # Pagination middleware
    ├── extractors/
    │   ├── mod.rs
    │   ├── pagination.rs   # Pagination extractors
    │   └── query.rs        # Query parameter extractors
    └── utils/
        ├── mod.rs
        └── response.rs     # Response utilities
```

**Extraction Targets**:
- `ratchet-lib/src/rest/middleware/` → `middleware/`
- `ratchet-lib/src/rest/extractors/` → `extractors/`

**Dependencies**:
- `axum` for web framework integration
- `tower` and `tower-http` for middleware stack
- `ratchet-api-types` for error types

### Phase 4D: REST API Extraction (Medium Priority)

**Goal**: Extract REST API into standalone crate.

**New Crate**: `ratchet-rest-api/`
```
ratchet-rest-api/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── app.rs              # REST app factory
    ├── handlers/
    │   ├── mod.rs
    │   ├── tasks.rs        # Task endpoints
    │   ├── executions.rs   # Execution endpoints
    │   ├── jobs.rs         # Job endpoints
    │   ├── schedules.rs    # Schedule endpoints
    │   └── workers.rs      # Worker status endpoints
    └── models/
        ├── mod.rs
        ├── requests.rs     # Request types
        └── responses.rs    # Response types
```

**Dependencies**:
- `ratchet-api-types` for domain types
- `ratchet-interfaces` for repository traits
- `ratchet-web` for middleware and utilities
- `axum` for HTTP framework

**Architecture**:
```rust
// app.rs
pub fn create_rest_app<T, E, J>(
    task_repo: Arc<T>,
    execution_repo: Arc<E>, 
    job_repo: Arc<J>,
) -> Router
where
    T: TaskRepository,
    E: ExecutionRepository,
    J: JobRepository,
{
    // Route setup using injected repositories
}
```

### Phase 4E: GraphQL API Extraction (Medium Priority)

**Goal**: Extract GraphQL API into standalone crate.

**New Crate**: `ratchet-graphql-api/`
```
ratchet-graphql-api/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── schema.rs       # GraphQL schema builder
    ├── resolvers/
    │   ├── mod.rs
    │   ├── query.rs    # Query resolvers
    │   ├── mutation.rs # Mutation resolvers
    │   └── types.rs    # GraphQL-specific types
    └── context.rs      # GraphQL context setup
```

**Dependencies**:
- `ratchet-api-types` for domain types
- `ratchet-interfaces` for repository traits
- `async-graphql` for GraphQL implementation

### Phase 4F: Unified Server (Low Priority)

**Goal**: Create server crate that combines REST and GraphQL APIs.

**New Crate**: `ratchet-server/`
```
ratchet-server/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── app.rs          # Combined app builder
    ├── config.rs       # Server configuration
    └── startup.rs      # Server startup logic
```

## Migration Steps

### Step 1: Create ratchet-api-types
1. Create new crate with basic structure
2. Copy and refactor API types from ratchet-lib
3. Update dependencies in workspace Cargo.toml
4. Add appropriate feature flags

### Step 2: Create ratchet-interfaces  
1. Create trait definitions based on current concrete types
2. Start with TaskRepository and ExecutionRepository
3. Ensure traits are Send + Sync for async usage

### Step 3: Update ratchet-lib to use new types
1. Replace ratchet-lib API types with ratchet-api-types
2. Implement repository traits on existing repositories
3. Ensure backward compatibility

### Step 4: Extract REST API
1. Create ratchet-rest-api crate
2. Move handlers and models
3. Use dependency injection for repositories
4. Update CLI to use new REST API crate

### Step 5: Extract GraphQL API
1. Create ratchet-graphql-api crate
2. Move schema and resolvers
3. Use dependency injection pattern

### Step 6: Create unified server
1. Combine REST and GraphQL into single server
2. Maintain backward compatibility with existing serve command

## Testing Strategy

### Unit Testing
- Each extracted crate should have comprehensive unit tests
- Mock implementations of repository traits for testing
- Isolated testing of API endpoints

### Integration Testing
- Test API endpoints with real database backends
- Verify backward compatibility with existing functionality
- Performance testing to ensure no regressions

### Migration Testing
- Gradual migration with feature flags
- Side-by-side testing of old vs new implementations
- Rollback capability at each step

## Backward Compatibility

### Approach
- Maintain existing ratchet-lib exports during transition
- Use re-exports to maintain API compatibility
- Gradual deprecation of old APIs with clear migration paths

### Version Strategy
- Mark this as a minor version bump (0.x.y)
- Use feature flags to enable new modular components
- Provide clear upgrade documentation

## Risk Mitigation

### High-Risk Areas
1. **Database coupling**: Repository traits must cover all existing functionality
2. **Type conversions**: Ensure no data loss during type migrations
3. **Performance**: Trait dynamic dispatch overhead

### Mitigation Strategies
1. Comprehensive trait design with existing usage analysis
2. Automated conversion testing
3. Benchmarking before and after extraction

## Success Metrics

### Technical Metrics
- Reduced circular dependencies (measured by dependency graph analysis)
- Improved compilation times for individual components
- Reduced ratchet-lib size and complexity

### Quality Metrics
- Maintained test coverage (>90%)
- No performance regressions (within 5%)
- Successful backward compatibility (existing APIs continue to work)

## Timeline Estimate

- **Phase 4A (API Types)**: 2-3 days
- **Phase 4B (Interfaces)**: 2-3 days  
- **Phase 4C (Web Infrastructure)**: 3-4 days
- **Phase 4D (REST API)**: 4-5 days
- **Phase 4E (GraphQL API)**: 4-5 days
- **Phase 4F (Unified Server)**: 2-3 days

**Total**: ~3 weeks for complete Phase 4 implementation

## Next Steps

1. Begin with Phase 4A (API types extraction) as it provides immediate value and foundation
2. Create feature branch: `feature/phase4-extract-server-components`
3. Implement each phase incrementally with testing at each step
4. Document API changes and migration guides for users