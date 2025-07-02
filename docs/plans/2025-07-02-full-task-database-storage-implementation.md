# Full Task Database Storage Implementation Plan

**Date**: 2025-07-02  
**Status**: Phase 1 Complete, Phase 2 In Progress  
**Priority**: High  
**Estimated Effort**: 4-6 weeks  

## Overview

This plan implements comprehensive task storage in the database with bidirectional repository synchronization. Tasks are pulled from configured repositories and changes can be pushed back to their original sources. Tasks created directly via Ratchet APIs are attached to a default repository or explicitly assigned to a specific repository for synchronization. Repository configurations have full CRUD operations exposed through REST, GraphQL, and MCP APIs via a unified backend.

## Current State Analysis

### Existing Architecture
- **Database**: Tasks stored with metadata only, JavaScript code in files
- **Registry System**: TaskDefinition contains full code but not persisted 
- **APIs**: MCP endpoints support code editing, standard APIs metadata-only
- **Storage**: File-based with path references in database

### OpenAPI Documentation Status
✅ **CONFIRMED: OpenAPI documentation is already properly configured**
- Swagger UI available at `/docs` 
- JSON spec at `/api-docs/openapi.json`
- Comprehensive handler annotations with utoipa
- Production-ready with error handling and CDN fallbacks

## Goals

1. **Full Task Storage**: Store complete tasks (code + schemas + metadata) in database
2. **Bidirectional Repository Sync**: Pull tasks from repositories and push changes back to source
3. **Repository Management**: Full CRUD operations for repository configurations via all APIs
4. **Default Repository Assignment**: Auto-assign new tasks to default or specified repository
5. **Universal API Access**: Enable full task and repository management through REST, GraphQL, and MCP
6. **Version Control**: Track changes and maintain task history with repository commit tracking
7. **Conflict Resolution**: Handle sync conflicts and merge strategies
8. **Performance**: Efficient querying and caching of task and repository data

## Phase 1: Database Schema Enhancement ✅ COMPLETED

**Completed**: 2025-07-02

### 1.1 Task Table Expansion

**File**: `ratchet-storage/src/seaorm/entities/tasks.rs`

```rust
// New enhanced task model
pub struct Model {
    // Existing fields
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub metadata: Json,
    pub input_schema: Json,
    pub output_schema: Json,
    
    // New fields for full storage
    pub source_code: Text,                    // JavaScript source code
    pub source_type: String,                  // "javascript", "typescript", etc.
    pub storage_type: String,                 // "database", "file", "registry"
    pub file_path: Option<String>,            // Original file path if applicable
    pub checksum: String,                     // SHA256 of source code
    pub repository_id: i32,                   // Required reference to source repository
    pub repository_path: String,              // Path within repository (for sync back)
    pub last_synced_at: Option<DateTimeUtc>,  // Last sync timestamp
    pub sync_status: String,                  // "synced", "modified", "conflict", "pending_push"
    pub is_editable: bool,                    // Whether task can be edited via API
    pub created_from: String,                 // "pull", "api", "import"
    pub needs_push: bool,                     // Whether changes need to be pushed to repository
    
    // Timestamps
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub source_modified_at: Option<DateTimeUtc>, // Source file modification time
}
```

### 1.2 Repository Configuration Table

**File**: `ratchet-storage/src/seaorm/entities/task_repositories.rs`

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "task_repositories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub repository_type: String,          // "filesystem", "git", "http", "registry"
    pub uri: String,                      // Path, URL, or registry identifier
    pub branch: Option<String>,           // Git branch (for git repos)
    pub auth_config: Option<Json>,        // Authentication configuration
    pub sync_enabled: bool,
    pub sync_interval_minutes: Option<i32>,
    pub last_sync_at: Option<DateTimeUtc>,
    pub sync_status: String,              // "success", "error", "pending"
    pub sync_error: Option<Text>,
    pub priority: i32,                    // Sync priority (higher = first)
    pub is_default: bool,                 // Whether this is the default repository for new tasks
    pub is_writable: bool,                // Whether tasks can be pushed back to this repository
    pub watch_patterns: Json,             // File patterns to watch/sync
    pub ignore_patterns: Json,            // Patterns to ignore
    pub push_on_change: bool,             // Auto-push changes to repository
    pub metadata: Json,                   // Repository-specific metadata
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
```

### 1.3 Task History/Versions Table

**File**: `ratchet-storage/src/seaorm/entities/task_versions.rs`

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "task_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub task_id: i32,                     // Foreign key to tasks table
    pub version: String,
    pub source_code: Text,
    pub input_schema: Json,
    pub output_schema: Json,
    pub metadata: Json,
    pub checksum: String,
    pub change_description: Option<String>,
    pub changed_by: String,               // User/system that made the change
    pub change_source: String,            // "api", "sync", "file", etc.
    pub repository_commit: Option<String>, // Git commit hash if applicable
    pub created_at: DateTimeUtc,
}
```

