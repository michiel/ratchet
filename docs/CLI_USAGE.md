# Ratchet CLI Usage Guide

Ratchet provides a comprehensive command-line interface for task execution, server management, and configuration:

## Quick Start (5 Minutes)

### Get Started with Ratchet

```bash
# 1. Start the server (uses in-memory database for development)
ratchet serve

# 2. Open GraphQL playground in your browser
# Navigate to: http://localhost:8080/playground

# 3. Run a sample query to list available tasks
query ListTasks {
  registryTasks {
    tasks {
      id
      label
      description
    }
  }
}

# 4. Execute a task (example with addition task)
mutation ExecuteTask {
  executeTask(input: {
    taskId: 1,
    inputData: "{\"num1\": 5, \"num2\": 10}"
  }) {
    output
    executionTimeMs
  }
}

# 5. Check server health
curl http://localhost:8080/health
```

### Quick MCP Setup for Claude Desktop

```bash
# 1. Start MCP server
ratchet mcp-serve

# 2. Add to Claude Desktop config (~/.config/claude-desktop/config.json):
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve"]
    }
  }
}

# 3. Restart Claude Desktop - you now have access to 19 Ratchet tools!
```

### Quick Task Testing

```bash
# Test a task directly without a server
ratchet run-once --from-fs ./sample/js-tasks/addition --input-json '{"num1":15,"num2":25}'

# Validate a task structure  
ratchet validate --from-fs ./my-task

# Run task test suite
ratchet test --from-fs ./my-task
```

## Command Overview

- **`ratchet serve`** - Full HTTP/GraphQL API server for web applications
- **`ratchet mcp-serve`** - MCP (Model Context Protocol) server for LLM integration  
- **`ratchet run-once`** - Execute single tasks without a server
- **`ratchet validate`** - Validate task structure and schemas
- **`ratchet test`** - Run task test suites
- **`ratchet replay`** - Replay recorded task executions
- **`ratchet generate`** - Generate code templates
- **`ratchet config`** - Configuration management utilities

## Server Commands

### Regular Server (`ratchet serve`)
Full-featured server with HTTP REST API, GraphQL API, and job queue management.

### MCP Server (`ratchet mcp-serve`)
Specialized server for AI/LLM integration using the Model Context Protocol. By default, uses stdio transport for tool integration.

### Task Runner (`ratchet run-once`)
Direct task execution without starting a persistent server.

## Regular Server Usage (`ratchet serve`)

### Basic Usage (with defaults)
```bash
ratchet serve
```

This starts the server with default configuration:
- **Host**: 127.0.0.1
- **Port**: 8080
- **Database**: sqlite::memory: (in-memory database)
- **Workers**: Number of CPU cores

The default in-memory database is automatically initialized with migrations on startup, making it perfect for development and testing.

### With Configuration File
```bash
ratchet serve --config=/path/to/config.yaml
```

### Environment Variables
You can override settings using environment variables:

```bash
export RATCHET_SERVER_HOST=0.0.0.0
export RATCHET_SERVER_PORT=3000
export RATCHET_DATABASE_URL=sqlite:my-ratchet.db
ratchet serve
```

## Available Endpoints

When the server is running, the following endpoints are available:

- **GraphQL API**: `http://localhost:8080/graphql`
- **GraphQL Playground**: `http://localhost:8080/playground`
- **Health Check**: `http://localhost:8080/health`
- **Version Info**: `http://localhost:8080/version`
- **Root**: `http://localhost:8080/`

## Configuration File Format

Create a YAML file with the following structure:

```yaml
# Task execution configuration
execution:
  max_execution_duration: 300  # 5 minutes in seconds
  validate_schemas: true

# HTTP client configuration
http:
  timeout: 30  # seconds
  user_agent: "Ratchet/1.0"

# Server configuration
server:
  bind_address: "127.0.0.1"
  port: 8080
  database:
    url: "sqlite:ratchet.db"  # or "sqlite::memory:" for in-memory
    max_connections: 10
    connection_timeout: 30

# Task registry configuration (optional)
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"  # Load tasks from local directory
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RATCHET_SERVER_HOST` | Server bind address | `127.0.0.1` |
| `RATCHET_SERVER_PORT` | Server port | `8080` |
| `RATCHET_DATABASE_URL` | Database connection URL | `sqlite:ratchet.db` |
| `RATCHET_DATABASE_MAX_CONNECTIONS` | Max database connections | `10` |
| `RATCHET_DATABASE_TIMEOUT` | Database connection timeout (seconds) | `30` |
| `RATCHET_LOG_LEVEL` | Log level (trace, debug, info, warn, error) | `info` |

