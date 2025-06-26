# Task Development Guide Using MCP Interface

**Version**: 1.0.0  
**Protocol**: Model Context Protocol (MCP) v2024-11-05  
**Target Audience**: AI Agents and Developers

This guide demonstrates how to develop tasks for Ratchet using the Model Context Protocol (MCP) interface. We'll walk through creating a complete task that fetches data from an HTTP API and extracts specific information.

## Prerequisites

Before starting, ensure you have:
- Ratchet server running with MCP enabled on `http://localhost:8090`
- HTTP access to the MCP endpoint
- Basic understanding of JSON-RPC 2.0 protocol

## Overview

We'll create a task called `get_origin_info` that:
1. Makes an HTTP GET request to `https://httpbin.org/get`
2. Extracts the `origin` field from the JSON response
3. Returns the origin IP address

## Step 1: Initialize MCP Connection

First, establish a connection to the Ratchet MCP server:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "clientInfo": {
      "name": "task-developer",
      "version": "1.0.0"
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {
        "listChanged": true
      }
    },
    "serverInfo": {
      "name": "ratchet-mcp-server",
      "version": "0.4.9"
    }
  }
}
```

## Step 2: List Available Tools

Verify that the task development tools are available:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

**Expected Response** (partial):
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "ratchet_create_task",
        "description": "Create a new JavaScript task with schemas and tests"
      },
      {
        "name": "ratchet_execute_task", 
        "description": "Execute a task with input data"
      },
      {
        "name": "ratchet_list_available_tasks",
        "description": "List all available tasks"
      }
    ]
  }
}
```

## Step 3: Create the Task

Now we'll create our `get_origin_info` task using the `ratchet_create_task` tool:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task",
    "arguments": {
      "name": "get_origin_info",
      "description": "Fetches origin IP address from httpbin.org/get endpoint",
      "code": "async function main(input) {\n  // Make HTTP GET request to httpbin.org\n  const response = await fetch('https://httpbin.org/get', {\n    method: 'GET',\n    headers: {\n      'accept': 'application/json'\n    }\n  });\n  \n  if (!response.ok) {\n    throw new Error(`HTTP request failed with status ${response.status}: ${response.statusText}`);\n  }\n  \n  const data = await response.json();\n  \n  // Extract the origin field\n  if (!data.origin) {\n    throw new Error('Origin field not found in response');\n  }\n  \n  return {\n    origin: data.origin,\n    success: true,\n    timestamp: new Date().toISOString()\n  };\n}",
      "input_schema": {
        "type": "object",
        "properties": {},
        "additionalProperties": false,
        "description": "No input parameters required"
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "origin": {
            "type": "string",
            "description": "The origin IP address from httpbin.org"
          },
          "success": {
            "type": "boolean",
            "description": "Indicates if the operation was successful"
          },
          "timestamp": {
            "type": "string",
            "format": "date-time",
            "description": "ISO timestamp of when the request was made"
          }
        },
        "required": ["origin", "success", "timestamp"],
        "additionalProperties": false
      },
      "version": "1.0.0",
      "enabled": true,
      "tags": ["http", "api", "example"],
      "metadata": {
        "author": "task-developer",
        "category": "networking",
        "documentation_url": "https://httpbin.org"
      },
      "test_cases": [
        {
          "name": "basic_execution",
          "description": "Test basic task execution with no input",
          "input": {},
          "expected_output": {
            "origin": "string",
            "success": true,
            "timestamp": "string"
          },
          "should_fail": false
        }
      ]
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9J\", \"database_id\": 1, \"message\": \"Task created successfully\"}"
      }
    ]
  }
}
```

## Step 4: Validate the Task

Before executing, let's validate that our task is syntactically correct and passes tests:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "ratchet_validate_task",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "run_tests": true,
      "syntax_only": false
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"syntax_valid\": true, \"schema_valid\": true, \"tests_passed\": 1, \"tests_failed\": 0, \"validation_errors\": [], \"test_results\": [{\"name\": \"basic_execution\", \"passed\": true, \"duration_ms\": 342}]}"
      }
    ]
  }
}
```

## Step 5: Execute the Task

