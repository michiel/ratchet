# Refine.dev REST API Implementation Plan

## Overview

This plan outlines the implementation of a REST API for Ratchet that is fully compatible with Refine.dev's Simple REST data provider. The API will provide standard CRUD operations for all Ratchet resources while maintaining compatibility with the existing GraphQL API.

## Refine.dev Compatibility Requirements

### Data Provider Interface
Refine.dev's Simple REST data provider expects specific endpoint patterns and response formats:

- **Base URL Pattern**: `{apiUrl}/{resource}/{id?}`
- **Standard HTTP Methods**: GET, POST, PATCH, DELETE
- **Response Format**: `{ data: T | T[] }` with optional metadata
- **Error Handling**: HttpError with statusCode and message
- **Pagination**: Uses query parameters with `x-total-count` header response

### Expected Endpoint Mapping
```
getList   → GET    /api/v1/{resource}?_start=0&_end=25&_sort=id&_order=ASC
getOne    → GET    /api/v1/{resource}/{id}
create    → POST   /api/v1/{resource}
update    → PATCH  /api/v1/{resource}/{id}
deleteOne → DELETE /api/v1/{resource}/{id}
```

## Resource Mapping

### Core Resources
Map Ratchet entities to REST resources:

1. **Tasks** (`/api/v1/tasks`)
   - Unified tasks from registry and database
   - Supports filtering, sorting, pagination
   - Includes version management

2. **Executions** (`/api/v1/executions`)
   - Task execution history
   - Real-time status updates
   - Result data and error information

3. **Jobs** (`/api/v1/jobs`)
   - Queued and scheduled jobs
   - Priority and retry management
   - Job status tracking

4. **Schedules** (`/api/v1/schedules`)
   - Cron-based task scheduling
   - Schedule management and history

5. **Workers** (`/api/v1/workers`)
   - Worker process status
   - Health monitoring
   - Performance metrics

## API Design

### Base Configuration
```
Base URL: http://localhost:8080/api/v1
Content-Type: application/json
Error Format: HttpError compatible
```

### Response Format Standards

#### Success Response
```json
{
  "data": {
    "id": "123",
    "attribute": "value"
  }
}
```

#### List Response
```json
{
  "data": [
    {"id": "1", "attribute": "value1"},
    {"id": "2", "attribute": "value2"}
  ]
}
```

#### Error Response
```json
{
  "message": "Resource not found",
  "statusCode": 404,
  "errors": ["Detailed error information"]
}
```

### Pagination & Filtering

#### Query Parameters
```
?_start=0&_end=25           # Pagination
&_sort=field&_order=ASC     # Sorting
&field=value                # Filtering
&field_like=partial         # Fuzzy search
&field_gte=10               # Range filtering
```

#### Response Headers
```
x-total-count: 150          # Total number of records
content-range: tasks 0-24/150
```

## Detailed Endpoint Specifications

### 1. Tasks Resource (`/api/v1/tasks`)

#### GET /api/v1/tasks
```
Query Parameters:
- _start, _end: Pagination
- _sort, _order: Sorting (id, label, version, created_at, updated_at)
- uuid, label, version: Filtering
- label_like: Search tasks by label
- registry_source: Filter by source (true/false)
- enabled: Filter by enabled status

Response:
{
  "data": [
    {
      "id": "uuid-string",
      "uuid": "uuid-string", 
      "version": "1.0.0",
      "label": "Task Name",
      "description": "Task description",
      "enabled": true,
      "registrySource": true,
      "availableVersions": ["1.0.0", "1.1.0"],
      "createdAt": "2024-01-01T00:00:00Z",
      "updatedAt": "2024-01-01T00:00:00Z",
      "inSync": true
    }
  ]
}
```

