# Ratchet MCP Development Guide

A comprehensive walkthrough for agents on using the Ratchet MCP interface to create, execute, monitor, and manage tasks.

**Version**: 1.0.0  
**Protocol**: Model Context Protocol (MCP) v2024-11-05  
**Last Updated**: 2025-01-20

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Core Workflow: Create and Execute a Task](#core-workflow-create-and-execute-a-task)
4. [Step-by-Step Walkthrough](#step-by-step-walkthrough)
5. [Monitoring and Debugging](#monitoring-and-debugging)
6. [Administrative Operations](#administrative-operations)
7. [Advanced Features](#advanced-features)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

## Overview

This guide provides agents with a complete walkthrough of using Ratchet's MCP interface to:

- **Create Tasks**: Build JavaScript-based tasks with proper schemas
- **Execute Tasks**: Run tasks with tracing and progress monitoring
- **Monitor Executions**: Track status, logs, and performance traces
- **Debug Issues**: Analyze errors and get fix suggestions
- **Manage Tasks**: Version, edit, import/export, and organize tasks
- **Administrative Actions**: Sync registries, check health, manage resources

All examples use JSON-RPC 2.0 format compatible with MCP clients like Claude Desktop.

## Getting Started

### Prerequisites

1. **Ratchet MCP Server**: Running and accessible
2. **MCP Client**: Claude Desktop or similar MCP-compatible client
3. **Connection**: Established MCP connection with tools list

### Initial Setup Check

First, verify your connection and available tools:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list"
}
```

You should see 28+ tools including `ratchet_execute_task`, `ratchet_create_task`, etc.

## Core Workflow: Create and Execute a Task

Let's walk through creating a task that calls `https://httpbin.org/get` and returns the "origin" key from the response.

### Step 1: Create the Task

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task",
    "arguments": {
      "name": "httpbin_get_origin",
      "description": "Calls httpbin.org/get and returns the origin IP address",
      "code": "async function main(input) {\n  const response = await fetch('https://httpbin.org/get');\n  const data = await response.json();\n  return { origin: data.origin };\n}",
      "input_schema": {
        "type": "object",
        "properties": {},
        "additionalProperties": false
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "origin": {
            "type": "string",
            "description": "The origin IP address from httpbin"
          }
        },
        "required": ["origin"]
      },
      "version": "1.0.0",
      "enabled": true,
      "test_cases": [
        {
          "name": "basic_execution",
          "description": "Test basic httpbin.org/get call",
          "input": {},
          "expected_output": {
            "origin": "string"
          }
        }
      ]
    }
  }
}
```

**Expected Response:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"task_id\":\"httpbin_get_origin\",\"database_id\":42,\"message\":\"Task created successfully\",\"version\":\"1.0.0\"}"
    }
  ],
  "isError": false,
  "metadata": {
    "task_name": "httpbin_get_origin",
    "created_at": "2025-01-20T10:30:00Z"
  }
}
```

### Step 2: Validate the Task

Before execution, validate the task to ensure it's properly formed:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "ratchet_validate_task",
    "arguments": {
      "task_id": "httpbin_get_origin",
      "run_tests": true
    }
  }
}
```

### Step 3: Execute the Task

Now execute the task with tracing enabled:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "ratchet_execute_task",
    "arguments": {
      "task_id": "httpbin_get_origin",
      "input": {},
      "trace": true,
      "timeout": 30
    }
  }
}
```

**Expected Response:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"result\":{\"origin\":\"203.0.113.42\"},\"execution_id\":\"exec_abc123\",\"status\":\"completed\",\"duration_ms\":1234}"
    }
  ],
  "isError": false,
  "metadata": {
    "task_id": "httpbin_get_origin",
    "execution_id": "exec_abc123",
    "trace_enabled": true
  }
}
```

## Step-by-Step Walkthrough

### 1. Task Discovery and Listing

Start by exploring existing tasks:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_available_tasks",
    "arguments": {
      "include_schemas": true,
      "limit": 10,
      "sort_by": "name"
    }
  }
}
```

### 2. Task Creation with Comprehensive Schema

