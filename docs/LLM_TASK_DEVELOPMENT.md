# LLM Guide: Developing Ratchet Tasks

This guide provides comprehensive instructions for Large Language Models (LLMs) to develop, test, and debug JavaScript tasks using the Ratchet platform. You can interact with Ratchet through direct binary invocation or via the Model Context Protocol (MCP) server.

## Table of Contents

1. [Setup and Configuration](#setup-and-configuration)
2. [Development Methods](#development-methods)
3. [Task Structure and Requirements](#task-structure-and-requirements)
4. [Binary Usage for Task Development](#binary-usage-for-task-development)
5. [MCP Server Usage](#mcp-server-usage)
6. [Development Workflow](#development-workflow)
7. [Testing and Debugging](#testing-and-debugging)
8. [Troubleshooting Guide](#troubleshooting-guide)
9. [Complete Examples](#complete-examples)

## Setup and Configuration

### Prerequisites

Before starting task development, ensure you have:

1. **Ratchet Binary**: The `ratchet` executable available in your PATH
2. **Working Directory**: A directory where you can create task files
3. **Configuration** (optional): A config file for advanced setups

### Initial Setup

#### 1. Verify Ratchet Installation

```bash
# Check if ratchet is available
ratchet --version

# Display help to see available commands
ratchet --help
```

#### 2. Create Working Directory

```bash
# Create a directory for your tasks
mkdir my-ratchet-tasks
cd my-ratchet-tasks
```

#### 3. Basic Configuration (Optional)

Create a minimal config file if needed:

```yaml
# config.yaml
execution:
  max_execution_duration: 60  # seconds
  validate_schemas: true

http:
  timeout: 30
  verify_ssl: true
```

Use with: `ratchet --config config.yaml [command]`

## Development Methods

Ratchet offers two primary ways to develop and test tasks:

### Method 1: Direct Binary Usage
- **Best for**: Quick development, testing, validation
- **Pros**: Simple, direct, no setup required
- **Use**: CLI commands like `ratchet run-once`, `ratchet test`, `ratchet validate`

### Method 2: MCP Server Integration  
- **Best for**: Interactive development, real-time monitoring, complex debugging
- **Pros**: Rich toolset, execution monitoring, error analysis
- **Use**: MCP tools through LLM integration

## Task Structure and Requirements

### Directory Structure

Every Ratchet task is a directory containing these files:

```
my-task/
├── metadata.json      # Task metadata and configuration
├── main.js           # JavaScript implementation (required)
├── input.schema.json # Input validation schema (required)
├── output.schema.json# Output validation schema (required)
└── tests/            # Test cases (optional but recommended)
    ├── test-001.json
    ├── test-002.json
    └── test-003.json
```

### Required Files

#### 1. metadata.json
```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "version": "1.0.0",
  "label": "My Task",
  "description": "What this task does",
  "author": "LLM Assistant",
  "tags": ["api", "data"],
  "timeout": 30000,
  "memory_limit": "128MB"
}
```

#### 2. main.js (Function Wrapper Required)
```javascript
(function(input, context) {
    // Your task logic here
    // input: validated against input.schema.json
    // context: optional execution context
    
    try {
        const { param1, param2 } = input;
        
        // Your implementation
        const result = processData(param1, param2);
        
        return {
            result: result,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        throw new Error(`Task failed: ${error.message}`);
    }
})
```

**Critical Requirements:**
- Function must be wrapped: `(function(input, context) { ... })`
- No async/await - all operations must be synchronous
- Use built-in `fetch()` for HTTP requests
- No external modules or Node.js APIs

#### 3. input.schema.json
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["param1"],
  "properties": {
    "param1": {
      "type": "string",
      "description": "Description of param1",
      "minLength": 1
    },
    "param2": {
      "type": "number",
      "description": "Optional parameter",
      "minimum": 0
    }
  }
}
```

#### 4. output.schema.json
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["result", "timestamp"],
  "properties": {
    "result": {
      "type": "object",
      "description": "Task processing result"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "When task completed"
    }
  }
}
```

### Available JavaScript APIs

#### fetch(url, options, body) - HTTP Requests
```javascript
// GET request
const response = fetch("https://api.example.com/data");
if (response.ok) {
    const data = response.body;
    // Process response data
}

// POST request
const response = fetch(
    "https://api.example.com/create",
    { 
        method: "POST", 
        headers: { "Content-Type": "application/json" } 
    },
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

#### Console Logging
```javascript
console.log("Debug information");
console.error("Error details");
console.warn("Warning message");
```

## Binary Usage for Task Development

### Development Commands

#### 1. Generate Task Scaffold

```bash
# Create a new task with basic structure
ratchet generate task my-api-task \
  --label "API Integration Task" \
  --description "Fetches data from external API"

# This creates:
# my-api-task/
# ├── metadata.json
# ├── main.js (with template)
# ├── input.schema.json (basic template)
# └── output.schema.json (basic template)
```

#### 2. Validate Task Structure

```bash
# Validate all task files and schemas
ratchet validate my-api-task/

# Expected output:
# ✅ Task structure is valid
# ✅ Metadata is valid
# ✅ Input schema is valid
# ✅ Output schema is valid
# ✅ JavaScript syntax is valid
```

#### 3. Execute Task (Development Testing)

```bash
# Execute task with JSON input
ratchet run-once my-api-task/ \
  --input-json='{"param1": "test", "param2": 42}'

# Execute with input from file
echo '{"param1": "test", "param2": 42}' > input.json
ratchet run-once my-api-task/ --input-file input.json

# Execute with recording for debugging
ratchet run-once my-api-task/ \
  --input-json='{"param1": "test"}' \
  --record ./debug-session/
```

#### 4. Run Test Suite

```bash
# Run all test cases in tests/ directory
ratchet test my-api-task/

# Expected output:
# Running tests for task: my-api-task
# ✅ test-001.json - PASSED
# ✅ test-002.json - PASSED  
# ❌ test-003.json - FAILED: NetworkError: Connection timeout
# 
# 2 passed, 1 failed
```

#### 5. Start Development Server (Optional)

```bash
# Start server for web access (optional)
ratchet serve --config config.yaml

# Server will be available at:
# - REST API: http://localhost:8080/api/v1
# - GraphQL: http://localhost:8080/graphql
# - Health: http://localhost:8080/health
```

### Development Workflow with Binary

#### Step 1: Create and Structure Task

```bash
# Generate scaffold
ratchet generate task weather-api --label "Weather API Fetcher"

# Navigate to task directory
cd weather-api/
```

#### Step 2: Design Schemas

Edit `input.schema.json`:
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
      "enum": ["metric", "imperial"],
      "default": "metric"
    }
  }
}
```

#### Step 3: Implement Logic

Edit `main.js`:
```javascript
(function(input, context) {
    const { location, apiKey, units = 'metric' } = input;
    
    try {
        const url = `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(location)}&appid=${apiKey}&units=${units}`;
        
        const response = fetch(url, {
            method: "GET",
            headers: { 'Content-Type': 'application/json' }
        });
        
        if (!response.ok) {
            throw new NetworkError(`Weather API returned ${response.status}`);
        }
        
        const data = response.body;
        
        return {
            location: data.name,
            temperature: data.main.temp,
            description: data.weather[0].description,
            units: units,
            timestamp: new Date().toISOString()
        };
        
    } catch (error) {
        if (error instanceof NetworkError) {
            throw error;
        }
        throw new Error(`Weather fetch failed: ${error.message}`);
    }
})
```

#### Step 4: Test and Validate

```bash
# Validate structure
ratchet validate .

# Test with sample data
ratchet run-once . \
  --input-json='{"location": "London", "apiKey": "test-key", "units": "metric"}'

# If successful, create test cases
mkdir tests
cat > tests/test-001.json << EOF
{
  "name": "London weather test",
  "input": {
    "location": "London",
    "apiKey": "test-key",
    "units": "metric"
  },
  "expected_output": {
    "location": "London",
    "temperature": 20.5,
    "units": "metric"
  }
}
EOF

# Run test suite
ratchet test .
```

## MCP Server Usage

### Starting MCP Server

The MCP server provides rich interactive tools for task development:

#### Start MCP Server

```bash
# Start with stdio transport (for CLI integration)
ratchet mcp-serve --transport stdio

# Start with SSE transport (for HTTP access)
ratchet mcp-serve --transport sse --port 3001

# Start with configuration file
ratchet mcp-serve --config mcp-config.yaml
```

#### MCP Configuration

```yaml
# mcp-config.yaml
mcp:
  enabled: true
  transport: sse
  host: localhost
  port: 3001
  auth_type: none
  max_connections: 10
  request_timeout: 30
  rate_limit_per_minute: 100
```

### Available MCP Tools

The MCP server provides these tools for task development:

#### Core Execution Tools

1. **`ratchet.execute_task`** - Execute a task with input data
2. **`ratchet.list_available_tasks`** - List all tasks in the registry
3. **`ratchet.get_task_info`** - Get detailed task information

#### Monitoring & Debugging Tools

4. **`ratchet.get_execution_status`** - Get execution status
5. **`ratchet.get_execution_logs`** - Get execution logs
6. **`ratchet.get_execution_trace`** - Get detailed execution trace
7. **`ratchet.analyze_execution_error`** - AI-powered error analysis

#### Management Tools

8. **`ratchet.cancel_execution`** - Cancel running execution
9. **`ratchet.validate_task_input`** - Validate input against schema

### MCP Development Workflow

#### Step 1: List Available Tasks

```json
{
  "tool": "ratchet.list_available_tasks",
  "arguments": {}
}
```

#### Step 2: Examine Existing Task

```json
{
  "tool": "ratchet.get_task_info",
  "arguments": {
    "task_id_or_path": "sample/js-tasks/addition"
  }
}
```

#### Step 3: Validate Input Schema

```json
{
  "tool": "ratchet.validate_task_input",
  "arguments": {
    "task_id_or_path": "weather-api/",
    "input": {
      "location": "London",
      "apiKey": "test-key",
      "units": "metric"
    }
  }
}
```

#### Step 4: Execute Task

```json
{
  "tool": "ratchet.execute_task",
  "arguments": {
    "task_id_or_path": "weather-api/",
    "input": {
      "location": "London", 
      "apiKey": "your-real-api-key",
      "units": "metric"
    }
  }
}
```

#### Step 5: Monitor Execution

```json
{
  "tool": "ratchet.get_execution_status",
  "arguments": {
    "execution_id": "uuid-from-execute-response"
  }
}
```

#### Step 6: Debug Issues (if needed)

```json
{
  "tool": "ratchet.get_execution_logs",
  "arguments": {
    "execution_id": "uuid-from-execute-response",
    "level": "debug",
    "limit": 50
  }
}
```

```json
{
  "tool": "ratchet.analyze_execution_error",
  "arguments": {
    "execution_id": "uuid-from-execute-response",
    "include_suggestions": true
  }
}
```

## Development Workflow

### Recommended Development Process

Whether using binary commands or MCP tools, follow this workflow:

#### 1. Planning and Design
- Define task purpose and requirements
- Design input/output schemas with proper validation
- Identify external dependencies (APIs, data sources)

#### 2. Implementation
- Generate task scaffold (binary: `ratchet generate task`)
- Implement main.js with proper error handling
- Update schemas based on implementation

#### 3. Testing and Validation
- Validate task structure (binary: `ratchet validate`)
- Test with sample inputs (binary: `ratchet run-once` or MCP: `execute_task`)
- Create comprehensive test cases
- Run test suite (binary: `ratchet test`)

#### 4. Debugging and Optimization
- Use execution logs to identify issues
- Monitor performance and optimize if needed
- Test edge cases and error scenarios

#### 5. Documentation and Completion
- Ensure schemas have clear descriptions
- Add helpful error messages
- Create comprehensive test coverage

### Binary vs MCP: When to Use What

#### Use Binary Commands When:
- Creating new tasks (scaffolding)
- Validating task structure  
- Running comprehensive test suites
- Quick one-off testing
- Batch processing multiple tasks

#### Use MCP Tools When:
- Interactive development and debugging
- Real-time execution monitoring
- Error analysis and suggestions
- Understanding execution flows
- Iterative testing with variations

## Testing and Debugging

### Test Case Structure

Create test cases in the `tests/` directory:

```json
{
  "name": "Successful API call",
  "description": "Test successful weather data retrieval",
  "input": {
    "location": "London",
    "apiKey": "test-key",
    "units": "metric"
  },
  "expected_output": {
    "location": "London",
    "temperature": 20.5,
    "units": "metric",
    "timestamp": "2023-01-01T00:00:00.000Z"
  },
  "mock_http": {
    "requests": [
      {
        "method": "GET",
        "url": "https://api.openweathermap.org/data/2.5/weather",
        "response": {
          "status": 200,
          "body": {
            "name": "London",
            "main": {"temp": 20.5},
            "weather": [{"description": "clear sky"}]
          }
        }
      }
    ]
  }
}
```

### Debugging Strategies

#### Using Binary Commands

```bash
# Enable detailed logging
ratchet run-once my-task/ \
  --input-json='{"param": "value"}' \
  --log-level debug

# Record execution for analysis
ratchet run-once my-task/ \
  --input-json='{"param": "value"}' \
  --record ./debug-output/

# Validate specific components
ratchet validate my-task/ --verbose
```

#### Using MCP Tools

```json
// Get detailed execution trace
{
  "tool": "ratchet.get_execution_trace",
  "arguments": {
    "execution_id": "uuid"
  }
}

// Analyze errors with AI suggestions
{
  "tool": "ratchet.analyze_execution_error", 
  "arguments": {
    "execution_id": "uuid",
    "include_suggestions": true,
    "include_similar_errors": true
  }
}
```

## Troubleshooting Guide

### Common Issues and Solutions

#### 1. Schema Validation Failures

**Problem**: Input validation fails
```
Error: Input validation failed: property 'apiKey' is required
```

**Solution**:
```bash
# Check schema with binary
ratchet validate my-task/ --verbose

# Or test with MCP
{
  "tool": "ratchet.validate_task_input",
  "arguments": {
    "task_id_or_path": "my-task/",
    "input": {"apiKey": "test-key"}
  }
}
```

#### 2. JavaScript Runtime Errors

**Problem**: Task execution fails with JS errors
```
Error: Cannot read property 'x' of undefined
```

**Solution**:
```bash
# Test with minimal input
ratchet run-once my-task/ \
  --input-json='{}' \
  --log-level debug

# Use MCP for detailed analysis
{
  "tool": "ratchet.analyze_execution_error",
  "arguments": {
    "execution_id": "uuid",
    "include_suggestions": true
  }
}
```

#### 3. HTTP Request Failures

**Problem**: External API calls fail
```
NetworkError: Failed to connect to API
```

**Solution**:
```bash
# Test with recording to see HTTP details
ratchet run-once my-task/ \
  --input-json='{"apiKey": "test"}' \
  --record ./http-debug/

# Check execution trace via MCP
{
  "tool": "ratchet.get_execution_trace",
  "arguments": {"execution_id": "uuid"}
}
```

#### 4. Timeout Issues

**Problem**: Task execution times out
```
Error: Task execution timed out after 30000ms
```

**Solution**:
- Increase timeout in metadata.json
- Optimize performance 
- Check for infinite loops
- Use execution logs to identify bottlenecks

### Debugging Checklist

When troubleshooting a task:

- [ ] Validate task structure: `ratchet validate my-task/`
- [ ] Check input schema compliance
- [ ] Test with minimal input data
- [ ] Review execution logs for errors
- [ ] Check HTTP request/response details
- [ ] Verify external API credentials and endpoints
- [ ] Test individual components in isolation
- [ ] Use MCP error analysis for suggestions

## Complete Examples

### Example 1: Simple Data Processing Task

**Goal**: Create a task that processes and filters an array of data

#### Setup
```bash
ratchet generate task data-filter --label "Data Filter Task"
cd data-filter/
```

#### Input Schema (input.schema.json)
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["data", "filter"],
  "properties": {
    "data": {
      "type": "array",
      "items": {"type": "object"},
      "description": "Array of objects to filter"
    },
    "filter": {
      "type": "object",
      "properties": {
        "field": {"type": "string"},
        "value": {},
        "operator": {
          "type": "string",
          "enum": ["equals", "contains", "greater_than", "less_than"]
        }
      },
      "required": ["field", "value", "operator"]
    }
  }
}
```

#### Implementation (main.js)
```javascript
(function(input, context) {
    const { data, filter } = input;
    
    if (!Array.isArray(data)) {
        throw new DataError("Input 'data' must be an array");
    }
    
    const { field, value, operator } = filter;
    
    let filteredData;
    try {
        filteredData = data.filter(item => {
            const itemValue = item[field];
            
            switch (operator) {
                case 'equals':
                    return itemValue === value;
                case 'contains':
                    return String(itemValue).includes(String(value));
                case 'greater_than':
                    return Number(itemValue) > Number(value);
                case 'less_than':
                    return Number(itemValue) < Number(value);
                default:
                    throw new DataError(`Unknown operator: ${operator}`);
            }
        });
    } catch (error) {
        throw new DataError(`Filter operation failed: ${error.message}`);
    }
    
    return {
        originalCount: data.length,
        filteredCount: filteredData.length,
        filter: filter,
        data: filteredData,
        timestamp: new Date().toISOString()
    };
})
```

#### Test and Validate
```bash
# Validate structure
ratchet validate .

# Test with sample data
ratchet run-once . --input-json='{
  "data": [
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25},
    {"name": "Charlie", "age": 35}
  ],
  "filter": {
    "field": "age",
    "value": 30,
    "operator": "greater_than"
  }
}'

# Create test case
mkdir tests
cat > tests/test-001.json << 'EOF'
{
  "name": "Filter by age greater than 30",
  "input": {
    "data": [
      {"name": "Alice", "age": 30},
      {"name": "Bob", "age": 25}, 
      {"name": "Charlie", "age": 35}
    ],
    "filter": {
      "field": "age",
      "value": 30,
      "operator": "greater_than"
    }
  },
  "expected_output": {
    "originalCount": 3,
    "filteredCount": 1,
    "data": [{"name": "Charlie", "age": 35}]
  }
}
EOF

# Run test suite
ratchet test .
```

### Example 2: REST API Integration Task

**Goal**: Create a task that fetches data from a REST API with error handling

#### Setup
```bash
ratchet generate task api-fetcher --label "REST API Fetcher"
cd api-fetcher/
```

#### Input Schema
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["url"],
  "properties": {
    "url": {
      "type": "string",
      "format": "uri",
      "description": "API endpoint URL"
    },
    "method": {
      "type": "string",
      "enum": ["GET", "POST", "PUT", "DELETE"],
      "default": "GET"
    },
    "headers": {
      "type": "object",
      "description": "HTTP headers to send"
    },
    "body": {
      "type": "object",
      "description": "Request body for POST/PUT requests"
    },
    "timeout": {
      "type": "number",
      "minimum": 1,
      "maximum": 60,
      "default": 30
    }
  }
}
```

#### Implementation
```javascript
(function(input, context) {
    const { 
        url, 
        method = 'GET', 
        headers = {}, 
        body = null, 
        timeout = 30 
    } = input;
    
    try {
        // Prepare request options
        const requestOptions = {
            method: method,
            headers: {
                'Content-Type': 'application/json',
                ...headers
            }
        };
        
        console.log(`Making ${method} request to: ${url}`);
        
        // Make HTTP request
        const response = fetch(url, requestOptions, body);
        
        if (!response.ok) {
            throw new NetworkError(
                `HTTP ${response.status}: ${response.statusText}`
            );
        }
        
        const responseData = response.body;
        
        return {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers || {},
            data: responseData,
            url: url,
            method: method,
            timestamp: new Date().toISOString(),
            executionId: context ? context.executionId : null
        };
        
    } catch (error) {
        if (error instanceof NetworkError) {
            throw error;
        }
        throw new Error(`API request failed: ${error.message}`);
    }
})
```

#### Testing with Binary
```bash
# Test successful API call
ratchet run-once . --input-json='{
  "url": "https://httpbin.org/json",
  "method": "GET"
}'

# Test with custom headers
ratchet run-once . --input-json='{
  "url": "https://httpbin.org/headers",
  "method": "GET",
  "headers": {
    "User-Agent": "Ratchet-Task/1.0",
    "Accept": "application/json"
  }
}'
```

#### Testing with MCP
```json
{
  "tool": "ratchet.execute_task",
  "arguments": {
    "task_id_or_path": "api-fetcher/",
    "input": {
      "url": "https://httpbin.org/json",
      "method": "GET",
      "timeout": 30
    }
  }
}
```

Then monitor:
```json
{
  "tool": "ratchet.get_execution_status",
  "arguments": {
    "execution_id": "execution-uuid-from-previous"
  }
}
```

### Development Best Practices

1. **Start Simple**: Begin with minimal functionality and build up
2. **Validate Early**: Use `ratchet validate` and MCP validation frequently
3. **Test Incrementally**: Test each component as you build it
4. **Handle Errors**: Provide specific, helpful error messages
5. **Monitor Execution**: Use logs and traces to understand behavior
6. **Document Schemas**: Include clear descriptions in JSON schemas
7. **Use Both Methods**: Combine binary commands and MCP tools effectively

This guide provides everything an LLM needs to develop, test, and debug Ratchet tasks using both direct binary usage and MCP integration. The combination provides a powerful, flexible development environment for creating robust JavaScript task automation.