#### GET /api/v1/tasks/{id}
```
Response:
{
  "data": {
    "id": "uuid-string",
    "uuid": "uuid-string",
    "version": "1.0.0", 
    "label": "Task Name",
    "description": "Task description",
    "inputSchema": {...},
    "outputSchema": {...},
    "enabled": true,
    "registrySource": true,
    "availableVersions": ["1.0.0"],
    "createdAt": "2024-01-01T00:00:00Z",
    "updatedAt": "2024-01-01T00:00:00Z",
    "validatedAt": "2024-01-01T00:00:00Z",
    "inSync": true
  }
}
```

#### POST /api/v1/tasks (Registry tasks are read-only)
```
Status: 405 Method Not Allowed
{
  "message": "Tasks are managed through the registry system",
  "statusCode": 405
}
```

#### PATCH /api/v1/tasks/{id}
```
Body: { "enabled": true }
Response: { "data": { updated task } }
```

#### DELETE /api/v1/tasks/{id} (Registry tasks are read-only)
```
Status: 405 Method Not Allowed
```

### 2. Executions Resource (`/api/v1/executions`)

#### GET /api/v1/executions
```
Query Parameters:
- task_id: Filter by task
- status: Filter by status (pending, running, completed, failed)
- _start, _end: Pagination
- _sort, _order: Sorting (id, started_at, completed_at, status)

Response:
{
  "data": [
    {
      "id": 123,
      "uuid": "execution-uuid",
      "taskId": 456,
      "taskUuid": "task-uuid",
      "taskLabel": "Task Name",
      "status": "completed",
      "startedAt": "2024-01-01T00:00:00Z",
      "completedAt": "2024-01-01T00:00:01Z",
      "executionTimeMs": 1000,
      "inputData": {...},
      "outputData": {...},
      "errorMessage": null
    }
  ]
}
```

#### POST /api/v1/executions (Execute Task)
```
Body:
{
  "taskId": 456,
  "inputData": {"key": "value"}
}

Response:
{
  "data": {
    "id": 123,
    "uuid": "execution-uuid",
    "status": "pending",
    "taskId": 456,
    "inputData": {"key": "value"},
    "startedAt": "2024-01-01T00:00:00Z"
  }
}
```

### 3. Jobs Resource (`/api/v1/jobs`)

#### GET /api/v1/jobs
```
Query Parameters:
- status: pending, running, completed, failed
- priority: high, medium, low
- task_id: Filter by task

Response:
{
  "data": [
    {
      "id": 789,
      "uuid": "job-uuid",
      "taskId": 456,
      "priority": "medium",
      "status": "pending",
      "retryCount": 0,
      "maxRetries": 3,
      "scheduledFor": "2024-01-01T00:00:00Z",
      "createdAt": "2024-01-01T00:00:00Z",
      "metadata": {...}
    }
  ]
}
```

#### POST /api/v1/jobs (Create Job)
```
Body:
{
  "taskId": 456,
  "priority": "high",
  "scheduledFor": "2024-01-01T12:00:00Z",
  "inputData": {...},
  "metadata": {...}
}
```

### 4. Schedules Resource (`/api/v1/schedules`)