Now let's execute our task to see it in action:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "ratchet_execute_task",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
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
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"origin\": \"203.0.113.42\", \"success\": true, \"timestamp\": \"2024-06-25T23:15:42.123Z\"}"
      }
    ],
    "isError": false,
    "metadata": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "streaming": false,
      "trace_enabled": true,
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K",
      "duration_ms": 456
    }
  }
}
```

## Step 6: Run Task Tests

Let's run the automated tests to ensure our task works correctly:

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "ratchet_run_task_tests",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "parallel": false,
      "stop_on_failure": false,
      "include_traces": true
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"tests_run\": 1, \"tests_passed\": 1, \"tests_failed\": 0, \"total_duration_ms\": 523, \"results\": [{\"name\": \"basic_execution\", \"status\": \"passed\", \"duration_ms\": 523, \"output\": {\"origin\": \"203.0.113.42\", \"success\": true, \"timestamp\": \"2024-06-25T23:16:15.456Z\"}}]}"
      }
    ]
  }
}
```

## Step 7: Get Execution Details

After executing a task, you can retrieve detailed logs, traces, and performance information to understand what happened during execution.

### Retrieve Execution Logs

Get structured logs from the task execution:

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_logs",
    "arguments": {
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K",
      "level": "info",
      "format": "json",
      "limit": 100
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"logs\": [{\"timestamp\": \"2024-06-25T23:15:42.100Z\", \"level\": \"info\", \"message\": \"Task execution started\", \"context\": {\"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9J\", \"execution_id\": \"exec_01HF6K8T5A2B3C4D5E6F7G8H9K\"}}, {\"timestamp\": \"2024-06-25T23:15:42.120Z\", \"level\": \"debug\", \"message\": \"Initializing JavaScript runtime\", \"context\": {\"memory_limit\": \"128MB\", \"timeout\": 30000}}, {\"timestamp\": \"2024-06-25T23:15:42.145Z\", \"level\": \"info\", \"message\": \"HTTP request initiated\", \"context\": {\"url\": \"https://httpbin.org/get\", \"method\": \"GET\"}}, {\"timestamp\": \"2024-06-25T23:15:42.523Z\", \"level\": \"info\", \"message\": \"HTTP request completed\", \"context\": {\"status_code\": 200, \"response_time_ms\": 378, \"content_length\": 425}}, {\"timestamp\": \"2024-06-25T23:15:42.556Z\", \"level\": \"info\", \"message\": \"Task execution completed successfully\", \"context\": {\"duration_ms\": 456, \"output_size_bytes\": 128}}], \"total_logs\": 5, \"execution_summary\": {\"status\": \"completed\", \"duration_ms\": 456, \"memory_used_mb\": 12.5, \"http_requests\": 1}}"
      }
    ]
  }
}
```

### Get Detailed Execution Trace

Retrieve a detailed execution trace with timing information and context:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_trace",
    "arguments": {
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K",
      "format": "json",
      "include_http_calls": true
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"trace\": {\"execution_id\": \"exec_01HF6K8T5A2B3C4D5E6F7G8H9K\", \"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9J\", \"start_time\": \"2024-06-25T23:15:42.100Z\", \"end_time\": \"2024-06-25T23:15:42.556Z\", \"duration_ms\": 456, \"status\": \"completed\", \"steps\": [{\"step\": \"initialization\", \"start_ms\": 0, \"duration_ms\": 20, \"details\": {\"js_engine\": \"V8\", \"memory_allocated_mb\": 8}}, {\"step\": \"input_validation\", \"start_ms\": 20, \"duration_ms\": 5, \"details\": {\"schema_validation\": \"passed\", \"input_size_bytes\": 2}}, {\"step\": \"main_execution\", \"start_ms\": 25, \"duration_ms\": 420, \"details\": {\"function_name\": \"main\", \"sub_steps\": [{\"operation\": \"fetch_request\", \"start_ms\": 45, \"duration_ms\": 378, \"details\": {\"url\": \"https://httpbin.org/get\", \"method\": \"GET\", \"status_code\": 200, \"response_size_bytes\": 425}}]}}, {\"step\": \"output_processing\", \"start_ms\": 445, \"duration_ms\": 8, \"details\": {\"serialization_time_ms\": 3, \"validation_time_ms\": 5}}, {\"step\": \"cleanup\", \"start_ms\": 453, \"duration_ms\": 3, \"details\": {\"memory_released_mb\": 8}}], \"http_calls\": [{\"sequence\": 1, \"url\": \"https://httpbin.org/get\", \"method\": \"GET\", \"request_headers\": {\"accept\": \"application/json\", \"user-agent\": \"ratchet-js-runtime/0.4.9\"}, \"response_status\": 200, \"response_headers\": {\"content-type\": \"application/json\", \"content-length\": \"425\"}, \"timing\": {\"dns_lookup_ms\": 15, \"tcp_connect_ms\": 45, \"tls_handshake_ms\": 87, \"request_sent_ms\": 2, \"response_received_ms\": 229}, \"response_preview\": \"{\\\"args\\\": {}, \\\"headers\\\": {\\\"Accept\\\": \\\"application/json\\\", \\\"Host\\\": \\\"httpbin.org\\\"}, \\\"origin\\\": \\\"203.0.113.42\\\", \\\"url\\\": \\\"https://httpbin.org/get\\\"}\"}], \"performance_metrics\": {\"cpu_time_ms\": 12, \"memory_peak_mb\": 12.5, \"gc_collections\": 0, \"gc_time_ms\": 0}}}"
      }
    ]
  }
}
```

