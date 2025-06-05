# Ratchet Architecture Migration Analysis

## Current Migration Status

The Ratchet project is in the middle of a significant architectural migration from a monolithic `ratchet-lib` crate to a modular multi-crate architecture. This analysis provides a comprehensive view of the current state and migration path forward.

## Project Structure Overview

### Completed Modular Crates
1. **ratchet-core** - Core domain models and types ✅
2. **ratchet-storage** - Storage abstraction and repository pattern ✅
3. **ratchet-api** - Unified API layer (REST and GraphQL) ✅
4. **ratchet-runtime** - Task execution runtime ✅
5. **ratchet-ipc** - Inter-process communication ✅
6. **ratchet-resilience** - Circuit breakers, retry logic ✅
7. **ratchet-caching** - Caching abstractions ✅
8. **ratchet-config** - Domain-driven configuration ✅
9. **ratchet-mcp** - Model Context Protocol implementation ✅
10. **ratchet-plugins** - Plugin system ✅
11. **ratchet-plugin** - Plugin management ✅

### Legacy Monolith
- **ratchet-lib** - Contains significant functionality that needs migration ⚠️

## Critical Analysis: What's Still in ratchet-lib

### 1. Database Layer Duplication
**Problem**: Complete database implementation exists in both places
- `ratchet-lib/src/database/` - Full Sea-ORM implementation with migrations, repositories, entities
- `ratchet-storage/src/` - New abstracted storage layer

**Impact**: 
- Tests and MCP server still use `ratchet-lib` database
- Dual maintenance burden
- Inconsistent entity definitions

### 2. API Layer Duplication  
**Problem**: REST and GraphQL implementations in both crates
- `ratchet-lib/src/rest/` - Complete REST API with handlers, middleware
- `ratchet-lib/src/graphql/` - Full GraphQL schema and resolvers
- `ratchet-api/src/` - New minimal API structure

**Impact**:
- CLI and MCP use old API layer
- Feature development happening in wrong place

### 3. Configuration System Duplication
**Problem**: Two competing configuration systems
- `ratchet-lib/src/config.rs` - Original monolithic config
- `ratchet-config/src/` - New domain-driven config

**Current Usage**:
- CLI: Uses `ratchet_lib::config::RatchetConfig` 
- MCP: Uses `ratchet_lib::config::McpServerConfig`
- New crates: Use `ratchet-config`

### 4. Execution Engine Split
**Problem**: Execution logic scattered across multiple places
- `ratchet-lib/src/execution/` - Original execution engine with IPC, workers, job queue
- `ratchet-runtime/src/` - New minimal runtime (only 4 files)
- `ratchet-lib/src/js_executor/` - JavaScript execution engine

**Missing in new runtime**:
- Process management
- Worker coordination
- Load balancing
- Task caching
- IPC implementation

### 5. Business Logic Still in ratchet-lib
**Critical modules not yet migrated**:
- `js_executor/` - JavaScript task execution
- `http/` - HTTP client management  
- `logging/` - Comprehensive logging system
- `output/` - Output destination management
- `registry/` - Task registry and loading
- `task/` - Task validation and caching
- `validation/` - JSON schema validation
- `services/` - Service provider pattern

## Dependency Analysis

### Who Still Depends on ratchet-lib
1. **ratchet-cli** - Heavily dependent on ratchet-lib for everything
2. **ratchet-mcp** - Uses ratchet-lib for services, execution, database
3. **All integration tests** - 24 test files use ratchet-lib

### New Crate Dependencies (Good)
- ratchet-api → ratchet-core, ratchet-storage
- ratchet-storage → ratchet-core, ratchet-caching  
- ratchet-runtime → ratchet-core, ratchet-ipc, ratchet-resilience, ratchet-caching
- ratchet-mcp → ratchet-core, ratchet-ipc, ratchet-storage (+ ratchet-lib)

## Migration Blockers

### 1. Database Migration Complexity
- Need to consolidate Sea-ORM entities between ratchet-lib and ratchet-storage
- Migration scripts exist in ratchet-lib but not ratchet-storage
- Active connections and repositories need migration