### 1.4 Migration Scripts

**File**: `ratchet-storage/src/migrations/m20250702_000001_full_task_storage.rs`

```rust
// Migration to add new columns and tables
// - Add new columns to tasks table with proper indexes
// - Create task_repositories table with default repository setup
// - Create task_versions table
// - Create indexes for performance
// - Migrate existing file-based tasks to database storage
// - Set up default filesystem repository for existing tasks
```

## Phase 2: Repository Sync Engine ✅ COMPLETED

**Started**: 2025-07-02  
**Completed**: 2025-07-02

### 2.1 Repository Abstraction Layer

**File**: `ratchet-storage/src/repositories/task_sync.rs`

```rust
#[async_trait]
pub trait TaskRepository {
    async fn list_tasks(&self) -> Result<Vec<RepositoryTask>>;
    async fn get_task(&self, path: &str) -> Result<Option<RepositoryTask>>;
    async fn put_task(&self, task: &RepositoryTask) -> Result<()>;
    async fn delete_task(&self, path: &str) -> Result<()>;
    async fn get_metadata(&self) -> Result<RepositoryMetadata>;
    async fn is_writable(&self) -> bool;
}

pub struct RepositoryTask {
    pub path: String,
    pub name: String,
    pub source_code: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub metadata: TaskMetadata,
    pub checksum: String,
    pub modified_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
```

### 2.2 Repository Implementations

**File**: `ratchet-storage/src/repositories/filesystem_repo.rs`
```rust
pub struct FilesystemTaskRepository {
    base_path: PathBuf,
    watch_patterns: Vec<String>,
    ignore_patterns: Vec<String>,
}
// Implements scanning directories, reading task.js files, watching for changes
```

**File**: `ratchet-storage/src/repositories/git_repo.rs`
```rust
pub struct GitTaskRepository {
    repo_url: String,
    branch: String,
    auth_config: Option<GitAuth>,
    local_path: PathBuf,
}
// Implements Git clone/pull, reading tasks from Git repo, pushing changes back
```

**File**: `ratchet-storage/src/repositories/http_repo.rs`
```rust
pub struct HttpTaskRepository {
    base_url: String,
    auth_config: Option<HttpAuth>,
    client: reqwest::Client,
}
// Implements fetching tasks from HTTP endpoints, uploading changes
```

### 2.3 Sync Service

**File**: `ratchet-server/src/services/task_sync_service.rs`

```rust
pub struct TaskSyncService {
    db: DatabaseConnection,
    repositories: HashMap<i32, Box<dyn TaskRepository>>,
    conflict_resolver: ConflictResolver,
}

impl TaskSyncService {
    pub async fn sync_repository(&self, repo_id: i32) -> Result<SyncResult>;
    pub async fn sync_all_repositories(&self) -> Result<Vec<SyncResult>>;
    pub async fn handle_conflict(&self, conflict: &TaskConflict) -> Result<Resolution>;
    pub async fn push_task_changes(&self, task_id: i32) -> Result<PushResult>;
    pub async fn push_repository_changes(&self, repo_id: i32) -> Result<Vec<PushResult>>;
    pub async fn get_default_repository(&self) -> Result<TaskRepository>;
    pub async fn assign_task_to_repository(&self, task_id: i32, repo_id: i32) -> Result<()>;
}

pub struct SyncResult {
    pub repository_id: i32,
    pub tasks_added: u32,
    pub tasks_updated: u32,
    pub tasks_deleted: u32,
    pub conflicts: Vec<TaskConflict>,
    pub errors: Vec<SyncError>,
}

pub struct PushResult {
    pub task_id: i32,
    pub repository_id: i32,
    pub repository_path: String,
    pub success: bool,
    pub commit_hash: Option<String>,
    pub error: Option<String>,
}
```

