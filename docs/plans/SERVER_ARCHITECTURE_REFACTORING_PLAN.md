# Server Architecture Refactoring Plan

> **Status**: This plan was created during the architecture migration process. The core issues identified have since been resolved through infrastructure extraction and the current architecture is the intended target state. See [ARCHITECTURE_MIGRATION_ANALYSIS.md](../ARCHITECTURE_MIGRATION_ANALYSIS.md) for the final architecture.

## Executive Summary

This document outlines a comprehensive plan to refactor the Ratchet server architecture, breaking down the monolithic `ratchet-lib` server components into focused, reusable crates. This refactoring builds on successful extractions of HTTP, logging, JavaScript execution, and task consolidation components.

## Current Architecture Analysis

### Existing Server Components

```
ratchet-lib/
├── src/server/           # Core server functionality
├── src/rest/            # REST API implementation
├── src/graphql/         # GraphQL server implementation
└── src/services.rs      # Service layer

ratchet-mcp/
└── src/server/          # MCP server implementation
```

### Identified Issues

1. **Monolithic Structure**
   - `ratchet-lib` contains 15,000+ lines of server code
   - Multiple server types coupled in single crate
   - Difficult to use individual server components

2. **Circular Dependencies**
   - `ratchet-mcp` → `ratchet_lib` → storage/execution dependencies
   - Compilation complexity and slow build times
   - Limited modularity

3. **Tight Coupling**
   - Shared state structures across server types
   - No clear interfaces between components
   - Difficult to test in isolation

4. **Mixed Abstraction Levels**
   - Business logic intermingled with HTTP handling
   - Infrastructure concerns mixed with domain logic
   - Unclear separation of responsibilities

## Proposed Architecture

### Target Structure

```
ratchet-server-core/     # Server abstractions and traits
ratchet-rest/           # REST API server
ratchet-graphql/        # GraphQL server
ratchet-servers/        # Multi-server orchestration
ratchet-mcp/           # MCP server (isolated)
```

### Architectural Principles

1. **Single Responsibility**: Each crate has one clear purpose
2. **Dependency Inversion**: Depend on abstractions, not concretions
3. **Interface Segregation**: Small, focused interfaces
4. **Open/Closed**: Extensible without modification
5. **Separation of Concerns**: Clear boundaries between layers

## Refactoring Plan

### Phase 1: Server Core Extraction (Week 1-2)

**Objective**: Create foundational server abstractions

**Tasks**:
1. Create `ratchet-server-core` crate
2. Define core server traits and types
3. Extract common server functionality

**Deliverables**:
```rust
// ratchet-server-core/src/lib.rs
pub trait Server {
    type Config;
    type Error;
    
    async fn start(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
    fn health(&self) -> HealthStatus;
}

pub trait RequestHandler<Req, Resp> {
    type Error;
    async fn handle(&self, request: Req) -> Result<Resp, Self::Error>;
}

pub struct ServerState {
    storage: Arc<dyn StorageFactory>,
    execution: Arc<dyn ExecutionEngine>,
    config: ServerConfig,
}
```

**Dependencies**:
- `ratchet-core` (for types)
- `ratchet-storage` (for traits)
- `ratchet-execution` (for traits)

### Phase 2: REST API Extraction (Week 3-4)

**Objective**: Extract REST API into dedicated crate

**Tasks**:
1. Create `ratchet-rest` crate
2. Move REST handlers and middleware
3. Create REST-specific abstractions

**Components to Extract**:
- `rest/handlers/` → `ratchet-rest/src/handlers/`
- `rest/middleware/` → `ratchet-rest/src/middleware/`
- `rest/models/` → `ratchet-rest/src/models/`
- `rest/extractors/` → `ratchet-rest/src/extractors/`

**Dependencies**:
- `ratchet-server-core`
- `ratchet-storage`
- `axum` ecosystem
- `serde` for JSON handling

### Phase 3: GraphQL Server Extraction (Week 5-6)

**Objective**: Extract GraphQL functionality

**Tasks**:
1. Create `ratchet-graphql` crate
2. Move GraphQL schema and resolvers
3. Implement GraphQL server abstraction

**Components to Extract**:
- `graphql/schema.rs` → `ratchet-graphql/src/schema/`
- `graphql/resolvers.rs` → `ratchet-graphql/src/resolvers/`
- `graphql/types.rs` → `ratchet-graphql/src/types/`

**Dependencies**:
- `ratchet-server-core`
- `ratchet-storage`
- `async-graphql` ecosystem

### Phase 4: MCP Server Isolation (Week 7-8)

**Objective**: Remove MCP dependency on `ratchet_lib`

**Tasks**:
1. Remove `ratchet_lib` dependency from `ratchet-mcp`
2. Add direct dependencies on storage/execution
3. Implement MCP-specific service layer

**Dependency Changes**:
```toml
# Before
[dependencies]
ratchet_lib = { path = "../ratchet-lib" }

# After
[dependencies]
ratchet-server-core = { path = "../ratchet-server-core" }
ratchet-storage = { path = "../ratchet-storage" }
ratchet-execution = { path = "../ratchet-execution" }
```

### Phase 5: Unified Server Orchestration (Week 9-10)

**Objective**: Create multi-server management

**Tasks**:
1. Create `ratchet-servers` crate
2. Implement server discovery and lifecycle
3. Create unified configuration

