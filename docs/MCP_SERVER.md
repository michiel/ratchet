# MCP (Model Context Protocol) Server Documentation

## Overview

Ratchet now includes a comprehensive MCP server implementation that allows Language Learning Models (LLMs) to interact with Ratchet's task execution engine through a standardized protocol. The MCP server exposes Ratchet's capabilities as tools that can be invoked by any MCP-compatible client.

## Architecture

### Core Components

#### 1. **Protocol Layer** (`ratchet-mcp/src/protocol/`)
- **JSON-RPC 2.0 Implementation**: Full support for request/response and notification patterns
- **MCP Message Types**: Initialize, tools/list, tools/call, resources/list, resources/read
- **Server Capabilities**: Dynamic capability negotiation during initialization
- **Protocol Versions**: Supports MCP "0.1.0", "2024-11-05", "2025-03-26", "1.0.0" for broad compatibility

#### 2. **Transport Layer** (`ratchet-mcp/src/transport/`)
- **Stdio Transport**: ✅ Implemented - For local process communication
- **SSE Transport**: ✅ Implemented - For HTTP-based connections and web applications
- **Connection Pooling**: Health monitoring, automatic cleanup, and lifecycle management

#### 3. **Security Layer** (`ratchet-mcp/src/security/`)
- **Authentication**: Support for None, API Key, JWT, and OAuth2
- **Authorization**: Fine-grained permissions with task pattern matching
- **Rate Limiting**: Configurable limits per operation type
- **Audit Logging**: Complete audit trail for all operations

#### 4. **Server Implementation** (`ratchet-mcp/src/server/`)
- **Tool Registry**: Extensible system for exposing Ratchet capabilities
- **Request Handler**: Processes MCP protocol messages
- **Task Executor Adapter**: Bridge between MCP and ProcessTaskExecutor

### Thread Safety and Process Isolation

The MCP server uses Ratchet's `ProcessTaskExecutor` for all task executions, ensuring:
- JavaScript tasks run in isolated worker processes
- Full thread safety (Send + Sync) for the MCP server
- No blocking of the main server thread during task execution
- Proper resource isolation between concurrent executions

## Available Tools

### 1. `ratchet.execute_task`
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
      "city": "San Francisco"
    }
  }
}
```

### 2. `ratchet.list_available_tasks`
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

### 3. `ratchet.get_execution_status` (Placeholder)
Get status and progress of a running execution.

**Parameters:**
- `execution_id` (string, required): ID of the execution to check

### 4. `ratchet.get_execution_logs` (Placeholder)
Retrieve logs for a specific execution.

**Parameters:**
- `execution_id` (string, required): ID of the execution
- `level` (string, optional): Minimum log level (trace/debug/info/warn/error)
- `limit` (integer, optional): Maximum number of entries (default: 100)
- `format` (string, optional): Output format (json/text, default: json)

### 5. `ratchet.get_execution_trace` (Placeholder)
Get detailed execution trace with timing and context.

**Parameters:**
- `execution_id` (string, required): ID of the execution
- `include_http_calls` (boolean, optional): Include HTTP traces (default: true)
- `format` (string, optional): Output format (json/flamegraph, default: json)

### 6. `ratchet.analyze_execution_error` (Placeholder)
Analyze failed executions with fix suggestions.

**Parameters:**
- `execution_id` (string, required): ID of the failed execution
- `include_suggestions` (boolean, optional): Include fix suggestions (default: true)
- `include_context` (boolean, optional): Include execution context (default: true)

## Security Model

### Authentication Methods

1. **None**: No authentication (development only)
2. **API Key**: Static or dynamic API key validation
3. **JWT**: JSON Web Token with configurable validation
4. **OAuth2**: Full OAuth2 flow support

### Permission System

```rust
pub struct ClientPermissions {
    pub can_execute_tasks: bool,
    pub can_read_logs: bool,
    pub can_read_traces: bool,
    pub allowed_task_patterns: Vec<String>, // e.g., ["safe-*", "read-only-*"]
    pub rate_limits: RateLimits,
    pub resource_quotas: ResourceQuotas,
}
```

### Rate Limiting

Configurable limits for:
- Task executions per minute
- Log retrievals per minute
- Trace requests per minute
- Total requests per minute
- Maximum concurrent executions

## Integration with Ratchet

### Task Execution Flow

1. MCP client sends `tools/call` request with task parameters
2. MCP server validates permissions and rate limits
3. Request is passed to `RatchetMcpAdapter`
4. Adapter looks up task in `TaskRepository`
5. `ProcessTaskExecutor` spawns worker process for execution
6. Results are returned through MCP protocol

### Database Integration

The MCP server directly integrates with Ratchet's database layer:
- `TaskRepository`: For task discovery and metadata
- `ExecutionRepository`: For execution monitoring (future)
- Direct SQL queries for efficient filtering and pagination

## Configuration

### Server Configuration

```yaml
mcp:
  server:
    enabled: true
    transport: "stdio"  # or "sse"
    
    # For SSE transport
    bind_address: "0.0.0.0:8090"
    
    auth:
      type: "api_key"
      api_keys:
        - key: "${MCP_API_KEY}"
          name: "llm-client"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            allowed_task_patterns: ["safe-*", "api-*"]
    
    security:
      max_execution_time: 300
      max_log_entries: 1000
      audit_log_enabled: true
      input_sanitization: true
      
    performance:
      max_concurrent_executions_per_client: 5
      connection_pool_size: 20
```

## Usage Examples

### Starting the MCP Server

```bash
# Start with stdio transport (for local LLM integration)
ratchet mcp-serve --transport stdio

# Start with SSE transport (for network access)
ratchet mcp-serve --transport sse --port 8090
```

### Client Integration

```python
# Example using an MCP client library
import mcp_client

# Connect to Ratchet MCP server
client = mcp_client.StdioClient("ratchet mcp-serve")
await client.initialize()

# List available tasks
tasks = await client.call_tool("ratchet.list_available_tasks", {
    "include_schemas": True
})

# Execute a task
result = await client.call_tool("ratchet.execute_task", {
    "task_id": "weather-api",
    "input": {"city": "San Francisco"}
})
```

## Current Status

### ✅ Completed
- Full MCP protocol implementation
- JSON-RPC 2.0 message handling
- Stdio transport
- Security and authentication system
- Tool registry with 6 built-in tools
- Integration with ProcessTaskExecutor
- Task discovery via database
- Connection pooling infrastructure

### 🚧 In Progress
- SSE transport implementation
- Execution monitoring tools
- Log retrieval implementation
- Trace analysis tools

### 📋 Planned
- WebSocket transport
- Streaming responses for long-running tasks
- Real-time progress updates
- Enhanced error analysis with AI suggestions
- Resource management and quotas

## Development

### Adding New Tools

1. Define the tool in `RatchetToolRegistry::register_builtin_tools()`
2. Implement the execution logic as a method on `RatchetToolRegistry`
3. Update the tool execution match statement
4. Add documentation for the new tool

### Testing

```bash
# Run MCP-specific tests
cargo test -p ratchet-mcp

# Test with a mock client
ratchet mcp-serve --config test-config.yaml
```

## Troubleshooting

### Common Issues

1. **"Task executor not configured"**: Ensure the MCP server is properly initialized with database connections
2. **Authentication failures**: Check API key configuration and permissions
3. **Rate limit exceeded**: Adjust rate limits in configuration or implement backoff
4. **Task not found**: Verify task is registered and enabled in the database

### Debug Logging

Enable detailed logging:
```yaml
logging:
  level: debug
  targets:
    ratchet_mcp: trace
```