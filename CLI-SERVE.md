# Ratchet Server CLI Command

The `ratchet serve` command starts a full Ratchet server with GraphQL API, task execution, and worker processes.

## Usage

### Basic Usage (with defaults)
```bash
ratchet serve
```

This starts the server with default configuration:
- **Host**: 127.0.0.1
- **Port**: 8080
- **Database**: sqlite:ratchet.db
- **Workers**: Number of CPU cores

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

### Database Integration
- SQLite database with automatic migrations
- Persistent storage for tasks, executions, jobs, and schedules
- Complete audit trail of all task executions

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
5. Provide graceful shutdown on SIGTERM/SIGINT