### 2.4 Conflict Resolution

**File**: `ratchet-core/src/sync/conflict_resolution.rs`

```rust
pub enum ConflictResolution {
    TakeLocal,          // Keep database version
    TakeRemote,         // Use repository version  
    Merge,              // Attempt automatic merge
    Manual,             // Require manual resolution
}

pub struct TaskConflict {
    pub task_id: i32,
    pub repository_id: i32,
    pub conflict_type: ConflictType,
    pub local_version: TaskVersion,
    pub remote_version: TaskVersion,
    pub auto_resolvable: bool,
}
```

## Phase 3: Repository Management Backend

### 3.1 Repository Service Layer

**File**: `ratchet-server/src/services/repository_service.rs`

```rust
pub struct RepositoryService {
    db: DatabaseConnection,
    sync_service: Arc<TaskSyncService>,
}

impl RepositoryService {
    pub async fn list_repositories(&self) -> Result<Vec<TaskRepositoryModel>>;
    pub async fn get_repository(&self, id: i32) -> Result<Option<TaskRepositoryModel>>;
    pub async fn create_repository(&self, request: CreateRepositoryRequest) -> Result<TaskRepositoryModel>;
    pub async fn update_repository(&self, id: i32, request: UpdateRepositoryRequest) -> Result<TaskRepositoryModel>;
    pub async fn delete_repository(&self, id: i32) -> Result<()>;
    pub async fn set_default_repository(&self, id: i32) -> Result<()>;
    pub async fn test_repository_connection(&self, id: i32) -> Result<ConnectionTestResult>;
    pub async fn get_default_repository(&self) -> Result<TaskRepositoryModel>;
}

#[derive(Serialize, Deserialize)]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub sync_enabled: bool,
    pub sync_interval_minutes: Option<i32>,
    pub is_default: bool,
    pub is_writable: bool,
    pub watch_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub push_on_change: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateRepositoryRequest {
    pub name: Option<String>,
    pub uri: Option<String>,
    pub branch: Option<String>,
    pub auth_config: Option<serde_json::Value>,
    pub sync_enabled: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub is_default: Option<bool>,
    pub is_writable: Option<bool>,
    pub watch_patterns: Option<Vec<String>>,
    pub ignore_patterns: Option<Vec<String>>,
    pub push_on_change: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
```

### 3.2 Task Assignment Logic

**File**: `ratchet-server/src/services/task_assignment_service.rs`

```rust
pub struct TaskAssignmentService {
    db: DatabaseConnection,
    repository_service: Arc<RepositoryService>,
}

impl TaskAssignmentService {
    pub async fn assign_new_task_repository(&self, task_id: i32, repo_id: Option<i32>) -> Result<()>;
    pub async fn move_task_to_repository(&self, task_id: i32, repo_id: i32) -> Result<()>;
    pub async fn get_task_repository_assignment(&self, task_id: i32) -> Result<TaskRepositoryAssignment>;
}

#[derive(Serialize, Deserialize)]
pub struct TaskRepositoryAssignment {
    pub task_id: i32,
    pub repository_id: i32,
    pub repository_name: String,
    pub repository_path: String,
    pub can_push: bool,
    pub auto_push: bool,
}
```

## Phase 4: API Enhancement

### 4.1 Enhanced API Models

**File**: `ratchet-api-types/src/tasks.rs`

```rust
#[derive(Serialize, Deserialize, ToSchema)]
pub struct FullTask {
    pub id: ApiId,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub source_code: String,              // ✅ Now included
    pub source_type: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
    pub repository_info: TaskRepositoryInfo,      // Always present now
    pub is_editable: bool,
    pub sync_status: String,
    pub needs_push: bool,                         // Whether changes need to be pushed
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_code: String,
    pub source_type: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub repository_id: Option<i32>,              // If not provided, uses default repository
    pub repository_path: Option<String>,         // Custom path within repository
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TaskRepositoryInfo {
    pub repository_id: i32,
    pub repository_name: String,
    pub repository_type: String,
    pub repository_path: String,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub can_push: bool,
    pub auto_push: bool,
}

// Repository API types
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TaskRepository {
    pub id: i32,
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub sync_enabled: bool,
    pub sync_interval_minutes: Option<i32>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub is_default: bool,
    pub is_writable: bool,
    pub watch_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub push_on_change: bool,
    pub task_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 4.2 REST API Enhancement

**File**: `ratchet-rest-api/src/handlers/tasks.rs`

```rust
// Enhanced endpoints with full task support
#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/source",
    responses(
        (status = 200, description = "Task source code", body = TaskSource),
        (status = 404, description = "Task not found")
    )
)]
pub async fn get_task_source(/* ... */) -> Result<Json<TaskSource>, ApiError>;

