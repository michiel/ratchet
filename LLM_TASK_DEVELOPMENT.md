# LLM Task Development Guide for Ratchet

This guide provides comprehensive instructions for Large Language Models (LLMs) to understand, instrument, and develop tasks for the Ratchet system.

## Table of Contents

1. [System Overview](#system-overview)
2. [Architecture](#architecture)
3. [Task Structure](#task-structure)
4. [Development Workflow](#development-workflow)
5. [Task Generation](#task-generation)
6. [Implementation Guidelines](#implementation-guidelines)
7. [Testing Framework](#testing-framework)
8. [Examples](#examples)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

## System Overview

Ratchet is a task execution engine designed for automated workflow processing. It provides:

- **Task Registry**: Manages JavaScript-based task definitions
- **Execution Engine**: Runs tasks in isolated processes with resource management
- **REST/GraphQL APIs**: Unified interface for task management and execution
- **Scheduler**: Cron-based task scheduling
- **Output Destinations**: Flexible result delivery (filesystem, webhooks, databases)
- **Testing Framework**: Comprehensive test suite for task validation

### Key Concepts

- **Task**: A JavaScript module with defined input/output schemas and business logic
- **Job**: An execution instance of a task with specific input data
- **Execution**: The runtime state and results of a job
- **Schedule**: Automated task execution based on cron expressions

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   REST/GraphQL  │    │   Task Registry │    │ Execution Engine│
│      APIs       │◄──►│   (File-based)  │◄──►│  (Process-based)│
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│    Database     │    │    Scheduler    │    │Output Destinations│
│ (Jobs/Results)  │    │  (Cron-based)  │    │ (Files/Webhooks) │
└─────────────────┘    └─────────────────┘    └─────────────────┘
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
  "memory_limit": "128MB",
  "requirements": {
    "node_version": ">=14.0.0"
  }
}
```

### Schema Files

- **input.schema.json**: JSON Schema defining expected input structure
- **output.schema.json**: JSON Schema defining expected output structure

### main.js Implementation

```javascript
// Required: Main execution function
async function execute(input, context) {
    // Your task logic here
    // - input: validated against input.schema.json
    // - context: execution context with utilities
    // Returns: object validated against output.schema.json
}

// Optional: Setup function called once per process
async function setup(context) {
    // Initialize resources, connections, etc.
}

// Optional: Cleanup function called on process exit
async function cleanup(context) {
    // Clean up resources
}

// Export the functions
module.exports = { execute, setup, cleanup };
```

### Context Object

The `context` parameter provides utilities:

```javascript
{
  // HTTP client for external API calls
  http: {
    get: async (url, options) => response,
    post: async (url, data, options) => response,
    put: async (url, data, options) => response,
    delete: async (url, options) => response
  },
  
  // Logging utilities
  log: {
    info: (message, data) => void,
    error: (message, error) => void,
    warn: (message, data) => void,
    debug: (message, data) => void
  },
  
  // Task metadata
  meta: {
    uuid: "task-uuid",
    version: "1.0.0",
    label: "Task Name"
  },
  
  // Execution metadata
  execution: {
    id: "execution-id",
    jobId: "job-id",
    startTime: Date,
    timeout: 30000
  }
}
```

## Development Workflow

### 1. Understanding Requirements

When given a task request, analyze:
- **Input requirements**: What data does the task need?
- **Processing logic**: What operations to perform?
- **Output format**: What should be returned?
- **External dependencies**: APIs, services, or data sources needed?
- **Error handling**: What could go wrong and how to handle it?

### 2. Using Ratchet Generate

Use the `ratchet generate` command to scaffold a new task:

```bash
# Generate a new task
ratchet generate task my-task-name

# This creates:
# sample/js-tasks/my-task-name/
# ├── metadata.json
# ├── input.schema.json  
# ├── output.schema.json
# ├── main.js
# └── tests/
#     └── test-001.json
```

### 3. Development Process

1. **Define schemas first**: Start with input and output schemas
2. **Implement core logic**: Write the main execute function
3. **Add error handling**: Handle edge cases and failures
4. **Create comprehensive tests**: Cover happy path and error cases
5. **Validate and iterate**: Test thoroughly before finalizing

## Task Generation

### Command Usage

```bash
# Basic task generation
ratchet generate task task-name

# With specific options
ratchet generate task task-name \
  --author "Your Name" \
  --description "Task description" \
  --tags "tag1,tag2"
```

### Generated Template

The generator creates a basic template that you should customize:

```javascript
// Generated main.js template
async function execute(input, context) {
    // TODO: Implement your task logic here
    
    try {
        // Your implementation
        return {
            success: true,
            result: "placeholder"
        };
    } catch (error) {
        context.log.error("Task execution failed", error);
        throw error;
    }
}

module.exports = { execute };
```

## Implementation Guidelines

### Input Validation

Always rely on schema validation - the input is pre-validated:

```javascript
async function execute(input, context) {
    // Input is already validated against input.schema.json
    // Access properties directly
    const { apiKey, endpoint, params } = input;
    
    // Perform additional business logic validation if needed
    if (!apiKey) {
        throw new Error("API key is required");
    }
}
```

### HTTP Requests

Use the provided HTTP context for external calls:

```javascript
async function execute(input, context) {
    try {
        // GET request
        const response = await context.http.get(input.url, {
            headers: {
                'Authorization': `Bearer ${input.apiKey}`,
                'Content-Type': 'application/json'
            },
            timeout: 10000
        });
        
        // POST request
        const postResponse = await context.http.post(input.webhookUrl, {
            data: response.data
        }, {
            headers: { 'Content-Type': 'application/json' }
        });
        
        return { success: true, data: response.data };
    } catch (error) {
        context.log.error("HTTP request failed", error);
        throw new Error(`API call failed: ${error.message}`);
    }
}
```

### Error Handling

Implement comprehensive error handling:

```javascript
async function execute(input, context) {
    try {
        // Main logic
        const result = await performOperation(input);
        return { success: true, data: result };
        
    } catch (error) {
        // Log the error
        context.log.error("Operation failed", {
            error: error.message,
            stack: error.stack,
            input: input
        });
        
        // Return structured error (if schema allows) or re-throw
        if (error.code === 'TIMEOUT') {
            return { 
                success: false, 
                error: "Operation timed out",
                retryable: true 
            };
        }
        
        throw error; // Re-throw for system handling
    }
}
```

### Logging Best Practices

Use appropriate log levels:

```javascript
async function execute(input, context) {
    context.log.info("Starting task execution", { 
        taskId: context.meta.uuid,
        inputSize: JSON.stringify(input).length 
    });
    
    context.log.debug("Processing data", { 
        step: "validation",
        details: input 
    });
    
    try {
        // ... logic
        context.log.info("Task completed successfully");
    } catch (error) {
        context.log.error("Task failed", error);
        throw error;
    }
}
```

## Testing Framework

### Test Case Structure

Each test case is a JSON file in the `tests/` directory:

```json
{
  "name": "Test case description",
  "description": "Detailed explanation of what this test validates",
  "input": {
    // Input data matching input.schema.json
  },
  "expected_output": {
    // Expected output matching output.schema.json
  },
  "should_fail": false,
  "timeout": 10000,
  "mock_http": {
    "requests": [
      {
        "method": "GET",
        "url": "https://api.example.com/data",
        "response": {
          "status": 200,
          "headers": { "Content-Type": "application/json" },
          "body": { "result": "mocked data" }
        }
      }
    ]
  }
}
```

### Test Categories

Create tests for different scenarios:

1. **Happy Path Tests** (`test-001-success.json`):
   ```json
   {
     "name": "Successful execution with valid input",
     "input": { /* valid input */ },
     "expected_output": { /* expected result */ },
     "should_fail": false
   }
   ```

2. **Error Handling Tests** (`test-002-invalid-input.json`):
   ```json
   {
     "name": "Handle invalid input gracefully",
     "input": { /* invalid input */ },
     "should_fail": true,
     "expected_error": "ValidationError"
   }
   ```

3. **Edge Case Tests** (`test-003-edge-cases.json`):
   ```json
   {
     "name": "Handle edge cases",
     "input": { /* edge case data */ },
     "expected_output": { /* expected behavior */ }
   }
   ```

4. **Mocked External API Tests** (`test-004-api-mock.json`):
   ```json
   {
     "name": "Test with mocked external API",
     "mock_http": {
       "requests": [
         {
           "method": "GET",
           "url": "https://api.example.com/endpoint",
           "response": {
             "status": 200,
             "body": { "data": "mocked response" }
           }
         }
       ]
     }
   }
   ```

### Running Tests

```bash
# Test a specific task
ratchet test sample/js-tasks/my-task

# Test all tasks
ratchet test sample/js-tasks/

# Run with verbose output
ratchet test sample/js-tasks/my-task --verbose

# Run specific test case
ratchet test sample/js-tasks/my-task --test test-001.json
```

## Examples

### Example 1: Weather API Task

Reference the existing `sample/js-tasks/weather-api/` for a complete example:

```javascript
// main.js
async function execute(input, context) {
    const { location, apiKey, units = 'metric' } = input;
    
    try {
        context.log.info("Fetching weather data", { location, units });
        
        const response = await context.http.get(
            `https://api.openweathermap.org/data/2.5/weather`,
            {
                params: {
                    q: location,
                    appid: apiKey,
                    units: units
                },
                timeout: 10000
            }
        );
        
        const weather = response.data;
        
        return {
            location: weather.name,
            country: weather.sys.country,
            temperature: weather.main.temp,
            description: weather.weather[0].description,
            humidity: weather.main.humidity,
            windSpeed: weather.wind.speed,
            timestamp: new Date().toISOString()
        };
        
    } catch (error) {
        if (error.response?.status === 401) {
            throw new Error("Invalid API key");
        } else if (error.response?.status === 404) {
            throw new Error(`Location '${location}' not found`);
        }
        
        context.log.error("Weather API request failed", error);
        throw new Error(`Weather service error: ${error.message}`);
    }
}

module.exports = { execute };
```

### Example 2: Data Processing Task

```javascript
// main.js - Data transformation task
async function execute(input, context) {
    const { data, transformations, outputFormat = 'json' } = input;
    
    context.log.info("Processing data", { 
        recordCount: data.length,
        transformations: transformations.length 
    });
    
    let processedData = [...data];
    
    // Apply transformations
    for (const transform of transformations) {
        switch (transform.type) {
            case 'filter':
                processedData = processedData.filter(
                    item => evaluateFilter(item, transform.condition)
                );
                break;
                
            case 'map':
                processedData = processedData.map(
                    item => applyMapping(item, transform.mapping)
                );
                break;
                
            case 'sort':
                processedData.sort((a, b) => 
                    compareValues(a[transform.field], b[transform.field], transform.order)
                );
                break;
        }
    }
    
    return {
        originalCount: data.length,
        processedCount: processedData.length,
        data: processedData,
        format: outputFormat,
        processedAt: new Date().toISOString()
    };
}

function evaluateFilter(item, condition) {
    // Implement filter logic
}

function applyMapping(item, mapping) {
    // Implement mapping logic
}

function compareValues(a, b, order) {
    // Implement sorting logic
}

module.exports = { execute };
```

## Best Practices

### 1. Schema Design

- **Be specific**: Use precise types and constraints
- **Document everything**: Add descriptions to all fields
- **Version schemas**: Consider backward compatibility
- **Validate thoroughly**: Use JSON Schema features effectively

Example input schema:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "title": "Weather API Input",
  "description": "Input parameters for weather data retrieval",
  "required": ["location", "apiKey"],
  "properties": {
    "location": {
      "type": "string",
      "description": "City name or coordinates (lat,lon)",
      "minLength": 1,
      "examples": ["London", "40.7128,-74.0060"]
    },
    "apiKey": {
      "type": "string",
      "description": "OpenWeatherMap API key",
      "pattern": "^[a-f0-9]{32}$"
    },
    "units": {
      "type": "string",
      "description": "Temperature units",
      "enum": ["metric", "imperial", "kelvin"],
      "default": "metric"
    }
  }
}
```

### 2. Error Handling Strategy

- **Catch and categorize**: Different error types need different handling
- **Provide context**: Include relevant information in error messages
- **Log appropriately**: Use correct log levels
- **Fail fast**: Don't continue processing with invalid state

### 3. Performance Considerations

- **Set timeouts**: For all external calls
- **Batch operations**: When possible
- **Stream large data**: Don't load everything into memory
- **Cache results**: For expensive operations within the task

### 4. Security Guidelines

- **Validate inputs**: Even though schemas handle structure
- **Sanitize outputs**: Prevent injection attacks
- **Secure external calls**: Use HTTPS, validate certificates
- **Handle secrets properly**: Never log sensitive data

## Troubleshooting

### Common Issues

1. **Schema Validation Errors**:
   ```
   Error: Input validation failed
   ```
   - Check input.schema.json syntax
   - Verify input data matches schema exactly
   - Use online JSON Schema validators

2. **Module Loading Errors**:
   ```
   Error: Cannot find module 'xyz'
   ```
   - Ratchet runs in isolated environment
   - Only built-in Node.js modules available
   - Use context.http instead of external HTTP libraries

3. **Timeout Errors**:
   ```
   Error: Task execution timed out
   ```
   - Increase timeout in metadata.json
   - Optimize task performance
   - Use streaming for large operations

4. **Memory Errors**:
   ```
   Error: JavaScript heap out of memory
   ```
   - Increase memory_limit in metadata.json
   - Process data in chunks
   - Clean up variables after use

### Debugging Tips

1. **Use logging extensively**:
   ```javascript
   context.log.debug("Current state", { variable1, variable2 });
   ```

2. **Test with simple cases first**:
   ```json
   {
     "name": "Minimal test case",
     "input": { /* minimal valid input */ }
   }
   ```

3. **Check generated files**:
   ```bash
   # Verify task structure
   find sample/js-tasks/my-task -type f
   
   # Validate schemas
   ratchet validate sample/js-tasks/my-task
   ```

## Development Checklist

Before considering a task complete:

- [ ] Metadata.json has correct UUID, version, and description
- [ ] Input schema validates all required fields with proper constraints
- [ ] Output schema matches actual return structure
- [ ] Main.js implements execute function correctly
- [ ] Error handling covers common failure scenarios
- [ ] Logging provides useful information without exposing secrets
- [ ] At least 3 test cases: success, failure, edge case
- [ ] Test cases include proper mocking for external APIs
- [ ] All tests pass when run with `ratchet test`
- [ ] Performance is acceptable for expected input sizes
- [ ] Documentation is clear and complete

## Execution Commands

```bash
# Development workflow
ratchet generate task my-task        # Create task scaffold
# Edit files: metadata.json, schemas, main.js, tests/
ratchet validate sample/js-tasks/my-task  # Validate structure
ratchet test sample/js-tasks/my-task      # Run tests
ratchet execute sample/js-tasks/my-task --input '{"key":"value"}'  # Test execution

# API interaction
curl -X POST http://localhost:3000/api/v1/tasks/execute \
  -H "Content-Type: application/json" \
  -d '{"taskId": "task-uuid", "input": {"key": "value"}}'

# GraphQL
curl -X POST http://localhost:3000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { executeTask(input: {taskId: \"uuid\", inputData: {key: \"value\"}}) { id status } }"}'
```

This guide provides everything needed to understand and develop tasks for the Ratchet system. Follow the patterns shown in `sample/js-tasks/weather-api/` as a reference implementation.