#### GET /api/v1/schedules
```
Response:
{
  "data": [
    {
      "id": 321,
      "uuid": "schedule-uuid",
      "taskId": 456,
      "cronExpression": "0 0 * * *",
      "isActive": true,
      "lastRun": "2024-01-01T00:00:00Z",
      "nextRun": "2024-01-02T00:00:00Z",
      "createdAt": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### POST /api/v1/schedules
```
Body:
{
  "taskId": 456,
  "cronExpression": "0 0 * * *",
  "isActive": true
}
```

### 5. Workers Resource (`/api/v1/workers`)

#### GET /api/v1/workers
```
Response:
{
  "data": [
    {
      "id": "worker-1",
      "status": "ready",
      "currentTask": null,
      "totalTasks": 150,
      "startedAt": "2024-01-01T00:00:00Z",
      "lastHeartbeat": "2024-01-01T00:01:00Z",
      "memoryUsage": 1024000,
      "cpuUsage": 15.5
    }
  ]
}
```

## Implementation Architecture

### Module Structure
```
ratchet-lib/src/
├── rest/
│   ├── mod.rs              # Module exports
│   ├── app.rs              # REST API router setup
│   ├── handlers/           # REST endpoint handlers
│   │   ├── mod.rs
│   │   ├── tasks.rs        # Task CRUD operations
│   │   ├── executions.rs   # Execution management
│   │   ├── jobs.rs         # Job queue operations
│   │   ├── schedules.rs    # Schedule management
│   │   └── workers.rs      # Worker status
│   ├── models/             # REST API models
│   │   ├── mod.rs
│   │   ├── tasks.rs        # Task DTOs
│   │   ├── executions.rs   # Execution DTOs
│   │   ├── jobs.rs         # Job DTOs
│   │   ├── schedules.rs    # Schedule DTOs
│   │   ├── workers.rs      # Worker DTOs
│   │   └── common.rs       # Shared types (pagination, errors)
│   ├── middleware/         # REST-specific middleware
│   │   ├── mod.rs
│   │   ├── cors.rs         # CORS handling
│   │   ├── pagination.rs   # Pagination middleware
│   │   └── error_handler.rs # Error conversion
│   └── extractors/         # Request extractors
│       ├── mod.rs
│       ├── pagination.rs   # Pagination params
│       ├── sorting.rs      # Sort params
│       └── filtering.rs    # Filter params
```

### Integration Points

#### Server Integration
```rust
// In server/app.rs
pub fn create_app(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    task_executor: Arc<ProcessTaskExecutor>,
    registry: Option<Arc<TaskRegistry>>,
    sync_service: Option<Arc<TaskSyncService>>,
) -> Router {
    let graphql_app = create_graphql_app(/* ... */);
    let rest_app = rest::create_rest_app(/* ... */);
    
    Router::new()
        .merge(graphql_app)
        .nest("/api/v1", rest_app)
        .layer(/* shared middleware */)
}
```

#### Shared Infrastructure
- **Repository Layer**: Reuse existing database repositories
- **Service Layer**: Leverage TaskSyncService for unified task views
- **Execution Layer**: Use existing ProcessTaskExecutor
- **Error Handling**: Convert to Refine-compatible HttpError format

## Implementation Phases

### Phase 1: Core Infrastructure (2-3 days)
- [ ] Create REST module structure
- [ ] Implement base models and DTOs
- [ ] Set up REST router and middleware
- [ ] Add pagination, sorting, filtering extractors
- [ ] Implement error handling middleware

### Phase 2: Tasks Resource (2-3 days)
- [ ] Implement Tasks endpoints (GET list, GET one, PATCH)
- [ ] Add filtering by uuid, label, version, registry_source
- [ ] Implement sorting by label, version, created_at
- [ ] Add search functionality (label_like)
- [ ] Handle version management

### Phase 3: Executions Resource (2-3 days)
- [ ] Implement Executions CRUD endpoints
- [ ] Add execution creation (task execution)
- [ ] Implement filtering by task_id, status
- [ ] Add real-time execution status
- [ ] Handle execution results and errors

### Phase 4: Jobs Resource (1-2 days)
- [ ] Implement Jobs CRUD endpoints
- [ ] Add job creation and scheduling
- [ ] Implement filtering by status, priority, task_id
- [ ] Handle job retry logic
- [ ] Add job cancellation

### Phase 5: Schedules Resource (1-2 days)
- [ ] Implement Schedules CRUD endpoints
- [ ] Add cron expression validation
- [ ] Implement schedule activation/deactivation
- [ ] Handle schedule execution history

### Phase 6: Workers Resource (1 day)
- [ ] Implement Workers read-only endpoints
- [ ] Add worker status monitoring
- [ ] Implement worker metrics
- [ ] Add worker health checks

### Phase 7: Testing & Documentation (2-3 days)
- [ ] Unit tests for all endpoints
- [ ] Integration tests with Refine.dev
- [ ] API documentation (OpenAPI/Swagger)
- [ ] Performance testing
- [ ] Error handling validation

### Phase 8: Advanced Features (Optional, 2-3 days)
- [ ] Real-time updates via Server-Sent Events
- [ ] API versioning support
- [ ] Rate limiting
- [ ] Authentication/Authorization
- [ ] API metrics and monitoring

## Testing Strategy

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    
    #[tokio::test]
    async fn test_get_tasks_list() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .get("/api/v1/tasks?_start=0&_end=10")
            .await;
            
        assert_eq!(response.status_code(), 200);
        // Verify response structure
    }
}
```