Create a more complex task with validation and error handling:

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task",
    "arguments": {
      "name": "enhanced_httpbin_task",
      "description": "Enhanced HTTP client with error handling and timeout",
      "code": "async function main(input) {\n  const { timeout = 5000, endpoint = '/get' } = input;\n  \n  try {\n    const controller = new AbortController();\n    const timeoutId = setTimeout(() => controller.abort(), timeout);\n    \n    const response = await fetch(`https://httpbin.org${endpoint}`, {\n      signal: controller.signal\n    });\n    \n    clearTimeout(timeoutId);\n    \n    if (!response.ok) {\n      throw new Error(`HTTP ${response.status}: ${response.statusText}`);\n    }\n    \n    const data = await response.json();\n    \n    return {\n      success: true,\n      origin: data.origin,\n      headers: data.headers,\n      timestamp: new Date().toISOString()\n    };\n  } catch (error) {\n    return {\n      success: false,\n      error: error.message,\n      timestamp: new Date().toISOString()\n    };\n  }\n}",
      "input_schema": {
        "type": "object",
        "properties": {
          "timeout": {
            "type": "integer",
            "default": 5000,
            "minimum": 1000,
            "maximum": 30000,
            "description": "Request timeout in milliseconds"
          },
          "endpoint": {
            "type": "string",
            "default": "/get",
            "enum": ["/get", "/post", "/put", "/delete"],
            "description": "HTTPBin endpoint to call"
          }
        },
        "additionalProperties": false
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "success": {
            "type": "boolean",
            "description": "Whether the request succeeded"
          },
          "origin": {
            "type": "string",
            "description": "Origin IP address (on success)"
          },
          "headers": {
            "type": "object",
            "description": "Request headers (on success)"
          },
          "error": {
            "type": "string",
            "description": "Error message (on failure)"
          },
          "timestamp": {
            "type": "string",
            "format": "date-time",
            "description": "Execution timestamp"
          }
        },
        "required": ["success", "timestamp"]
      },
      "test_cases": [
        {
          "name": "default_get_request",
          "description": "Test default GET request",
          "input": {},
          "expected_output": {
            "success": true,
            "origin": "string",
            "timestamp": "string"
          }
        },
        {
          "name": "custom_timeout",
          "description": "Test with custom timeout",
          "input": {
            "timeout": 10000
          },
          "expected_output": {
            "success": true
          }
        }
      ]
    }
  }
}
```

### 3. Task Execution with Progress Monitoring

Execute with progress streaming enabled:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "ratchet_execute_task",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "input": {
        "timeout": 10000,
        "endpoint": "/get"
      },
      "trace": true,
      "stream_progress": true,
      "progress_filter": {
        "min_progress_delta": 0.1,
        "max_frequency_ms": 1000,
        "include_data": true
      }
    }
  }
}
```

### 4. Batch Execution

Execute multiple tasks in parallel:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "ratchet_batch_execute",
    "arguments": {
      "requests": [
        {
          "id": "req1",
          "task_id": "httpbin_get_origin",
          "input": {}
        },
        {
          "id": "req2",
          "task_id": "enhanced_httpbin_task",
          "input": {
            "endpoint": "/get",
            "timeout": 5000
          }
        }
      ],
      "execution_mode": "parallel",
      "max_parallel": 2,
      "stop_on_error": false,
      "correlation_token": "batch_demo_001"
    }
  }
}
```

## Monitoring and Debugging

### 1. Check Execution Status

Monitor a running execution:

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_status",
    "arguments": {
      "execution_id": "exec_abc123"
    }
  }
}
```

### 2. Retrieve Execution Logs

Get detailed logs for debugging:

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_logs",
    "arguments": {
      "execution_id": "exec_abc123",
      "level": "debug",
      "limit": 100,
      "format": "json"
    }
  }
}
```

### 3. Get Execution Trace

Analyze performance and timing:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_trace",
    "arguments": {
      "execution_id": "exec_abc123",
      "include_http_calls": true,
      "format": "json"
    }
  }
}
```

### 4. Error Analysis