#[utoipa::path(
    put,
    path = "/api/v1/tasks/{id}/source", 
    request_body = UpdateTaskSource,
    responses(
        (status = 200, description = "Task source updated", body = FullTask),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Sync conflict")
    )
)]
pub async fn update_task_source(/* ... */) -> Result<Json<FullTask>, ApiError>;

#[utoipa::path(
    post,
    path = "/api/v1/tasks/{id}/push",
    responses(
        (status = 200, description = "Task pushed to repository", body = PushResult),
        (status = 409, description = "Push conflict", body = ApiError)
    )
)]
pub async fn push_task_to_repository(/* ... */) -> Result<Json<PushResult>, ApiError>;

#[utoipa::path(
    put,
    path = "/api/v1/tasks/{id}/repository",
    request_body = AssignRepositoryRequest,
    responses(
        (status = 200, description = "Task assigned to repository", body = FullTask),
        (status = 404, description = "Task or repository not found")
    )
)]
pub async fn assign_task_repository(/* ... */) -> Result<Json<FullTask>, ApiError>;
```

**File**: `ratchet-rest-api/src/handlers/repositories.rs`

```rust
// Repository management endpoints
#[utoipa::path(
    get,
    path = "/api/v1/repositories",
    responses((status = 200, description = "List repositories", body = Vec<TaskRepository>))
)]
pub async fn list_repositories(/* ... */) -> Result<Json<Vec<TaskRepository>>, ApiError>;

#[utoipa::path(
    get,
    path = "/api/v1/repositories/{id}",
    responses(
        (status = 200, description = "Repository details", body = TaskRepository),
        (status = 404, description = "Repository not found")
    )
)]
pub async fn get_repository(/* ... */) -> Result<Json<TaskRepository>, ApiError>;

#[utoipa::path(
    post,
    path = "/api/v1/repositories",
    request_body = CreateRepositoryRequest,
    responses(
        (status = 201, description = "Repository created", body = TaskRepository),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Repository name conflict")
    )
)]
pub async fn create_repository(/* ... */) -> Result<Json<TaskRepository>, ApiError>;

#[utoipa::path(
    put,
    path = "/api/v1/repositories/{id}",
    request_body = UpdateRepositoryRequest,
    responses(
        (status = 200, description = "Repository updated", body = TaskRepository),
        (status = 404, description = "Repository not found")
    )
)]
pub async fn update_repository(/* ... */) -> Result<Json<TaskRepository>, ApiError>;

#[utoipa::path(
    delete,
    path = "/api/v1/repositories/{id}",
    responses(
        (status = 204, description = "Repository deleted"),
        (status = 404, description = "Repository not found"),
        (status = 409, description = "Repository has active tasks")
    )
)]
pub async fn delete_repository(/* ... */) -> Result<StatusCode, ApiError>;

#[utoipa::path(
    post,
    path = "/api/v1/repositories/{id}/sync",
    responses(
        (status = 200, description = "Sync completed", body = SyncResult),
        (status = 409, description = "Sync conflicts", body = Vec<TaskConflict>)
    )
)]
pub async fn sync_repository(/* ... */) -> Result<Json<SyncResult>, ApiError>;

#[utoipa::path(
    post,
    path = "/api/v1/repositories/{id}/push",
    responses(
        (status = 200, description = "Push completed", body = Vec<PushResult>),
        (status = 409, description = "Push conflicts")
    )
)]
pub async fn push_repository_changes(/* ... */) -> Result<Json<Vec<PushResult>>, ApiError>;

