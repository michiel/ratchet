# Ratchet Architecture Guide

This document outlines the architecture, design principles, and conventions used in the Ratchet codebase.

## Table of Contents

- [Overview](#overview)
- [Code Layout](#code-layout)
- [Module Structure](#module-structure)
- [Process Execution IPC Model](#process-execution-ipc-model)
- [Conventions](#conventions)
- [Error Handling](#error-handling)
- [Type Safety](#type-safety)
- [Testing Strategy](#testing-strategy)
- [Development Guidelines](#development-guidelines)

## Overview

Ratchet is a JavaScript task execution framework written in Rust, designed with modularity, type safety, and maintainability as core principles. The architecture follows a layered approach with clear separation of concerns.

### Core Components

- **Task Management**: Loading, validation, and execution of JavaScript tasks
- **JavaScript Engine**: Secure JavaScript execution environment using Boa
- **HTTP Client**: Type-safe HTTP request handling with mock support
- **Validation**: JSON schema validation for inputs and outputs
- **Recording**: Session recording and replay functionality
- **CLI Interface**: Command-line interface for task operations

## Code Layout

### Workspace Structure

```
ratchet/
├── ratchet-lib/          # Core library functionality
│   └── src/
├── ratchet-cli/          # Command-line interface
│   └── src/
├── sample/               # Example tasks and test data
├── docs/                 # Documentation
└── target/               # Build artifacts
```

### Library Module Organization

The `ratchet-lib` crate is organized into focused, single-responsibility modules:

```
ratchet-lib/src/
├── lib.rs                # Public API and module exports (30 lines)
├── errors.rs             # Centralized error type definitions (65 lines)
├── types.rs              # Type-safe enums and conversions (396 lines)
├── js_executor.rs        # JavaScript execution engine (588 lines)
├── task.rs               # Task loading and management (713 lines)
├── test.rs               # Test execution framework (449 lines)
├── generate.rs           # Task template generation (298 lines)
├── js_task.rs            # JavaScript task wrapper (107 lines)
├── validation/           # JSON schema validation
│   ├── mod.rs            # Module exports (2 lines)
│   └── schema.rs         # Validation logic (28 lines)
├── recording/            # Session recording functionality
│   ├── mod.rs            # Module exports (5 lines)
│   └── session.rs        # Recording implementation (216 lines)
└── http/                 # HTTP client functionality
    ├── mod.rs            # Module exports (9 lines)
    ├── manager.rs        # HTTP client implementation (307 lines)
    ├── errors.rs         # HTTP-specific errors (28 lines)
    ├── fetch.rs          # JavaScript fetch integration (120 lines)
    └── tests.rs          # HTTP testing suite (272 lines)
```

### Design Principles

1. **Single Responsibility**: Each module has one clear purpose
2. **Minimal Dependencies**: Modules depend only on what they need
3. **Clear Interfaces**: Public APIs are well-defined and documented
4. **Type Safety**: Strong typing throughout with minimal `unwrap()`
5. **Error Handling**: Comprehensive error types with context
6. **Testability**: All modules are thoroughly tested

## Module Structure

### Core Modules

#### `lib.rs` - Public API
- **Purpose**: Module exports and public API surface
- **Size**: 30 lines (97% reduction from original 1063 lines)
- **Contents**: Module declarations and re-exports for convenience
- **Dependencies**: All other modules

#### `errors.rs` - Error Types
- **Purpose**: Centralized error type definitions
- **Contents**: `JsErrorType`, `JsExecutionError` with comprehensive error variants
- **Design**: Hierarchical error types with rich context information

#### `types.rs` - Type Safety
- **Purpose**: Type-safe enums replacing string-based types
- **Contents**: `HttpMethod`, `LogLevel`, `TaskStatus` with conversions
- **Features**: Serialization, parsing, validation, and error handling

#### `js_executor.rs` - JavaScript Engine
- **Purpose**: JavaScript task execution and environment management
- **Contents**: Boa engine integration, error type registration, HTTP integration
- **Key Functions**: `execute_task()`, `call_js_function()`, error handling

#### `task.rs` - Task Management
- **Purpose**: Task loading, validation, and lifecycle management
- **Contents**: Task struct, file/ZIP loading, content caching, validation
- **Features**: Lazy loading, LRU caching, ZIP support

### Supporting Modules

#### `services/` - Service Layer
- **Purpose**: Business logic and cross-cutting concerns
- **Structure**:
  - `task_sync_service.rs`: Synchronizes registry and database tasks
  - Main service traits and implementations
- **Features**: 
  - Automatic task synchronization
  - Unified task view combining registry and database
  - Service provider pattern for dependency injection

#### `validation/` - Schema Validation
- **Purpose**: JSON schema validation for task inputs/outputs
- **Structure**: 
  - `schema.rs`: Core validation logic using jsonschema crate
  - `mod.rs`: Public API exports
- **Integration**: Used by js_executor for input/output validation

#### `recording/` - Session Recording
- **Purpose**: HTTP request recording and session management
- **Structure**:
  - `session.rs`: Recording state management and HAR file generation
  - `mod.rs`: Public API exports
- **Features**: HAR format output, thread-safe recording state

#### `http/` - HTTP Client
- **Purpose**: HTTP request handling with mock support
- **Structure**:
  - `manager.rs`: Main HTTP client implementation
  - `errors.rs`: HTTP-specific error types
  - `fetch.js`: JavaScript fetch API integration
  - `tests.rs`: Comprehensive test suite
  - `mod.rs`: Module exports and public API

#### `registry/` - Task Registry
- **Purpose**: Task discovery, loading, and version management
- **Structure**:
  - `registry.rs`: Core registry implementation with version management
  - `service.rs`: Registry service for loading from configured sources
  - `loaders/`: Task loader implementations
    - `filesystem.rs`: Loads tasks from directories, ZIP files, or collections
    - `http.rs`: HTTP loader stub for future implementation
  - `mod.rs`: Module exports and public API
- **Features**: 
  - Multi-source task loading (filesystem, HTTP)
  - Version management with duplicate detection
  - GraphQL API integration
  - Lazy loading with caching

## Process Execution IPC Model

### Overview

The Process Execution IPC (Inter-Process Communication) Model is a core architectural component that solves Send/Sync compliance issues by isolating JavaScript execution in separate worker processes. This enables the main coordinator process to remain fully thread-safe while still executing JavaScript tasks using the Boa engine.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Coordinator Process                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   GraphQL API   │  │   Job Queue     │  │   Database      │  │
│  │   (Send/Sync)   │  │   Manager       │  │   Repositories  │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │            ProcessTaskExecutor                              │  │
│  │  ┌─────────────────────────────────────────────────────────┐│  │
│  │  │          WorkerProcessManager                           ││  │
│  │  │                                                         ││  │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ ││  │
│  │  │  │ WorkerProcess│  │ WorkerProcess│  │ WorkerProcess│ ││  │
│  │  │  │     #1       │  │     #2       │  │     #3       │ ││  │
│  │  │  └──────────────┘  └──────────────┘  └──────────────┘ ││  │
│  │  └─────────────────────────────────────────────────────────┘│  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │ IPC Messages
                              │ (STDIN/STDOUT)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Worker Process #1                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    Worker                                   │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │  │
│  │  │  RatchetEngine  │  │  Task Cache     │  │ IPC Transport││  │
│  │  │  (Boa Engine)   │  │                 │  │ (Stdio)     ││  │
│  │  │  [NOT Send/Sync]│  │                 │  │             ││  │
│  │  └─────────────────┘  └─────────────────┘  └─────────────┘ │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Send/Sync Problem Solution

#### The Challenge
- **Boa JavaScript Engine**: Not Send/Sync compatible due to internal non-thread-safe structures
- **GraphQL/Axum Requirements**: Require Send/Sync bounds for multi-threaded async runtime
- **Direct Conflict**: Cannot use Boa engine directly in GraphQL resolvers or async handlers

#### The Solution
```rust
// ❌ This doesn't work - Boa engine is not Send/Sync
pub struct DirectExecutor {
    engine: RatchetEngine, // Contains Boa - not Send/Sync
}

// ✅ This works - ProcessTaskExecutor is Send/Sync
pub struct ProcessTaskExecutor {
    worker_manager: Arc<RwLock<WorkerProcessManager>>, // Send/Sync
    repositories: RepositoryFactory,                   // Send/Sync
    config: RatchetConfig,                            // Send/Sync
}

impl TaskExecutor for ProcessTaskExecutor {
    // This can be used in GraphQL resolvers safely
    async fn execute_task(&self, ...) -> Result<ExecutionResult, ExecutionError> {
        // Delegates to worker processes via IPC
    }
}
```

### IPC Protocol

#### Message Format
All messages use JSON serialization with versioned envelopes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub protocol_version: u32,      // For backward compatibility
    pub timestamp: DateTime<Utc>,   // For debugging and monitoring
    pub message: T,                 // Actual message payload
}
```

#### Message Types

**Coordinator → Worker Messages**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerMessage {
    ExecuteTask {
        job_id: i32,
        task_id: i32,
        task_path: String,
        input_data: JsonValue,
        correlation_id: Uuid,  // For request/response matching
    },
    ValidateTask {
        task_path: String,
        correlation_id: Uuid,
    },
    Ping {
        correlation_id: Uuid,
    },
    Shutdown,
}
```

**Worker → Coordinator Messages**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorMessage {
    TaskExecutionResponse {
        correlation_id: Uuid,
        result: TaskExecutionResult,
    },
    TaskExecutionError {
        correlation_id: Uuid,
        error: WorkerError,
    },
    WorkerStatusUpdate {
        status: WorkerStatus,
    },
    WorkerReady {
        worker_id: String,
        capabilities: Vec<String>,
    },
    Pong {
        correlation_id: Uuid,
    },
}
```

#### Transport Implementation
Communication uses STDIN/STDOUT with line-delimited JSON:

```rust
#[async_trait::async_trait]
pub trait IpcTransport {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn send<T: Serialize + Send + Sync>(
        &mut self, 
        message: &MessageEnvelope<T>
    ) -> Result<(), Self::Error>;
    
    async fn receive<T: for<'de> Deserialize<'de>>(
        &mut self
    ) -> Result<MessageEnvelope<T>, Self::Error>;
}

pub struct StdioTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}
```

### Process Lifecycle

#### Worker Startup Sequence
1. **Spawn**: Coordinator spawns worker process using `tokio::process::Command`
2. **Initialize**: Worker loads RatchetEngine and establishes IPC transport
3. **Handshake**: Worker sends `WorkerReady` message with capabilities
4. **Registration**: Coordinator adds worker to available pool
5. **Health Check**: Initial ping/pong to verify communication

#### Task Execution Flow
```
Coordinator                           Worker Process
     │                                      │
     │ 1. ExecuteTask{correlation_id}      │
     ├─────────────────────────────────────►│
     │                                      │ 2. Load task from filesystem
     │                                      │ 3. Validate input schema
     │                                      │ 4. Execute in Boa engine
     │                                      │ 5. Validate output schema
     │                                      │
     │ 6. TaskExecutionResponse            │
     ◄─────────────────────────────────────┤
     │                                      │
     │ 7. Update job status in database     │
     │                                      │
```

#### Error Handling and Recovery
- **Process Crash**: Detected via process exit code, automatic worker respawn
- **Communication Timeout**: Correlation IDs enable request timeout handling
- **Task Failure**: Detailed error information via `TaskExecutionError` messages
- **Resource Limits**: Process-level memory and CPU monitoring

### Integration with Existing Architecture

#### TaskExecutor Trait Compatibility
```rust
// Existing trait - no changes needed
#[async_trait(?Send)]
pub trait TaskExecutor {
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError>;
    
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError>;
    async fn health_check(&self) -> Result<(), ExecutionError>;
}

// ProcessTaskExecutor implements this trait
impl TaskExecutor for ProcessTaskExecutor {
    // Send/Sync compatible implementation using worker processes
}
```

#### GraphQL Context Integration
```rust
// Before: Could not include engine due to Send/Sync constraints
pub struct GraphQLContext {
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    // pub engine: RatchetEngine, // ❌ Not Send/Sync
}

// After: Process executor is Send/Sync compatible
pub struct GraphQLContext {
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    pub task_executor: Arc<ProcessTaskExecutor>, // ✅ Send/Sync
}
```

### Performance Characteristics

#### Benefits
- **True Parallelism**: Multiple worker processes can execute tasks simultaneously
- **Fault Isolation**: Worker crashes don't affect coordinator or other workers
- **Resource Management**: Per-process memory limits and monitoring
- **Scalability**: Worker pool can be scaled based on load

#### Trade-offs
- **Process Overhead**: Higher memory usage and spawn cost vs threads
- **IPC Latency**: Message serialization/deserialization overhead
- **Complexity**: More complex than direct in-process execution

#### Optimization Strategies
- **Process Pooling**: Reuse worker processes for multiple tasks
- **Task Batching**: Send multiple tasks per worker process
- **Caching**: Cache parsed tasks and schemas in worker processes
- **Binary Protocol**: Consider binary serialization for performance-critical paths

### Configuration and Monitoring

#### Worker Configuration
```rust
pub struct WorkerConfig {
    pub worker_count: usize,              // Number of worker processes
    pub max_tasks_per_worker: u32,        // Restart threshold
    pub worker_timeout: Duration,         // Task execution timeout
    pub health_check_interval: Duration,  // Health monitoring frequency
    pub restart_delay: Duration,          // Delay before worker restart
    pub max_restarts: u32,               // Maximum restart attempts
}
```

#### Monitoring and Observability
- **Worker Health**: Process status, memory usage, task counts
- **IPC Metrics**: Message throughput, latency, error rates
- **Task Execution**: Success rates, execution times, error patterns
- **Resource Usage**: Memory consumption, CPU utilization per worker

### Security Considerations

#### Process Isolation
- **Sandboxing**: Each worker runs in separate process space
- **Resource Limits**: OS-level memory and CPU constraints
- **File System**: Limited file system access for workers
- **Network**: No direct network access (coordinator proxies HTTP requests)

#### Data Flow Security
- **Input Validation**: All task inputs validated before worker execution
- **Output Sanitization**: Task outputs validated before returning to client
- **Error Information**: Sensitive data filtered from error messages
- **Audit Trail**: All IPC messages logged for security monitoring

## Database Architecture

### Overview

Ratchet uses SQLite with Sea-ORM for persistent storage of tasks, executions, jobs, and schedules. The database layer provides full CRUD operations with proper relationship management and migration support.

### Entity Relationship Diagram

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│      Tasks      │       │   Executions    │       │      Jobs       │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id (PK)         │◄──────┤│ id (PK)         │       │ id (PK)         │
│ uuid            │   1:N ││ uuid            │   N:1 │ uuid            │
│ name            │       ││ task_id (FK)    │──────►│ task_id (FK)    │
│ description     │       ││ job_id (FK)     │◄──────┤│ priority        │
│ input_schema    │       ││ status          │   1:N ││ status          │
│ output_schema   │       ││ started_at      │       ││ created_at      │
│ content         │       ││ completed_at    │       ││ scheduled_for   │
│ created_at      │       ││ error_message   │       ││ retry_count     │
│ updated_at      │       ││ input_data      │       ││ max_retries     │
└─────────────────┘       ││ output_data     │       ││ metadata        │
                          ││ execution_time  │       └─────────────────┘
                          │└─────────────────┘                │
                          │                                    │
                          │ ┌─────────────────┐               │
                          │ │   Schedules     │               │
                          │ ├─────────────────┤               │
                          │ │ id (PK)         │               │
                          │ │ uuid            │               │
                          │ │ task_id (FK)    │               │
                          │ │ cron_expression │               │
                          └►│ last_run        │◄──────────────┘
                            │ next_run        │
                            │ is_active       │
                            │ created_at      │
                            │ updated_at      │
                            └─────────────────┘
```

## Task Registry Architecture

### Overview

The Task Registry provides a centralized system for discovering, loading, and managing tasks from multiple sources. It supports filesystem and HTTP sources (HTTP is currently stubbed), with automatic version management and duplicate detection.

### Architecture Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Task Registry                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   TaskRegistry  │  │ RegistryService │  │  Task Loaders   │  │
│  │                 │  │                 │  │                 │  │
│  │ - Version Map   │  │ - Load Sources  │  │ - Filesystem    │  │
│  │ - Task Storage  │  │ - Initialize    │  │ - HTTP (stub)   │  │
│  │ - Dedup Logic   │  │ - Coordinate    │  │ - Future: Git   │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Task Sources                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  Directory   │  │   ZIP File   │  │  Collection  │         │
│  │              │  │              │  │              │         │
│  │ metadata.json│  │ task.zip     │  │ ├── task1/  │         │
│  │ input.schema │  │ └── task/    │  │ ├── task2.zip│        │
│  │ output.schema│  │     ├── ...  │  │ └── task3/  │         │
│  │ main.js      │  │              │  │              │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### Registry Data Model

```rust
pub struct TaskRegistry {
    // Task ID -> Version -> Task
    tasks: Arc<RwLock<HashMap<Uuid, HashMap<String, Arc<Task>>>>>,
    sources: Vec<TaskSource>,
}

pub enum TaskSource {
    Filesystem { path: PathBuf },
    Http { url: String },  // Future implementation
}
```

### Task Loading Process

1. **Source Configuration**: Registry sources defined in YAML config
2. **Source Parsing**: URIs parsed into TaskSource enum variants
3. **Task Discovery**: Loaders scan sources for task directories/ZIPs
4. **Version Management**: Tasks indexed by ID and version
5. **Duplicate Detection**: Warns on duplicate ID/version combinations
6. **GraphQL Exposure**: Registry contents queryable via GraphQL API

### Configuration

```yaml
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch: true  # Future: filesystem watching
    - name: "remote-registry"
      uri: "https://registry.example.com/tasks"  # Future
      config:
        auth_token: "${REGISTRY_TOKEN}"
```

### Unified Task System

The registry and database work together through the TaskSyncService:

1. **Registry**: Authoritative source for task definitions (code, schemas)
2. **Database**: Stores task metadata and execution history
3. **TaskSyncService**: Automatically synchronizes registry tasks to database
4. **UnifiedTask**: Combined view presenting both registry and database information

### GraphQL API

The unified task system exposes a single, consistent interface:

```graphql
type Query {
  # List all tasks from unified registry/database view
  tasks(pagination: PaginationInput): UnifiedTaskListResponse!
  
  # Get a specific task by UUID and optional version
  task(uuid: ID!, version: String): UnifiedTask
}

type UnifiedTask {
  # Database ID (if task exists in database)
  id: Int
  # Task UUID from registry
  uuid: ID!
  # Current version
  version: String!
  # Task label/name
  label: String!
  # Task description
  description: String!
  # All available versions in registry
  availableVersions: [String!]!
  # Whether task is from registry
  registrySource: Boolean!
  # Whether task is enabled for execution
  enabled: Boolean!
  # Database timestamps
  createdAt: DateTime
  updatedAt: DateTime
  validatedAt: DateTime
  # Sync status between registry and database
  inSync: Boolean!
}
```

### Integration Points

1. **Server Startup**: 
   - Registry initialized from config during server boot
   - TaskSyncService created to bridge registry and database
   - All registry tasks automatically synced to database

2. **GraphQL Context**: 
   - TaskSyncService passed to GraphQL resolvers
   - Unified queries use sync service for consistent view
   - Fallback to database-only mode if registry unavailable

3. **Task Execution**: 
   - Executions reference tasks by database ID
   - Task content loaded from registry at execution time
   - Execution history stored in database

4. **Data Flow**:
   ```
   Registry (Source) → TaskSyncService → Database (Reference)
                            ↓
                      GraphQL API
                            ↓
                      UnifiedTask View
   ```

## Server Architecture

### Overview

Ratchet provides a complete server implementation with GraphQL API, REST endpoints, and background job processing. The server architecture follows clean architecture principles with clear separation between API, business logic, and data persistence layers.

## Configuration Management

### Configuration Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetConfig {
    pub server: Option<ServerConfig>,
    pub database: Option<DatabaseConfig>,
    pub execution: Option<ExecutionConfig>,
    pub logging: Option<LoggingConfig>,
}
```

The configuration system provides comprehensive management of all Ratchet settings with YAML file loading and environment variable overrides. See the complete implementation in the server architecture section above.

## Conventions

### Naming Conventions

#### Modules
- **snake_case**: All module names use snake_case (e.g., `js_executor`, `http_manager`)
- **Descriptive**: Names clearly indicate module purpose
- **Consistent**: Related functionality grouped under common prefixes

#### Types
- **PascalCase**: All type names use PascalCase (e.g., `HttpMethod`, `TaskStatus`)
- **Descriptive**: Names indicate the type's purpose and domain
- **Suffixed**: Error types end with `Error` (e.g., `JsExecutionError`)

#### Functions
- **snake_case**: All function names use snake_case
- **Verb-based**: Functions start with verbs (e.g., `execute_task`, `validate_json`)
- **Clear intent**: Names indicate what the function does

#### Constants
- **SCREAMING_SNAKE_CASE**: All constants use SCREAMING_SNAKE_CASE
- **Descriptive**: Names clearly indicate the constant's purpose
- **Grouped**: Related constants are grouped together

### Code Organization

#### File Structure
```rust
// 1. Imports - organized by scope
use std::collections::HashMap;     // Standard library
use serde::{Deserialize, Serialize}; // External crates  
use crate::errors::HttpError;      // Internal modules

// 2. Types - public then private
pub struct PublicType { }
struct PrivateType { }

// 3. Constants
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// 4. Implementations
impl PublicType {
    pub fn new() -> Self { }       // Constructors first
    pub fn public_method(&self) { } // Public methods
    fn private_method(&self) { }   // Private methods
}

// 5. Functions - public then private
pub fn public_function() { }
fn private_function() { }

// 6. Tests
#[cfg(test)]
mod tests { }
```

#### Import Organization
1. **Standard library**: `std::*` imports
2. **External crates**: Third-party dependencies
3. **Internal modules**: `crate::*` imports
4. **Blank lines**: Separate each group

#### Documentation
- **Module docs**: Every public module has comprehensive documentation
- **Function docs**: All public functions have doc comments
- **Example usage**: Complex APIs include usage examples
- **Error documentation**: Error conditions are documented

### Error Handling Patterns

#### Result Types
```rust
// Always use Result for fallible operations
pub fn execute_task(task: &Task) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}

// Use specific error types, not generic Error
pub fn parse_schema(path: &Path) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}
```

#### Error Propagation
```rust
// Use ? operator for error propagation
pub fn complex_operation() -> Result<(), MyError> {
    let data = load_data()?;          // Propagate LoadError
    let processed = process(data)?;    // Propagate ProcessError
    save_result(processed)?;          // Propagate SaveError
    Ok(())
}

// Add context when helpful
pub fn load_task(path: &Path) -> Result<Task, TaskError> {
    Task::from_fs(path)
        .with_context(|| format!("Failed to load task from: {}", path.display()))
}
```

## Error Handling

### Error Type Hierarchy

```rust
// Top-level error categories
pub enum JsExecutionError {
    FileReadError(#[from] std::io::Error),
    CompileError(String),
    ExecutionError(String),
    TypedJsError(#[from] JsErrorType),
    SchemaValidationError(String),
    // ...
}

// Domain-specific JavaScript errors
pub enum JsErrorType {
    AuthenticationError(String),
    AuthorizationError(String),
    NetworkError(String),
    HttpError { status: u16, message: String },
    // ...
}
```

### Error Design Principles

#### 1. **Hierarchical Structure**
- **Category errors**: Broad error categories (e.g., `JsExecutionError`)
- **Specific errors**: Detailed error types (e.g., `AuthenticationError`)
- **Context preservation**: Errors maintain context through the call stack

#### 2. **Rich Error Information**
```rust
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(#[from] HttpMethodError),

    #[error("HTTP error {status}: {message}")]
    HttpStatusError { status: u16, message: String },
}
```

#### 3. **Error Conversion**
- **Automatic conversion**: Use `#[from]` for automatic conversions
- **Context addition**: Add context when converting between error types
- **Preservation**: Maintain original error information

#### 4. **User-Friendly Messages**
```rust
#[error("Invalid HTTP method: '{0}'. Supported methods are: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS")]
InvalidMethod(String),

#[error("Invalid log level: '{0}'. Supported levels are: trace, debug, info, warn, error")]
InvalidLevel(String),
```

### Error Handling Best Practices

#### 1. **Fail Fast**
- Validate inputs early and return errors immediately
- Use type system to prevent errors at compile time
- Prefer `Result` over panics for recoverable errors

#### 2. **Error Context**
```rust
// Good: Provides context about what failed
fn load_task_file(path: &Path) -> Result<String, TaskError> {
    std::fs::read_to_string(path)
        .map_err(|e| TaskError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })
}

// Better: Use with_context for dynamic messages
fn process_task(name: &str) -> Result<Task, TaskError> {
    load_task_file(&format!("{}.json", name))
        .with_context(|| format!("Failed to process task: {}", name))
}
```

#### 3. **Error Recovery**
```rust
// Provide fallback mechanisms where appropriate
pub fn get_method_or_default(params: &JsonValue) -> HttpMethod {
    params.get("method")
        .and_then(|m| m.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(HttpMethod::Get)  // Safe default
}
```

## Type Safety

### Strongly Typed APIs

#### Replace String Types
```rust
// Before: Error-prone string handling
fn add_mock(method: &str, url: &str, response: JsonValue) {
    // "GET", "get", "Get" all different - runtime errors
}

// After: Compile-time safety
fn add_mock(method: HttpMethod, url: &str, response: JsonValue) {
    // Only valid HttpMethod values accepted
}
```

#### Enum Design
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get, Post, Put, Delete, Patch, Head, Options
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str { /* ... */ }
    pub fn all() -> &'static [HttpMethod] { /* ... */ }
}

impl FromStr for HttpMethod {
    type Err = HttpMethodError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { /* ... */ }
}
```

### Validation and Conversion

#### Parse, Don't Validate
```rust
// Good: Parse into validated type
pub fn parse_log_level(s: &str) -> Result<LogLevel, LogLevelError> {
    match s.to_lowercase().as_str() {
        "debug" => Ok(LogLevel::Debug),
        "info" => Ok(LogLevel::Info),
        // ...
        _ => Err(LogLevelError::InvalidLevel(s.to_string())),
    }
}

// Use the parsed type throughout the system
fn configure_logging(level: LogLevel) {
    // level is guaranteed to be valid
}
```

## Testing Strategy

### Test Organization

#### Module Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_functionality() { }
    
    #[tokio::test]
    async fn test_async_functionality() { }
}
```

#### Integration Tests
- **Location**: `tests/` directory in each crate
- **Purpose**: Test public APIs and cross-module interactions
- **Scope**: End-to-end functionality testing

#### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test module interactions
3. **Property Tests**: Test invariants and edge cases
4. **Performance Tests**: Benchmark critical paths

### Test Patterns

#### Arrange, Act, Assert
```rust
#[test]
fn test_http_method_parsing() {
    // Arrange
    let input = "POST";
    
    // Act
    let result = HttpMethod::from_str(input);
    
    // Assert
    assert_eq!(result.unwrap(), HttpMethod::Post);
}
```

#### Error Testing
```rust
#[test]
fn test_invalid_method_error() {
    let result = HttpMethod::from_str("INVALID");
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("INVALID"));
    assert!(error.to_string().contains("GET, POST, PUT"));
}
```

## Development Guidelines

### Code Quality

#### 1. **Clippy Compliance**
- Run `cargo clippy` regularly and address warnings
- Use `#[allow(clippy::lint_name)]` sparingly and with justification
- Follow Clippy suggestions for idiomatic Rust

#### 2. **Formatting**
- Use `cargo fmt` for consistent code formatting
- Configure editor to format on save
- Follow Rust standard formatting conventions

#### 3. **Documentation**
```rust
/// Execute a JavaScript task with the given input data.
/// 
/// This function loads the task content, validates input against the schema,
/// executes the JavaScript code in a secure environment, and validates the output.
/// 
/// # Arguments
/// 
/// * `task` - The task to execute (will be modified to load content)
/// * `input_data` - Input data that must match the task's input schema
/// * `http_manager` - HTTP client for fetch API calls
/// 
/// # Returns
/// 
/// Returns the task output as JSON if successful, or a `JsExecutionError` if:
/// - The task content cannot be loaded
/// - Input validation fails
/// - JavaScript execution fails
/// - Output validation fails
/// 
/// # Example
/// 
/// ```rust
/// use ratchet_lib::{Task, HttpManager, execute_task};
/// use serde_json::json;
/// 
/// let mut task = Task::from_fs("path/to/task")?;
/// let input = json!({"num1": 5, "num2": 10});
/// let http_manager = HttpManager::new();
/// 
/// let result = execute_task(&mut task, input, &http_manager).await?;
/// println!("Result: {}", result);
/// ```
pub async fn execute_task(
    task: &mut Task,
    input_data: JsonValue,
    http_manager: &HttpManager,
) -> Result<JsonValue, JsExecutionError> {
    // Implementation
}
```

### Performance Considerations

#### 1. **Async/Await Usage**
- Use async functions for I/O operations
- Avoid blocking operations in async contexts
- Use `tokio::spawn` for independent concurrent tasks

#### 2. **Memory Management**
- Use `Arc` for shared ownership of immutable data
- Use `Rc` for single-threaded shared ownership
- Implement caching for expensive computations

#### 3. **Error Handling Performance**
- Use `Result` instead of exceptions for control flow
- Avoid string allocations in hot paths
- Use static strings for error messages when possible

### Security Guidelines

#### 1. **JavaScript Execution**
- Validate all inputs before JavaScript execution
- Limit resource usage in JavaScript environment
- Sanitize outputs from JavaScript execution

#### 2. **HTTP Requests**
- Validate URLs before making requests
- Implement request timeouts
- Use type-safe HTTP methods and headers

#### 3. **File Operations**
- Validate file paths to prevent directory traversal
- Use safe file operations with proper error handling
- Implement size limits for file operations

---

This architecture document serves as a living guide for maintaining and extending the Ratchet codebase. It should be updated as the architecture evolves and new patterns emerge.