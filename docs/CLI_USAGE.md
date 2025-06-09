# Ratchet CLI Usage Guide

Ratchet provides a comprehensive command-line interface for task execution, server management, and configuration:

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

When connected, LLMs can use these tools:

1. **`ratchet.execute_task`** - Execute a Ratchet task
2. **`ratchet.list_available_tasks`** - Discover available tasks
3. **`ratchet.get_execution_status`** - Monitor task execution
4. **`ratchet.get_execution_logs`** - Retrieve execution logs
5. **`ratchet.get_execution_trace`** - Get detailed traces
6. **`ratchet.analyze_execution_error`** - AI-powered error analysis

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
- [Claude MCP Setup Guide](CLAUDE_MCP_SETUP.md)
- [Server Configuration Guide](SERVER_CONFIGURATION_GUIDE.md)
- [MCP Server Documentation](MCP_SERVER.md)