For failed executions, get detailed error analysis:

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "ratchet_analyze_execution_error",
    "arguments": {
      "execution_id": "exec_failed_456",
      "include_suggestions": true,
      "include_context": true
    }
  }
}
```

### 5. Debug Task Execution

Use debugging tools for step-by-step analysis:

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "tools/call",
  "params": {
    "name": "ratchet_debug_task_execution",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "input": {
        "timeout": 1000,
        "endpoint": "/delay/10"
      },
      "breakpoints": [15, 25],
      "capture_variables": true,
      "timeout_ms": 60000
    }
  }
}
```

## Administrative Operations

### 1. Registry Management

Sync registry sources to load new tasks:

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "tools/call",
  "params": {
    "name": "ratchet_sync_registry",
    "arguments": {
      "force_refresh": true,
      "include_dependencies": true
    }
  }
}
```

Check registry health:

```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "method": "tools/call",
  "params": {
    "name": "ratchet_registry_health",
    "arguments": {}
  }
}
```

### 2. Task Discovery

Discover tasks in filesystem directories:

```json
{
  "jsonrpc": "2.0",
  "id": 16,
  "method": "tools/call",
  "params": {
    "name": "ratchet_discover_tasks",
    "arguments": {
      "path": "/path/to/task/directory",
      "recursive": true,
      "include_tests": true,
      "auto_import": false
    }
  }
}
```

### 3. Task Import/Export

Export tasks for backup or sharing:

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "method": "tools/call",
  "params": {
    "name": "ratchet_export_tasks",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "format": "json",
      "options": {
        "include_metadata": true,
        "include_tests": true,
        "include_versions": false
      }
    }
  }
}
```

Import tasks from another system:

```json
{
  "jsonrpc": "2.0",
  "id": 18,
  "method": "tools/call",
  "params": {
    "name": "ratchet_import_tasks",
    "arguments": {
      "data": {
        "tasks": [
          {
            "name": "imported_task",
            "description": "Task imported from another system",
            "code": "async function main(input) { return { status: 'imported' }; }"
          }
        ]
      },
      "format": "json",
      "overwrite_existing": false,
      "options": {
        "validate_tasks": true,
        "name_prefix": "imported_"
      }
    }
  }
}
```

### 4. List Operations

List executions for monitoring:

```json
{
  "jsonrpc": "2.0",
  "id": 19,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_executions",
    "arguments": {
      "status": "completed",
      "limit": 20,
      "sort_by": "completed_at",
      "sort_order": "desc",
      "include_output": false
    }
  }
}
```

List jobs in the queue:

```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_jobs",
    "arguments": {
      "status": "queued",
      "limit": 50,
      "sort_by": "priority",
      "sort_order": "desc"
    }
  }
}
```

List scheduled tasks:

```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_schedules",
    "arguments": {
      "enabled": true,
      "ready_to_run": true,
      "limit": 25
    }
  }
}
```

## Advanced Features

### 1. Task Templates

Use templates for rapid task creation:

```json
{
  "jsonrpc": "2.0",
  "id": 22,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_templates",
    "arguments": {}
  }
}
```

Generate task from template:

```json
{
  "jsonrpc": "2.0",
  "id": 23,
  "method": "tools/call",
  "params": {
    "name": "ratchet_generate_from_template",
    "arguments": {
      "template": "http_client",
      "name": "github_api_client",
      "description": "GitHub API client task",
      "parameters": {
        "base_url": "https://api.github.com",
        "auth_required": true
      }
    }
  }
}
```

### 2. Task Versioning

Create new versions of tasks:

```json
{
  "jsonrpc": "2.0",
  "id": 24,
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task_version",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "new_version": "1.1.0",
      "description": "Added retry logic and better error handling",
      "breaking_change": false,
      "make_active": true
    }
  }
}
```

### 3. Task Testing

Run comprehensive tests:

```json
{
  "jsonrpc": "2.0",
  "id": 25,
  "method": "tools/call",
  "params": {
    "name": "ratchet_run_task_tests",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "test_names": ["default_get_request", "custom_timeout"],
      "parallel": false,
      "stop_on_failure": false,
      "include_traces": true
    }
  }
}
```

### 4. Result Storage and Retrieval

