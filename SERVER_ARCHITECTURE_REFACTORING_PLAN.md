# Server Architecture Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan for the Ratchet workspace's server architecture. The analysis reveals significant opportunities to improve modularity, reduce coupling, and create focused server-specific crates that support the project's growth and maintainability.

## Current Architecture Analysis

### Server Components Overview

The current server architecture is distributed across multiple locations:

1. **ratchet-lib/src/server/** - Main application server with Axum routing
2. **ratchet-lib/src/rest/** - REST API implementation with comprehensive handlers
3. **ratchet-lib/src/graphql/** - GraphQL server with resolvers and schema
4. **ratchet-mcp/src/server/** - MCP server for LLM integration

### Key Architectural Issues Identified

#### 1. Monolithic Structure in ratchet-lib
- **Issue**: The `ratchet-lib` crate contains multiple server implementations (REST, GraphQL, core server)
- **Impact**: Large crate size, complex dependencies, difficult to maintain and test individual components
- **Evidence**: 95+ source files in ratchet-lib with mixed concerns

#### 2. Circular Dependencies
- **Issue**: `ratchet-mcp` depends on `ratchet_lib`, creating potential circular dependencies
- **Impact**: Compilation complexity, harder to extract components independently
- **Evidence**: `ratchet-mcp/Cargo.toml` line 55: `ratchet_lib = { path = "../ratchet-lib" }`

#### 3. Tight Coupling Between Server Types
- **Issue**: Server components share state structures and dependencies without clear interfaces
- **Impact**: Changes to one server type affect others, reduced modularity
- **Evidence**: Shared `ServerState` in `/server/app.rs` lines 23-31

#### 4. Mixed Abstraction Levels
- **Issue**: High-level server orchestration mixed with low-level request handling
- **Impact**: Difficult to understand data flow and responsibility boundaries
- **Evidence**: `create_app()` function handling routing, middleware, and state management

#### 5. Unclear Separation of Concerns
- **Issue**: Business logic, HTTP handling, and server configuration intermingled
- **Impact**: Reduced testability, harder to modify individual components
- **Evidence**: GraphQL resolvers directly accessing repositories and executors

## Proposed Refactoring Strategy

### Phase 1: Server Core Extraction

#### Create `ratchet-server-core` Crate
**Purpose**: Foundational server abstractions and shared functionality

**Components**:
- `ServerTrait` - Core server interface
- `ServerState` - Unified state management
- `ServerConfig` - Configuration abstractions
- `ServerLifecycle` - Start/stop/health check operations
- `MiddlewareChain` - Composable middleware system

**Benefits**:
- Clear separation between server implementation and business logic
- Reusable across different server types
- Simplified testing and mocking

```rust
// Example interface
#[async_trait]
pub trait Server {
    type Config: ServerConfig;
    type State: Send + Sync;
    
    async fn start(&self, config: Self::Config) -> Result<(), ServerError>;
    async fn stop(&self) -> Result<(), ServerError>;
    async fn health_check(&self) -> HealthStatus;
}
```

### Phase 2: REST API Extraction

#### Create `ratchet-rest` Crate
**Purpose**: Dedicated REST API server implementation

**Components**:
- `RestServer` - Main REST server implementation
- `RestConfig` - REST-specific configuration
- `handlers/` - Request handlers (tasks, executions, jobs, schedules, workers)
- `middleware/` - REST-specific middleware (CORS, rate limiting, validation)
- `models/` - REST API models and serialization
- `extractors/` - Custom Axum extractors
- `errors/` - REST-specific error handling

**Dependencies**:
- `ratchet-server-core` - Core server abstractions
- `ratchet-storage` - Data access
- `ratchet-execution` - Task execution
- `axum`, `tower`, `tower-http` - HTTP framework

### Phase 3: GraphQL Server Extraction

#### Create `ratchet-graphql` Crate
**Purpose**: Dedicated GraphQL server implementation

**Components**:
- `GraphQLServer` - Main GraphQL server
- `schema/` - GraphQL schema definitions
- `resolvers/` - Query, mutation, and subscription resolvers
- `types/` - GraphQL type definitions
- `subscriptions/` - Real-time subscriptions
- `playground/` - Development tooling

**Dependencies**:
- `ratchet-server-core` - Core server abstractions
- `ratchet-storage` - Data access
- `ratchet-execution` - Task execution
- `async-graphql`, `async-graphql-axum` - GraphQL framework

### Phase 4: MCP Server Isolation

#### Refactor `ratchet-mcp` Crate
**Purpose**: Independent MCP server without ratchet-lib dependency

**Changes**:
- Remove dependency on `ratchet_lib`
- Add direct dependencies on `ratchet-storage`, `ratchet-execution`
- Implement `Server` trait from `ratchet-server-core`
- Create MCP-specific abstractions

**Components**:
- `McpServer` - MCP server implementation
- `protocol/` - MCP protocol handling
- `tools/` - MCP tool registry and execution
- `transport/` - STDIO and SSE transport layers
- `security/` - Authentication and authorization

### Phase 5: Unified Server Orchestration

#### Create `ratchet-servers` Crate
**Purpose**: High-level server orchestration and management

**Components**:
- `ServerManager` - Manages multiple server instances
- `ServerRouter` - Routes requests to appropriate servers
- `ServerConfig` - Unified configuration management
- `ServerMetrics` - Cross-server metrics and monitoring
- `ServerDiscovery` - Service discovery and health checking

**Example Usage**:
```rust
let server_manager = ServerManager::builder()
    .with_rest_server(rest_config)
    .with_graphql_server(graphql_config)
    .with_mcp_server(mcp_config)
    .build()?;

server_manager.start_all().await?;
```

## Detailed Implementation Roadmap

### Priority 1: Foundation (Weeks 1-2)

#### Step 1.1: Create ratchet-server-core
- [ ] Create new crate structure
- [ ] Define core `Server` trait and related abstractions
- [ ] Implement shared state management
- [ ] Create configuration traits
- [ ] Add comprehensive tests

#### Step 1.2: Extract Common Server Utilities
- [ ] Move middleware abstractions to server-core
- [ ] Extract health check functionality
- [ ] Create server lifecycle management
- [ ] Implement metrics collection interfaces

### Priority 2: REST API Extraction (Weeks 3-4)

#### Step 2.1: Create ratchet-rest Crate
- [ ] Set up crate structure and dependencies
- [ ] Move REST handlers from ratchet-lib
- [ ] Implement `Server` trait for REST server
- [ ] Migrate middleware and extractors

#### Step 2.2: Update REST Dependencies
- [ ] Remove REST code from ratchet-lib
- [ ] Update import statements across codebase
- [ ] Ensure backward compatibility
- [ ] Add integration tests

### Priority 3: GraphQL Server Extraction (Weeks 5-6)

#### Step 3.1: Create ratchet-graphql Crate
- [ ] Set up crate structure and dependencies
- [ ] Move GraphQL schema and resolvers
- [ ] Implement `Server` trait for GraphQL server
- [ ] Migrate playground functionality

#### Step 3.2: Update GraphQL Dependencies
- [ ] Remove GraphQL code from ratchet-lib
- [ ] Update schema references
- [ ] Ensure subscription functionality works
- [ ] Add comprehensive tests

### Priority 4: MCP Server Refactoring (Weeks 7-8)

#### Step 4.1: Remove ratchet-lib Dependency
- [ ] Identify all ratchet-lib usages in ratchet-mcp
- [ ] Replace with direct dependencies
- [ ] Update tool registry implementations
- [ ] Ensure MCP functionality is preserved

#### Step 4.2: Implement Server Core Integration
- [ ] Implement `Server` trait for MCP server
- [ ] Update configuration management
- [ ] Integrate with unified metrics
- [ ] Add integration tests

### Priority 5: Server Orchestration (Weeks 9-10)

#### Step 5.1: Create ratchet-servers Crate
- [ ] Design server manager architecture
- [ ] Implement multi-server coordination
- [ ] Create unified configuration system
- [ ] Add service discovery capabilities

#### Step 5.2: Integration and Testing
- [ ] Update ratchet-cli to use new server architecture
- [ ] Create end-to-end integration tests
- [ ] Performance testing and optimization
- [ ] Documentation updates

## Implementation Guidelines

### Code Organization Principles

1. **Single Responsibility**: Each crate should have one clear purpose
2. **Interface Segregation**: Define minimal, focused interfaces
3. **Dependency Inversion**: Depend on abstractions, not concretions
4. **Open/Closed**: Open for extension, closed for modification

### Dependency Management

```toml
# Proposed dependency structure
ratchet-server-core = { version = "0.3.0", no dependencies on other ratchet crates }
ratchet-rest = { depends = ["ratchet-server-core", "ratchet-storage", "ratchet-execution"] }
ratchet-graphql = { depends = ["ratchet-server-core", "ratchet-storage", "ratchet-execution"] }
ratchet-mcp = { depends = ["ratchet-server-core", "ratchet-storage", "ratchet-execution"] }
ratchet-servers = { depends = ["ratchet-rest", "ratchet-graphql", "ratchet-mcp"] }
```

### Backward Compatibility Strategy

1. **Phase-in Period**: Maintain old interfaces for 2 releases
2. **Deprecation Warnings**: Clear migration paths for users
3. **Documentation**: Comprehensive migration guides
4. **Examples**: Updated examples showing new patterns

### Testing Strategy

1. **Unit Tests**: Each crate maintains >90% test coverage
2. **Integration Tests**: Cross-crate functionality testing
3. **Performance Tests**: Ensure no regression in server performance
4. **Compatibility Tests**: Verify existing functionality works

## Expected Benefits

### Immediate Benefits

1. **Reduced Compilation Time**: Smaller, focused crates compile faster
2. **Improved Testability**: Isolated components are easier to test
3. **Clear Boundaries**: Better understanding of component responsibilities
4. **Enhanced Modularity**: Easier to add new server types or modify existing ones

### Long-term Benefits

1. **Scalability**: Architecture supports growth and new requirements
2. **Maintainability**: Easier to maintain and debug individual components
3. **Flexibility**: Can deploy different server combinations as needed
4. **Performance**: Optimized builds with only necessary dependencies

### Quantitative Improvements

- **Build Time**: Expected 30-40% reduction in full workspace build time
- **Binary Size**: 20-30% reduction when using only needed servers
- **Memory Usage**: Reduced runtime memory footprint
- **Development Velocity**: Faster iteration on individual server components

## Risk Mitigation

### Technical Risks

1. **Breaking Changes**: Mitigated by careful interface design and migration periods
2. **Performance Regression**: Addressed through comprehensive benchmarking
3. **Integration Complexity**: Managed through extensive integration testing

### Project Risks

1. **Timeline Overruns**: Mitigated by phased approach and clear milestones
2. **Resource Allocation**: Addressed through dedicated team assignments
3. **User Adoption**: Managed through clear migration documentation

## Success Metrics

### Technical Metrics

- [ ] Compilation time reduction of >30%
- [ ] Test coverage maintained at >90% across all server crates
- [ ] Zero performance regression in benchmark tests
- [ ] All existing functionality preserved

### Quality Metrics

- [ ] Reduced cyclomatic complexity in server components
- [ ] Improved separation of concerns (measured via dependency analysis)
- [ ] Enhanced documentation coverage
- [ ] Positive developer feedback on new architecture

## Conclusion

This refactoring plan addresses the identified architectural issues while maintaining backward compatibility and improving the overall developer experience. The phased approach allows for incremental progress and validation at each step, ensuring the project remains stable throughout the transformation.

The proposed architecture follows modern software engineering principles and positions the Ratchet workspace for future growth and evolution. By creating focused, well-defined crates with clear interfaces, we establish a foundation that supports both current requirements and future expansion.