### Get Trace in Flamegraph Format

For performance analysis, you can request the trace in flamegraph format:

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_trace",
    "arguments": {
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K",
      "format": "flamegraph",
      "include_http_calls": true
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"flamegraph_data\": \"main_execution;fetch_request 378\\nmain_execution;json_parsing 15\\nmain_execution;response_processing 12\\ninitialization;js_engine_startup 15\\ninitialization;memory_allocation 5\\noutput_processing;serialization 3\\noutput_processing;validation 5\\ncleanup;memory_release 3\", \"total_samples\": 456, \"sampling_rate_ms\": 1}"
      }
    ]
  }
}
```

## Step 8: List All Tasks

Verify your task appears in the available tasks list:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_available_tasks",
    "arguments": {
      "limit": 50,
      "filter": "get_origin",
      "include_schemas": true
    }
  }
}
```

## Task Development Best Practices

### 1. Error Handling
Always include proper error handling in your task code:

```javascript
async function main(input) {
  try {
    const response = await fetch('https://httpbin.org/get', {
      method: 'GET',
      headers: { 'accept': 'application/json' }
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const data = await response.json();
    
    if (!data.origin) {
      throw new Error('Missing origin field in response');
    }
    
    return { origin: data.origin, success: true };
  } catch (error) {
    throw new Error(`Failed to fetch origin: ${error.message}`);
  }
}
```

### 2. Input Validation
Define clear input schemas and validate inputs:

```json
{
  "input_schema": {
    "type": "object",
    "properties": {
      "endpoint": {
        "type": "string",
        "format": "uri",
        "default": "https://httpbin.org/get"
      }
    },
    "additionalProperties": false
  }
}
```

### 3. Comprehensive Testing
Include multiple test cases covering success and failure scenarios:

```json
{
  "test_cases": [
    {
      "name": "success_case",
      "description": "Normal execution should return origin IP",
      "input": {},
      "should_fail": false
    },
    {
      "name": "timeout_case", 
      "description": "Should handle network timeouts gracefully",
      "input": {"timeout": 1},
      "should_fail": true
    }
  ]
}
```

### 4. Documentation
Use clear descriptions and metadata:

```json
{
  "description": "Fetches origin IP address from httpbin.org/get endpoint",
  "metadata": {
    "author": "your-name",
    "category": "networking",
    "documentation_url": "https://example.com/docs",
    "version_notes": "Initial implementation"
  }
}
```

## Monitoring and Log Analysis

### List Recent Executions

Monitor all recent task executions across your system:

```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_executions",
    "arguments": {
      "limit": 20,
      "sort_by": "started_at",
      "sort_order": "desc",
      "include_output": false
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"executions\": [{\"execution_id\": \"exec_01HF6K8T5A2B3C4D5E6F7G8H9K\", \"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9J\", \"task_name\": \"get_origin_info\", \"status\": \"completed\", \"started_at\": \"2024-06-25T23:15:42.100Z\", \"completed_at\": \"2024-06-25T23:15:42.556Z\", \"duration_ms\": 456, \"priority\": \"normal\"}, {\"execution_id\": \"exec_01HF6K8T5A2B3C4D5E6F7G8H9L\", \"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9M\", \"task_name\": \"another_task\", \"status\": \"failed\", \"started_at\": \"2024-06-25T23:10:15.200Z\", \"completed_at\": \"2024-06-25T23:10:18.450Z\", \"duration_ms\": 3250, \"error_message\": \"Network timeout\"}], \"total\": 45, \"page\": 0, \"has_more\": true}"
      }
    ]
  }
}
```

### Filter Executions by Task

Monitor executions for a specific task:

```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "method": "tools/call",
  "params": {
    "name": "ratchet_list_executions",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "status": "completed",
      "limit": 10,
      "include_output": true
    }
  }
}
```

### Get Execution Status for Long-Running Tasks

Check the current status of a running execution:

```json
{
  "jsonrpc": "2.0",
  "id": 22,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_status",
    "arguments": {
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K"
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 22,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"execution_id\": \"exec_01HF6K8T5A2B3C4D5E6F7G8H9K\", \"task_id\": \"01HF6K8T5A2B3C4D5E6F7G8H9J\", \"status\": \"completed\", \"progress\": 1.0, \"current_step\": \"completed\", \"started_at\": \"2024-06-25T23:15:42.100Z\", \"completed_at\": \"2024-06-25T23:15:42.556Z\", \"duration_ms\": 456, \"output_available\": true, \"error_message\": null, \"performance\": {\"memory_peak_mb\": 12.5, \"cpu_time_ms\": 12, \"http_requests\": 1}}"
      }
    ]
  }
}
```

### Advanced Log Filtering

Get logs with specific filters and levels:

```json
{
  "jsonrpc": "2.0",
  "id": 23,
  "method": "tools/call",
  "params": {
    "name": "ratchet_get_execution_logs",
    "arguments": {
      "execution_id": "exec_01HF6K8T5A2B3C4D5E6F7G8H9K",
      "level": "debug",
      "format": "text",
      "limit": 50
    }
  }
}
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 23,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "2024-06-25T23:15:42.100Z [INFO] Task execution started for get_origin_info\n2024-06-25T23:15:42.120Z [DEBUG] Initializing JavaScript runtime with 128MB memory limit\n2024-06-25T23:15:42.125Z [DEBUG] Loading task code (1247 bytes)\n2024-06-25T23:15:42.130Z [DEBUG] Validating input schema\n2024-06-25T23:15:42.135Z [DEBUG] Input validation passed: {}\n2024-06-25T23:15:42.140Z [DEBUG] Starting main function execution\n2024-06-25T23:15:42.145Z [INFO] HTTP request initiated to https://httpbin.org/get\n2024-06-25T23:15:42.150Z [DEBUG] DNS lookup started for httpbin.org\n2024-06-25T23:15:42.165Z [DEBUG] DNS lookup completed (15ms)\n2024-06-25T23:15:42.170Z [DEBUG] TCP connection initiated\n2024-06-25T23:15:42.215Z [DEBUG] TCP connection established (45ms)\n2024-06-25T23:15:42.220Z [DEBUG] TLS handshake started\n2024-06-25T23:15:42.307Z [DEBUG] TLS handshake completed (87ms)\n2024-06-25T23:15:42.309Z [DEBUG] HTTP request sent (2ms)\n2024-06-25T23:15:42.523Z [INFO] HTTP request completed with status 200 (378ms total)\n2024-06-25T23:15:42.525Z [DEBUG] Response size: 425 bytes\n2024-06-25T23:15:42.530Z [DEBUG] Parsing JSON response\n2024-06-25T23:15:42.545Z [DEBUG] Extracting origin field: 203.0.113.42\n2024-06-25T23:15:42.550Z [DEBUG] Preparing output object\n2024-06-25T23:15:42.553Z [DEBUG] Validating output schema\n2024-06-25T23:15:42.556Z [INFO] Task execution completed successfully (456ms total)"
      }
    ]
  }
}
```

### Understanding Trace Data

The execution trace provides detailed timing and performance information:

- **steps**: Breakdown of execution phases with timing
- **http_calls**: Complete HTTP request/response details with timing breakdown
- **performance_metrics**: Memory usage, CPU time, garbage collection stats