**Features**:
```rust
// ratchet-servers/src/lib.rs
pub struct ServerOrchestrator {
    servers: Vec<Box<dyn Server>>,
    config: OrchestrationConfig,
}

impl ServerOrchestrator {
    pub async fn start_all(&mut self) -> Result<(), OrchestratorError>;
    pub async fn stop_all(&mut self) -> Result<(), OrchestratorError>;
    pub fn discover_servers(&self) -> Vec<ServerInfo>;
}
```

## Implementation Details

### Server Core Abstractions

```rust
// ratchet-server-core/src/server.rs
#[async_trait]
pub trait Server: Send + Sync {
    type Config: Clone + Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn start(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;
    async fn restart(&mut self) -> Result<(), Self::Error> {
        self.stop().await?;
        self.start(self.config().clone()).await
    }
    
    fn health(&self) -> HealthStatus;
    fn metrics(&self) -> ServerMetrics;
    fn config(&self) -> &Self::Config;
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

pub struct ServerMetrics {
    pub requests_total: u64,
    pub requests_per_second: f64,
    pub response_time_p99: Duration,
    pub error_rate: f64,
}
```

### REST Server Implementation

```rust
// ratchet-rest/src/server.rs
pub struct RestServer {
    app: Option<axum::Router>,
    listener: Option<TcpListener>,
    config: RestConfig,
    state: Arc<ServerState>,
}

#[async_trait]
impl Server for RestServer {
    type Config = RestConfig;
    type Error = RestError;
    
    async fn start(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        let app = create_router(self.state.clone()).await?;
        let listener = TcpListener::bind(&config.address).await?;
        
        self.config = config;
        self.app = Some(app);
        self.listener = Some(listener);
        
        Ok(())
    }
    
    // ... implementation
}
```

### GraphQL Server Implementation

```rust
// ratchet-graphql/src/server.rs
pub struct GraphQLServer {
    schema: Option<GraphQLSchema>,
    config: GraphQLConfig,
    state: Arc<ServerState>,
}

#[async_trait]
impl Server for GraphQLServer {
    type Config = GraphQLConfig;
    type Error = GraphQLError;
    
    async fn start(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        let schema = create_schema(self.state.clone()).await?;
        self.schema = Some(schema);
        self.config = config;
        Ok(())
    }
    
    // ... implementation
}
```

## Migration Strategy

### Backward Compatibility

1. **Re-exports**: Maintain existing public APIs in `ratchet-lib`
2. **Feature Flags**: Allow gradual migration with feature toggles
3. **Version Alignment**: Coordinate version bumps across crates

### Migration Steps

```rust
// Phase 1: ratchet-lib maintains compatibility
pub use ratchet_rest as rest;
pub use ratchet_graphql as graphql;
pub use ratchet_server_core as server;

// Phase 2: Deprecation warnings
#[deprecated(note = "Use ratchet-rest crate directly")]
pub mod rest {
    pub use ratchet_rest::*;
}

// Phase 3: Removal in next major version
```

### Configuration Migration

```rust
// Before (monolithic)
#[derive(Deserialize)]
pub struct ServerConfig {
    pub rest: RestConfig,
    pub graphql: GraphQLConfig,
    pub mcp: McpConfig,
}

// After (modular)
#[derive(Deserialize)]
pub struct OrchestrationConfig {
    pub servers: HashMap<String, ServerInstanceConfig>,
}

#[derive(Deserialize)]
pub enum ServerInstanceConfig {
    Rest(ratchet_rest::RestConfig),
    GraphQL(ratchet_graphql::GraphQLConfig),
    Mcp(ratchet_mcp::McpConfig),
}
```

## Testing Strategy

### Unit Testing
- Each crate has isolated unit tests
- Mock dependencies using traits
- Test server lifecycle independently

### Integration Testing
- Server orchestration tests
- Multi-server interaction tests
- End-to-end workflow tests

### Performance Testing
- Benchmark individual servers
- Compare before/after performance
- Load testing with realistic workloads

## Risk Assessment

### High Risk
- **Breaking Changes**: Potential API incompatibilities
  - *Mitigation*: Comprehensive re-export strategy and feature flags

### Medium Risk  
- **Performance Regression**: Additional abstraction overhead
  - *Mitigation*: Zero-cost abstractions and performance benchmarks

### Low Risk
- **Increased Complexity**: More crates to manage
  - *Mitigation*: Clear documentation and automated tooling

## Success Metrics

### Quantitative Targets
- **Build Time**: 30-40% reduction in incremental builds
- **Binary Size**: 20-30% reduction when using subset of servers
- **Test Coverage**: Maintain >90% coverage across all crates
- **Memory Usage**: <5% increase in runtime memory usage

### Qualitative Goals
- **Developer Experience**: Easier to add new server types
- **Maintainability**: Clear separation of concerns
- **Testability**: Isolated testing capabilities
- **Documentation**: Comprehensive API documentation

## Timeline and Milestones

| Week | Phase | Deliverables | Dependencies |
|------|-------|-------------|--------------|
| 1-2  | Core  | `ratchet-server-core` crate | None |
| 3-4  | REST  | `ratchet-rest` extraction | Core complete |
| 5-6  | GraphQL | `ratchet-graphql` extraction | Core complete |
| 7-8  | MCP   | Isolated `ratchet-mcp` | Storage/execution |
| 9-10 | Orchestration | `ratchet-servers` crate | All servers |

## Conclusion

This refactoring plan addresses the current architectural limitations while building on successful component extractions. The modular approach will improve maintainability, testability, and performance while preserving backward compatibility.

The implementation should proceed incrementally, with each phase delivering value independently. Success will be measured through quantitative metrics and qualitative improvements in developer experience.

---

**Next Steps**: Begin Phase 1 implementation with server core abstractions.