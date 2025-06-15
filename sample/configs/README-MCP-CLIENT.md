# Claude Code MCP Client for Running Ratchet Servers

This configuration allows Claude Desktop to connect to an **already running** Ratchet MCP server using Server-Sent Events (SSE) transport, instead of spawning a new server process.

## Prerequisites

1. **Node.js 18+** installed
2. **Running Ratchet MCP server** with SSE transport
3. **Claude Desktop** application

## Setup Instructions

### 1. Install Dependencies

```bash
cd /home/michiel/dev/ratchet/sample/configs
npm install
```

### 2. Start Ratchet MCP Server (SSE Mode)

Start your Ratchet MCP server with SSE transport:

```bash
# Option 1: Use the general MCP command (SSE default)
ratchet mcp --host 127.0.0.1 --port 8090

# Option 2: Use mcp-serve with explicit SSE transport
ratchet mcp-serve --transport sse --host 127.0.0.1 --port 8090

# Option 3: With configuration file
ratchet mcp --config /path/to/config.yaml
```

### 3. Configure Claude Desktop

Copy the configuration to Claude Desktop's config location:

**macOS:**
```bash
cp claude-desktop-mcp-client.json ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

**Windows:**
```cmd
copy claude-desktop-mcp-client.json %APPDATA%\Claude\claude_desktop_config.json
```

**Linux:**
```bash
cp claude-desktop-mcp-client.json ~/.config/Claude/claude_desktop_config.json
```

### 4. Update File Paths

Edit the copied configuration file and update the absolute path to the script:

```json
{
  "mcpServers": {
    "ratchet-sse": {
      "command": "node",
      "args": [
        "/absolute/path/to/your/ratchet/sample/configs/mcp-sse-client.js"
      ],
      "env": {
        "RATCHET_SSE_URL": "http://localhost:8090",
        "RATCHET_TIMEOUT": "30000",
        "RATCHET_AUTH_TOKEN": ""
      }
    }
  }
}
```

### 5. Restart Claude Desktop

Close and restart Claude Desktop to load the new configuration.

## Configuration Options

You can customize the connection by modifying the environment variables:

- **`RATCHET_SSE_URL`**: URL of the running Ratchet MCP server (default: `http://localhost:8090`)
- **`RATCHET_TIMEOUT`**: Request timeout in milliseconds (default: `30000`)
- **`RATCHET_AUTH_TOKEN`**: Optional Bearer token for authentication

## Testing the Connection

1. Open Claude Desktop
2. Start a new conversation
3. Ask: "What Ratchet MCP tools are available?"
4. You should see a list of available Ratchet tools

## Example: Connecting to Remote Server

To connect to a remote Ratchet MCP server with authentication:

```json
{
  "mcpServers": {
    "ratchet-production": {
      "command": "node",
      "args": [
        "/path/to/mcp-sse-client.js"
      ],
      "env": {
        "RATCHET_SSE_URL": "https://ratchet.your-company.com:8443",
        "RATCHET_TIMEOUT": "60000",
        "RATCHET_AUTH_TOKEN": "your-bearer-token-here"
      }
    }
  }
}
```

## Architecture

```
┌─────────────────┐    stdio    ┌─────────────────┐    SSE/HTTP    ┌─────────────────┐
│  Claude Desktop │◄───────────►│ mcp-sse-client  │◄──────────────►│ Ratchet MCP     │
│                 │             │   (Node.js)     │                │ Server (SSE)    │
└─────────────────┘             └─────────────────┘                └─────────────────┘
```

The `mcp-sse-client.js` script acts as a bridge:
- Receives JSON-RPC messages from Claude Desktop via stdio
- Forwards them to the Ratchet MCP server via HTTP POST
- Receives responses from Ratchet server via Server-Sent Events
- Sends responses back to Claude Desktop via stdout

## Advantages Over Spawning New Servers

1. **Resource Efficiency**: No need to spawn new server processes
2. **Shared State**: Multiple clients can connect to the same server instance
3. **Centralized Management**: Single server can be monitored and managed
4. **Network Access**: Can connect to remote Ratchet servers
5. **Authentication**: Supports Bearer token authentication
6. **Scalability**: Server can handle multiple concurrent connections

## Troubleshooting

### Connection Issues

Check that the Ratchet MCP server is running and accessible:

```bash
curl -v http://localhost:8090/health
```

### Authentication Issues

If using authentication, verify your token:

```bash
curl -H "Authorization: Bearer your-token" http://localhost:8090/health
```

### Debug Logging

The client logs to stderr, which you can see in Claude Desktop's logs or by running the script directly:

```bash
echo '{"jsonrpc":"2.0","id":"1","method":"tools/list"}' | node mcp-sse-client.js
```

### Claude Desktop Not Connecting

1. Verify the configuration file is in the correct location
2. Check that all file paths are absolute paths
3. Ensure Node.js and dependencies are installed
4. Restart Claude Desktop after configuration changes

## Security Considerations

- Use HTTPS URLs for production deployments
- Store authentication tokens securely (consider environment variables)
- Implement proper access controls on the Ratchet MCP server
- Monitor connections and implement rate limiting if needed