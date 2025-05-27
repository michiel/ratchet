# Ratchet Refactoring & Improvement TODO

## Code Quality & Refactoring

### 1. Error Handling Improvements
- [x] **Extract JavaScript Error Type Generation** (`lib.rs:111-211`) ✅ **COMPLETED**
  - [x] Create macro or data structure for error types
  - [x] Implement `generate_error_class` function to reduce duplication
  - [x] Replace 200+ lines of repetitive error class definitions
  - [x] Add unit tests for error type generation
  
  **Summary**: Refactored 200+ lines of repetitive JavaScript error class definitions into a data-driven approach using `JsErrorConfig` struct and generation functions. Reduced code duplication by ~85% while maintaining the same functionality. Added comprehensive unit tests covering standard errors, status-aware errors (HttpError), and integration testing.

### 2. Function Complexity Reduction
- [ ] **Break Down `call_js_function`** (`lib.rs:253-493`)
  - [ ] Extract `execute_javascript_function` for core JS execution
  - [ ] Extract `handle_fetch_processing` for HTTP request handling
  - [ ] Extract `convert_js_result_to_json` for result conversion
  - [ ] Add comprehensive tests for each extracted function

### 3. Magic String Constants
- [ ] **Extract Hardcoded Variables** 
  - [ ] Create constants for `__fetch_url`, `__fetch_params`, `__fetch_body`
  - [ ] Create constants for `__http_result`, `__temp_result`
  - [ ] Create constants for common JSON schema property names
  - [ ] Update all references to use constants

### 4. HTTP Manager Enhancements
- [ ] **Improve HTTP Module** (`http.rs`)
  - [ ] Extract header building logic into separate function
  - [ ] Add more specific error variants for different HTTP scenarios
  - [ ] Make timeouts and other settings configurable
  - [ ] Add connection pooling for better performance
  - [ ] Add retry logic with exponential backoff

### 5. Task Loading Performance
- [ ] **Optimize Task Loading** (`task.rs`)
  - [ ] Implement lazy schema validation (only when needed)
  - [ ] Replace `std::fs` with `tokio::fs` for non-blocking I/O
  - [ ] Add content streaming for large JavaScript files
  - [ ] Implement progressive loading for large task archives

### 6. CLI User Experience
- [ ] **Enhance CLI Interface** (`main.rs`)
  - [ ] Improve error messages with actionable suggestions
  - [ ] Add progress indicators for long-running operations
  - [ ] Implement config file support for CLI defaults
  - [ ] Add colored output for better readability
  - [ ] Add `--dry-run` mode for validation without execution

## Type Safety & Architecture

### 7. Type Safety Improvements
- [x] **Replace String-based Types** ✅ **COMPLETED**
  - [x] Create `HttpMethod` enum to replace string-based method handling
  - [x] Create `LogLevel` enum for better log level management  
  - [x] Create `TaskStatus` enum for task execution states
  - [x] Add proper type conversion methods with error handling
  
  **Summary**: Created comprehensive type-safe enums for HTTP methods, log levels, and task statuses. Replaced string-based method handling throughout the HTTP module with strongly-typed `HttpMethod` enum. Added proper error handling, serialization support, and conversion methods. Maintained backward compatibility while providing safer, more maintainable APIs.

### 8. Plugin Architecture
- [ ] **Implement TaskExecutor Trait**
  - [ ] Design `TaskExecutor` trait interface
  - [ ] Extract JavaScript execution into trait implementation
  - [ ] Add support for multiple execution engines
  - [ ] Create plugin discovery mechanism

### 9. Configuration Management
- [ ] **Centralized Configuration**
  - [ ] Create `RatchetConfig` struct with all settings
  - [ ] Implement configuration loading from files and environment
  - [ ] Add configuration validation
  - [ ] Support for profile-based configurations (dev, test, prod)

### 10. Module Organization
- [x] **Better Code Structure** ✅ **COMPLETED**
  - [x] Move recording functionality to `ratchet-lib/src/recording/` module
  - [x] Split `lib.rs` (954 lines) into smaller focused modules
  - [x] Create dedicated `error.rs` for all error types
  - [x] Organize HTTP-related code into submodules
  - [x] Create `validation/` module for schema validation logic
  
  **Summary**: Completely reorganized the codebase into focused, maintainable modules. Reduced lib.rs from 1063 lines to just 30 lines by extracting functionality into dedicated modules. Created logical groupings for HTTP functionality, error handling, validation, and recording. Improved code organization while maintaining 100% backward compatibility and test coverage.

## Performance Optimizations

### 11. Memory Management
- [ ] **Optimize Memory Usage**
  - [ ] Make LRU cache size configurable (`task.rs:23`)
  - [ ] Implement automatic content purging based on usage patterns
  - [ ] Add memory usage monitoring and reporting
  - [ ] Implement memory limits and cleanup strategies

