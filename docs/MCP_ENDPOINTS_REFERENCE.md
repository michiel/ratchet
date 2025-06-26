# Ratchet MCP Endpoints Reference

**Version**: 1.1.0  
**Protocol**: Model Context Protocol (MCP) v2024-11-05  
**Transport**: HTTP JSON-RPC 2.0 / Server-Sent Events (SSE) / Stdio  
**Last Updated**: 2025-01-20

## Base Configuration

### HTTP Endpoint
```
POST http://localhost:8090/mcp
Content-Type: application/json
```

### JSON-RPC Request Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      // tool-specific parameters
    }
  }
}
```

## Core MCP Protocol Methods

### Initialize Connection
```json
{
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "clientInfo": {
      "name": "client-name",
      "version": "1.0.0"
    }
  }
}
```

### List Available Tools
```json
{
  "method": "tools/list"
}
```

**Response**: Array of available tools with descriptions and schemas.

## Task Execution Endpoints

### 1. Execute Task
**Tool**: `ratchet_execute_task`

Execute a task with optional progress streaming and detailed tracing.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_execute_task",
    "arguments": {
      "task_id": "string (required)",
      "input": "object (required)",
      "trace": "boolean (optional, default: true)",
      "timeout": "integer (optional, seconds)",
      "stream_progress": "boolean (optional, default: false)",
      "progress_filter": {
        "min_progress_delta": "number (0.0-1.0)",
        "max_frequency_ms": "integer",
        "step_filter": ["string"],
        "include_data": "boolean"
      }
    }
  }
}
```

**Response** (Synchronous):
```json
{
  "content": [
    {
      "type": "text",
      "text": "{ \"result\": \"task output\", \"execution_id\": \"uuid\" }"
    }
  ],
  "isError": false,
  "metadata": {
    "task_id": "string",
    "execution_id": "string",
    "streaming": false,
    "trace_enabled": true
  }
}
```

### 2. Batch Execute Tasks
**Tool**: `ratchet_batch_execute`

Execute multiple tasks with dependency management and parallel execution.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_batch_execute",
    "arguments": {
      "requests": [
        {
          "id": "string (required)",
          "task_id": "string (required)",
          "input": "object (required)",
          "dependencies": ["string"],
          "priority": "integer",
          "timeout_ms": "integer"
        }
      ],
      "execution_mode": "parallel|sequential|dependency|priority_dependency",
      "max_parallel": "integer",
      "stop_on_error": "boolean",
      "timeout_ms": "integer",
      "correlation_token": "string"
    }
  }
}
```

## Task Management Endpoints

### 3. List Available Tasks
**Tool**: `ratchet_list_available_tasks`

List all available tasks with filtering, pagination, and schema options.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_available_tasks",
    "arguments": {
      "filter": "string (name pattern)",
      "include_schemas": "boolean (default: false)",
      "category": "string",
      "page": "integer (default: 0, 0-based)",
      "limit": "integer (default: 50, max: 1000)",
      "sort_by": "name|created_at|updated_at|version",
      "sort_order": "asc|desc"
    }
  }
}
```

### 4. Create Task
**Tool**: `ratchet_create_task`

Create a new JavaScript task with code, schemas, and test cases.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task",
    "arguments": {
      "name": "string (required, unique)",
      "description": "string (required)",
      "code": "string (required, JavaScript code)",
      "input_schema": "object (required, JSON Schema)",
      "output_schema": "object (required, JSON Schema)",
      "version": "string (default: 0.1.0)",
      "enabled": "boolean (default: true)",
      "tags": ["string"],
      "metadata": "object",
      "test_cases": [
        {
          "name": "string (required)",
          "description": "string",
          "input": "object (required)",
          "expected_output": "object",
          "should_fail": "boolean"
        }
      ]
    }
  }
}
```

### 5. Edit Task
**Tool**: `ratchet_edit_task`

Edit existing task code, schemas, and metadata.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_edit_task",
    "arguments": {
      "task_id": "string (required)",
      "code": "string (JavaScript code)",
      "description": "string",
      "input_schema": "object (JSON Schema)",
      "output_schema": "object (JSON Schema)",
      "tags": ["string"],
      "validate_changes": "boolean (default: true)",
      "create_backup": "boolean (default: true)"
    }
  }
}
```