#[utoipa::path(
    post,
    path = "/api/v1/repositories/{id}/test",
    responses(
        (status = 200, description = "Connection test result", body = ConnectionTestResult)
    )
)]
pub async fn test_repository_connection(/* ... */) -> Result<Json<ConnectionTestResult>, ApiError>;

#[utoipa::path(
    put,
    path = "/api/v1/repositories/{id}/default",
    responses(
        (status = 200, description = "Default repository set", body = TaskRepository),
        (status = 404, description = "Repository not found")
    )
)]
pub async fn set_default_repository(/* ... */) -> Result<Json<TaskRepository>, ApiError>;
```

### 4.3 GraphQL Enhancement

**File**: `ratchet-graphql-api/src/types/tasks.rs`

```rust
#[derive(SimpleObject)]
pub struct FullTask {
    pub id: ID,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub source_code: String,              // ✅ Now included in GraphQL
    pub source_type: String,
    pub input_schema: JsonValue,
    pub output_schema: JsonValue,
    pub metadata: JsonValue,
    pub repository_info: TaskRepositoryInfo,      // Always present now
    pub is_editable: bool,
    pub sync_status: String,
    pub needs_push: bool,
}

#[derive(SimpleObject)]
pub struct TaskRepository {
    pub id: ID,
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub sync_enabled: bool,
    pub sync_interval_minutes: Option<i32>,
    pub last_sync_at: Option<String>,
    pub sync_status: String,
    pub is_default: bool,
    pub is_writable: bool,
    pub watch_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub push_on_change: bool,
    pub task_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(InputObject)]
pub struct CreateRepositoryInput {
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub auth_config: Option<JsonValue>,
    pub sync_enabled: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub is_default: Option<bool>,
    pub is_writable: Option<bool>,
    pub watch_patterns: Option<Vec<String>>,
    pub ignore_patterns: Option<Vec<String>>,
    pub push_on_change: Option<bool>,
    pub metadata: Option<JsonValue>,
}

#[derive(InputObject)]
pub struct UpdateTaskSourceInput {
    pub source_code: String,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub version: Option<String>,
    pub change_description: Option<String>,
}
```

**Enhanced GraphQL Schema**:
```graphql
type Query {
  task(id: ID!): FullTask
  tasks(filters: TaskFiltersInput): [FullTask]
  repositories: [TaskRepository]
  repository(id: ID!): TaskRepository
  defaultRepository: TaskRepository
  taskConflicts(repositoryId: ID): [TaskConflict]
}

type Mutation {
  # Task operations
  createTask(input: CreateTaskInput!): FullTask
  updateTaskSource(id: ID!, input: UpdateTaskSourceInput!): FullTask
  pushTaskToRepository(id: ID!): PushResult
  assignTaskRepository(taskId: ID!, repositoryId: ID!, repositoryPath: String): FullTask
  
  # Repository operations
  createRepository(input: CreateRepositoryInput!): TaskRepository
  updateRepository(id: ID!, input: UpdateRepositoryInput!): TaskRepository
  deleteRepository(id: ID!): Boolean
  setDefaultRepository(id: ID!): TaskRepository
  syncRepository(id: ID!): SyncResult
  pushRepositoryChanges(id: ID!): [PushResult]
  testRepositoryConnection(id: ID!): ConnectionTestResult
  
  # Conflict resolution
  resolveConflict(conflictId: ID!, resolution: ConflictResolution!): Task
  
  # MCP compatibility
  mcpCreateTask(input: McpCreateTaskInput!): JsonValue
  mcpEditTask(input: McpEditTaskInput!): JsonValue
}
```

### 4.4 MCP Protocol Enhancement

**File**: `ratchet-mcp/src/handlers/tasks.rs`

```rust
// Enhanced MCP handlers for full task and repository management
pub async fn mcp_get_task_with_source(/* ... */) -> Result<JsonValue>;
pub async fn mcp_update_task_source(/* ... */) -> Result<JsonValue>;
pub async fn mcp_push_task_to_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_assign_task_repository(/* ... */) -> Result<JsonValue>;
```

**File**: `ratchet-mcp/src/handlers/repositories.rs`

```rust
// Repository management through MCP
pub async fn mcp_list_repositories(/* ... */) -> Result<JsonValue>;
pub async fn mcp_get_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_create_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_update_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_delete_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_sync_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_push_repository_changes(/* ... */) -> Result<JsonValue>;
pub async fn mcp_test_repository_connection(/* ... */) -> Result<JsonValue>;
pub async fn mcp_set_default_repository(/* ... */) -> Result<JsonValue>;
pub async fn mcp_list_conflicts(/* ... */) -> Result<JsonValue>;
pub async fn mcp_resolve_conflict(/* ... */) -> Result<JsonValue>;
```

## Phase 5: Background Sync & Monitoring

### 5.1 Sync Scheduler

**File**: `ratchet-server/src/scheduler/sync_scheduler.rs`

```rust
pub struct SyncScheduler {
    sync_service: Arc<TaskSyncService>,
    scheduler: JobScheduler,
}