Use this data to:
- Identify performance bottlenecks
- Optimize slow HTTP requests
- Monitor memory usage patterns
- Debug timing-sensitive operations

### Trace Analysis Examples

**Finding Slow Operations:**
Look for steps with high `duration_ms` values in the trace data.

**HTTP Performance Analysis:**
Check the `timing` breakdown in `http_calls` to identify network vs. processing delays:
- `dns_lookup_ms`: DNS resolution time
- `tcp_connect_ms`: TCP connection establishment
- `tls_handshake_ms`: SSL/TLS negotiation
- `response_received_ms`: Data transfer time

**Memory Monitoring:**
Monitor `memory_peak_mb` and `gc_collections` to identify memory-intensive tasks.

## Debugging Failed Tasks

If a task fails during execution, use the debug and analysis tools:

### Get Error Details
```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "tools/call",
  "params": {
    "name": "ratchet_analyze_execution_error",
    "arguments": {
      "execution_id": "failed-execution-id",
      "include_context": true,
      "include_suggestions": true
    }
  }
}
```

### Debug Execution
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "tools/call",
  "params": {
    "name": "ratchet_debug_task_execution",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "input": {},
      "capture_variables": true,
      "step_mode": false
    }
  }
}
```

## Task Modification and Versioning

### Editing Tasks
To modify an existing task:

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "tools/call",
  "params": {
    "name": "ratchet_edit_task",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "code": "// Updated code here",
      "description": "Updated description",
      "validate_changes": true,
      "create_backup": true
    }
  }
}
```

### Creating Versions
For major changes, create a new version:

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "tools/call",
  "params": {
    "name": "ratchet_create_task_version",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "new_version": "2.0.0",
      "description": "Added support for custom endpoints",
      "breaking_change": true,
      "make_active": true
    }
  }
}
```

## Advanced Features

### Batch Execution
Execute multiple tasks with dependencies:

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "tools/call",
  "params": {
    "name": "ratchet_batch_execute",
    "arguments": {
      "requests": [
        {
          "id": "req1",
          "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
          "input": {}
        }
      ],
      "execution_mode": "parallel",
      "max_parallel": 5,
      "stop_on_error": false
    }
  }
}
```

### Progress Streaming
For long-running tasks, enable progress streaming:

```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "method": "tools/call",
  "params": {
    "name": "ratchet_execute_task",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "input": {},
      "stream_progress": true,
      "progress_filter": {
        "min_progress_delta": 0.1,
        "max_frequency_ms": 1000
      }
    }
  }
}
```

## Task Export and Import

### Export Tasks
```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "method": "tools/call",
  "params": {
    "name": "ratchet_export_tasks",
    "arguments": {
      "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
      "format": "json",
      "options": {
        "include_metadata": true,
        "include_tests": true
      }
    }
  }
}
```

### Import Tasks
```json
{
  "jsonrpc": "2.0", 
  "id": 16,
  "method": "tools/call",
  "params": {
    "name": "ratchet_import_tasks",
    "arguments": {
      "data": {
        "tasks": [/* exported task data */]
      },
      "format": "json",
      "overwrite_existing": false,
      "options": {
        "validate_tasks": true,
        "include_tests": true
      }
    }
  }
}
```

## Complete HTTP Request Example

Here's a complete curl command to execute our task:

```bash
curl -X POST http://localhost:8090/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "ratchet_execute_task",
      "arguments": {
        "task_id": "01HF6K8T5A2B3C4D5E6F7G8H9J",
        "input": {},
        "trace": true
      }
    }
  }'
```

## Summary

This guide covered the complete workflow for developing tasks using the Ratchet MCP interface:

1. **Initialize** the MCP connection
2. **Create** a task with proper schemas and tests
3. **Validate** the task syntax and functionality
4. **Execute** the task and verify results
5. **Debug** and troubleshoot issues
6. **Version** and maintain tasks over time

The example `get_origin_info` task demonstrates:
- Making HTTP requests with proper error handling
- Extracting specific data from JSON responses
- Returning structured output with validation
- Including comprehensive test cases

All steps in this guide use the documented MCP endpoints and should work with a properly configured Ratchet server running on `http://localhost:8090`.