### 6. Validate Task
**Tool**: `ratchet_validate_task`

Validate task code, schemas, and run tests without execution.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_validate_task",
    "arguments": {
      "task_id": "string (required)",
      "code": "string (optional, for validation)",
      "input_schema": "object (optional)",
      "output_schema": "object (optional)",
      "run_tests": "boolean (default: true)",
      "syntax_only": "boolean (default: false)"
    }
  }
}
```

### 7. Delete Task
**Tool**: `ratchet_delete_task`

Delete an existing task with optional backup and file cleanup.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_delete_task",
    "arguments": {
      "task_id": "string (required)",
      "create_backup": "boolean (default: true)",
      "delete_files": "boolean (default: false)",
      "force": "boolean (default: false)"
    }
  }
}
```

## Monitoring & Analysis Endpoints

### 8. Get Execution Status
**Tool**: `ratchet_get_execution_status`

Get status and progress of a running execution.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_status",
    "arguments": {
      "execution_id": "string (required)"
    }
  }
}
```

### 9. Get Execution Logs
**Tool**: `ratchet_get_execution_logs`

Retrieve logs for a specific execution.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_logs",
    "arguments": {
      "execution_id": "string (required)",
      "level": "trace|debug|info|warn|error",
      "limit": "integer (default: 100)",
      "format": "json|text (default: json)"
    }
  }
}
```

### 10. Get Execution Trace
**Tool**: `ratchet_get_execution_trace`

Get detailed execution trace with timing and context.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_trace",
    "arguments": {
      "execution_id": "string (required)",
      "include_http_calls": "boolean (default: true)",
      "format": "json|flamegraph (default: json)"
    }
  }
}
```

### 11. Analyze Execution Error
**Tool**: `ratchet_analyze_execution_error`

Get detailed error analysis for failed execution with fix suggestions.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_analyze_execution_error",
    "arguments": {
      "execution_id": "string (required)",
      "include_suggestions": "boolean (default: true)",
      "include_context": "boolean (default: true)"
    }
  }
}
```

### 12. List Executions
**Tool**: `ratchet_list_executions`

List task executions with filtering and pagination.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_executions",
    "arguments": {
      "task_id": "string (optional filter)",
      "status": "pending|running|completed|failed|cancelled",
      "limit": "integer (default: 50, max: 1000)",
      "page": "integer (default: 0)",
      "sort_by": "queued_at|started_at|completed_at|duration_ms|status",
      "sort_order": "asc|desc",
      "include_output": "boolean (default: false)"
    }
  }
}
```

## Debugging & Testing Endpoints

### 13. Debug Task Execution
**Tool**: `ratchet_debug_task_execution`

Debug task execution with breakpoints and variable inspection.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_debug_task_execution",
    "arguments": {
      "task_id": "string (required)",
      "input": "object (required)",
      "breakpoints": ["integer (line numbers)"],
      "capture_variables": "boolean (default: true)",
      "step_mode": "boolean (default: false)",
      "timeout_ms": "integer (default: 300000)"
    }
  }
}
```

### 14. Run Task Tests
**Tool**: `ratchet_run_task_tests`

Execute test cases for a task and report results.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_run_task_tests",
    "arguments": {
      "task_id": "string (required)",
      "test_names": ["string (specific tests)"],
      "parallel": "boolean (default: false)",
      "stop_on_failure": "boolean (default: false)",
      "include_traces": "boolean (default: true)"
    }
  }
}
```

## Data Management Endpoints

### 15. Store Result
**Tool**: `ratchet_store_result`

Store task execution result in the database.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_store_result",
    "arguments": {
      "task_id": "string (required)",
      "input": "object (required)",
      "output": "object (required)",
      "status": "pending|running|completed|failed|cancelled",
      "duration_ms": "integer",
      "error_message": "string",
      "error_details": "object",
      "http_requests": "object",
      "recording_path": "string"
    }
  }
}
```

