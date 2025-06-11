# Ratchet 🚀

**Production-Ready JavaScript Task Execution Platform**

Ratchet is a high-performance, scalable task execution platform that runs JavaScript code with enterprise-grade reliability. Built with Rust for performance and safety, it provides comprehensive APIs, persistent storage, and advanced execution capabilities.

[![Tests](https://img.shields.io/badge/tests-486%20passing-brightgreen)](.) [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![Status](https://img.shields.io/badge/status-production--ready-green)]()

## 🎯 Key Features by Category

### **Task Execution Engine**
- **JavaScript Runtime**: Secure, isolated JavaScript execution with Boa engine
- **Schema Validation**: Input/output validation using JSON Schema
- **Process Isolation**: Thread-safe execution through process separation architecture
- **Resource Management**: Configurable execution timeouts and resource limits
- **Error Handling**: Comprehensive error types with stack traces and context

### **API & Integration**
- **GraphQL API**: Full-featured GraphQL server with queries, mutations, and subscriptions
- **REST API**: Refine.dev compatible REST endpoints for web applications
- **OpenAPI Spec**: Complete API documentation with OpenAPI 3.0
- **Rate Limiting**: Token bucket algorithm with configurable limits per client
- **CORS Support**: Ready for cross-origin web application integration

### **Job Queue & Scheduling**
- **Priority Queue**: Multi-priority job execution with configurable batch sizes
- **Retry Logic**: Exponential backoff with circuit breaker patterns
- **Cron Scheduling**: Schedule tasks with cron expressions
- **Worker Pool**: Scalable worker processes with health monitoring
- **Job Management**: Cancel, retry, and monitor job execution

### **Data Persistence**
- **Database Layer**: SQLite with Sea-ORM for reliable data persistence
- **Migration System**: Schema evolution with versioned migrations
- **Repository Pattern**: Clean, testable database operations
- **Connection Pooling**: Efficient database connection management
- **Transaction Support**: ACID compliance for data integrity

### **Development Tools**
- **CLI Interface**: Comprehensive command-line tools for all operations
- **Task Registry**: Automatic task discovery and version management
- **File Watching**: Auto-reload tasks on file changes during development
- **Recording & Replay**: Capture and replay task executions for debugging
- **Test Framework**: Built-in testing with mock HTTP responses
- **MCP Server**: Model Context Protocol server for LLM integration

### **Security & Reliability**
- **SQL Injection Prevention**: Safe query builder with input sanitization
- **Input Validation**: Comprehensive validation for all user inputs
- **Rate Limiting**: Protect against abuse with configurable limits
- **Error Isolation**: Failures in one task don't affect others
- **Audit Ready**: Structured logging and error tracking

### **Monitoring & Operations**
- **Health Checks**: REST and GraphQL health endpoints
- **Metrics Collection**: Execution metrics and performance data
- **Worker Monitoring**: Real-time worker status and load distribution
- **Structured Logging**: JSON logs with correlation IDs
- **Performance Tracking**: Execution duration and resource usage metrics

## 🚀 Quick Start

### Installation

#### One-Line Install (Recommended)

The easiest way to install Ratchet is using our install script:

```bash
# Install latest release directly from GitHub
curl -fsSL https://raw.githubusercontent.com/michiel/ratchet/master/scripts/install.sh | bash
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

### Execute a Task

```bash
# Run a task from the filesystem
ratchet run-once --from-fs sample/js-tasks/addition \
  --input-json='{"num1": 5, "num2": 10}'

# Run with recording for debugging
ratchet run-once --from-fs sample/js-tasks/weather-api \
  --input-json='{"city": "Berlin"}' \
  --record ./recordings
```

## 📁 Project Structure

Ratchet is migrating to a fully modular architecture. **Phase 1** (infrastructure extraction) is complete, **Phase 2** (server component extraction) is next:

```
ratchet/
├── ratchet-cli/          # Command-line interface
├── ratchet-lib/          # 🎯 Monolith targeted for decomposition
│   ├── rest/             # → ratchet-rest (Phase 2)
│   ├── graphql/          # → ratchet-graphql (Phase 2)
│   ├── server/           # → ratchet-server-core (Phase 2)
│   └── services/         # → ratchet-services (Phase 3)
├── ratchet-mcp/          # Model Context Protocol server for LLM integration
├── ratchet-execution/    # ✅ Process execution infrastructure (extracted)
├── ratchet-storage/      # ✅ Database layer with repositories (extracted)
├── ratchet-core/         # ✅ Domain types and models
├── ratchet-http/         # ✅ HTTP client with mocking (extracted)
├── ratchet-logging/      # ✅ Structured logging system (extracted)
├── ratchet-js/           # ✅ JavaScript execution engine (extracted)
├── ratchet-config/       # ✅ Configuration management (extracted)
├── ratchet-caching/      # ✅ Caching abstractions
├── ratchet-resilience/   # ✅ Circuit breakers, retry logic
├── ratchet-runtime/      # ✅ Alternative task execution patterns
├── ratchet-ipc/          # ✅ Inter-process communication
├── ratchet-plugin/       # ✅ Plugin infrastructure
├── sample/               # Example tasks and configurations
└── docs/                 # Documentation and API specs
```

**Goal**: Complete decomposition of ratchet-lib into focused, single-responsibility crates.

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

**main.js**:
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

## 🤖 MCP (Model Context Protocol) Server

Ratchet includes a built-in MCP server that allows Language Learning Models (LLMs) to interact with the task execution engine through a standardized protocol.

### Available MCP Tools

- **`ratchet.execute_task`**: Execute any Ratchet task with input data
- **`ratchet.list_available_tasks`**: Discover available tasks with filtering
- **`ratchet.get_execution_status`**: Monitor running executions
- **`ratchet.get_execution_logs`**: Retrieve execution logs
- **`ratchet.get_execution_trace`**: Get detailed execution traces
- **`ratchet.analyze_execution_error`**: Analyze failures with suggestions

### Starting the MCP Server

```bash
# Start with stdio transport (for local LLM integration)
ratchet mcp-serve --transport stdio

# Start with SSE transport (for network access)
ratchet mcp-serve --transport sse --port 3001

# Or configure in your config.yaml and start with the main server
server:
  # ... server config ...

mcp:
  enabled: true
  transport: sse
  host: localhost
  port: 3001
  auth_type: none
  max_connections: 10
  request_timeout: 30
  rate_limit_per_minute: 100
```

### LLM Integration Example

For Claude Desktop, add to your config:

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

See [MCP User Guide](docs/MCP_USER_GUIDE.md) for detailed configuration and usage.

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

## 🚦 Production Deployment

### Current Production-Ready Features

✅ **Ready Now**:
- Complete REST and GraphQL APIs
- Persistent database with migrations
- Job queue with scheduling and retry logic
- Worker process management
- Rate limiting and basic security
- Comprehensive error handling
- Health monitoring endpoints

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

- **`run-once`** - Execute a single task
  ```bash
  ratchet run-once --from-fs <path> --input-json='<json>'
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

### Common Options

- `--log-level <level>` - Set log verbosity (trace, debug, info, warn, error)
- `--record <dir>` - Record execution with HAR and logs
- `--config <path>` - Specify configuration file

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
