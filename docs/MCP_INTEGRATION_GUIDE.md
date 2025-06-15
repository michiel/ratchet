# MCP (Model Context Protocol) Integration Guide

This comprehensive guide explains how to set up and use Ratchet's MCP server for AI/LLM integration, particularly with Claude Desktop and other MCP-compatible clients.

## Table of Contents

1. [Overview](#overview)
2. [Quick Setup for Claude Desktop](#quick-setup-for-claude-desktop)
3. [Available MCP Tools](#available-mcp-tools)
4. [Configuration Options](#configuration-options)
5. [Advanced Setup](#advanced-setup)
6. [Troubleshooting](#troubleshooting)
7. [Example Workflows](#example-workflows)
8. [Architecture Details](#architecture-details)
9. [Best Practices](#best-practices)

## Overview

Ratchet provides comprehensive MCP (Model Context Protocol) support that allows Language Learning Models (LLMs) to interact with Ratchet's task execution engine through a standardized protocol. The MCP server exposes Ratchet's capabilities as tools that can be invoked by any MCP-compatible client.

### Server Modes

Ratchet offers multiple server modes:

1. **Regular Server** (`ratchet serve`) - Full HTTP/GraphQL API server for web applications
2. **MCP Server** (`ratchet mcp`) - General-purpose MCP server with SSE transport (default)
3. **MCP Server for Claude Desktop** (`ratchet mcp-serve`) - Optimized MCP server with stdio transport

Both MCP commands support both stdio and SSE transports with `--transport` option.

### Command Quick Reference

```bash
# General MCP server (SSE transport, port 8090)
ratchet mcp

# Claude Desktop optimized (stdio transport) 
ratchet mcp-serve

# Custom configuration
ratchet mcp --transport sse --port 8091 --config config.yaml
ratchet mcp-serve --config config.yaml

# Using Claude Code CLI (automatic integration)
claude mcp add ratchet ratchet mcp-serve
```

## Quick Setup for Claude Desktop

### 1. Installation & Basic Configuration

Create a basic configuration file:

```yaml
# config.yaml
database:
  url: "sqlite:ratchet.db"

logging:
  level: info
  sinks:
    - type: console
      level: info
    - type: file
      path: ratchet.log
      level: debug

# MCP-specific configuration
mcp:
  enabled: true
  transport: stdio
  auth_type: none  # For development - use api_key in production
  max_connections: 10
  request_timeout: 30
  rate_limit_per_minute: 100

# Task registry (optional)
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch_for_changes: true
        auto_reload: true
```

### 2. Configure Claude Desktop

Add Ratchet to your Claude Desktop MCP configuration:

**Configuration File Locations:**
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux:** `~/.config/claude/claude_desktop_config.json`

**Basic Configuration:**
```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/your/config.yaml"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Alternative (without config file):**
```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": [
        "mcp-serve", 
        "--transport", "stdio",
        "--host", "127.0.0.1",
        "--port", "8090"
      ]
    }
  }
}
```

### 3. Test the Integration

1. **Start Claude Desktop** - It will automatically connect to Ratchet
2. **Verify Connection** - Look for Ratchet tools in Claude's tool list
3. **Test Basic Functionality:**

```
Ask Claude: "What Ratchet tasks are available?"
→ Claude will use: ratchet.list_available_tasks

Ask Claude: "Execute the weather-api task for San Francisco"
→ Claude will use: ratchet.execute_task
```

## Available MCP Tools

### Task Discovery & Management

#### `ratchet.list_available_tasks`
List all available tasks with optional filtering.

**Parameters:**
- `filter` (string, optional): Filter tasks by name pattern
- `include_schemas` (boolean, optional): Include input/output schemas
- `category` (string, optional): Filter by task category

**Example:**
```json
{
  "name": "ratchet.list_available_tasks",
  "arguments": {
    "filter": "api",
    "include_schemas": true
  }
}
```

### Task Execution

#### `ratchet.execute_task`
Execute a Ratchet task with given input data.

**Parameters:**
- `task_id` (string, required): Task name or UUID
- `input` (object, required): Input data for the task
- `trace` (boolean, optional): Enable detailed tracing (default: true)
- `timeout` (integer, optional): Execution timeout in seconds

**Example:**
```json
{
  "name": "ratchet.execute_task",
  "arguments": {
    "task_id": "weather-api",
    "input": {
      "city": "San Francisco",
      "units": "metric"
    },
    "trace": true
  }
}
```

#### `ratchet.batch_execute`
Execute multiple tasks with dependency management.

**Parameters:**
- `tasks` (array, required): Array of task execution requests
- `max_parallel` (integer, optional): Maximum parallel executions
- `fail_fast` (boolean, optional): Stop on first failure

### Monitoring & Debugging

#### `ratchet.get_execution_status`
Get status and progress of a running execution.

**Parameters:**
- `execution_id` (string, required): ID of the execution to check

#### `ratchet.get_execution_logs`
Retrieve logs for a specific execution.

**Parameters:**
- `execution_id` (string, required): ID of the execution
- `level` (string, optional): Minimum log level (trace/debug/info/warn/error)
- `limit` (integer, optional): Maximum number of entries (default: 100)
- `format` (string, optional): Output format (json/text, default: json)

#### `ratchet.get_execution_trace`
Get detailed execution trace with timing and context.

**Parameters:**
- `execution_id` (string, required): ID of the execution
- `include_http_calls` (boolean, optional): Include HTTP traces (default: true)
- `format` (string, optional): Output format (json/flamegraph, default: json)

#### `ratchet.analyze_execution_error`
Analyze failed executions with fix suggestions.

**Parameters:**
- `execution_id` (string, required): ID of the failed execution
- `include_suggestions` (boolean, optional): Include fix suggestions (default: true)
- `include_context` (boolean, optional): Include execution context (default: true)

### Task Development (Available)

#### `ratchet.create_task`
Create new tasks with code and schemas.

#### `ratchet.edit_task`
Edit existing task code and metadata.

#### `ratchet.validate_task`
Validate task code and schemas.

#### `ratchet.run_task_tests`
Execute test cases for tasks.

#### `ratchet.debug_task_execution`
Debug with breakpoints and inspection.

## Configuration Options

### Transport Types

#### STDIO Transport (Recommended)
Direct process communication - ideal for desktop AI applications.

```yaml
mcp:
  transport: stdio
```

#### SSE Transport (For Web Integration)
HTTP-based Server-Sent Events for web applications.

```yaml
mcp:
  transport: sse
  host: "0.0.0.0"
  port: 8090
  cors:
    allowed_origins: ["http://localhost:3000"]
```

### Authentication Options

#### No Authentication (Development)
```yaml
mcp:
  auth_type: none
```

#### API Key Authentication (Production)
```yaml
mcp:
  auth_type: api_key
  api_keys:
    - key: "${MCP_API_KEY_CLAUDE}"
      name: "claude-desktop"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
        can_read_traces: true
        allowed_task_patterns: ["safe-*", "api-*"]
        rate_limits:
          executions_per_minute: 30
          logs_per_minute: 100
```

#### JWT Authentication (Enterprise)
```yaml
mcp:
  auth_type: jwt
  jwt:
    secret: "${JWT_SECRET}"
    issuer: "ratchet-server"
    audience: "mcp-clients"
    expiry: 3600
```

### Security Configuration

```yaml
mcp:
  security:
    max_execution_time: 300
    max_log_entries: 1000
    audit_log_enabled: true
    input_sanitization: true
    rate_limiting:
      execute_task_per_minute: 120
      batch_execute_per_minute: 30
      global_per_minute: 1000
    request_size_limit: 10485760  # 10MB
    response_size_limit: 52428800  # 50MB
```

## Advanced Setup

### Production Security Setup

For production environments, enable comprehensive security:

```yaml
mcp:
  enabled: true
  transport: sse
  host: "0.0.0.0"
  port: 8090
  auth_type: api_key
  
  api_keys:
    - key: "${MCP_API_KEY_CLAUDE}"
      name: "claude-desktop-prod"
      description: "Claude Desktop production access"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
        can_read_traces: true
        can_access_system_info: true
        allowed_task_patterns: ["safe-*", "read-only-*"]
        denied_task_patterns: ["admin-*", "system-*"]
      created_at: "2025-01-01T00:00:00Z"
      active: true
      allowed_ips: []
    
    - key: "${MCP_API_KEY_DEV}"
      name: "development"
      description: "Development and testing"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
        can_read_traces: true
        allowed_task_patterns: ["*"]
        denied_task_patterns: []

  security:
    audit_log_enabled: true
    input_sanitization: true
    rate_limiting:
      execute_task_per_minute: 60
      batch_execute_per_minute: 10
      global_per_minute: 500

  # Tool availability
  tools:
    enable_execution: true
    enable_logging: true
    enable_monitoring: true
    enable_debugging: true
    enable_filesystem: false
    enable_batch: true
    enable_progress: true
```

**Claude Desktop config with API key:**
```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/config.yaml"],
      "env": {
        "MCP_API_KEY_CLAUDE": "your-secret-api-key-here",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Multiple Ratchet Instances

Configure multiple Ratchet instances for different environments:

```json
{
  "mcpServers": {
    "ratchet-dev": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/dev-config.yaml"],
      "env": {
        "RATCHET_ENV": "development"
      }
    },
    "ratchet-staging": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/staging-config.yaml"],
      "env": {
        "RATCHET_ENV": "staging"
      }
    },
    "ratchet-prod": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/prod-config.yaml"],
      "env": {
        "RATCHET_ENV": "production",
        "MCP_API_KEY_CLAUDE": "prod-api-key"
      }
    }
  }
}
```

### Enhanced Logging for Debugging

```yaml
logging:
  level: debug
  sinks:
    - type: console
      level: info
      use_json: false
    - type: file
      path: ratchet-mcp.log
      level: debug
      max_size: 10485760  # 10MB
      max_backups: 5
      buffered:
        size: 1000
        flush_interval: 5s
  
# Enable MCP-specific logging
mcp:
  log_execution_traces: true
  enable_structured_logs: true
  audit_log_file: audit.log
```

## Troubleshooting

### Common Issues

#### Claude Can't Connect
**Symptoms:** Claude shows "Server connection failed" or doesn't list Ratchet tools

**Solutions:**

1. **Check the ratchet binary path:**
   ```bash
   which ratchet
   # Use the full path in Claude config if needed
   ```

2. **Verify configuration file exists and is readable:**
   ```bash
   ratchet mcp-serve --config /path/to/config.yaml --help
   ```

3. **Test MCP server directly:**
   ```bash
   ratchet mcp-serve --transport stdio --config /path/to/config.yaml
   # Should start without errors and accept JSON-RPC input
   ```

4. **Check Claude Desktop logs:**
   - macOS: `~/Library/Logs/Claude/`
   - Windows: `%APPDATA%\Claude\logs\`

#### "No tasks available"
**Symptoms:** `ratchet.list_available_tasks` returns empty list

**Solutions:**

1. **Ensure database is initialized:**
   ```bash
   ratchet serve --config /path/to/config.yaml &
   # Start regular server first to initialize DB
   curl http://localhost:8080/health
   ```

2. **Load sample tasks:**
   ```bash
   # Copy sample tasks to your tasks directory
   cp -r sample/js-tasks /path/to/your/tasks
   ```

3. **Check task registry configuration:**
   ```yaml
   registry:
     sources:
       - name: "local-tasks"
         uri: "file://./sample/js-tasks"
         config:
           watch_for_changes: true
           auto_reload: true
   ```

4. **Verify tasks are loaded:**
   ```bash
   curl http://localhost:8080/api/v1/tasks
   ```

#### Permission Denied Errors
**Symptoms:** Claude gets "Permission denied" when executing tasks

**Solutions:**
1. Check API key configuration (if using auth)
2. Verify task patterns in `allowed_task_patterns`
3. Review rate limits configuration
4. Check audit logs for specific permission failures

#### Slow Performance or Timeouts
**Symptoms:** Task execution takes too long or times out

**Solutions:**

1. **Increase timeout values:**
   ```yaml
   mcp:
     request_timeout: 60
     max_execution_time: 300
   ```

2. **Enable performance logging:**
   ```yaml
   mcp:
     log_execution_traces: true
   ```

3. **Monitor resource usage:**
   ```bash
   # Check system resources
   htop
   # Check log file size
   ls -lh ratchet*.log
   ```

4. **Optimize task execution:**
   ```yaml
   workers:
     count: 4  # Adjust based on CPU cores
     restart_on_failure: true
   ```

### Debug Commands

```bash
# Test configuration
ratchet mcp-serve --config config.yaml --help

# Start with verbose logging
RUST_LOG=debug ratchet mcp-serve --config config.yaml

# Test database connectivity
ratchet serve --config config.yaml &
curl http://localhost:8080/health

# Validate specific task
ratchet validate --from-fs ./path/to/task

# Test task execution directly
ratchet run-once --from-fs ./path/to/task --input-json '{"test": true}'
```

### Log Analysis

Check MCP server logs for connection and execution details:

```bash
# Follow live logs
tail -f ratchet-mcp.log

# Search for specific errors
grep -i "error\|fail" ratchet-mcp.log

# Check Claude connection attempts
grep -i "initialize\|connect" ratchet-mcp.log

# Analyze execution patterns
grep "execute_task" ratchet-mcp.log | jq .

# Check rate limiting
grep "rate_limit" ratchet-mcp.log
```

## Example Workflows

### 1. Data Processing Pipeline
```
You: "I have a CSV file with sales data. Can you process it?"

Claude: "I'll help you process the CSV data. Let me first see what data processing tasks are available."
→ Uses: ratchet.list_available_tasks with filter "csv"

Claude: "I found a csv-parser task. Can you provide the CSV data or file path?"

You: [Provide data]

Claude: "I'll process this data now."
→ Uses: ratchet.execute_task with csv-parser task

Claude: "Processing completed! Let me get the execution logs to show you the details."
→ Uses: ratchet.get_execution_logs
```

### 2. API Integration Testing
```
You: "Test our weather API integration"

Claude: "I'll test the weather API. Let me execute the weather-api task."
→ Uses: ratchet.execute_task with weather-api task

Claude: "The API call succeeded. Here are the results... Let me also check the execution logs for any warnings."
→ Uses: ratchet.get_execution_logs for detailed analysis

Claude: "I notice the API response time was 2.3 seconds. Let me get the execution trace to see the breakdown."
→ Uses: ratchet.get_execution_trace with include_http_calls: true
```

### 3. Error Debugging
```
You: "Something went wrong with execution abc-123"

Claude: "Let me investigate that execution for you."
→ Uses: ratchet.get_execution_logs with execution_id

Claude: "I see there was a network timeout. Let me get the full trace to understand the timing."
→ Uses: ratchet.get_execution_trace

Claude: "Based on the trace, here's what happened and how to fix it..."
→ Uses: ratchet.analyze_execution_error with include_suggestions: true
```

### 4. Batch Task Processing
```
You: "Process these three datasets in parallel"

Claude: "I'll set up a batch execution for efficient parallel processing."
→ Uses: ratchet.batch_execute with multiple tasks

Claude: "All tasks are running. Let me monitor their progress."
→ Uses: ratchet.get_execution_status for each task

Claude: "Processing complete! Here's a summary of all results."
```

## Architecture Details

### Core Components

#### 1. Protocol Layer (`ratchet-mcp/src/protocol/`)
- **JSON-RPC 2.0 Implementation**: Full support for request/response and notification patterns
- **MCP Message Types**: Initialize, tools/list, tools/call, resources/list, resources/read
- **Server Capabilities**: Dynamic capability negotiation during initialization
- **Protocol Version**: MCP 1.0.0 compliant

#### 2. Transport Layer (`ratchet-mcp/src/transport/`)
- **Stdio Transport**: ✅ Implemented - For local process communication
- **SSE Transport**: ✅ Implemented - For HTTP-based connections
- **Connection Pooling**: Health monitoring, automatic cleanup, and lifecycle management

#### 3. Security Layer (`ratchet-mcp/src/security/`)
- **Authentication**: Support for None, API Key, JWT, and OAuth2
- **Authorization**: Fine-grained permissions with task pattern matching
- **Rate Limiting**: Configurable limits per operation type
- **Audit Logging**: Complete audit trail for all operations

#### 4. Server Implementation (`ratchet-mcp/src/server/`)
- **Tool Registry**: Extensible system for exposing Ratchet capabilities
- **Request Handler**: Processes MCP protocol messages
- **Task Executor Adapter**: Bridge between MCP and ProcessTaskExecutor

### Thread Safety and Process Isolation

The MCP server uses Ratchet's `ProcessTaskExecutor` for all task executions, ensuring:
- JavaScript tasks run in isolated worker processes
- Full thread safety (Send + Sync) for the MCP server
- No blocking of the main server thread during task execution
- Proper resource isolation between concurrent executions

### Integration with Ratchet

#### Task Execution Flow

1. MCP client sends `tools/call` request with task parameters
2. MCP server validates permissions and rate limits
3. Request is passed to `RatchetMcpAdapter`
4. Adapter looks up task in `TaskRepository`
5. `ProcessTaskExecutor` spawns worker process for execution
6. Results are returned through MCP protocol

#### Database Integration

The MCP server directly integrates with Ratchet's database layer:
- `TaskRepository`: For task discovery and metadata
- `ExecutionRepository`: For execution monitoring
- Direct SQL queries for efficient filtering and pagination

## Best Practices

### 1. Task Organization
- Use clear, descriptive task names
- Organize tasks in logical categories
- Include comprehensive input/output schemas
- Add meaningful descriptions and examples

### 2. Security
- Always use authentication in production
- Implement least-privilege permissions
- Regularly rotate API keys
- Monitor and audit task executions
- Use IP allowlists when possible

### 3. Performance
- Set appropriate timeouts for your use case
- Monitor resource usage and adjust limits
- Use task patterns to restrict heavy operations
- Enable structured logging for better debugging
- Configure worker processes based on workload

### 4. Monitoring
- Set up log rotation for long-running instances
- Monitor execution patterns and failures
- Use execution traces for performance optimization
- Implement health checks for production systems
- Set up alerting for rate limit violations

### 5. Development
- Test MCP integration in development environment first
- Use trace mode for debugging task executions
- Validate task schemas before deployment
- Implement comprehensive error handling
- Document custom tasks for LLM consumption

## Integration with Other Tools

### VS Code Extension (Future)
```json
{
  "ratchet.mcp": {
    "enabled": true,
    "serverPath": "/usr/local/bin/ratchet",
    "configPath": "/path/to/config.yaml",
    "autoStart": true
  }
}
```

### Jupyter Notebook Integration (Future)
```python
import ratchet_mcp

# Connect to local Ratchet MCP server
client = ratchet_mcp.Client("ratchet mcp-serve --config config.yaml")

# Execute task from notebook
result = client.execute_task("data-processor", {"input": data})
```

### Web Application Integration
```javascript
// Connect to Ratchet MCP server via SSE
const ratchetMcp = new RatchetMCPClient({
  baseUrl: 'http://localhost:8090',
  apiKey: 'your-api-key'
});

// Execute task from web app
const result = await ratchetMcp.executeTask('weather-api', {
  location: 'San Francisco'
});
```

## Support Resources

- **Ratchet Documentation:** [../README.md](../README.md)
- **Architecture Overview:** [../ARCHITECTURE.md](../ARCHITECTURE.md)
- **CLI Usage Guide:** [CLI_USAGE.md](CLI_USAGE.md)
- **MCP Protocol Specification:** https://spec.modelcontextprotocol.io/
- **Claude Desktop Documentation:** https://claude.ai/desktop
- **Issue Tracker:** Create issues for bugs or feature requests
- **Community Examples:** Share your configuration and use cases

## What's New

### ✅ Recently Completed
- Full MCP 1.0.0 protocol implementation
- Comprehensive security and authentication system
- 17 built-in tools for task management and monitoring
- SSE transport for web integration
- Advanced rate limiting and audit logging
- Batch execution capabilities

### 🚧 In Progress
- Enhanced error analysis with AI-powered suggestions
- Real-time progress streaming for long-running tasks
- WebSocket transport implementation
- Advanced resource management and quotas

### 📋 Planned
- Integration with more AI platforms
- Enhanced debugging tools
- Task marketplace integration
- Distributed execution support