### 12. Execution Performance
- [ ] **Batch Operations**
  - [ ] Combine multiple schema validations into single operations
  - [ ] Implement parallel test execution for test suites
  - [ ] Add compilation caching for JavaScript functions
  - [ ] Optimize JSON parsing and serialization

### 13. Concurrency Improvements
- [ ] **Better Async Handling**
  - [ ] Review and optimize async/await usage patterns
  - [ ] Implement proper cancellation support
  - [ ] Add timeout handling for all async operations
  - [ ] Use `tokio::spawn` for independent concurrent tasks

## Developer Experience

### 14. Documentation & Tooling
- [ ] **Improve Documentation**
  - [ ] Add comprehensive code examples in documentation
  - [ ] Create JSON schemas for task metadata files
  - [ ] Add inline documentation for complex algorithms
  - [ ] Create architecture decision records (ADRs)

### 15. Debug & Development Tools
- [ ] **Enhanced Debugging**
  - [ ] Add built-in task profiling mode
  - [ ] Implement step-by-step debugging for JavaScript execution
  - [ ] Add execution trace visualization
  - [ ] Create development mode with enhanced logging

### 16. Testing Infrastructure
- [ ] **Comprehensive Testing**
  - [ ] Add integration tests for end-to-end workflows
  - [ ] Implement property-based testing for core functions
  - [ ] Add performance benchmarks
  - [ ] Create test data generators for various scenarios

## Security & Reliability

### 17. Security Enhancements
- [ ] **Improve Security**
  - [ ] Add input sanitization for JavaScript code execution
  - [ ] Implement resource limits for JavaScript execution
  - [ ] Add content security policies for HTTP requests
  - [ ] Audit dependencies for security vulnerabilities

### 18. Error Recovery
- [ ] **Robust Error Handling**
  - [ ] Implement graceful degradation for network failures
  - [ ] Add automatic retry mechanisms with circuit breakers
  - [ ] Improve error context and diagnostic information
  - [ ] Add health check endpoints for monitoring

## Server Architecture & Persistence

### 19. Database Layer & Models
- [ ] **Database Infrastructure** (Critical for Server)
  - [ ] Create database schema for tasks, executions, schedules, and jobs
  - [ ] Implement SQLite connection pool and migration system
  - [ ] Create domain models: `TaskEntity`, `ExecutionEntity`, `ScheduleEntity`, `JobEntity`
  - [ ] Add database traits/interfaces for testability (Repository pattern)
  - [ ] Implement CRUD operations for all entities with proper error handling

### 20. Configuration Management (Server Prerequisites)
- [ ] **Server Configuration System** (Critical for Server)
  - [ ] Design `ServerConfig` struct with database, HTTP, security settings
  - [ ] Implement YAML configuration loading with validation
  - [ ] Add environment variable override support
  - [ ] Create configuration profiles (development, testing, production)
  - [ ] Add configuration validation and error reporting

### 21. Async Task Execution Framework
- [ ] **Background Job System** (Critical for Server)
  - [ ] Abstract task execution into `TaskExecutor` trait
  - [ ] Create job queue system with priority and retry logic
  - [ ] Implement task scheduler with cron-like syntax
  - [ ] Add execution status tracking and progress reporting
  - [ ] Create worker pool for concurrent task execution

### 22. API Layer Foundation
- [ ] **GraphQL & HTTP Infrastructure** (Critical for Server)
  - [ ] Separate CLI logic from core library in main.rs
  - [ ] Create `ratchet-server` crate with GraphQL schema
  - [ ] Implement REST endpoints for health checks and metrics
  - [ ] Add authentication/authorization middleware
  - [ ] Create error handling for API responses

### 23. Core Library Abstraction
- [ ] **Library Preparation for Server** (Critical for Server)
  - [ ] Extract task execution logic from CLI-specific code
  - [ ] Create service layer abstraction (`TaskService`, `ExecutionService`)
  - [ ] Make HTTP manager configurable and injectable
  - [ ] Add proper async/await throughout execution pipeline
  - [ ] Ensure thread-safety for concurrent task execution

## Advanced Features

### 24. Extended Task Capabilities
- [ ] **Advanced Task Features**
  - [ ] Add support for task dependencies and pipelines
  - [ ] Implement task chaining and conditional execution
  - [ ] Add support for streaming data processing
  - [ ] Create task composition and workflow management

### 25. Monitoring & Observability
- [ ] **Production Readiness**
  - [ ] Add metrics collection and reporting
  - [ ] Implement distributed tracing
  - [ ] Add health check and readiness probes
  - [ ] Create performance monitoring dashboard

---

## Priority Levels

