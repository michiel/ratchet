# Ratchet-lib Usage Analysis

## Overview
This document analyzes all remaining usages of ratchet-lib in the codebase to understand dependencies and migration requirements.

## Crates Still Depending on ratchet-lib

### 1. ratchet-cli (Optional Dependency)
- **Type**: Optional dependency (`dep:ratchet_lib`)
- **Features**: Enabled when `server`, `rest-api`, `graphql-api`, `javascript`, or `output` features are active
- **Main Usage**:
  - Server functionality (REST API, GraphQL)
  - JavaScript task execution
  - Output destinations
  - Database operations (via feature flags)
  - Process task executor
  - Configuration conversion between new and legacy formats

### 2. ratchet-mcp (Direct Dependency)
- **Type**: Direct dependency (always included)
- **Main Usage**:
  - `ProcessTaskExecutor` from `ratchet_lib::execution`
  - Execution IPC types (`ExecutionContext`, `ExecutionError`, `ExecutionResult`)
  - Logging infrastructure (`LogEvent`, `LogLevel`, `StructuredLogger`)
  - Configuration types for MCP server

### 3. ratchet-lib tests
- **Type**: Test dependencies
- **Usage**: Self-testing of ratchet-lib functionality

## Specific Module Dependencies

### Execution Module (`ratchet_lib::execution`)
**Used by**: ratchet-cli, ratchet-mcp
- `ProcessTaskExecutor` - Core task execution engine
- IPC types:
  - `CoordinatorMessage`
  - `MessageEnvelope`
  - `TaskExecutionResult`
  - `WorkerMessage`
  - `ExecutionContext`
  - `TaskValidationResult`
  - `WorkerStatus`
- `JobQueueManager`
- `ExecutionError`, `ExecutionResult`

### HTTP Module (`ratchet_lib::http`)
**Used by**: ratchet-cli
- `HttpManager` - HTTP client management for task execution

### JavaScript Executor (`ratchet_lib::js_executor`)
**Used by**: ratchet-cli
- `execute_task` - JavaScript task execution

### Task Module (`ratchet_lib::task`)
**Used by**: ratchet-cli
- `Task` - Task loading and validation

### Logging Module (`ratchet_lib::logging`)
**Used by**: ratchet-cli, ratchet-mcp
- `LoggingConfig` - Logging configuration
- `init_logging_from_config` - Logging initialization
- `LogEvent`, `LogLevel` - Log event types
- `StructuredLogger` - Structured logging

### Recording Module (`ratchet_lib::recording`)
**Used by**: ratchet-cli
- Session recording functionality
- `set_recording_dir`, `get_recording_dir`, `finalize_recording`

### Database Module (`ratchet_lib::database`)
**Used by**: ratchet-cli
- `DatabaseConnection`
- `RepositoryFactory`
- Migration and entity types (in tests)

### Configuration Module (`ratchet_lib::config`)
**Used by**: ratchet-cli
- Legacy configuration types:
  - `RatchetConfig`
  - `DatabaseConfig`
  - `ServerConfig`
  - `ExecutionConfig`
  - `HttpConfig`
  - `McpServerConfig`
  - `CacheConfig`
  - `OutputConfig`

### Server Module (`ratchet_lib::server`)
**Used by**: ratchet-cli
- `create_app` - Main server application factory

### GraphQL Module (`ratchet_lib::graphql`)
**Used by**: ratchet-cli tests
- GraphQL schema creation

### REST Module (`ratchet_lib::rest`)
**Used by**: ratchet-cli tests
- REST API application

### Generation Module (`ratchet_lib::generate`)
**Used by**: ratchet-cli
- Task template generation

## Migration Categorization

### 1. **Easy to Migrate** (Use existing modular crates)
- Database operations → Use `ratchet-storage` directly
- Basic configuration → Use `ratchet-config` directly
- IPC protocol types → Already available in `ratchet-ipc`

### 2. **Medium Difficulty** (Need abstraction or wrapper)
- `ProcessTaskExecutor` → Could use `ratchet-runtime` with adapter
- HTTP management → Create shared HTTP client in `ratchet-core` or new crate
- Logging infrastructure → Extract to `ratchet-logging` crate

### 3. **Complex Migration** (Core functionality)
- Server application (`create_app`) → Need to recreate in modular structure
- JavaScript executor → Extract to `ratchet-js` or enhance `ratchet-runtime`
- Task loading/validation → Move to `ratchet-core` or `ratchet-runtime`
- Recording functionality → Extract to `ratchet-recording` crate

### 4. **Configuration Compatibility Layer**
- The CLI maintains a conversion layer between new modular config and legacy config
- This allows gradual migration while maintaining compatibility

## Recommendations

1. **Create New Crates**:
   - `ratchet-logging` - Extract logging infrastructure
   - `ratchet-http` - Shared HTTP client functionality
   - `ratchet-js` - JavaScript execution engine
   - `ratchet-recording` - Session recording functionality

2. **Enhance Existing Crates**:
   - Move task loading/validation to `ratchet-core`
   - Add execution coordination to `ratchet-runtime`
   - Move server creation logic to a new `ratchet-server` crate

3. **Phased Migration**:
   - Phase 1: Extract independent modules (logging, HTTP, recording)
   - Phase 2: Migrate execution engine to use `ratchet-runtime`
   - Phase 3: Recreate server functionality in modular structure
   - Phase 4: Remove configuration conversion layer

4. **Maintain Compatibility**:
   - Keep ratchet-lib as a compatibility layer during migration
   - Gradually deprecate modules as they're extracted
   - Use feature flags to control migration path

## Blockers for Complete Removal

1. **ProcessTaskExecutor** - Core execution engine deeply integrated
2. **Server application factory** - Ties together many components
3. **JavaScript execution** - Tightly coupled with task system
4. **Configuration compatibility** - Need to maintain backward compatibility

## Next Steps

1. Start with extracting independent modules (logging, HTTP)
2. Create abstraction layers for execution engine
3. Design modular server architecture
4. Plan JavaScript executor extraction
5. Implement gradual deprecation strategy