Store execution results for later analysis:

```json
{
  "jsonrpc": "2.0",
  "id": 26,
  "method": "tools/call",
  "params": {
    "name": "ratchet_store_result",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "input": {"timeout": 5000},
      "output": {"success": true, "origin": "203.0.113.42"},
      "status": "completed",
      "duration_ms": 1234
    }
  }
}
```

Retrieve stored results:

```json
{
  "jsonrpc": "2.0",
  "id": 27,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_results",
    "arguments": {
      "task_id": "enhanced_httpbin_task",
      "limit": 50,
      "include_data": true,
      "include_errors": true
    }
  }
}
```

## Best Practices

### 1. Task Design

- **Use descriptive names**: Choose clear, meaningful task names
- **Comprehensive schemas**: Define complete input/output schemas with descriptions
- **Error handling**: Always include proper error handling in task code
- **Timeout management**: Set appropriate timeouts for external calls
- **Test cases**: Include comprehensive test cases for validation

### 2. Execution Patterns

- **Enable tracing**: Use `trace: true` for debugging and monitoring
- **Batch operations**: Use batch execution for related tasks
- **Progress monitoring**: Enable progress streaming for long-running tasks
- **Resource management**: Set appropriate timeouts and limits

### 3. Monitoring Strategy

- **Regular health checks**: Monitor registry and system health
- **Log analysis**: Review execution logs for patterns and issues
- **Performance tracking**: Use traces to identify bottlenecks
- **Error analysis**: Leverage error analysis tools for debugging

### 4. Administrative Tasks

- **Regular syncing**: Keep registry sources synchronized
- **Backup tasks**: Export important tasks regularly
- **Version management**: Use semantic versioning for task updates
- **Resource cleanup**: Monitor and clean up old executions/results

## Troubleshooting

### Common Issues and Solutions

#### 1. Task Creation Failures

**Problem**: Task creation fails with validation errors

**Solution**: 
```json
{
  "jsonrpc": "2.0",
  "id": 28,
  "method": "tools/call",
  "params": {
    "name": "ratchet_validate_task",
    "arguments": {
      "task_id": "problematic_task",
      "syntax_only": true
    }
  }
}
```

#### 2. Execution Timeouts

**Problem**: Tasks timeout during execution

**Solution**: Check execution logs and adjust timeout:
```json
{
  "jsonrpc": "2.0",
  "id": 29,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_logs",
    "arguments": {
      "execution_id": "exec_timeout_123",
      "level": "warn"
    }
  }
}
```

#### 3. Registry Issues

**Problem**: Tasks not appearing in listings

**Solution**: Sync registry and check health:
```json
{
  "jsonrpc": "2.0",
  "id": 30,
  "method": "tools/call",
  "params": {
    "name": "ratchet_sync_registry",
    "arguments": {
      "force_refresh": true
    }
  }
}
```

#### 4. Performance Issues

**Problem**: Slow execution performance

**Solution**: Analyze execution traces:
```json
{
  "jsonrpc": "2.0",
  "id": 31,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_trace",
    "arguments": {
      "execution_id": "exec_slow_456",
      "format": "flamegraph"
    }
  }
}
```

### Error Recovery Patterns

1. **Retry Logic**: Implement retry mechanisms in task code
2. **Graceful Degradation**: Handle partial failures appropriately
3. **Circuit Breakers**: Prevent cascade failures in dependent tasks
4. **Monitoring Alerts**: Set up monitoring for critical task failures

## Complete Example: HTTPBin Origin Task

Here's the complete implementation of our example task:

```javascript
async function main(input) {
  const response = await fetch('https://httpbin.org/get');
  const data = await response.json();
  return { origin: data.origin };
}
```

This task demonstrates:
- ✅ Simple, focused functionality
- ✅ Proper async/await usage
- ✅ Clean return structure
- ✅ External API integration
- ✅ Error-free execution

The task successfully calls HTTPBin's GET endpoint and extracts the origin IP address, providing a foundation for more complex HTTP client tasks.

---

This development guide provides agents with comprehensive patterns and examples for effectively using Ratchet's MCP interface for task development, execution, monitoring, and administration.