**Critical for Server Implementation** (Must complete before server):
- [ ] Configuration management (#9, #20) - Required for server config
- [ ] Function complexity reduction (#2) - Needed for service abstraction 
- [ ] Core library abstraction (#23) - Extract CLI-specific logic
- [ ] Task execution framework (#21) - Background job system
- [ ] Database layer (#19) - Persistence infrastructure

**High Priority** (Foundation improvements):
- [ ] Magic string constants (#3)
- [ ] HTTP manager enhancements (#4) - Make configurable/injectable
- [ ] Memory management (#11) - Critical for long-running server
- [x] Type safety improvements (#7) ✅
- [x] Module organization (#10) ✅

**Medium Priority** (Performance & UX):
- [ ] CLI user experience (#6)
- [ ] API layer foundation (#22) - GraphQL & HTTP infrastructure
- [ ] Async improvements (#13) - Better concurrency
- [ ] Documentation improvements (#14)

**Low Priority** (Advanced features):
- [ ] Plugin architecture (#8)
- [ ] Extended task capabilities (#24)
- [ ] Monitoring & observability (#25)

---

## Server Implementation Roadmap

### Phase 1: Foundation Refactoring (Critical)
**Objective**: Prepare codebase for server architecture

1. **Configuration Management (#9, #20)**
   - Extract hardcoded values to configuration structs
   - Implement YAML config loading with validation
   - Support for different environments (dev/test/prod)

2. **Function Complexity Reduction (#2)**
   - Break down large functions in js_executor.rs:call_js_function
   - Create focused, testable functions for service layer

3. **Core Library Abstraction (#23)**  
   - Separate CLI-specific logic from library code
   - Create service layer interfaces (TaskService, ExecutionService)
   - Make dependencies injectable (HTTP manager, configurations)

### Phase 2: Persistence & Background Jobs (Critical)
**Objective**: Add database and async execution capabilities

4. **Database Layer (#19)**
   - Design schema for tasks, executions, schedules, jobs
   - Implement SQLite with connection pooling
   - Create repository pattern with proper error handling

5. **Task Execution Framework (#21)**
   - Abstract execution into TaskExecutor trait
   - Implement job queue with priority and retry logic
   - Add scheduling system with cron-like syntax

### Phase 3: Server Infrastructure (Medium Priority)
**Objective**: Build GraphQL API and server foundation

6. **API Layer Foundation (#22)**
   - Create ratchet-server crate
   - Implement GraphQL schema and resolvers
   - Add authentication and error handling

### Phase 4: Production Readiness (Low Priority)
**Objective**: Monitoring, security, and advanced features

7. **Monitoring & Security**
   - Add health checks and metrics
   - Implement proper authentication/authorization
   - Add request validation and rate limiting

## Getting Started

**For Server Implementation:**
1. Complete all **Critical for Server Implementation** items in order
2. Each phase builds on the previous - don't skip ahead
3. Maintain backward compatibility for CLI functionality
4. Add comprehensive tests for new server components

**For General Development:**
1. Start with **High Priority** items to establish a solid foundation
2. Focus on one category at a time to maintain code stability
3. Write tests for each refactoring before making changes
4. Update documentation as you implement improvements
5. Consider breaking large changes into smaller, reviewable commits

## Current Codebase Analysis for Server

### Key Issues Preventing Server Implementation

1. **Tight CLI Coupling** (`ratchet-cli/src/main.rs:410-493`)
   - Task execution logic embedded in CLI command handlers
   - HTTP manager instantiated directly in CLI code
   - No service layer abstraction for reuse

2. **Hardcoded Values** (Multiple files)
   - Magic strings in js_executor.rs: `__fetch_url`, `__http_result`
   - No centralized configuration management
   - Environment-specific settings scattered throughout code

3. **Synchronous Task Loading** (`task.rs:85-100`)
   - Uses `std::fs` instead of `tokio::fs` for file operations
   - LRU cache size hardcoded to 100 entries
   - No async/await in task loading pipeline

4. **Complex Execution Function** (`js_executor.rs` - `call_js_function`)
   - Single large function handling multiple responsibilities
   - Difficult to unit test individual components
   - Hard to extract for service layer usage

5. **No Persistence Layer**
   - All task data loaded from filesystem on each execution
   - No tracking of execution history or job status
   - No support for scheduled or queued executions

### Required Dependencies for Server

**New Crates Needed:**
- `sqlx` or `sea-orm` - Database operations and migrations
- `async-graphql` - GraphQL schema and resolvers  
- `axum` or `warp` - HTTP server and routing
- `serde_yaml` - Configuration file parsing
- `tokio-cron-scheduler` - Job scheduling
- `uuid` - Already available, extend usage for job IDs

**Workspace Structure After Server Addition:**
```
ratchet/
├── ratchet-lib/          # Core task execution (current)
├── ratchet-cli/          # CLI interface (refactored)
├── ratchet-server/       # GraphQL server (new)
├── ratchet-db/           # Database models & migrations (new)
└── ratchet-common/       # Shared types & configs (new)
```

## Notes

- All changes should maintain backward compatibility where possible
- Add deprecation warnings before removing existing APIs
- Update the CHANGELOG.md for any user-facing changes
- Consider the impact on existing task definitions and user workflows
- Server implementation requires completing Foundation Refactoring first
- Plan for database migrations and schema evolution from the start