### 16. Get Results
**Tool**: `ratchet_get_results`

Retrieve task execution results from the database.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_results",
    "arguments": {
      "task_id": "string (optional filter)",
      "execution_id": "string (specific execution)",
      "status": "pending|running|completed|failed|cancelled",
      "limit": "integer (default: 50, max: 1000)",
      "offset": "integer (default: 0)",
      "include_data": "boolean (default: true)",
      "include_errors": "boolean (default: true)"
    }
  }
}
```

## Import/Export & Templates

### 17. Import Tasks
**Tool**: `ratchet_import_tasks`

Import tasks from JSON or other formats.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_import_tasks",
    "arguments": {
      "data": "object (required)",
      "format": "json|zip|directory",
      "overwrite_existing": "boolean (default: false)",
      "options": {
        "validate_tasks": "boolean (default: true)",
        "include_tests": "boolean (default: true)",
        "name_prefix": "string"
      }
    }
  }
}
```

### 18. Export Tasks
**Tool**: `ratchet_export_tasks`

Export tasks to JSON or other formats.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_export_tasks",
    "arguments": {
      "task_id": "string (optional, exports all if not provided)",
      "format": "json|zip|individual",
      "options": {
        "include_metadata": "boolean (default: true)",
        "include_tests": "boolean (default: true)",
        "include_versions": "boolean (default: false)"
      }
    }
  }
}
```

### 19. List Templates
**Tool**: `ratchet_list_templates`

List all available task templates.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_templates",
    "arguments": {}
  }
}
```

### 20. Generate from Template
**Tool**: `ratchet_generate_from_template`

Generate a new task from a predefined template.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_generate_from_template",
    "arguments": {
      "template": "string (required)",
      "name": "string (required)",
      "description": "string",
      "parameters": "object (template-specific)"
    }
  }
}
```

## Job & Schedule Management

### 21. List Jobs
**Tool**: `ratchet_list_jobs`

List jobs with filtering, sorting, and pagination.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_jobs",
    "arguments": {
      "task_id": "string (optional filter)",
      "status": "queued|processing|completed|failed|cancelled",
      "priority": "low|normal|high|urgent",
      "limit": "integer (default: 50, max: 1000)",
      "page": "integer (default: 0)",
      "sort_by": "queued_at|scheduled_for|priority|status|retry_count",
      "sort_order": "asc|desc"
    }
  }
}
```

### 22. List Schedules
**Tool**: `ratchet_list_schedules`

List schedules with filtering and pagination.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_schedules",
    "arguments": {
      "task_id": "string (optional filter)",
      "enabled": "boolean",
      "ready_to_run": "boolean",
      "limit": "integer (default: 50, max: 1000)",
      "page": "integer (default: 0)",
      "sort_by": "name|created_at|updated_at|next_run|last_run",
      "sort_order": "asc|desc"
    }
  }
}
```

## Version Management

### 23. Create Task Version
**Tool**: `ratchet_create_task_version`

Create a new version of an existing task.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task_version",
    "arguments": {
      "task_id": "string (required)",
      "new_version": "string (required, semver)",
      "description": "string (required, changelog)",
      "breaking_change": "boolean (default: false)",
      "make_active": "boolean (default: true)",
      "migration_script": "string (for breaking changes)"
    }
  }
}
```

## Discovery & Registry Management

### 24. Discover Tasks
**Tool**: `ratchet_discover_tasks`