impl SyncScheduler {
    pub async fn start(&self) -> Result<()>;
    pub async fn schedule_repository_sync(&self, repo_id: i32, interval: Duration) -> Result<()>;
    pub async fn trigger_immediate_sync(&self, repo_id: i32) -> Result<()>;
}
```

### 5.2 File System Watching

**File**: `ratchet-server/src/watchers/filesystem_watcher.rs`

```rust
// Uses notify crate for real-time file system monitoring
pub struct FilesystemWatcher {
    watcher: RecommendedWatcher,
    sync_service: Arc<TaskSyncService>,
}

impl FilesystemWatcher {
    pub async fn watch_repository(&self, repo_id: i32) -> Result<()>;
    pub async fn handle_file_change(&self, event: &Event) -> Result<()>;
}
```

### 5.3 Health Monitoring

**File**: `ratchet-server/src/monitoring/sync_health.rs`

```rust
pub struct SyncHealthMonitor {
    metrics: Arc<PrometheusMetrics>,
    alert_service: Arc<AlertService>,
}

// Metrics tracked:
// - Sync success/failure rates
// - Conflict resolution statistics  
// - Repository health status
// - Task modification frequencies
```

## Phase 6: Configuration & Security

### 6.1 Repository Configuration

**File**: `config/repositories.yaml` (example)

```yaml
repositories:
  - name: "local-tasks"
    type: "filesystem"
    uri: "/opt/ratchet/tasks"
    sync_enabled: true
    sync_interval_minutes: 5
    watch_patterns: ["**/*.js", "**/task.yaml"]
    ignore_patterns: ["**/node_modules/**", "**/.git/**"]
    priority: 1
    is_default: true
    is_writable: true
    push_on_change: false
    
  - name: "company-tasks"
    type: "git"
    uri: "https://github.com/company/ratchet-tasks.git"
    branch: "main"
    auth_config:
      type: "token"
      token_env: "GITHUB_TOKEN"
    sync_enabled: true
    sync_interval_minutes: 15
    priority: 2
    is_default: false
    is_writable: true
    push_on_change: true
    
  - name: "shared-registry"
    type: "http"
    uri: "https://registry.company.com/api/tasks"
    auth_config:
      type: "bearer"
      token_env: "REGISTRY_TOKEN"
    sync_enabled: true
    sync_interval_minutes: 30
    priority: 3
    is_default: false
    is_writable: false
    push_on_change: false
```

### 6.2 Security & Permissions

**File**: `ratchet-server/src/auth/task_permissions.rs`

```rust
pub enum TaskPermission {
    Read,
    Edit,
    Sync,
    Delete,
    Push,
    ManageRepository,
    CreateRepository,
    DeleteRepository,
}

pub struct TaskAuthService {
    // Role-based access control for task operations
    // Repository-specific permissions
    // API key management for sync operations
}
```

## Phase 7: Migration & Rollout

### 7.1 Data Migration Strategy

**File**: `ratchet-cli/src/commands/migrate_tasks.rs`

```rust
pub struct TaskMigrationCommand {
    pub dry_run: bool,
    pub backup_before: bool,
    pub source_paths: Vec<PathBuf>,
    pub repository_config: PathBuf,
}

