# API Consistency Guide

This document describes the unified API design implemented across Ratchet's REST and GraphQL APIs to ensure consistent client integration and developer experience.

## Overview

Ratchet now provides a unified API layer (`ratchet-lib/src/api/`) that standardizes types, pagination, and error handling across both REST and GraphQL endpoints. This eliminates previous inconsistencies and provides a coherent API experience.

## Unified Type System

### ID Representation

**Problem Solved**: Previously, REST APIs used string IDs while GraphQL used typed IDs (i32, Uuid), causing client integration complexity.

**Solution**: All APIs now use `ApiId` type that:
- Accepts both string and numeric input
- Always returns string representation in responses
- Maintains type safety internally
- Works seamlessly across both API types

```rust
// Unified ID type
pub struct ApiId(pub String);

// Usage in both REST and GraphQL
{
  "id": "123",           // Always string in JSON
  "taskId": "550e8400-..."  // UUIDs as strings
}
```

### Field Naming Conventions

**Standardized to camelCase** across all API responses:

| Field | REST (Before) | GraphQL (Before) | Unified |
|-------|---------------|------------------|---------|
| created_at | createdAt | created_at | createdAt |
| task_id | task_id | task_id | taskId |
| registry_source | registrySource | registry_source | registrySource |
| error_message | error_message | error_message | errorMessage |

### Unified Data Types

All major entities now use unified types that work in both APIs:

```rust
// Unified Task type
pub struct UnifiedTask {
    pub id: ApiId,                    // Consistent ID type
    pub uuid: Uuid,
    pub name: String,                 // Standardized field names
    pub description: Option<String>,
    pub version: String,
    pub enabled: bool,
    pub registrySource: bool,         // camelCase
    pub availableVersions: Vec<String>,
    pub createdAt: DateTime<Utc>,     // camelCase
    pub updatedAt: DateTime<Utc>,
    pub validatedAt: Option<DateTime<Utc>>,
    pub inSync: bool,
    // ... additional fields
}
```

## Unified Pagination System

### Consistent Parameters

Both APIs now support the same pagination approach:

```graphql
# GraphQL
query {
  tasks(pagination: {
    page: 1,
    limit: 25,
    offset: 0  # Alternative to page-based
  }) {
    items { id, name }
    meta {
      page, limit, total, totalPages
      hasNext, hasPrevious, offset
    }
  }
}
```

```http
# REST API
GET /api/tasks?page=1&limit=25
# Or legacy Refine.dev format:
GET /api/tasks?_start=0&_end=25

Response:
{
  "data": {
    "items": [...],
    "meta": {
      "page": 1,
      "limit": 25,
      "total": 150,
      "totalPages": 6,
      "hasNext": true,
      "hasPrevious": false,
      "offset": 0
    }
  }
}
```

### Backward Compatibility

REST API maintains Refine.dev compatibility:
- `_start` and `_end` parameters still work
- Converted internally to unified pagination
- Response headers still include X-Total-Count, etc.

## Unified Error Handling

### Consistent Error Structure

All APIs now return errors in the same format:

```json
{
  "code": "NOT_FOUND",
  "message": "Task with ID '123' not found",
  "requestId": "req_abc123",
  "timestamp": "2024-01-15T10:30:00Z",
  "path": "/api/tasks/123",
  "suggestions": [
    "Verify that the task ID is correct",
    "Check if the task still exists"
  ]
}
```

### Error Codes

Standardized error codes across both APIs:

| Code | HTTP Status | GraphQL | Description |
|------|-------------|---------|-------------|
| `NOT_FOUND` | 404 | Error | Resource not found |
| `BAD_REQUEST` | 400 | Error | Invalid request |
| `VALIDATION_ERROR` | 400 | Error | Input validation failed |
| `UNAUTHORIZED` | 401 | Error | Authentication required |
| `FORBIDDEN` | 403 | Error | Access denied |
| `CONFLICT` | 409 | Error | Resource conflict |
| `RATE_LIMITED` | 429 | Error | Too many requests |
| `TIMEOUT` | 408 | Error | Operation timeout |
| `INTERNAL_ERROR` | 500 | Error | Server error |

### GraphQL Error Extensions

GraphQL errors include additional context:

```json
{
  "errors": [{
    "message": "Task with ID '123' not found",
    "extensions": {
      "code": "NOT_FOUND",
      "requestId": "req_abc123",
      "suggestions": ["Verify that the task ID is correct"],
      "timestamp": "2024-01-15T10:30:00Z"
    }
  }]
}
```

## API Migration Guide

### For Client Developers

#### ID Handling
```javascript
// Before (inconsistent)
const taskId = task.id;  // Could be number or string
const graphqlQuery = `task(id: ${taskId})`;  // Type issues

// After (consistent)
const taskId = task.id;  // Always string
const graphqlQuery = `task(id: "${taskId}")`;  // Always works
```

#### Field Access
```javascript
// Before (inconsistent)
const createdAt = task.created_at || task.createdAt;

// After (consistent)
const createdAt = task.createdAt;  // Always camelCase
```

#### Pagination
```javascript
// Before (different for each API)
const restPagination = { _start: 0, _end: 25 };
const graphqlPagination = { page: 1, limit: 25 };

// After (unified)
const pagination = { page: 1, limit: 25 };  // Works for both
```

### For Backend Developers

#### Using Unified Types
```rust
// Import unified types
use crate::api::types::*;
use crate::api::pagination::*;
use crate::api::errors::*;

// REST handler
async fn get_tasks(pagination: PaginationInput) -> ApiResult<ListResponse<UnifiedTask>> {
    let tasks = fetch_tasks().await?;
    Ok(ListResponse::new(tasks, &pagination, total))
}

// GraphQL resolver
async fn tasks(&self, pagination: Option<PaginationInput>) -> Result<ListResponse<UnifiedTask>> {
    let pagination = pagination.unwrap_or_default();
    let tasks = fetch_tasks().await
        .map_err(|e| Error::from(ApiError::internal_error(e.to_string())))?;
    Ok(ListResponse::new(tasks, &pagination, total))
}
```

## Benefits

### For API Consumers
1. **Consistent Integration**: Same types and patterns across APIs
2. **Predictable Responses**: Standardized field names and structures
3. **Better Error Handling**: Detailed, actionable error information
4. **Simplified Pagination**: Unified pagination across all endpoints

### For Developers
1. **Reduced Duplication**: Shared types between REST and GraphQL
2. **Type Safety**: Compile-time guarantees for API consistency
3. **Easier Maintenance**: Single source of truth for API types
4. **Better Testing**: Consistent error scenarios across APIs

## Implementation Details

### Code Structure
```
ratchet-lib/src/api/
├── types.rs          # Unified data types
├── pagination.rs     # Pagination utilities
├── errors.rs         # Error handling
└── conversions.rs    # Type conversions
```

### Backward Compatibility

The implementation maintains backward compatibility:
- Legacy REST response formats still work
- Refine.dev query parameters supported
- Existing GraphQL queries unchanged
- Gradual migration path for clients

### Performance Considerations

- Zero-copy conversions where possible
- Lazy field serialization
- Efficient pagination calculations
- Minimal allocation overhead

## Future Enhancements

1. **OpenAPI Schema Generation**: Generate unified OpenAPI specs
2. **Client SDK Generation**: Auto-generate SDKs from unified types
3. **Validation**: Unified input validation across APIs
4. **Caching**: Consistent caching strategies
5. **Rate Limiting**: Unified rate limiting across endpoints

## Examples

### Complete CRUD Operations

#### REST API
```http
# Create
POST /api/tasks
{
  "name": "my-task",
  "description": "A sample task",
  "version": "1.0.0"
}

# Read
GET /api/tasks/123

# Update
PUT /api/tasks/123
{
  "enabled": false
}

# Delete
DELETE /api/tasks/123

# List with pagination
GET /api/tasks?page=1&limit=10
```

#### GraphQL API
```graphql
# Create
mutation {
  createTask(input: {
    name: "my-task"
    description: "A sample task"
    version: "1.0.0"
  }) {
    id, name, version
  }
}

# Read
query {
  task(id: "123") {
    id, name, description, enabled
  }
}

# Update
mutation {
  updateTask(input: {
    id: "123"
    enabled: false
  }) {
    id, enabled
  }
}

# List with pagination
query {
  tasks(pagination: { page: 1, limit: 10 }) {
    items { id, name, version }
    meta { total, hasNext }
  }
}
```

Both APIs now provide identical functionality with consistent data structures and error handling.