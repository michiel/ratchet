# Ratchet REST API Documentation

## Overview

The Ratchet REST API provides a unified, Refine.dev-compatible interface for managing tasks and executions. This API complements the existing GraphQL API and is designed specifically for integration with React admin panels and other REST-based tools.

## API Design Principles

Ratchet implements a **unified API layer** that standardizes types, pagination, and error handling across both REST and GraphQL endpoints. This eliminates inconsistencies and provides a coherent API experience.

### Unified Type System

#### ID Representation
All APIs use consistent string-based IDs:
- REST and GraphQL always return string IDs
- UUIDs are formatted as strings (e.g., `"550e8400-e29b-41d4-a716-446655440000"`)
- Numeric IDs are converted to strings (e.g., `"123"`)

```json
{
  "id": "123",
  "taskId": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Field Naming Conventions
All API responses use **camelCase** field names consistently:

| Field | Legacy Format | Unified Format |
|-------|---------------|----------------|
| created_at | `created_at` | `createdAt` |
| task_id | `task_id` | `taskId` |
| error_message | `error_message` | `errorMessage` |

### Unified Error Handling

All APIs return errors in a consistent format:

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

#### Standard Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `NOT_FOUND` | 404 | Resource not found |
| `BAD_REQUEST` | 400 | Invalid request |
| `VALIDATION_ERROR` | 400 | Input validation failed |
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Access denied |
| `CONFLICT` | 409 | Resource conflict |
| `RATE_LIMITED` | 429 | Too many requests |
| `TIMEOUT` | 408 | Operation timeout |
| `INTERNAL_ERROR` | 500 | Server error |

## OpenAPI Specification

The complete API specification is available in `openapi.yaml`. This specification follows OpenAPI 3.0 standards and includes:

- All endpoint definitions
- Request/response schemas
- Authentication requirements
- Error response formats
- Examples for all operations

### Viewing the Documentation

#### Option 1: Using the HTML Viewer
Open `openapi-viewer.html` in a web browser to view the interactive Swagger UI documentation.

```bash
# If you have a local web server:
python3 -m http.server 8000
# Then open http://localhost:8000/docs/openapi-viewer.html

# Or simply open the file directly:
open docs/openapi-viewer.html  # macOS
xdg-open docs/openapi-viewer.html  # Linux
```

#### Option 2: Using Swagger Editor
1. Go to [editor.swagger.io](https://editor.swagger.io)
2. Copy the contents of `openapi.yaml`
3. Paste into the editor

#### Option 3: Using ReDoc
```bash
# Install ReDoc CLI
npm install -g @redocly/cli

# Generate static documentation
redocly build-docs docs/openapi.yaml -o docs/api-docs.html
```

## API Base URL

The REST API is served at `/api/v1` relative to your Ratchet server URL.

Example:
- Development: `http://localhost:8080/api/v1`
- Production: `https://your-ratchet-server.com/api/v1`

## Key Features

### Refine.dev Compatibility

The API is designed to work seamlessly with [Refine.dev](https://refine.dev)'s Simple REST data provider:

- Standard resource endpoints (`/api/v1/{resource}`)
- Pagination with `_start` and `_end` parameters (legacy) or `page` and `limit` (unified)
- Sorting with `_sort` and `_order` parameters
- Field-based filtering
- Response format: `{ data: T | T[] }`
- Pagination headers: `x-total-count` and `content-range`

### Unified Pagination

Both legacy and modern pagination formats are supported:

```http
# Modern unified format (preferred)
GET /api/v1/tasks?page=1&limit=25

# Legacy Refine.dev format (still supported)
GET /api/v1/tasks?_start=0&_end=25
```

Response format:
```json
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

## API Resources

### Tasks
- **GET /api/v1/tasks** - List all tasks with pagination, filtering, and sorting
- **GET /api/v1/tasks/{id}** - Get a specific task by ID
- **PATCH /api/v1/tasks/{id}** - Update a task (limited to enable/disable)
- **POST /api/v1/tasks** - Create a task (not supported - returns 405)
- **DELETE /api/v1/tasks/{id}** - Delete a task (not supported - returns 405)

### Executions
- **GET /api/v1/executions** - List all executions with pagination, filtering, and sorting
- **GET /api/v1/executions/{id}** - Get a specific execution by ID
- **POST /api/v1/executions** - Create a new execution
- **PATCH /api/v1/executions/{id}** - Update an execution (status, output, error)
- **DELETE /api/v1/executions/{id}** - Delete an execution (not allowed for running executions)
- **POST /api/v1/executions/{id}/retry** - Retry a failed execution
- **POST /api/v1/executions/{id}/cancel** - Cancel a pending or running execution

### Jobs
- **GET /api/v1/jobs** - List all jobs with pagination, filtering, and sorting
- **GET /api/v1/jobs/{id}** - Get a specific job by ID
- **POST /api/v1/jobs** - Create a new job with optional output destinations
- **PATCH /api/v1/jobs/{id}** - Update a job
- **DELETE /api/v1/jobs/{id}** - Delete a job (not allowed for running jobs)
- **POST /api/v1/jobs/test-output-destinations** - Test output destination configurations

## Filtering and Sorting

### Filtering
Resources support field-based filtering:
- Exact match: `?field=value`
- Like search: `?field_like=partial`
- Date ranges: `?date_gte=2024-01-01&date_lte=2024-12-31`
- In array: `?status_in=pending,running`

### Sorting
- Sort field: `_sort=field_name`
- Sort order: `_order=ASC` or `_order=DESC`

## Integration Examples

### Using with Refine.dev

```typescript
import { Refine } from "@refinedev/core";
import dataProvider from "@refinedev/simple-rest";

const App = () => {
  return (
    <Refine
      dataProvider={dataProvider("http://localhost:8080/api/v1")}
      resources={[
        {
          name: "tasks",
          list: "/tasks",
          show: "/tasks/show/:id",
        },
        {
          name: "executions",
          list: "/executions",
          show: "/executions/show/:id",
          create: "/executions/create",
          edit: "/executions/edit/:id",
        },
      ]}
    />
  );
};
```

### Output Destinations

Jobs support optional output destinations for delivering task results to various endpoints:

- **Filesystem** - Save outputs to local files with configurable formats
- **Webhook** - Send outputs to HTTP endpoints with authentication and retry policies
- **Database** - Store outputs in database tables (coming soon)
- **S3** - Upload outputs to AWS S3 buckets (coming soon)

See the [Output Destinations Guide](./OUTPUT_DESTINATIONS.md) for detailed configuration and usage examples.

Output destinations are also fully supported in the GraphQL API with equivalent functionality.

## Migration Guide

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

// After (unified, both work)
const pagination = { page: 1, limit: 25 };  // Modern format
const legacyPagination = { _start: 0, _end: 25 };  // Still supported
```

## Future Enhancements

The following resources are planned for future implementation:

- **Schedules** - Scheduled task execution
- **Workers** - Worker process management
- **Database Destinations** - Direct database output storage
- **S3 Destinations** - AWS S3 output storage

## Development

To update the OpenAPI specification:

1. Edit `openapi.yaml` with your changes
2. Validate the spec:
   ```bash
   npx @apidevtools/swagger-cli validate docs/openapi.yaml
   ```
3. Test with the HTML viewer or Swagger Editor

## Support

For issues or questions about the REST API:
- Check the OpenAPI specification for detailed endpoint documentation
- Review the examples in the spec
- Refer to the Refine.dev documentation for client-side integration
- See the unified API types in `ratchet-lib/src/api/` for implementation details