// Migration process:
// 1. Scan existing file-based tasks
// 2. Create default filesystem repository configuration
// 3. Import tasks into database with full source code and repository assignment
// 4. Verify data integrity and repository assignments
// 5. Update API endpoints to use database with repository context
// 6. Maintain backward compatibility during transition
// 7. Set up automatic push-back for modified tasks if repository is writable
```

### 7.2 Rollback Plan

- Database migration rollback scripts
- API endpoint feature flags for gradual rollout
- File-based fallback mechanism
- Data export tools for emergency recovery

## Implementation Timeline

### Week 1-2: Database Schema & Migrations
- Implement enhanced task table schema with required repository_id
- Create repository and version tracking tables
- Write and test migration scripts with default repository setup
- Set up development database

### Week 3-4: Repository Management Backend
- Build repository service layer with CRUD operations
- Implement task assignment service for repository management
- Create repository abstraction layer with sync capabilities
- Add default repository detection and assignment logic

### Week 5-6: Repository Sync Engine  
- Implement filesystem, Git, and HTTP repository implementations
- Create bidirectional sync service with push/pull capabilities
- Add conflict resolution and merge strategies
- Background sync scheduling with push-on-change support

### Week 7-8: API Enhancement
- Update REST API endpoints for full task and repository access
- Enhance GraphQL schema and resolvers with repository operations
- Extend MCP protocol handlers for repository management
- Update OpenAPI documentation with new endpoints

### Week 9-10: Background Services & Monitoring
- Add file system watching for real-time sync
- Implement health monitoring and metrics
- Create push/pull automation workflows
- Security and permission controls

### Week 11-12: Migration & Production Readiness
- Data migration from file-based to database storage
- Repository assignment and sync validation
- Performance testing and optimization
- Documentation and deployment guides

## Success Metrics

1. **Functionality**: All tasks (metadata + code + schemas) stored in database with repository assignment
2. **Repository Management**: Full CRUD operations for repositories via all APIs
3. **Bidirectional Sync**: Tasks pulled from repositories and changes pushed back successfully
4. **Default Assignment**: New tasks automatically assigned to default repository
5. **Sync Performance**: Repository sync completes in <30 seconds for 1000 tasks
6. **API Coverage**: Full CRUD operations for tasks and repositories via REST, GraphQL, and MCP
7. **Conflict Resolution**: <5% manual intervention required for conflicts
8. **Push Success Rate**: >95% successful push rate for repository changes
9. **Uptime**: 99.9% sync service availability
10. **Developer Experience**: Single API call to get complete task definition with repository context

## Risks & Mitigation

### Technical Risks
- **Database size growth**: Implement task archiving and cleanup policies
- **Repository sync conflicts**: Robust conflict detection and resolution mechanisms  
- **Push/pull failures**: Retry mechanisms and error handling for repository operations
- **Performance impact**: Database indexing and caching strategies for repository queries
- **Data corruption**: Comprehensive backup and recovery procedures
- **Repository authentication**: Secure credential management for repository access

### Operational Risks
- **Migration complexity**: Phased rollout with repository assignment validation
- **API breaking changes**: Versioned APIs with repository-aware deprecation notices
- **Repository access issues**: Fallback to read-only mode when repositories are unavailable
- **Security vulnerabilities**: Security audits and repository-specific access controls
- **Dependencies**: Minimize external dependencies, fallback mechanisms for repository operations

## Dependencies

### Internal
- Database migration system (ratchet-storage)
- API type definitions (ratchet-api-types)
- Authentication system (ratchet-auth)
- Configuration management (ratchet-config)
- Repository management services
- Task assignment and sync services

### External
- **notify**: File system watching
- **git2**: Git repository operations  
- **reqwest**: HTTP repository access
- **sea-orm**: Database ORM enhancements
- **tokio-cron-scheduler**: Background sync scheduling

## Future Enhancements

1. **Repository Templates**: Reusable repository configurations for common setups
2. **Multi-Repository Sync**: Synchronize tasks across multiple repositories simultaneously
3. **Repository Mirroring**: Automatic mirroring between repository sources
4. **Collaborative Editing**: Real-time collaborative task editing with repository awareness
5. **Task Marketplace**: Public registry of shared tasks with repository metadata
6. **Visual Editor**: Web-based visual task editor with repository context
7. **CI/CD Integration**: Automated testing and deployment of tasks to repositories
8. **Repository Webhooks**: Real-time sync triggers from repository events
9. **Task Dependencies**: Task composition and dependency management across repositories
10. **Repository Analytics**: Metrics and insights for repository usage and health