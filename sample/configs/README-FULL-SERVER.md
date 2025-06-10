# Ratchet Full Server Configuration

This configuration sets up a complete Ratchet server with all features enabled: HTTP APIs (REST/GraphQL), MCP SSE server, file logging, and local task directory management.

## Features Enabled

### 🌐 **HTTP Server (Port 8080)**
- **REST API**: Full REST API for task management
- **GraphQL API**: GraphQL endpoint for complex queries
- **Web UI**: Built-in web interface (if available)

### 🔌 **MCP SSE Server (Port 8090)**
- **Server-Sent Events**: Real-time MCP communication
- **LLM Integration**: Direct integration with Claude Desktop
- **Bidirectional**: Full request-response support

### 📝 **File Logging**
- **Structured Logs**: JSON-formatted logs to `/tmp/ratchet/logs/ratchet.log`
- **Rich Context**: Includes execution metadata, task info, and debugging data
- **Security**: Filters sensitive information (passwords, tokens, etc.)

### 📁 **Local Task Directory**
- **Watch Mode**: Automatically detects new tasks in `/tmp/ratchet/tasks/`
- **Auto-reload**: Updates task registry when files change
- **Sample Tasks**: Includes project sample tasks

### 💾 **Persistent Storage**
- **SQLite Database**: Stores execution history, jobs, and schedules
- **Output Files**: Execution results saved to `/tmp/ratchet/outputs/`

## Quick Start

### 1. Start the Server

```bash
cd /home/michiel/dev/ratchet/sample/configs
./setup-ratchet-server.sh
```

This script will:
- Create necessary directories
- Set up a sample "Hello World" task
- Start the Ratchet server with full configuration using `ratchet serve`

### 2. Configure Claude Desktop

Copy the URL-based configuration:

**macOS:**
```bash
cp claude-desktop-http-client.json ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

**Windows:**
```cmd
copy claude-desktop-http-client.json %APPDATA%\Claude\claude_desktop_config.json
```

**Linux:**
```bash
mkdir -p ~/.config/Claude
cp claude-desktop-http-client.json ~/.config/Claude/claude_desktop_config.json
```

### 3. Restart Claude Desktop

Close and restart Claude Desktop to load the new configuration.

### 4. Test the Setup

1. **Test HTTP API:**
   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8080/api/v1/tasks
   ```

2. **Test GraphQL:**
   ```bash
   curl -X POST http://localhost:8080/graphql \
     -H "Content-Type: application/json" \
     -d '{"query": "{ tasks { id name version } }"}'
   ```

3. **Test MCP Connection:**
   ```bash
   curl http://localhost:8090/health
   ```

4. **Test Claude Integration:**
   - Open Claude Desktop
   - Ask: "What Ratchet tasks are available?"
   - Try: "Execute the hello-world task with name 'Claude'"

## Directory Structure

After running the setup script:

```
/tmp/ratchet/
├── logs/
│   └── ratchet.log              # Server logs
├── outputs/
│   └── {date}/                  # Task execution outputs
│       └── {task}-{id}.json
├── tasks/                       # Local task directory (watched)
│   └── hello-world/             # Sample task
│       ├── metadata.json
│       ├── input.schema.json
│       ├── output.schema.json
│       ├── main.js
│       └── tests/
│           └── test-001.json
├── data/                        # Database and cache
│   └── ratchet.db              # SQLite database
```

## Configuration Details

### Server Endpoints

| Service | URL | Description |
|---------|-----|-------------|
| REST API | `http://localhost:8080/api/v1/` | RESTful API endpoints |
| GraphQL | `http://localhost:8080/graphql` | GraphQL query endpoint |
| Health Check | `http://localhost:8080/health` | Server health status |
| MCP SSE | `http://localhost:8090/sse/{session}` | MCP SSE endpoint |
| MCP Messages | `http://localhost:8090/message/{session}` | MCP message endpoint |
| MCP Health | `http://localhost:8090/health` | MCP server health |

### Claude Desktop Configuration Options

The `claude-desktop-http-client.json` supports these environment variables:

- **`MCP_SERVER_URL`**: Ratchet MCP server URL (default: `http://localhost:8090`)
- **`MCP_AUTH_TOKEN`**: Optional Bearer token for authentication
- **`MCP_TIMEOUT`**: Request timeout in milliseconds (default: `30000`)

### Remote Server Configuration

To connect to a remote Ratchet server:

```json
{
  "mcpServers": {
    "ratchet-remote": {
      "command": "npx",
      "args": ["--yes", "--package=eventsource@^2.0.2", "node", "-e", "..."],
      "env": {
        "MCP_SERVER_URL": "https://ratchet.your-domain.com:8090",
        "MCP_AUTH_TOKEN": "your-bearer-token-here",
        "MCP_TIMEOUT": "60000"
      }
    }
  }
}
```

## Adding Custom Tasks

### 1. Create Task Directory

```bash
mkdir -p /tmp/ratchet/tasks/my-custom-task
```

### 2. Add Task Files

Create the required files:
- `metadata.json` - Task metadata
- `input.schema.json` - Input validation schema
- `output.schema.json` - Output validation schema  
- `main.js` - Task implementation
- `tests/` - Test cases (optional)

### 3. Auto-Detection

The task will be automatically detected and loaded due to the watch configuration:

```yaml
registry:
  sources:
    - name: "local-tasks"
      uri: "file:///tmp/ratchet/tasks"
      config:
        watch_for_changes: true
        auto_reload: true
```

## Monitoring and Debugging

### View Logs

```bash
# Follow live logs
tail -f /tmp/ratchet/logs/ratchet.log

# View structured logs with jq
tail -f /tmp/ratchet/logs/ratchet.log | jq .
```

### Check Task Outputs

```bash
# List execution outputs
ls -la /tmp/ratchet/outputs/

# View specific execution result
cat /tmp/ratchet/outputs/2024-01-15/hello-world-uuid.json
```

### API Health Checks

```bash
# Check main server
curl -s http://localhost:8080/health | jq .

# Check MCP server  
curl -s http://localhost:8090/health | jq .

# List available tasks
curl -s http://localhost:8080/api/v1/tasks | jq .
```

### Database Inspection

```bash
# Install sqlite3 if needed
sudo apt install sqlite3  # Ubuntu/Debian
brew install sqlite3      # macOS

# Inspect database
sqlite3 /tmp/ratchet/data/ratchet.db ".tables"
sqlite3 /tmp/ratchet/data/ratchet.db "SELECT * FROM tasks LIMIT 5;"
```

## Production Considerations

### Security

1. **Change bind addresses** from `0.0.0.0` to specific interfaces
2. **Enable authentication** for MCP endpoints
3. **Use HTTPS/TLS** for production deployments
4. **Implement rate limiting** for public APIs
5. **Secure file permissions** for log and data directories

### Performance

1. **Use PostgreSQL** instead of SQLite for high load
2. **Configure resource limits** in execution config
3. **Monitor disk usage** for logs and outputs
4. **Set up log rotation** for long-running servers

### High Availability

1. **Use external database** (PostgreSQL/MySQL)
2. **Implement load balancing** for multiple instances
3. **Set up monitoring** and alerting
4. **Configure backup strategies** for database and task data

## Troubleshooting

### Common Issues

**1. Server Won't Start**
```bash
# Check if ports are available
netstat -tulpn | grep -E ':8080|:8090'

# Check Ratchet binary
which ratchet
ratchet --version
```

**2. Claude Desktop Connection Issues**
```bash
# Test MCP endpoint directly
curl -v http://localhost:8090/health

# Check Node.js and dependencies
node --version
npx --yes --package=eventsource@^2.0.2 node -e "console.log('OK')"
```

**3. Task Not Loading**
```bash
# Check task directory
ls -la /tmp/ratchet/tasks/

# Validate task files
ratchet validate --from-fs /tmp/ratchet/tasks/my-task
```

**4. Log Analysis**
```bash
# Check for errors in logs
grep -i error /tmp/ratchet/logs/ratchet.log

# Check MCP-specific logs
grep "mcp" /tmp/ratchet/logs/ratchet.log
```

This configuration provides a complete, production-ready Ratchet server setup with full observability, API access, and LLM integration capabilities!