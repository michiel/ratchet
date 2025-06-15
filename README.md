# Ratchet 🚀

**Production-Ready JavaScript Task Execution Platform with Interactive Administration**

Ratchet is a high-performance, scalable task execution platform that runs JavaScript code with enterprise-grade reliability. Built with Rust for performance and safety, it provides comprehensive APIs, persistent storage, advanced execution capabilities, and a powerful interactive console for administration. Features include real HTTP fetching, Model Context Protocol (MCP) server for LLM integration, complete TLS support with rustls, and an intuitive command-line interface with tab completion and variable expansion.

[![Tests](https://img.shields.io/badge/tests-486%20passing-brightgreen)](.) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Status](https://img.shields.io/badge/status-production--ready-green)]()

## 🎯 Key Features

Ratchet is a comprehensive JavaScript task execution platform built with Rust for performance and reliability. At its core, it provides secure, isolated JavaScript execution with schema validation, process separation for thread safety, and configurable resource management. The platform includes both GraphQL and REST APIs with OpenAPI documentation, making it ideal for web applications and microservices architectures.

The system features a robust job queue with priority handling, retry logic with exponential backoff, worker pools that scale with your hardware, and comprehensive monitoring capabilities. For development and operations, Ratchet offers a powerful interactive console with tab completion, variable expansion, script execution, and real-time server administration. Additional tools include CLI commands, automatic task discovery, file watching for live reloading, recording and replay for debugging, and a complete test framework. Enterprise features include SQLite persistence with migrations, rate limiting, structured logging, health checks, a Model Context Protocol (MCP) server for seamless LLM integration, and real HTTP networking with fetch API support for external data retrieval.

**🎯 Interactive Console**: Modern REPL interface with intelligent tab completion, variable expansion (including environment variables and default values), script execution, command history, and comprehensive server administration capabilities. Perfect for development, debugging, and production operations.

## 🚀 Quick Start

### Installation

#### One-Line Install (Recommended)

The easiest way to install Ratchet is using our install script:

**Linux/macOS:**
```bash
# Install latest release directly from GitHub
curl -fsSL https://raw.githubusercontent.com/michiel/ratchet/master/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
# Install latest release directly from GitHub
irm https://raw.githubusercontent.com/michiel/ratchet/master/scripts/install.ps1 | iex
```

This script will:
- ✅ Detect your platform and architecture automatically
- ✅ Download the latest release from GitHub
- ✅ Install to `~/.local/bin` (no sudo required)
- ✅ Check if the install directory is in your PATH
- ✅ Provide instructions to add it to PATH if needed

#### Manual Installation Options

<details>
<summary>Build from Source</summary>

```bash
# Clone the repository
git clone https://github.com/michiel/ratchet.git
cd ratchet

# Build the project
cargo build --release

# The executable will be at target/release/ratchet
```
</details>

<details>
<summary>Download Pre-built Binary</summary>

1. Go to [GitHub Releases](https://github.com/michiel/ratchet/releases)
2. Download the appropriate archive for your platform
3. Extract and place the `ratchet` binary in your PATH
</details>

<details>
<summary>Custom Install Script Options</summary>

```bash
# Install to custom directory
RATCHET_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/michiel/ratchet/master/scripts/install.sh | bash

# Download and run locally for inspection
curl -O https://raw.githubusercontent.com/michiel/ratchet/master/scripts/install.sh
chmod +x install.sh
./install.sh --help
```
</details>

### Start the Server

```bash
# Start with default configuration
ratchet serve

# Start with custom configuration
ratchet serve --config=sample/configs/example-config.yaml

# Server will be available at:
# - GraphQL: http://localhost:8080/graphql
# - GraphQL Playground: http://localhost:8080/playground
# - REST API: http://localhost:8080/api/v1
# - Health Check: http://localhost:8080/health
```

### Start the Interactive Console

```bash
# Start console with default MCP connection
ratchet console

# Connect to remote Ratchet server
ratchet console --connect=http://remote-server:8090

# Use custom configuration and history
ratchet console --config=config.yaml --history-file=~/.ratchet_history

# Execute startup script
ratchet console --script=startup.rsh
```

### Execute a Task

```bash
# Run a task from the filesystem
ratchet run-once --from-fs sample/js-tasks/addition \
  --input-json='{"num1": 5, "num2": 10}'

# Run HTTP fetch task (demonstrates real network requests)
ratchet run-once --from-fs sample/js-tasks/tasks/test-fetch \
  --input-json='{"endpoint": "/json"}'

# Run with recording for debugging
ratchet run-once --from-fs sample/js-tasks/weather-api \
  --input-json='{"city": "Berlin"}' \
  --record ./recordings
```

### Quick Console Workflow

```bash
# Start console and connect to server
ratchet console

# In the console - intelligent tab completion available
ratchet> repo list                    # List task repositories
ratchet> task list                    # Browse available tasks
ratchet> task execute task-uuid       # Execute a task interactively
ratchet> server status                # Check server health
ratchet> set CITY = Berlin            # Set variables for reuse
ratchet> task execute weather --input '{"city": "$CITY"}'
ratchet> help                         # Show all available commands
```

## 📁 Project Structure

Ratchet uses a modular architecture with specialized crates for different responsibilities:

```
ratchet/
├── ratchet-cli/            # Command-line interface with interactive console and main binary
│   ├── commands/           # CLI command implementations
│   │   ├── console/        # Interactive REPL console with tab completion
│   │   │   ├── commands/   # Console command categories (repo, task, server, etc.)
│   │   │   ├── repl.rs     # Main REPL implementation with variable expansion
│   │   │   ├── parser.rs   # Command parsing and validation
│   │   │   ├── executor.rs # Command execution and MCP client
│   │   │   └── formatter.rs # Output formatting and display
│   │   └── ...             # Other CLI commands (serve, run-once, etc.)
├── ratchet-server/         # HTTP server with REST/GraphQL/MCP APIs
├── ratchet-mcp/            # Model Context Protocol server for LLM integration and console
├── ratchet-rest-api/       # REST API endpoints and handlers
├── ratchet-graphql-api/    # GraphQL schema and resolvers
├── ratchet-interfaces/     # Core interfaces - repository and service trait definitions
├── ratchet-api-types/      # Unified API types for REST and GraphQL
├── ratchet-web/            # Reusable web middleware and utilities
├── ratchet-storage/        # Database layer with Sea-ORM repositories
├── ratchet-execution/      # Process execution and worker management
├── ratchet-js/             # JavaScript runtime with Boa engine and fetch API
├── ratchet-http/           # HTTP client with recording and mocking (rustls TLS)
├── ratchet-config/         # Configuration management and validation
├── ratchet-logging/        # Structured logging system
├── ratchet-core/           # Core domain types and business logic
├── ratchet-caching/        # Caching abstractions and implementations
├── ratchet-resilience/     # Circuit breakers and retry logic
├── ratchet-registry/       # Task discovery and registry management
├── ratchet-output/         # Result delivery to various destinations
├── scripts/                # Installation scripts (install.sh, install.ps1)
├── sample/                 # Example tasks and configurations
│   ├── configs/            # Configuration examples including console setups
│   └── js-tasks/           # Sample JavaScript tasks for testing
└── docs/                   # Documentation and API specifications
```

## 🔧 Task Structure

Tasks are self-contained JavaScript functions with schema validation:

```
my-task/
├── metadata.json        # Task identification and versioning
├── main.js             # JavaScript implementation
├── input.schema.json   # Input validation schema
├── output.schema.json  # Output validation schema
└── tests/              # Test cases with optional mocks
    └── test-001.json
```

### Example Task

**main.js** (Basic arithmetic):
```javascript
function(input) {
  const { num1, num2 } = input;
  
  if (typeof num1 !== 'number' || typeof num2 !== 'number') {
    throw new Error('Both inputs must be numbers');
  }
  
  return {
    sum: num1 + num2,
    product: num1 * num2
  };
}
```

**main.js** (HTTP fetch example):
```javascript
function(input) {
  const { endpoint, include_headers } = input;
  const url = `https://httpbin.org${endpoint}`;
  
  let headers = {};
  if (include_headers) {
    headers = {
      'User-Agent': 'Ratchet-Test-Fetch/1.0',
      'X-Test-Header': 'Ratchet-Sample-Task'
    };
  }
  
  const response = fetch(url, {
    method: 'GET',
    headers: headers
  });
  
  return {
    success: true,
    status: 200,
    url: url,
    data: {
      response_body: response,
      request_info: {
        method: 'GET',
        url: url,
        headers_included: !!include_headers
      }
    }
  };
}
```

**input.schema.json**:
```json
{
  "type": "object",
  "properties": {
    "num1": { "type": "number" },
    "num2": { "type": "number" }
  },
  "required": ["num1", "num2"]
}
```

## 🌐 API Examples

### GraphQL

```graphql
# List all tasks
query {
  tasks {
    items {
      uuid
      label
      version
      description
    }
  }
}

# Execute a task
mutation {
  executeTaskDirect(
    taskUuid: "550e8400-e29b-41d4-a716-446655440000"
    input: "{\"num1\": 5, \"num2\": 10}"
  ) {
    success
    output
    executionId
  }
}

# Monitor job queue
query {
  jobs(status: PENDING) {
    items {
      id
      taskId
      status
      priority
      createdAt
    }
  }
}
```

### REST API

```bash
# List tasks with pagination
curl "http://localhost:8080/api/v1/tasks?_start=0&_end=10"

# Get job queue statistics
curl http://localhost:8080/api/v1/jobs/stats

# Create a new job
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Content-Type: application/json" \
  -d '{"task_id": 1, "input_data": {"num1": 5, "num2": 10}}'

# List workers
curl http://localhost:8080/api/v1/workers
```

## 🖥️ Interactive Console

The Ratchet console provides a powerful, interactive command-line interface for real-time server administration, task management, and development workflows. Built with modern REPL features including intelligent tab completion, variable expansion, command history, and script execution.

### Console Features

**🚀 Smart Tab Completion**
- Command categories: `repo`, `task`, `execution`, `job`, `server`, `db`, `monitor`
- Command actions: `list`, `show`, `execute`, `status`, `enable`, `disable`
- File path completion for script commands
- Context-aware suggestions based on current input

**🔧 Variable Management**
- Local variables: `set TASK_ID = abc123`
- Environment variables: `${ENV:PATH}` or `env PATH`
- Default values: `${VAR:-default_value}`
- Conditional expansion: `${VAR:+value_if_set}`
- Variable listing and inspection: `vars`

**📜 Command History & Scripts**
- Persistent command history across sessions
- Script execution: `source startup.rsh`
- History navigation with up/down arrows
- Search through command history

**🌐 Connection Management**
- Connect to local or remote Ratchet servers
- Support for stdio, SSE, and WebSocket transports
- Authentication token support
- Real-time connection status indication

### Console Command Categories

#### Repository Management
```bash
repo list                     # List all configured repositories
repo add my-repo file://./tasks  # Add new repository source
repo refresh                  # Refresh repository metadata
repo status                   # Show repository health
repo verify                   # Verify repository accessibility
```

#### Task Operations
```bash
task list                     # Browse available tasks
task show task-uuid           # Display task details and schema
task execute task-uuid        # Interactive task execution
task enable task-uuid         # Enable a disabled task
task disable task-uuid        # Disable a task
```

#### Execution Monitoring
```bash
execution list                # Show recent executions
execution show exec-id        # Display execution details
job list                      # View job queue status
job clear                     # Clear completed jobs
job pause                     # Pause job processing
job resume                    # Resume job processing
```

#### Server Administration
```bash
server status                 # Show server health and metrics
server workers                # Display worker pool status
server metrics                # Show performance metrics
health                        # Quick health check
stats                         # System statistics
monitor                       # Real-time monitoring dashboard
```

#### Database Operations
```bash
db status                     # Database connection status
db migrate                    # Run pending migrations
db stats                      # Database performance metrics
```

### Variable Expansion Examples

```bash
# Set local variables
set API_URL = https://api.example.com
set TASK_ID = weather-forecast

# Use variables in commands
task execute $TASK_ID --input '{"url": "$API_URL"}'

# Environment variable access
env HOME                      # Show specific env var
env                          # Show all environment variables

# Advanced variable features
set BASE_URL = ${ENV:API_BASE_URL:-http://localhost:8080}
task execute ${TASK_ID:-default-task} --input '{"debug": "${DEBUG:+true}"}'
```

### Console Configuration

Configure console behavior in your config file:

```yaml
# Console-specific settings (when running console locally)
console:
  # Default connection settings
  default_connection:
    transport: "sse"
    host: "127.0.0.1"
    port: 8090
    timeout: 30
  
  # History and interface settings
  history:
    file: "~/.ratchet_history"
    max_entries: 1000
  
  # Variable settings
  variables:
    auto_export_env: true     # Auto-export local vars to env
    case_sensitive: true      # Variable name case sensitivity
  
  # Script execution settings
  scripts:
    search_paths: ["./scripts", "~/.ratchet/scripts"]
    auto_source: ["~/.ratchet/startup.rsh"]

# MCP server configuration (for console to connect to)
mcp:
  enabled: true
  server:
    transport: "sse"
    host: "127.0.0.1"
    port: 8090
```

### Console Usage Patterns

**Development Workflow:**
```bash
# Start console with development config
ratchet console --config=dev-config.yaml

# Set up development environment
set ENV = development
set DEBUG = true
set LOG_LEVEL = debug

# Work with tasks interactively
repo refresh
task list
task execute my-task --input '{"env": "$ENV", "debug": "$DEBUG"}'
```

**Production Operations:**
```bash
# Connect to production server
ratchet console --connect=https://prod-server:8090 --auth-token=$PROD_TOKEN

# Monitor system health
server status
health
stats
monitor

# Review recent executions
execution list --limit=10
job list --status=failed
```

**Automation Scripts:**
```bash
# Create reusable script: ~/.ratchet/daily-health.rsh
echo "set DATE = $(date +%Y-%m-%d)"
echo "server status"
echo "db status" 
echo "execution list --since=$DATE"

# Execute from console
source ~/.ratchet/daily-health.rsh
```

## 🤖 MCP (Model Context Protocol) Server

Ratchet includes a built-in MCP server that allows Language Learning Models (LLMs) to interact with the task execution engine through a standardized protocol.

### Available MCP Tools

**Core Execution:**
- **`ratchet.execute_task`**: Execute tasks with input data and progress streaming
- **`ratchet.list_available_tasks`**: Discover available tasks with metadata
- **`ratchet.get_execution_status`**: Monitor running executions in real-time
- **`ratchet.get_execution_logs`**: Retrieve execution logs and traces
- **`ratchet.analyze_execution_error`**: Analyze failures with suggestions
- **`ratchet.batch_execute`**: Execute multiple tasks with dependency handling

**Task Development:**
- **`ratchet.create_task`**: Create new tasks with code and schemas
- **`ratchet.edit_task`**: Modify existing tasks and validation
- **`ratchet.delete_task`**: Remove tasks with backup options
- **`ratchet.validate_task`**: Validate task code and schemas
- **`ratchet.run_task_tests`**: Execute task test suites
- **`ratchet.create_task_version`**: Manage task versioning
- **`ratchet.import_tasks`**: Import task collections
- **`ratchet.export_tasks`**: Export tasks for distribution
- **`ratchet.generate_from_template`**: Create tasks from templates
- **`ratchet.list_templates`**: Browse available task templates

**Result Management:**
- **`ratchet.store_result`**: Store execution results for analysis
- **`ratchet.get_results`**: Retrieve stored execution results

### Starting the MCP Server

```bash
# Option 1: Start integrated server with MCP enabled
ratchet serve --config config.yaml

# Option 2: Start standalone MCP server with SSE transport (web applications)
ratchet mcp --transport sse --port 8090

# Option 3: Start MCP server for Claude Desktop (stdio transport)  
ratchet mcp-serve

# Option 4: Add to Claude Desktop using Claude Code CLI
claude mcp add ratchet ratchet mcp-serve
```

Configure MCP in your `config.yaml`:

```yaml
# Enable MCP API
mcp_api:
  enabled: true
  sse_enabled: true
  host: "127.0.0.1"
  port: 8081
  endpoint: "/mcp"

# Server configuration
server:
  host: "127.0.0.1"
  port: 8080
```

### LLM Integration Example

For Claude Desktop, add to your config:

```json
{
  "mcpServers": {
    "ratchet": {
      "command": "ratchet",
      "args": ["mcp-serve"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

The MCP server supports both stdio transport (for Claude Desktop) and SSE transport (for web applications). The `mcp-serve` command defaults to stdio transport for seamless Claude Desktop integration, while the `mcp` command defaults to SSE transport for general use. See the sample configs in `sample/configs/` for complete MCP setup examples.

## 🔐 Configuration

Ratchet can run without any configuration file, using sensible defaults. Configuration can be provided through:
1. **No config**: Uses all defaults (binds to 127.0.0.1:8080, SQLite in-memory database)
2. **Config file**: YAML file with partial or complete configuration
3. **Environment variables**: Override any setting
4. **Mix of all above**: Config file + environment overrides

### Minimal Configuration

```yaml
# Minimal config - everything else uses defaults
logging:
  level: debug
```

### Full Configuration Example (sample/configs/example-config.yaml)

```yaml
# Server settings
server:
  bind_address: "127.0.0.1"
  port: 8080
  
  # Database configuration
  database:
    url: "sqlite://ratchet.db"  # Or sqlite::memory: for in-memory
    max_connections: 10
    connection_timeout: 30
  
  # Rate limiting (optional)
  rate_limit:
    requests_per_minute: 60
    burst_size: 10
    
  # Job queue settings
  job_queue:
    max_dequeue_batch_size: 10
    max_queue_size: 1000
    default_retry_delay: 60
    default_max_retries: 3

  # Worker configuration
  workers:
    worker_count: 4
    restart_on_crash: true
    health_check_interval_seconds: 30

# Task registry
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch: true  # Auto-reload on changes

# Execution settings
execution:
  max_execution_duration: 300  # 5 minutes
  validate_schemas: true

# HTTP client settings
http:
  timeout: 30
  max_redirects: 10
  user_agent: "Ratchet/1.0"
  verify_ssl: true
```

### Console Configuration Examples

Configure the interactive console and MCP server for optimal administration experience:

**Development Console Configuration:**
```yaml
# MCP server for console connections
mcp:
  enabled: true
  server:
    transport: "sse"
    host: "127.0.0.1" 
    port: 8090
  authentication:
    method: "none"  # No auth for development
  tools:
    enable_execution: true
    enable_logging: true
    enable_monitoring: true
    enable_debugging: true

# Enhanced logging for development
logging:
  level: debug
  format: pretty
  structured: false
  context:
    enabled: true
    include_file_location: true
```

**Production Console Configuration:**
```yaml
# Secure MCP server configuration
mcp:
  enabled: true
  server:
    transport: "sse"
    host: "127.0.0.1"
    port: 8090
    enable_cors: false
  
  # Production authentication
  authentication:
    method: "token"
    token_file: "/etc/ratchet/auth.token"
    session:
      timeout_seconds: 3600  # 1 hour sessions
      max_sessions_per_client: 5
  
  # Production security
  security:
    rate_limiting:
      global_per_minute: 1000
      execute_task_per_minute: 100
    request_limits:
      max_request_size_bytes: 10485760  # 10MB
      max_concurrent_executions_per_client: 10
    ip_filtering:
      enabled: true
      default_policy: "deny"
      allowed_ranges: ["10.0.0.0/8", "192.168.0.0/16"]

# Structured logging for production
logging:
  level: info
  format: json
  structured: true
  destination: file
  file_config:
    path: "/var/log/ratchet/console.log"
    rotation:
      max_size_mb: 100
      max_files: 10
```

**Console Client Configuration (for connecting to remote servers):**
```yaml
# Local console settings when connecting to remote Ratchet servers
console:
  default_connection:
    transport: "sse"
    host: "ratchet-server.example.com"
    port: 8090
    timeout: 30
    auth_token_file: "~/.ratchet/token"
  
  history:
    file: "~/.ratchet_history"
    max_entries: 1000
    save_on_exit: true
  
  variables:
    auto_export_env: false
    case_sensitive: true
    predefined:
      PROD_HOST: "ratchet-server.example.com"
      DEV_HOST: "localhost"
  
  scripts:
    search_paths: ["./scripts", "~/.ratchet/scripts"]
    auto_source: ["~/.ratchet/startup.rsh"]
    allow_system_commands: false  # Security: disable for production
```

## 🚦 Production Deployment

### Current Production-Ready Features

✅ **Ready Now**:
- Complete REST and GraphQL APIs
- Interactive console with tab completion and variable expansion
- Persistent database with migrations
- Job queue with scheduling and retry logic
- Worker process management
- Rate limiting and basic security
- Comprehensive error handling
- Health monitoring endpoints
- Real HTTP networking with fetch API
- Pure Rust TLS implementation (rustls)
- Model Context Protocol server for LLM integration and console administration

⚠️ **Requires Configuration**:
- **Authentication**: Currently no auth - all endpoints are public (see [roadmap](TODO.md))
- **HTTPS/TLS**: Configure reverse proxy (nginx/caddy) for SSL
- **Database**: SQLite provides reliable persistence for most workloads
- **Monitoring**: Set up Prometheus/Grafana for metrics

### Deployment Checklist

1. **Database Setup**
   ```bash
   # Database URL for different environments
   export RATCHET_DATABASE_URL="sqlite://ratchet.db"
   ```

2. **Security Configuration**
   - Configure rate limiting appropriate for your load
   - Set up reverse proxy with HTTPS
   - Implement authentication (see [TODO.md](TODO.md) for roadmap)

3. **Performance Tuning**
   - Adjust worker count based on CPU cores
   - Configure connection pool size
   - Set appropriate execution timeouts
   - Enable fetch API for tasks requiring HTTP requests

4. **Monitoring Setup**
   - Health checks: `GET /health` and `GET /api/v1/health`
   - Metrics endpoint for Prometheus (planned)
   - Log aggregation with structured JSON logs

## 🛠️ CLI Commands

### Core Commands

- **`serve`** - Start the API server
  ```bash
  ratchet serve [--config=<path>]
  ```

- **`console`** - Start interactive administration console
  ```bash
  # Local console with MCP connection
  ratchet console [--config=<path>]
  
  # Connect to remote server
  ratchet console --connect=<url> [--auth-token=<token>]
  
  # Custom transport and connection
  ratchet console --transport=sse --host=<host> --port=<port>
  
  # Execute startup script
  ratchet console --script=<script.rsh> --history-file=<path>
  ```

- **`run-once`** - Execute a single task
  ```bash
  ratchet run-once --from-fs <path> --input-json='<json>'
  ```

- **`mcp`** - Start MCP server with SSE transport (general use)
  ```bash
  ratchet mcp [--config=<path>] [--transport=stdio|sse] [--host=<host>] [--port=<port>]
  ```

- **`mcp-serve`** - Start MCP server with stdio transport (Claude Desktop)
  ```bash
  ratchet mcp-serve [--config=<path>] [--transport=stdio|sse] [--host=<host>] [--port=<port>]
  ```

- **`test`** - Run task test suite
  ```bash
  ratchet test --from-fs <path>
  ```

- **`validate`** - Validate task structure
  ```bash
  ratchet validate --from-fs <path>
  ```

- **`replay`** - Replay recorded execution
  ```bash
  ratchet replay --from-fs <path> --recording=<dir>
  ```

### Management Commands

- **`generate`** - Generate code templates
  ```bash
  # Generate new task template
  ratchet generate task --path=./my-task --label="My Task"
  ```

- **`config`** - Configuration management
  ```bash
  # Validate configuration
  ratchet config validate --config-file=<path>
  
  # Generate sample configurations
  ratchet config generate --config-type=dev --output=config.yaml
  
  # Show current configuration
  ratchet config show [--mcp-only] [--format=yaml|json]
  ```

- **`repo`** - Repository management
  ```bash
  # Initialize new repository
  ratchet repo init ./my-repo --name="My Repository"
  
  # Refresh repository metadata
  ratchet repo refresh-metadata [./repo-path]
  
  # Show repository status
  ratchet repo status [--detailed] [--format=table|json|yaml]
  
  # Verify repository accessibility
  ratchet repo verify [--repository=<name>] [--list-tasks]
  ```

### Common Options

- `--log-level <level>` - Set log verbosity (trace, debug, info, warn, error)
- `--record <dir>` - Record execution with HAR and logs
- `--config <path>` - Specify configuration file
- `--worker` - Run as worker process (internal use)
- `--worker-id <id>` - Worker ID for process management

## 📊 Performance & Scalability

- **Execution Model**: Process isolation ensures thread safety
- **Worker Pool**: Scales with CPU cores (configurable)
- **Database**: SQLite for all environments
- **Caching**: LRU cache for task content
- **Rate Limiting**: Per-client quotas with token bucket algorithm

### Benchmarks (on 4-core machine)

- Task execution: ~5ms overhead per task
- HTTP requests: Concurrent with connection pooling
- Database queries: <1ms for simple queries with indexes
- Worker scaling: Linear up to CPU core count

## 🗺️ Roadmap

See [TODO.md](TODO.md) for the comprehensive architectural roadmap including:

1. **Phase 1 ✅**: Infrastructure Extraction (HTTP, logging, JS, execution, config)
2. **Phase 2 🎯**: Server Component Extraction (REST, GraphQL, server core)
3. **Phase 3 📋**: Business Logic Decomposition (services, output, registry)
4. **Phase 4 📋**: Complete ratchet-lib Elimination
5. **Future**: Advanced Features (security, distributed arch, observability)

## 🧪 Testing

```bash
# Run all tests (currently 486 passing)
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Integration tests only
cargo test --test '*'
```

## 🤝 Contributing

1. Check the [TODO.md](TODO.md) for planned improvements
2. Fork the repository
3. Create a feature branch
4. Write tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## 📄 License

[MIT License](LICENSE) - see LICENSE file for details

## 🙏 Acknowledgments

Built with:
- [Boa](https://github.com/boa-dev/boa) - JavaScript engine in Rust
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [async-graphql](https://github.com/async-graphql/async-graphql) - GraphQL server
- [Sea-ORM](https://github.com/SeaQL/sea-orm) - Database ORM
- [Tokio](https://tokio.rs/) - Async runtime

---

**Ready for Production** with security considerations. See [TODO.md](TODO.md) for the roadmap to enterprise features.
