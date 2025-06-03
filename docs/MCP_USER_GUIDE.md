# MCP (Model Context Protocol) User Guide

## Overview

Ratchet's MCP server enables LLMs to interact directly with your Ratchet task execution environment through a standardized protocol. This allows AI models to discover, execute, and monitor tasks while providing detailed logging and error analysis.

## Quick Start

### 1. Configuration

Add MCP configuration to your `config.yaml`:

```yaml
mcp:
  enabled: true
  transport: stdio
  host: localhost
  port: 3001
  auth_type: none
  max_connections: 10
  request_timeout: 30
  rate_limit_per_minute: 100

# Optional: Configure logging for better MCP integration
logging:
  level: info
  sinks:
    - type: file
      path: /var/log/ratchet.log
      level: debug
```

### 2. Start the MCP Server

```bash
# Using configuration file
ratchet mcp-serve --config config.yaml

# Or with CLI arguments
ratchet mcp-serve --transport stdio --port 3001
```

### 3. Connect Your LLM

The MCP server supports standard MCP clients. For example with Claude Desktop:

```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/config.yaml"]
    }
  }
}
```

## Available Tools

### Task Execution

#### `ratchet.execute_task`
Execute a Ratchet task with specified input.

**Parameters:**
- `task_id` (string, required): Task name or UUID
- `input` (object, required): Input data for the task
- `trace` (boolean, optional): Enable detailed tracing (default: true)
- `timeout` (integer, optional): Execution timeout in seconds

**Example:**
```json
{
  "task_id": "weather-api",
  "input": {
    "location": "San Francisco",
    "units": "metric"
  },
  "trace": true
}
```

#### `ratchet.list_available_tasks`
Discover available tasks with their schemas.

**Parameters:**
- `filter` (string, optional): Filter tasks by name pattern
- `include_schemas` (boolean, optional): Include input/output schemas
- `category` (string, optional): Filter by task category

**Example:**
```json
{
  "filter": "weather",
  "include_schemas": true
}
```

### Monitoring & Debugging

#### `ratchet.get_execution_status`
Get real-time status of a running execution.

**Parameters:**
- `execution_id` (string, required): Execution UUID

#### `ratchet.get_execution_logs`
Retrieve detailed logs for an execution.

**Parameters:**
- `execution_id` (string, required): Execution UUID
- `level` (string, optional): Minimum log level (trace, debug, info, warn, error)
- `limit` (integer, optional): Maximum log entries (default: 100)
- `format` (string, optional): Output format (json, text)

#### `ratchet.get_execution_trace`
Get detailed execution trace with timing information.

**Parameters:**
- `execution_id` (string, required): Execution UUID
- `include_http_calls` (boolean, optional): Include HTTP request traces
- `format` (string, optional): Output format (json, flamegraph)

#### `ratchet.analyze_execution_error`
Analyze failed executions with suggested fixes.

**Parameters:**
- `execution_id` (string, required): Failed execution UUID
- `include_suggestions` (boolean, optional): Include fix suggestions
- `include_context` (boolean, optional): Include execution context

## Transport Options

### STDIO Transport (Recommended)
Direct process communication - ideal for desktop AI applications.

```yaml
mcp:
  transport: stdio
```

### SSE Transport (Future)
HTTP-based Server-Sent Events for web applications.

```yaml
mcp:
  transport: sse
  host: localhost
  port: 3001
```

## Authentication & Security

### No Authentication (Development)
```yaml
mcp:
  auth_type: none
```

### API Key Authentication (Future)
```yaml
mcp:
  auth_type: api_key
  api_key: your-secret-key
```

## Example Workflows

### 1. Task Discovery and Execution
```
LLM: What tasks are available?
→ Tool: ratchet.list_available_tasks

LLM: Execute the weather task for London
→ Tool: ratchet.execute_task
  {
    "task_id": "weather-api",
    "input": {"location": "London"}
  }
```

### 2. Error Investigation
```
LLM: Task failed, let me investigate
→ Tool: ratchet.get_execution_logs
  {"execution_id": "abc-123", "level": "error"}

→ Tool: ratchet.analyze_execution_error
  {"execution_id": "abc-123", "include_suggestions": true}
```

### 3. Performance Analysis
```
LLM: How long did this task take?
→ Tool: ratchet.get_execution_trace
  {"execution_id": "def-456", "include_http_calls": true}
```

## Best Practices

### 1. Enable Detailed Logging
Configure file logging to capture execution details:

```yaml
logging:
  level: debug
  sinks:
    - type: file
      path: /var/log/ratchet.log
      level: debug
      buffered:
        size: 1000
        flush_interval: 5s
```

### 2. Use Tracing
Always enable tracing for better debugging:

```json
{
  "task_id": "my-task",
  "input": {...},
  "trace": true
}
```

### 3. Handle Rate Limits
Configure appropriate rate limits for your use case:

```yaml
mcp:
  rate_limit_per_minute: 60
  max_connections: 5
```

### 4. Monitor Resource Usage
Use execution traces to identify performance bottlenecks:

```json
{
  "execution_id": "xyz-789",
  "include_http_calls": true,
  "format": "json"
}
```

## Troubleshooting

### Common Issues

#### "Task executor not configured"
- Ensure your Ratchet instance has tasks loaded
- Check database connectivity
- Verify task registry is populated

#### "Execution not found"
- Verify the execution ID is correct
- Check if execution was cleaned up (older executions may be purged)

#### "Permission denied"
- Check authentication configuration
- Verify client permissions if using auth

#### Connection Issues
- Verify MCP server is running: `ps aux | grep ratchet`
- Check logs: `tail -f /var/log/ratchet.log`
- Test connectivity: `curl http://localhost:3001/health` (for SSE transport)

### Debug Mode

Enable debug logging for detailed MCP communication:

```yaml
logging:
  level: debug
```

### Log Analysis

Use log analysis tools to understand execution patterns:

```json
{
  "execution_id": "failed-exec-123",
  "level": "error",
  "limit": 50
}
```

## Integration Examples

### Claude Desktop
```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/config.yaml"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### VS Code Extension (Future)
```json
{
  "ratchet.mcp": {
    "serverPath": "/usr/local/bin/ratchet",
    "configPath": "/path/to/config.yaml"
  }
}
```

## Advanced Configuration

### Custom Tool Categories
Organize tools by adding metadata:

```yaml
mcp:
  tool_categories:
    - execution
    - monitoring
    - debugging
    - analytics
```

### Performance Tuning
```yaml
mcp:
  max_connections: 20
  request_timeout: 60
  connection_pool_size: 10
  enable_compression: true
```

### Logging Integration
```yaml
mcp:
  log_file_path: /var/log/ratchet.log
  enable_structured_logs: true
  log_execution_traces: true
```

## API Reference

See the [MCP Protocol Specification](https://spec.modelcontextprotocol.io/) for detailed protocol information.

## Support

- [Ratchet Documentation](../README.md)
- [Architecture Overview](ARCHITECTURE.md)
- [Logging Guide](LOGGING_OVERVIEW.md)
- [Issue Tracker](https://github.com/ratchet/ratchet/issues)