## Features

### GraphQL API
- Complete GraphQL schema for tasks, jobs, executions, and schedules
- Real-time task execution via GraphQL mutations
- Query execution history and results
- Job queue management

### Task Execution
- Process separation architecture (Send/Sync compliant)
- Multiple worker processes for parallel execution
- JavaScript task execution with Boa engine
- HTTP request recording and playback

### Task Registry
- Automatic task discovery from configured sources
- Support for filesystem sources (directories, ZIP files, collections)
- Version management with duplicate detection
- GraphQL queries for browsing available tasks

### Database Integration
- SQLite database with automatic migrations
- **Default**: In-memory database for development/testing (no persistence)
- **Production**: Set `RATCHET_DATABASE_URL=sqlite:filename.db` for persistent storage
- Complete audit trail of all task executions during session

### Graceful Shutdown
- Responds to SIGTERM and SIGINT (Ctrl+C)
- Gracefully shuts down all worker processes
- Ensures data integrity during shutdown

## Example Session

```bash
# Start server with custom config
ratchet serve --config=my-config.yaml

# Output:
# INFO ratchet: 🚀 Ratchet server starting on http://127.0.0.1:8080
# INFO ratchet: 📊 GraphQL playground available at http://127.0.0.1:8080/playground
# INFO ratchet: 🏥 Health check available at http://127.0.0.1:8080/health

# Server is now running and ready to accept requests
# Press Ctrl+C to shutdown gracefully
```

The server will automatically:
1. Load configuration from file or environment
2. Connect to database and run migrations
3. Start worker processes for task execution
4. Launch the GraphQL API server
5. Load tasks from registry sources (if configured)
6. Provide graceful shutdown on SIGTERM/SIGINT

## GraphQL Examples

### Query Registry Tasks
```graphql
query ListRegistryTasks {
  registryTasks {
    tasks {
      id
      version
      label
      description
      availableVersions
    }
    total
  }
}
```

### Get Specific Task Version
```graphql
query GetTask($id: ID!, $version: String) {
  registryTask(id: $id, version: $version) {
    id
    version
    label
    description
    availableVersions
  }
}
```

### Execute a Task
```graphql
mutation ExecuteTask {
  executeTask(input: {
    taskId: 1,
    inputData: "{\"num1\": 5, \"num2\": 10}"
  }) {
    output
    executionTimeMs
  }
}
```

## MCP Server Usage (`ratchet mcp-serve`)

The MCP (Model Context Protocol) server is designed for AI/LLM integration, particularly with Claude Desktop and other MCP-compatible clients.

### Basic Usage
```bash
# Start MCP server (always uses stdio transport for Claude Desktop)
ratchet mcp-serve

# With custom configuration
ratchet mcp-serve --config config.yaml
```

**Note:** The `mcp-serve` command automatically forces stdio transport mode and uses file-only logging to keep stdin/stdout clean for JSON-RPC communication. Transport arguments are accepted but ignored.

### Command Line Options
| Option | Description | Default |
|--------|-------------|--------|
| `--config PATH` | Configuration file path | None |
| `--transport TYPE` | Transport type (stdio, sse) | `stdio` |
| `--host HOST` | Host to bind (SSE transport) | `127.0.0.1` |
| `--port PORT` | Port to bind (SSE transport) | `8090` |

### Claude Desktop Configuration

