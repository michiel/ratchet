# Claude Desktop MCP Integration Setup Guide

This guide explains how to configure Claude Desktop to connect to your Ratchet MCP server.

## Prerequisites

1. Claude Desktop application installed
2. Ratchet MCP server built and ready
3. Sample tasks available in `./sample/js-tasks`

## Configuration Files

### 1. Claude Desktop Configuration

Add the following to your Claude Desktop configuration file:

**Location:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/claude/claude_desktop_config.json`

**Content:**
```json
{
  "mcpServers": {
    "ratchet": {
      "command": "/path/to/ratchet/target/release/ratchet-mcp",
      "args": [
        "-c",
        "/path/to/ratchet/sample/configs/ratchet-mcp-config.yaml",
        "serve",
        "--transport",
        "stdio"
      ],
      "env": {
        "RUST_LOG": "info",
        "RATCHET_ENV": "production"
      }
    }
  }
}
```

### 2. Ratchet MCP Configuration

Use the provided `sample/configs/ratchet-mcp-config.yaml` file. Key settings:

- **Transport**: `stdio` for Claude Desktop integration
- **Authentication**: API key based (update the keys for production use)
- **Database**: SQLite for persistence
- **Task Registry**: Points to local sample tasks

## Setup Steps

1. **Build Ratchet MCP Server**
   ```bash
   cargo build --bin ratchet-mcp --release
   ```

2. **Start Ratchet Backend Server** (required for task execution)
   ```bash
   cargo run --bin ratchet -- --config sample/configs/ratchet-mcp-config.yaml serve
   ```

3. **Test MCP Server Standalone**
   ```bash
   ./target/release/ratchet-mcp -c sample/configs/ratchet-mcp-config.yaml test
   ./target/release/ratchet-mcp -c sample/configs/ratchet-mcp-config.yaml tools
   ```

4. **Configure Claude Desktop**
   - Copy the Claude configuration to the appropriate location
   - Update paths to match your system
   - Restart Claude Desktop

5. **Verify Integration**
   - In Claude Desktop, you should see "ratchet" in the available tools
   - Try commands like:
     - "List available Ratchet tasks"
     - "Execute the addition task with a=5 and b=10"

## Available MCP Tools

Once configured, Claude will have access to these Ratchet tools:

- `ratchet.execute_task` - Execute tasks with input data
- `ratchet.batch_execute` - Execute multiple tasks
- `ratchet.list_available_tasks` - List all available tasks
- `ratchet.get_execution_status` - Check task execution status
- `ratchet.get_execution_logs` - Retrieve execution logs
- `ratchet.get_execution_trace` - Get detailed execution traces
- `ratchet.analyze_execution_error` - Analyze failed executions

## Security Considerations

1. **API Keys**: Generate secure API keys for production use
2. **File Paths**: Use absolute paths in configurations
3. **Permissions**: Restrict task patterns as needed
4. **Rate Limiting**: Adjust limits based on your usage

## Troubleshooting

1. **MCP Server Won't Start**
   - Check that the Ratchet backend server is running
   - Verify database file permissions
   - Check logs in `./logs/ratchet-mcp.log`

2. **Claude Can't Connect**
   - Verify paths in Claude configuration
   - Check that ratchet-mcp binary is executable
   - Look for errors in Claude's developer console

3. **Tasks Not Found**
   - Ensure task files exist in `./sample/js-tasks`
   - Check registry configuration in YAML
   - Verify file permissions

## Example Usage in Claude

Once configured, you can interact with Ratchet naturally:

```
User: "Can you list the available Ratchet tasks?"
Claude: [Uses ratchet.list_available_tasks tool]

User: "Execute the weather API task for London"
Claude: [Uses ratchet.execute_task with appropriate parameters]

User: "Show me the logs from the last execution"
Claude: [Uses ratchet.get_execution_logs tool]
```

## Advanced Configuration

For production deployments, consider:

1. Using PostgreSQL instead of SQLite
2. Implementing proper API key rotation
3. Setting up monitoring and alerting
4. Configuring SSL/TLS for remote connections
5. Using environment variables for sensitive data