### 2. JavaScript Execution Engine
- Complex JS executor with Boa engine integration
- HTTP integration for task execution
- No equivalent in new architecture

### 3. CLI Application Dependencies
- CLI directly imports many ratchet-lib modules
- Would need significant refactoring to use new crates

### 4. Test Suite Migration
- 24 integration tests depend on ratchet-lib
- Tests cover complex scenarios requiring multiple subsystems

## Recommended Migration Plan

### Phase 1: Foundation Consolidation (High Priority)
**Goal**: Consolidate core infrastructure to reduce duplication

1. **Database Layer Unification** (Priority: Critical)
   - Migrate Sea-ORM entities from ratchet-lib to ratchet-storage
   - Move migration scripts to ratchet-storage
   - Update ratchet-lib to use ratchet-storage as backend
   - Ensure entity compatibility between systems

2. **Configuration System Consolidation** (Priority: High)
   - Create compatibility layer in ratchet-config for old config format
   - Update CLI to use ratchet-config with compatibility mode
   - Update MCP to use new config system
   - Remove config.rs from ratchet-lib

3. **API Layer Decision** (Priority: High)
   - Choose primary API implementation (recommend new ratchet-api)
   - Migrate REST handlers from ratchet-lib to ratchet-api
   - Migrate GraphQL schema from ratchet-lib to ratchet-api
   - Update dependents to use new API

### Phase 2: Business Logic Migration (Medium Priority)
**Goal**: Move core business logic to appropriate new crates

4. **JavaScript Execution Engine** (Priority: Medium)
   - Move js_executor/ to ratchet-runtime
   - Move js_task.rs to ratchet-runtime
   - Ensure HTTP integration works with new architecture

5. **HTTP Client Management** (Priority: Medium)
   - Move http/ module to new dedicated crate or ratchet-core
   - Ensure task execution can still access HTTP manager

6. **Task Management** (Priority: Medium)
   - Move task/ module to ratchet-core
   - Move registry/ module to ratchet-core
   - Move validation/ module to ratchet-core

7. **Output System** (Priority: Medium)
   - Move output/ module to new ratchet-output crate
   - Integrate with task execution pipeline

### Phase 3: Infrastructure Migration (Lower Priority)
**Goal**: Complete the architecture migration

8. **Logging System** (Priority: Low)
   - Move logging/ to new ratchet-logging crate
   - Ensure all crates can use centralized logging

9. **Service Layer** (Priority: Low)
   - Move services/ to ratchet-core
   - Ensure MCP and CLI can use new service layer

10. **Execution Engine Completion** (Priority: Low)
    - Move remaining execution/ modules to ratchet-runtime
    - Ensure full feature parity with old system

### Phase 4: Final Cleanup (Lowest Priority)
**Goal**: Remove ratchet-lib entirely

11. **CLI Migration** (Priority: Low)
    - Update CLI to use new modular crates exclusively
    - Remove ratchet-lib dependency

12. **Test Migration** (Priority: Low)
    - Migrate integration tests to use new crates
    - Ensure test coverage is maintained

13. **ratchet-lib Removal** (Priority: Lowest)
    - Remove ratchet-lib crate entirely
    - Update workspace configuration

## Risk Assessment

### High Risk
- Database migration could break existing functionality
- JavaScript execution engine is complex and tightly coupled
- CLI refactoring could affect user experience

### Medium Risk  
- API migration might break existing integrations
- Configuration changes could affect deployment scripts
- Test migration might reduce coverage temporarily

### Low Risk
- Logging system migration
- Service layer restructuring
- Final cleanup tasks

## Success Metrics

1. **ratchet-lib dependency removal** from all new crates
2. **Zero duplication** between ratchet-lib and new crates
3. **All tests passing** with new architecture
4. **CLI functionality preserved** with new crates
5. **Performance maintained** or improved

## Conclusion

The migration is approximately 40% complete. The foundational crates exist but critical business logic remains in ratchet-lib. The primary blocker is database layer duplication, followed by the need to migrate the JavaScript execution engine and API layers.

Priority should be given to Phase 1 tasks to reduce maintenance burden and enable faster development in the new architecture.