To use Ratchet with Claude Desktop, add this to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve"]
    }
  }
}
```

Or with a custom configuration file:

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

### Available MCP Tools

✅ **MCP Integration Verified** - 19 tools available and fully functional:

1. **`ratchet.execute_task`** - Execute a Ratchet task with progress streaming
2. **`ratchet.list_available_tasks`** - Discover available tasks with schemas  
3. **`ratchet.get_execution_status`** - Monitor task execution status
4. **`ratchet.get_execution_logs`** - Retrieve execution logs
5. **`ratchet.get_execution_trace`** - Get detailed execution traces
6. **`ratchet.analyze_execution_error`** - AI-powered error analysis
7. **`ratchet.create_task`** - Create new tasks with code and schemas
8. **`ratchet.edit_task`** - Edit existing task code and metadata
9. **`ratchet.validate_task`** - Validate task code and schemas
10. **`ratchet.run_task_tests`** - Execute test cases for tasks
11. **`ratchet.debug_task_execution`** - Debug with breakpoints and inspection
12. **`ratchet.batch_execute`** - Execute multiple tasks with dependencies
13. **`ratchet.generate_from_template`** - Generate tasks from templates
14. **`ratchet.list_templates`** - List available task templates
15. **`ratchet.import_tasks`** - Import tasks from JSON/other formats
16. **`ratchet.export_tasks`** - Export tasks to JSON/other formats  
17. **`ratchet.create_task_version`** - Version existing tasks
18. **`ratchet.store_result`** - Store task execution results for analysis
19. **`ratchet.get_results`** - Retrieve historical execution results and patterns

### Example MCP Session

```bash
# Start MCP server
ratchet mcp-serve --config config.yaml

# Claude Desktop automatically connects and can now:
# - "What tasks are available?"
# - "Execute the weather task for London"
# - "Show me the logs for the last execution"
```

### MCP Configuration

Add MCP-specific settings to your config file:

```yaml
mcp:
  enabled: true
  transport: stdio
  auth_type: none  # Development
  max_connections: 10
  rate_limit_per_minute: 100
  
  # Production security
  auth_type: api_key
  api_keys:
    - key: "${MCP_API_KEY}"
      name: "claude-desktop"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
```

## Task Runner Usage (`ratchet run-once`)

Execute single tasks without starting a persistent server.

### Basic Usage
```bash
# Execute a task from filesystem
ratchet run-once --from-fs ./sample/js-tasks/addition --input-json '{"num1":5,"num2":10}'

# With recording for debugging
ratchet run-once --from-fs ./my-task --input-json '{"data":"test"}' --record ./debug-output
```

### Command Line Options
| Option | Description |
|--------|-------------|
| `--from-fs PATH` | Path to task directory or ZIP file |
| `--input-json JSON` | JSON input for the task |
| `--record PATH` | Record execution to directory |

### Example Session
```bash
# Execute addition task
ratchet run-once --from-fs ./sample/js-tasks/addition --input-json '{"num1":15,"num2":25}'

# Output:
# Result: {
#   "result": 40,
#   "operation": "addition"
# }
```

## Additional Commands

### Validate Tasks
```bash
# Validate a task structure
ratchet validate --from-fs ./my-task
```

### Test Tasks
```bash
# Run all tests for a task
ratchet test --from-fs ./my-task
```

### Generate Task Templates
```bash
# Generate a new task template
ratchet generate task --path ./new-task --label "My Task" --description "Task description"
```

### Configuration Management

#### Validate Configuration
```bash
# Validate a configuration file
ratchet config validate --config-file ./config.yaml
```

#### Generate Configuration Templates
```bash
# Generate development configuration
ratchet config generate --config-type dev --output ./dev-config.yaml

# Generate production configuration  
ratchet config generate --config-type production --output ./prod-config.yaml

# Available types: dev, production, enterprise, minimal, claude
# Use --force to overwrite existing files
```

#### Show Current Configuration
```bash
# Show full configuration (with defaults applied)
ratchet config show

# Show configuration from specific file
ratchet config show --config-file ./my-config.yaml

# Show only MCP configuration
ratchet config show --mcp-only

# Output in JSON format
ratchet config show --format json
```

### Replay Task Executions
```bash
# Replay a previously recorded execution
ratchet replay --from-fs ./my-task --recording ./debug-output/session_20240115_143022
```

## Configuration Integration

Both `serve` and `mcp-serve` can use the same configuration file with different sections:

```yaml
# config.yaml - Works for both servers

# Shared configuration
database:
  url: "sqlite:ratchet.db"

logging:
  level: info

# Regular server settings (ratchet serve)
server:
  host: "0.0.0.0"
  port: 8080

graphql:
  enabled: true
  playground: true