Discover tasks in a filesystem directory.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_discover_tasks",
    "arguments": {
      "path": "string (required)",
      "recursive": "boolean (default: true)",
      "include_tests": "boolean (default: true)",
      "auto_import": "boolean (default: false)"
    }
  }
}
```

### 25. Sync Registry
**Tool**: `ratchet_sync_registry`

Sync registry sources to load available tasks.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_sync_registry",
    "arguments": {
      "source_name": "string (optional, specific source)",
      "force_refresh": "boolean (default: false)",
      "include_dependencies": "boolean (default: true)"
    }
  }
}
```

### 26. Registry Health
**Tool**: `ratchet_registry_health`

Check registry health and status.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_registry_health",
    "arguments": {}
  }
}
```

## Documentation Endpoints

### 27. Get Developer Endpoint Reference
**Tool**: `ratchet_get_developer_endpoint_reference`

Get comprehensive MCP endpoints reference with all available tools.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_developer_endpoint_reference",
    "arguments": {}
  }
}
```

### 28. Get Developer Integration Guide
**Tool**: `ratchet_get_developer_integration_guide`

Get comprehensive MCP integration guide for setting up Claude Desktop.

```json
{
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_developer_integration_guide",
    "arguments": {}
  }
}
```

## Error Handling

All endpoints return standard MCP error responses:

```json
{
  "content": [
    {
      "type": "text",
      "text": "Error message description"
    }
  ],
  "isError": true,
  "metadata": {
    "error_type": "validation_error|execution_error|not_found|etc",
    "tool_name": "tool_name"
  }
}
```

## Common Error Types

- `validation_error`: Invalid input parameters
- `execution_error`: Task execution failed
- `not_found`: Task/execution not found
- `configuration_error`: Server configuration issue
- `timeout_error`: Operation timed out
- `permission_error`: Insufficient permissions

## Authentication

The MCP server supports multiple authentication methods:
- **None**: No authentication (development)
- **API Key**: Header-based API key
- **JWT**: JSON Web Token authentication
- **OAuth2**: OAuth2 flow authentication

## Rate Limiting

Default rate limits per operation type:
- Task execution: 100/minute
- Task creation: 20/minute
- Monitoring operations: 1000/minute

## Transport Options

### HTTP JSON-RPC
- **Endpoint**: `POST /mcp`
- **Protocol**: JSON-RPC 2.0
- **Content-Type**: `application/json`

### Server-Sent Events (SSE)
- **Endpoint**: `GET /sse/{session-id}`
- **Protocol**: SSE with JSON-RPC messages
- **Use Case**: Real-time progress updates

### Stdio
- **Transport**: Direct process communication
- **Use Case**: Desktop AI applications like Claude Desktop

## Example Usage

### Complete Task Creation and Execution Flow

```javascript
// 1. Initialize connection
const initResponse = await fetch('/mcp', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2024-11-05',
      capabilities: { tools: {} },
      clientInfo: { name: 'my-client', version: '1.0.0' }
    }
  })
});

// 2. Create task
const createResponse = await fetch('/mcp', {
  method: 'POST', 
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    id: 2,
    method: 'tools/call',
    params: {
      name: 'ratchet_create_task',
      arguments: {
        name: 'httpbin_get_origin',
        description: 'Calls httpbin.org/get and returns the origin IP',
        code: `
async function main(input) {
  const response = await fetch('https://httpbin.org/get');
  const data = await response.json();
  return { origin: data.origin };
}`,
        input_schema: {
          type: 'object',
          properties: {},
          additionalProperties: false
        },
        output_schema: {
          type: 'object',
          properties: {
            origin: { type: 'string' }
          },
          required: ['origin']
        }
      }
    }
  })
});

// 3. Execute task
const execResponse = await fetch('/mcp', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    id: 3,
    method: 'tools/call',
    params: {
      name: 'ratchet_execute_task',
      arguments: {
        task_id: 'httpbin_get_origin',
        input: {},
        trace: true
      }
    }
  })
});
```

This comprehensive reference covers all 28 MCP tools available in the Ratchet implementation, providing complete parameter specifications, JSON schemas, and usage examples for each endpoint.