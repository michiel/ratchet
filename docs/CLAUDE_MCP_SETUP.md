# Claude MCP Integration Setup Guide

This guide explains how to configure Ratchet's MCP server for use with Claude Desktop and other MCP-compatible clients.

## Overview

Ratchet provides two server modes for different use cases:

1. **Regular Server** (`ratchet serve`) - Full HTTP/GraphQL API server for web applications
2. **MCP Server** (`ratchet mcp-serve`) - Specialized MCP (Model Context Protocol) server for LLM integration

## Quick Setup for Claude Desktop

### 1. Installation & Configuration

Ensure Ratchet is installed and you have a basic configuration file:

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
```

### 2. Configure Claude Desktop

Add Ratchet to your Claude Desktop MCP configuration:

**Location:** 
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

**Configuration:**
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

## Available Claude Commands

Once configured, you can ask Claude to:

### Task Discovery
- "What tasks are available in Ratchet?"
- "Show me all API-related tasks"
- "List tasks with their input schemas"

### Task Execution
- "Run the weather task for London"
- "Execute addition task with numbers 5 and 10"
- "Process data using the csv-parser task"

### Monitoring & Debugging
- "Show me the logs for execution abc-123"
- "What went wrong with the failed task?"
- "Get the execution trace for performance analysis"

## Advanced Configuration

### Production Security Setup

For production environments, enable authentication:

```yaml
mcp:
  enabled: true
  transport: stdio
  auth_type: api_key
  api_keys:
    - key: "${MCP_API_KEY_CLAUDE}"
      name: "claude-desktop"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
        allowed_task_patterns: ["safe-*", "read-only-*"]
        rate_limits:
          executions_per_minute: 30
          logs_per_minute: 100
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

You can configure multiple Ratchet instances:

```json
{
  "mcpServers": {
    "ratchet-dev": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/dev-config.yaml"]
    },
    "ratchet-prod": {
      "command": "ratchet",
      "args": ["mcp-serve", "--config", "/path/to/prod-config.yaml"]
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
    - type: file
      path: ratchet-mcp.log
      level: debug
      buffered:
        size: 1000
        flush_interval: 5s
  
# Enable MCP-specific logging
mcp:
  log_execution_traces: true
  enable_structured_logs: true
```

## Troubleshooting

### Common Issues

#### Claude Can't Connect
**Symptoms:** Claude shows "Server connection failed" or doesn't list Ratchet tools

**Solutions:**
1. Check the ratchet binary path:
   ```bash
   which ratchet
   # Use the full path in Claude config
   ```

2. Verify configuration file exists and is readable:
   ```bash
   ratchet mcp-serve --config /path/to/config.yaml --help
   ```

3. Test MCP server directly:
   ```bash
   ratchet mcp-serve --transport stdio --config /path/to/config.yaml
   # Should start without errors
   ```

#### "No tasks available"
**Symptoms:** `ratchet.list_available_tasks` returns empty list

**Solutions:**
1. Ensure database is initialized:
   ```bash
   ratchet serve --config /path/to/config.yaml
   # Start regular server first to initialize DB
   ```

2. Load sample tasks:
   ```bash
   # Copy sample tasks to your tasks directory
   cp -r sample/js-tasks /path/to/your/tasks
   ```

3. Check task registry:
   ```bash
   # Verify tasks are loaded
   curl http://localhost:8080/api/v1/tasks
   ```

#### Permission Denied Errors
**Symptoms:** Claude gets "Permission denied" when executing tasks

**Solutions:**
1. Check API key configuration (if using auth)
2. Verify task patterns in allowed_task_patterns
3. Review rate limits configuration

#### Slow Performance
**Symptoms:** Task execution takes too long or times out

**Solutions:**
1. Increase timeout values:
   ```yaml
   mcp:
     request_timeout: 60
     max_execution_time: 300
   ```

2. Enable performance logging:
   ```yaml
   mcp:
     log_execution_traces: true
   ```

3. Monitor resource usage:
   ```bash
   # Check system resources
   htop
   # Check log file size
   ls -lh ratchet.log
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
```

### 2. API Integration Testing
```
You: "Test our weather API integration"

Claude: "I'll test the weather API. Let me execute the weather-api task."
→ Uses: ratchet.execute_task with weather-api task

Claude: "The API call succeeded. Here are the results... Let me also check the execution logs for any warnings."
→ Uses: ratchet.get_execution_logs for detailed analysis
```

### 3. Error Debugging
```
You: "Something went wrong with execution abc-123"

Claude: "Let me investigate that execution for you."
→ Uses: ratchet.get_execution_logs with execution_id

Claude: "I see there was a network timeout. Let me get the full trace to understand the timing."
→ Uses: ratchet.get_execution_trace

Claude: "Based on the trace, here's what happened and how to fix it..."
→ Uses: ratchet.analyze_execution_error
```

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

### 3. Performance
- Set appropriate timeouts for your use case
- Monitor resource usage and adjust limits
- Use task patterns to restrict heavy operations
- Enable structured logging for better debugging

### 4. Monitoring
- Set up log rotation for long-running instances
- Monitor execution patterns and failures
- Use execution traces for performance optimization
- Implement health checks for production systems

## Support Resources

- **Ratchet Documentation:** [../README.md](../README.md)
- **MCP Protocol Specification:** https://spec.modelcontextprotocol.io/
- **Claude Desktop Documentation:** https://claude.ai/desktop
- **Issue Tracker:** Create issues for bugs or feature requests
- **Community Examples:** Share your configuration and use cases