# MCP server settings (ratchet mcp-serve)  
mcp:
  enabled: true
  transport: stdio
  auth_type: none
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Port Already in Use
**Problem**: `Error: Address already in use (os error 98)`
```bash
# Solution: Check what's using port 8080
sudo lsof -i :8080

# Kill the process or use a different port
ratchet serve --port 8081
# or
RATCHET_SERVER_PORT=8081 ratchet serve
```

#### 2. Database Connection Failures
**Problem**: `Error: Failed to connect to database`
```bash
# Solution: Check database URL and permissions
ratchet config show --format json | grep database

# For SQLite: ensure directory exists and is writable
mkdir -p $(dirname "$RATCHET_DATABASE_URL")

# Test with in-memory database
RATCHET_DATABASE_URL="sqlite::memory:" ratchet serve
```

#### 3. Task Validation Errors
**Problem**: `Validation failed: missing required field`
```bash
# Solution: Validate task structure
ratchet validate --from-fs ./my-task --verbose

# Check schemas against input
ratchet run-once --from-fs ./my-task --input-json '{"test": "data"}' --record ./debug
```

#### 4. Permission Issues
**Problem**: `Permission denied` or access errors
```bash
# Solution: Check file permissions
ls -la ./my-task/
chmod +r ./my-task/*.json
chmod +x ./my-task/

# For system-wide installation
sudo chmod +x $(which ratchet)
```

#### 5. MCP Connection Issues
**Problem**: Claude Desktop doesn't see Ratchet tools
```bash
# Solution: Verify MCP server is running
ratchet mcp-serve --config debug-config.yaml

# Check Claude Desktop logs (varies by OS)
# macOS: ~/Library/Logs/Claude/
# Windows: %APPDATA%/Claude/logs/
# Linux: ~/.config/claude-desktop/logs/

# Test MCP server directly
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"0.1.0","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | ratchet mcp-serve
```

#### 6. JavaScript Runtime Errors
**Problem**: `JavaScript execution failed`
```bash
# Solution: Enable detailed logging
RUST_LOG=debug ratchet run-once --from-fs ./my-task --input-json '{}' --record ./js-debug

# Check for syntax errors
ratchet validate --from-fs ./my-task --syntax-only

# Test with minimal input
ratchet run-once --from-fs ./my-task --input-json '{}' --verbose
```

#### 7. High Memory Usage
**Problem**: Ratchet consuming too much memory
```bash
# Solution: Reduce worker count and connection limits
RATCHET_MAX_WORKERS=2 ratchet serve

# Use file-based database instead of memory
RATCHET_DATABASE_URL="sqlite:ratchet.db" ratchet serve

# Monitor memory usage
watch -n 1 'ps aux | grep ratchet'
```

### Debugging Checklist

When encountering issues:

- [ ] Check `ratchet --version` to verify installation
- [ ] Verify configuration with `ratchet config show`
- [ ] Enable debug logging with `RUST_LOG=debug`
- [ ] Test with minimal/default configuration
- [ ] Check system requirements (disk space, memory, ports)
- [ ] Review recent error logs
- [ ] Test with a known-good task (e.g., `sample/js-tasks/addition`)
- [ ] Verify network connectivity for external dependencies

### Environment Variables for Debugging

```bash
# Enable all debug logging
export RUST_LOG=debug

# Enable trace logging for specific components  
export RUST_LOG=ratchet=trace,ratchet_mcp=debug

# Use in-memory database for testing
export RATCHET_DATABASE_URL="sqlite::memory:"

# Force specific bind address
export RATCHET_SERVER_HOST="0.0.0.0"

# Increase timeouts for slow systems
export RATCHET_DATABASE_TIMEOUT=60
```

## Best Practices

### Development
- Use `ratchet serve` for web development and API testing
- Use `ratchet mcp-serve` for AI assistant integration
- Use `ratchet run-once` for quick task testing

### Production
- Run `ratchet serve` for web applications
- Run `ratchet mcp-serve` separately for AI integrations
- Use systemd or Docker for process management
- Enable authentication and rate limiting

### Debugging
- Enable debug logging: `RUST_LOG=debug ratchet serve`
- Use `--record` with `run-once` for execution analysis
- Monitor logs: `tail -f ratchet.log`

For detailed setup instructions, see:
- [MCP Integration Guide](MCP_INTEGRATION_GUIDE.md)
- [Configuration Guide](CONFIGURATION_GUIDE.md)
- [MCP Server Documentation](MCP_SERVER.md)