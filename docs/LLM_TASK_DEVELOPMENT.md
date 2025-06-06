# LLM Task Development Guide for Ratchet via MCP

This guide provides comprehensive instructions for Large Language Models (LLMs) to develop, test, and debug tasks for the Ratchet system using the Model Context Protocol (MCP) interface.

## Table of Contents

1. [MCP Integration Overview](#mcp-integration-overview)
2. [Available MCP Tools](#available-mcp-tools)
3. [Development Workflow](#development-workflow)
4. [Task Structure](#task-structure)
5. [Using MCP Tools for Development](#using-mcp-tools-for-development)
6. [Testing and Debugging](#testing-and-debugging)
7. [Best Practices](#best-practices)
8. [Troubleshooting](#troubleshooting)
9. [Examples](#examples)

## MCP Integration Overview

Ratchet provides a complete MCP (Model Context Protocol) server that allows LLMs to interact with the task execution system. This enables:

- **Direct task execution** via MCP tools
- **Real-time monitoring** of task execution
- **Comprehensive debugging** capabilities
- **Task generation and validation** through the CLI
- **Error analysis** with AI-powered suggestions

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│      LLM        │◄──►│   MCP Server    │◄──►│ Ratchet Engine  │
│   (You/Client)  │    │  (ratchet-mcp)  │    │   (ratchet)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       ▼                       ▼
         │              ┌─────────────────┐    ┌─────────────────┐
         │              │  Task Registry  │    │    Database     │
         │              │  (filesystem)   │    │ (SQLite/Postgres)│
         │              └─────────────────┘    └─────────────────┘
         │
         ▼
┌─────────────────┐
│  Ratchet CLI    │ # Available via direct execution
│   (generate,    │ # Not through MCP - use for scaffolding
│   validate,     │
│   test)         │
└─────────────────┘
```

## Available MCP Tools

The Ratchet MCP server provides these tools for task development and execution:

### Core Execution Tools

1. **`ratchet.execute_task`** - Execute a task with input data
2. **`ratchet.list_available_tasks`** - List all tasks in the registry
3. **`ratchet.get_task_info`** - Get detailed information about a specific task

### Monitoring & Debugging Tools

4. **`ratchet.get_execution_status`** - Get status of a task execution
5. **`ratchet.get_execution_logs`** - Get logs from a task execution
6. **`ratchet.get_execution_trace`** - Get detailed execution trace for debugging
7. **`ratchet.analyze_execution_error`** - AI-powered error analysis with suggestions
8. **`ratchet.list_executions`** - List recent task executions

### Management Tools

9. **`ratchet.cancel_execution`** - Cancel a running task execution
10. **`ratchet.validate_task_input`** - Validate input data against task schema

## Development Workflow

### 1. Initial Setup

When starting task development:

1. **Generate task scaffold** using the CLI:
   ```bash
   ratchet generate task my-new-task --label "My Task" --description "What this task does"
   ```

2. **List available tasks** to understand the current registry:
   ```json
   {
     "tool": "ratchet.list_available_tasks"
   }
   ```

3. **Examine existing tasks** for patterns and examples:
   ```json
   {
     "tool": "ratchet.get_task_info",
     "arguments": {
       "task_id_or_path": "sample/js-tasks/addition"
     }
   }
   ```

### 2. Development Cycle

Follow this iterative development process:

```
┌─────────────────┐
│  1. Design      │ ──► Define input/output schemas
│     Task        │     and core logic requirements
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  2. Implement   │ ──► Write main.js with proper
│     Core Logic  │     function wrapper and error handling  
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  3. Validate    │ ──► Test input validation and
│     Input       │     basic execution
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  4. Execute &   │ ──► Run test cases and debug
│     Debug       │     any issues with MCP tools
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  5. Monitor &   │ ──► Use monitoring tools to
│     Optimize    │     analyze performance and logs
└─────────────────┘
```

### 3. Testing Strategy

Use MCP tools to thoroughly test your tasks:

1. **Validate input schemas**:
   ```json
   {
     "tool": "ratchet.validate_task_input",
     "arguments": {
       "task_id_or_path": "sample/js-tasks/my-task",
       "input": {
         "param1": "test value",
         "param2": 42
       }
     }
   }
   ```

2. **Execute with test data**:
   ```json
   {
     "tool": "ratchet.execute_task",
     "arguments": {
       "task_id_or_path": "sample/js-tasks/my-task",
       "input": {
         "param1": "production value",
         "param2": 100
       }
     }
   }
   ```

3. **Monitor execution**:
   ```json
   {
     "tool": "ratchet.get_execution_status",
     "arguments": {
       "execution_id": "uuid-from-execute-response"
     }
   }
   ```

4. **Debug issues**:
   ```json
   {
     "tool": "ratchet.get_execution_logs",
     "arguments": {
       "execution_id": "uuid-from-execute-response",
       "level": "debug",
       "limit": 100
     }
   }
   ```

## Task Structure

Each task is a directory containing these required files:

```
task-name/
├── metadata.json      # Task metadata and configuration
├── input.schema.json  # JSON Schema for input validation
├── output.schema.json # JSON Schema for output validation
├── main.js           # Main task implementation
└── tests/            # Test cases directory
    ├── test-001.json # Test case 1
    ├── test-002.json # Test case 2
    └── ...
```

### metadata.json Structure

```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "label": "Human-readable task name",
  "description": "Detailed description of what this task does",
  "author": "Your Name",
  "tags": ["category1", "category2"],
  "timeout": 30000,
  "memory_limit": "128MB"
}
```

### main.js Implementation

The main.js file must contain a single function:

```javascript
(function(input, context) {
    // Your task logic here
    // - input: validated against input.schema.json
    // - context: execution context with metadata (optional)
    // Returns: object validated against output.schema.json
    
    try {
        // Extract input parameters
        const { param1, param2 } = input;
        
        // Access execution context if provided
        if (context) {
            console.log("Execution ID:", context.executionId);
            console.log("Task ID:", context.taskId);
        }
        
        // Perform your task logic
        const result = processData(param1, param2);
        
        // Return structured output
        return {
            result: result,
            timestamp: new Date().toISOString(),
            executedBy: context ? context.taskId : "unknown"
        };
    } catch (error) {
        // Handle errors appropriately
        throw new Error(`Task failed: ${error.message}`);
    }
})
```

**Important Notes:**
- Function must be wrapped: `(function(input, context) { ... })`
- No async/await support - all operations must be synchronous
- Use built-in `fetch` function for HTTP requests
- No external modules or Node.js APIs available

### Available APIs

#### fetch(url, options, body)

```javascript
// GET request
const response = fetch("https://api.example.com/data");
if (response.ok) {
    const data = response.body;
    // Process data
}

// POST request with body
const response = fetch(
    "https://api.example.com/create",
    { method: "POST", headers: { "Content-Type": "application/json" } },
    { name: "example", value: 42 }
);
```

#### Error Types

```javascript
// Network-related errors
throw new NetworkError("Failed to connect to API");

// Data validation errors  
throw new DataError("Invalid response format");

// General task errors
throw new Error("Something went wrong");
```

## Using MCP Tools for Development

### Step-by-Step Task Development

#### 1. Start with Task Information

First, understand existing tasks and patterns:

```json
{
  "tool": "ratchet.list_available_tasks",
  "arguments": {}
}
```

Examine a similar task:

```json
{
  "tool": "ratchet.get_task_info",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/weather-api"
  }
}
```

#### 2. Create and Test Input Validation

Before implementing logic, validate your input schema:

```json
{
  "tool": "ratchet.validate_task_input",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/my-new-task",
    "input": {
      "testParam": "testValue"
    }
  }
}
```

#### 3. Implement and Execute

Execute your task with test data:

```json
{
  "tool": "ratchet.execute_task",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/my-new-task",
    "input": {
      "apiKey": "test-key",
      "endpoint": "https://api.example.com/data",
      "params": {
        "limit": 10
      }
    }
  }
}
```

#### 4. Monitor Execution Progress

Track the execution status:

```json
{
  "tool": "ratchet.get_execution_status",
  "arguments": {
    "execution_id": "execution-uuid-from-step-3"
  }
}
```

#### 5. Debug Issues

If there are problems, get detailed logs:

```json
{
  "tool": "ratchet.get_execution_logs",
  "arguments": {
    "execution_id": "execution-uuid-from-step-3",
    "level": "debug",
    "limit": 50
  }
}
```

Get a detailed trace:

```json
{
  "tool": "ratchet.get_execution_trace",
  "arguments": {
    "execution_id": "execution-uuid-from-step-3"
  }
}
```

#### 6. Analyze Errors

Use AI-powered error analysis:

```json
{
  "tool": "ratchet.analyze_execution_error",
  "arguments": {
    "execution_id": "execution-uuid-from-step-3",
    "include_suggestions": true,
    "include_similar_errors": true
  }
}
```

## Testing and Debugging

### Test Case Development

Create comprehensive test cases in the `tests/` directory:

```json
{
  "name": "Successful API call",
  "description": "Test successful data retrieval from API",
  "input": {
    "apiKey": "test-key",
    "endpoint": "https://httpbin.org/json",
    "timeout": 30
  },
  "expected_output": {
    "status": "success",
    "data": {}
  },
  "mock_http": {
    "requests": [
      {
        "method": "GET",
        "url": "https://httpbin.org/json",
        "response": {
          "status": 200,
          "body": {"key": "value"}
        }
      }
    ]
  }
}
```

### Debugging Workflow

When a task fails:

1. **Check execution status** for high-level error information
2. **Get execution logs** to see console output and errors
3. **Get execution trace** for detailed step-by-step execution
4. **Use error analysis** for AI-powered suggestions
5. **Validate input** to ensure schema compliance
6. **Test with minimal input** to isolate the issue

### CLI Integration

Use the CLI for scaffolding and validation:

```bash
# Generate new task
ratchet generate task my-api-task --label "API Integration Task"

# Validate task structure
ratchet validate sample/js-tasks/my-api-task

# Run comprehensive tests
ratchet test sample/js-tasks/my-api-task

# Execute directly (alternative to MCP)
ratchet run-once sample/js-tasks/my-api-task --input '{"key":"value"}'
```

## Best Practices

### 1. Development Process

- **Start simple**: Begin with minimal input/output and basic logic
- **Test early**: Validate schemas and basic execution before adding complexity
- **Use MCP monitoring**: Leverage execution status and logs throughout development
- **Iterate quickly**: Make small changes and test frequently

### 2. Error Handling

- **Provide context**: Include relevant information in error messages
- **Categorize errors**: Use appropriate error types (NetworkError, DataError, Error)
- **Debug information**: Return debug data during development (remove in production)

### 3. MCP Tool Usage

- **Monitor all executions**: Always check status after executing tasks
- **Use error analysis**: Let AI help you understand and fix issues
- **Validate inputs**: Check schema compliance before complex logic
- **Leverage logs**: Use execution logs to understand task behavior

### 4. Schema Design

- **Be specific**: Use precise types and constraints
- **Document everything**: Add descriptions to all fields
- **Test edge cases**: Include boundary conditions in schemas
- **Version schemas**: Consider backward compatibility

### 5. Performance

- **Set appropriate timeouts**: In metadata.json and for HTTP calls
- **Monitor execution time**: Use MCP tools to track performance
- **Handle large data**: Process in chunks when necessary
- **Cache wisely**: Store expensive computations in variables

## Troubleshooting

### Common Issues and Solutions

#### 1. Schema Validation Failures

**Problem**: Task input validation fails
```
Error: Input validation failed: property 'apiKey' is required
```

**Solution**: Use MCP to validate and debug:
```json
{
  "tool": "ratchet.validate_task_input",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/my-task",
    "input": {
      "apiKey": "test-key"
    }
  }
}
```

#### 2. Execution Timeouts

**Problem**: Task execution times out
```
Error: Task execution timed out after 30000ms
```

**Solution**: 
1. Check execution logs for bottlenecks
2. Increase timeout in metadata.json
3. Optimize performance

```json
{
  "tool": "ratchet.get_execution_logs",
  "arguments": {
    "execution_id": "uuid",
    "level": "debug"
  }
}
```

#### 3. HTTP Request Failures

**Problem**: External API calls fail
```
NetworkError: Failed to connect to API
```

**Solution**: 
1. Check execution trace for detailed request info
2. Validate API endpoints and credentials
3. Add proper error handling

```json
{
  "tool": "ratchet.get_execution_trace",
  "arguments": {
    "execution_id": "uuid"
  }
}
```

#### 4. JavaScript Runtime Errors

**Problem**: JavaScript execution fails
```
Error: Cannot read property 'x' of undefined
```

**Solution**: Use error analysis for suggestions:
```json
{
  "tool": "ratchet.analyze_execution_error",
  "arguments": {
    "execution_id": "uuid",
    "include_suggestions": true
  }
}
```

### Debugging Checklist

When troubleshooting a task:

- [ ] Check if task structure is valid (`ratchet validate`)
- [ ] Verify input matches schema (`ratchet.validate_task_input`)
- [ ] Examine execution status (`ratchet.get_execution_status`)
- [ ] Review execution logs (`ratchet.get_execution_logs`)
- [ ] Get detailed trace (`ratchet.get_execution_trace`)
- [ ] Use AI error analysis (`ratchet.analyze_execution_error`)
- [ ] Test with minimal input to isolate issue
- [ ] Check for similar working tasks as examples

## Examples

### Example 1: Weather API Task

**Objective**: Create a task that fetches weather data from an external API

#### Step 1: Generate Task Scaffold
```bash
ratchet generate task weather-fetcher --label "Weather Data Fetcher"
```

#### Step 2: Design Input Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["location", "apiKey"],
  "properties": {
    "location": {
      "type": "string",
      "description": "City name or coordinates",
      "minLength": 1
    },
    "apiKey": {
      "type": "string", 
      "description": "OpenWeatherMap API key",
      "minLength": 1
    },
    "units": {
      "type": "string",
      "enum": ["metric", "imperial", "kelvin"],
      "default": "metric"
    }
  }
}
```

#### Step 3: Implement Logic
```javascript
(function(input, context) {
    const { location, apiKey, units = 'metric' } = input;
    
    try {
        // Build API URL
        const url = `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(location)}&appid=${apiKey}&units=${units}`;
        
        // Make API request
        const response = fetch(url, {
            method: "GET",
            headers: {
                'Content-Type': 'application/json'
            }
        });
        
        if (!response.ok) {
            throw new NetworkError(`Weather API returned ${response.status}: ${response.statusText}`);
        }
        
        const weatherData = response.body;
        
        if (!weatherData || !weatherData.main) {
            throw new DataError("Invalid weather data format received");
        }
        
        // Return formatted weather information
        return {
            location: weatherData.name,
            country: weatherData.sys.country,
            temperature: weatherData.main.temp,
            humidity: weatherData.main.humidity,
            description: weatherData.weather[0].description,
            units: units,
            timestamp: new Date().toISOString(),
            executionId: context ? context.executionId : null
        };
        
    } catch (error) {
        if (error instanceof NetworkError || error instanceof DataError) {
            throw error;
        }
        throw new Error(`Weather fetch failed: ${error.message}`);
    }
})
```

#### Step 4: Test with MCP

Validate input:
```json
{
  "tool": "ratchet.validate_task_input",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/weather-fetcher",
    "input": {
      "location": "London",
      "apiKey": "your-api-key",
      "units": "metric"
    }
  }
}
```

Execute task:
```json
{
  "tool": "ratchet.execute_task",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/weather-fetcher",
    "input": {
      "location": "London",
      "apiKey": "your-api-key",
      "units": "metric"
    }
  }
}
```

Monitor execution:
```json
{
  "tool": "ratchet.get_execution_status",
  "arguments": {
    "execution_id": "execution-uuid-from-previous-step"
  }
}
```

### Example 2: Data Processing Task

**Objective**: Create a task that processes and transforms data arrays

#### Implementation
```javascript
(function(input, context) {
    const { data, operations = [], outputFormat = 'json' } = input;
    
    if (!Array.isArray(data)) {
        throw new DataError("Input 'data' must be an array");
    }
    
    let processedData = [...data];
    let operationsSummary = [];
    
    // Apply each operation in sequence
    for (const [index, operation] of operations.entries()) {
        const beforeCount = processedData.length;
        
        try {
            switch (operation.type) {
                case 'filter':
                    processedData = processedData.filter(item => 
                        evaluateCondition(item, operation.condition)
                    );
                    break;
                    
                case 'map':
                    processedData = processedData.map(item => 
                        applyTransformation(item, operation.transformation)
                    );
                    break;
                    
                case 'sort':
                    const field = operation.field;
                    const order = operation.order || 'asc';
                    processedData.sort((a, b) => {
                        const aVal = a[field];
                        const bVal = b[field];
                        if (order === 'desc') {
                            return bVal > aVal ? 1 : -1;
                        }
                        return aVal > bVal ? 1 : -1;
                    });
                    break;
                    
                default:
                    throw new DataError(`Unknown operation type: ${operation.type}`);
            }
            
            operationsSummary.push({
                step: index + 1,
                operation: operation.type,
                itemsBefore: beforeCount,
                itemsAfter: processedData.length
            });
            
        } catch (error) {
            throw new DataError(`Operation ${index + 1} (${operation.type}) failed: ${error.message}`);
        }
    }
    
    return {
        originalCount: data.length,
        processedCount: processedData.length,
        operations: operationsSummary,
        data: processedData,
        format: outputFormat,
        processedAt: new Date().toISOString(),
        executionId: context ? context.executionId : null
    };
    
    function evaluateCondition(item, condition) {
        const { field, operator, value } = condition;
        const itemValue = item[field];
        
        switch (operator) {
            case 'equals':
                return itemValue === value;
            case 'not_equals':
                return itemValue !== value;
            case 'greater_than':
                return itemValue > value;
            case 'less_than':
                return itemValue < value;
            case 'contains':
                return String(itemValue).includes(String(value));
            default:
                throw new DataError(`Unknown operator: ${operator}`);
        }
    }
    
    function applyTransformation(item, transformation) {
        const result = {};
        for (const [newField, sourceField] of Object.entries(transformation)) {
            result[newField] = item[sourceField];
        }
        return result;
    }
})
```

#### Test with MCP
```json
{
  "tool": "ratchet.execute_task",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/data-processor",
    "input": {
      "data": [
        {"name": "Alice", "age": 30, "city": "London"},
        {"name": "Bob", "age": 25, "city": "Paris"},
        {"name": "Charlie", "age": 35, "city": "London"}
      ],
      "operations": [
        {
          "type": "filter",
          "condition": {
            "field": "city",
            "operator": "equals",
            "value": "London"
          }
        },
        {
          "type": "sort",
          "field": "age",
          "order": "desc"
        }
      ]
    }
  }
}
```

## Development Checklist

Before considering a task complete:

- [ ] **Task Structure**
  - [ ] metadata.json has correct UUID, version, and description
  - [ ] input.schema.json validates all required fields with proper constraints
  - [ ] output.schema.json matches actual return structure
  - [ ] main.js implements function wrapper correctly: `(function(input, context) { ... })`

- [ ] **Functionality**
  - [ ] Error handling covers common failure scenarios
  - [ ] Return values provide useful information without exposing secrets
  - [ ] HTTP requests use proper error handling and timeouts

- [ ] **Testing** 
  - [ ] Input validation passes: `ratchet.validate_task_input`
  - [ ] Basic execution succeeds: `ratchet.execute_task`
  - [ ] Execution status shows success: `ratchet.get_execution_status`
  - [ ] At least 3 test cases: success, failure, edge case
  - [ ] All CLI tests pass: `ratchet test`

- [ ] **Monitoring & Debugging**
  - [ ] Execution logs are clean: `ratchet.get_execution_logs`
  - [ ] No errors in execution trace: `ratchet.get_execution_trace`
  - [ ] Error analysis (if any) provides useful suggestions
  - [ ] Performance is acceptable for expected input sizes

- [ ] **Documentation**
  - [ ] Schemas have clear descriptions for all fields
  - [ ] Error messages are helpful and specific
  - [ ] Code comments explain complex logic

This guide provides everything needed for an LLM to effectively develop, test, and debug Ratchet tasks using the MCP interface. The combination of MCP tools for real-time interaction and CLI tools for scaffolding provides a complete development environment.