### Integration Testing
- Test with actual Refine.dev Simple REST data provider
- Verify pagination, sorting, filtering work correctly
- Test error handling and edge cases
- Performance testing under load

### Refine.dev Compatibility Testing
```javascript
// Test configuration
import dataProvider from "@refinedev/simple-rest";

const provider = dataProvider("http://localhost:8080/api/v1");

// Test all CRUD operations
await provider.getList({ resource: "tasks" });
await provider.getOne({ resource: "tasks", id: "uuid" });
await provider.update({ resource: "tasks", id: "uuid", variables: {...} });
```

## Configuration

### REST API Configuration
```yaml
# config.yaml
server:
  rest_api:
    enabled: true
    base_path: "/api/v1"
    cors:
      enabled: true
      origins: ["http://localhost:3000"]
    pagination:
      default_limit: 25
      max_limit: 100
    rate_limiting:
      enabled: false
      requests_per_minute: 1000
```

### Feature Flags
```rust
pub struct RestApiConfig {
    pub enabled: bool,
    pub base_path: String,
    pub cors: CorsConfig,
    pub pagination: PaginationConfig,
    pub rate_limiting: RateLimitingConfig,
}
```

## Documentation

### OpenAPI/Swagger Specification
- Generate OpenAPI 3.0 specification
- Include all endpoints, models, and examples
- Provide interactive documentation
- Export Postman collection

### Integration Guide
- Refine.dev setup instructions
- Configuration examples
- Troubleshooting guide
- Migration from GraphQL

## Backward Compatibility

### GraphQL Preservation
- REST API is additive, doesn't replace GraphQL
- Both APIs share same business logic
- No breaking changes to existing GraphQL API
- Clients can use either API or both

### Configuration Migration
- REST API disabled by default
- Opt-in configuration
- Existing server configurations unchanged

## Success Criteria

### Functional Requirements
- [ ] All CRUD operations work with Refine.dev Simple REST data provider
- [ ] Pagination, sorting, filtering function correctly
- [ ] Error handling provides meaningful feedback
- [ ] Performance matches GraphQL API benchmarks

### Non-Functional Requirements
- [ ] Response times < 100ms for simple queries
- [ ] Supports 1000+ concurrent requests
- [ ] Comprehensive test coverage (>90%)
- [ ] Complete API documentation
- [ ] Zero breaking changes to existing APIs

### Refine.dev Integration
- [ ] Successfully integrates with Refine.dev admin panels
- [ ] Supports all Refine.dev data provider methods
- [ ] Handles complex filtering and sorting scenarios
- [ ] Provides real-time updates where applicable

## Maintenance Considerations

### Code Organization
- Clear separation between REST and GraphQL
- Shared business logic in service layer
- Consistent error handling patterns
- Comprehensive logging and monitoring

### Future Extensibility
- Easy to add new resources
- Versioning strategy for API evolution
- Plugin architecture for custom endpoints
- Performance optimization opportunities

---

This implementation plan provides a comprehensive REST API that is fully compatible with Refine.dev while leveraging Ratchet's existing infrastructure and maintaining backward compatibility with the GraphQL API.