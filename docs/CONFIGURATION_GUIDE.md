# Ratchet Configuration Guide

This comprehensive guide covers all configuration options for Ratchet, including server modes, MCP integration, deployment scenarios, and examples.

## Table of Contents

1. [Configuration Files Overview](#configuration-files-overview)
2. [Basic Configuration](#basic-configuration)
3. [Server Configuration](#server-configuration)
4. [MCP Configuration](#mcp-configuration)
5. [Deployment Scenarios](#deployment-scenarios)
6. [Environment Variables](#environment-variables)
7. [Example Configurations](#example-configurations)
8. [Security Configuration](#security-configuration)
9. [Performance Tuning](#performance-tuning)
10. [Troubleshooting](#troubleshooting)

## Configuration Files Overview

Ratchet uses YAML configuration files for both server modes:

### Available Configuration Files

Located in `sample/configs/`:

#### General Configurations
- `example-config.yaml` - Basic Ratchet configuration example
- `test-config.yaml` - Configuration for running tests

#### MCP Configurations
- `example-mcp-minimal.yaml` - Minimal MCP configuration for getting started
- `example-mcp-dev.yaml` - Development environment MCP configuration
- `example-mcp-production.yaml` - Production-ready MCP configuration with security
- `example-mcp-enterprise.yaml` - Enterprise MCP configuration with advanced features
- `ratchet-mcp-config.yaml` - Complete MCP configuration with all features

#### SSE Transport Configurations
- `example-sse-config.yaml` - SSE transport configuration for HTTP-based connections

#### Claude Desktop Integration
- `claude-config.json` - Example Claude Desktop MCP server configuration
- `claude_desktop_config.json` - Annotated Claude Desktop configuration
- `claude-desktop-mcp-client.json` - Stdio transport configuration
- `claude-desktop-http-client.json` - SSE transport configuration

## Basic Configuration

### Minimal Configuration

```yaml
# config.yaml - Minimal working configuration
database:
  url: "sqlite:ratchet.db"

logging:
  level: info
  sinks:
    - type: console
      level: info

server:
  bind_address: "127.0.0.1"
  port: 8080
```

### Development Configuration

```yaml
# dev-config.yaml - Development with debugging
database:
  url: "sqlite::memory:"  # In-memory for fast restarts

logging:
  level: debug
  sinks:
    - type: console
      level: info
      use_json: false
    - type: file
      path: ratchet-dev.log
      level: debug

server:
  bind_address: "127.0.0.1"
  port: 8080
  api:
    enable_graphql: true
    enable_rest: true
    
# Auto-reload tasks during development
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch_for_changes: true
        auto_reload: true
        scan_interval: 5
```

## Server Configuration

### Server Modes

Ratchet provides two main server modes:

1. **Regular Server** (`ratchet serve`)
2. **MCP Server** (`ratchet mcp-serve`)

### Regular Server Configuration

```yaml
# Complete server configuration
server:
  bind_address: "0.0.0.0"  # Bind to all interfaces
  port: 8080
  
  # Database configuration
  database:
    url: "sqlite:ratchet.db"  # or PostgreSQL: "postgresql://user:pass@host/db"
    max_connections: 20
    connection_timeout: 30
    
  # API configuration
  api:
    enable_graphql: true
    enable_rest: true
    enable_websocket: false
    
  # CORS settings for web UI
  cors:
    allowed_origins: ["http://localhost:*", "http://127.0.0.1:*"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    allowed_headers: ["Content-Type", "Authorization"]
    max_age: 3600

# Task execution configuration
execution:
  max_execution_duration: 300  # 5 minutes
  validate_schemas: true
  fetch_variables:
    url_var: "__fetch_url"
    params_var: "__fetch_params"
    body_var: "__fetch_body"
    result_var: "__http_result"

# HTTP client settings
http:
  timeout: 30
  max_redirects: 5
  user_agent: "Ratchet/1.0"
  verify_ssl: true

# Worker process configuration
workers:
  count: 4  # Number of worker processes
  restart_on_failure: true
  max_restarts: 5
  restart_delay: 5

# Cache configuration
cache:
  task_content_cache_size: 1000
  enabled: true
  ttl: 3600  # 1 hour
```

### Output Configuration

```yaml
# Output delivery configuration
output:
  max_concurrent_deliveries: 20
  default_timeout: 60
  validate_on_startup: true
  retry_policy:
    max_attempts: 3
    initial_delay: 1
    max_delay: 30
    multiplier: 2

# Output destinations
output_destinations:
  - type: filesystem
    path: "outputs/{{job_id}}/{{timestamp}}.json"
    format: json
    permissions: 644
    create_dirs: true
    overwrite: false
    
  - type: webhook
    url: "https://api.example.com/webhook/{{job_id}}"
    method: POST
    headers:
      Authorization: "Bearer {{webhook_token}}"
    timeout: 30
    retry_policy:
      max_attempts: 3
      initial_delay: 1
      backoff_multiplier: 2.0
```

## MCP Configuration

### Basic MCP Configuration

```yaml
# MCP server configuration
mcp:
  enabled: true
  
  server:
    transport: "stdio"  # or "sse"
    database:
      url: "sqlite:./ratchet-mcp.db"
      max_connections: 10
      connection_timeout: 30
  
  # Authentication (development)
  authentication:
    method: "none"
    
  # Tools configuration
  tools:
    enable_execution: true
    enable_logging: true
    enable_monitoring: true
    enable_debugging: true
    enable_batch: true
```

### Production MCP Configuration

```yaml
mcp:
  enabled: true
  
  server:
    transport: "sse"
    bind_address: "0.0.0.0"
    port: 8090
    
  # Production authentication
  authentication:
    method: "api_key"
    api_key:
      header_name: "Authorization"
      prefix: "Bearer"
      keys:
        "claude-desktop-prod-key-2025":
          name: "Claude Desktop Production"
          description: "Claude Desktop application production access"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            can_read_traces: true
            can_access_system_info: true
            allowed_task_patterns: ["safe-*", "api-*"]
            denied_task_patterns: ["admin-*", "system-*"]
          created_at: "2025-01-01T00:00:00Z"
          active: true
          allowed_ips: []

  # Security settings
  security:
    rate_limiting:
      execute_task_per_minute: 120
      batch_execute_per_minute: 30
      global_per_minute: 1000
    request_size_limit: 10485760  # 10MB
    response_size_limit: 52428800  # 50MB
    audit_log_enabled: true

  # Batch processing
  batch:
    max_batch_size: 100
    max_parallel: 10
    default_timeout_ms: 300000  # 5 minutes
    enable_dependencies: true
    enable_progress: true
```

### SSE Transport Configuration

```yaml
mcp:
  server:
    transport: "sse"
    bind_address: "0.0.0.0"
    port: 8090
    
    # SSL/TLS configuration
    tls:
      enabled: true
      cert_file: "/path/to/cert.pem"
      key_file: "/path/to/key.pem"
      
  # CORS for web clients
  cors:
    allowed_origins: ["https://example.com", "http://localhost:3000"]
    allowed_methods: ["GET", "POST", "OPTIONS"]
    allowed_headers: ["Content-Type", "Authorization"]
    max_age: 3600
```

## Deployment Scenarios

### Development Setup

```yaml
# development.yaml
database:
  url: "sqlite::memory:"

logging:
  level: debug
  sinks:
    - type: console
      use_json: false

server:
  bind_address: "127.0.0.1"
  port: 8080

mcp:
  enabled: true
  authentication:
    method: "none"
  tools:
    enable_debugging: true

registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch_for_changes: true
        auto_reload: true
```

### Production Setup

```yaml
# production.yaml
database:
  url: "postgresql://ratchet:${DB_PASSWORD}@localhost:5432/ratchet_prod"
  max_connections: 50
  connection_timeout: 30

logging:
  level: info
  sinks:
    - type: file
      path: "/var/log/ratchet/ratchet.log"
      level: info
      max_size: 100MB
      max_backups: 10
    - type: file
      path: "/var/log/ratchet/error.log"
      level: error

server:
  bind_address: "0.0.0.0"
  port: 8080
  
mcp:
  enabled: true
  server:
    transport: "sse"
    bind_address: "0.0.0.0"
    port: 8090
    tls:
      enabled: true
      cert_file: "/etc/ssl/certs/ratchet.crt"
      key_file: "/etc/ssl/private/ratchet.key"
      
  authentication:
    method: "api_key"
    api_key:
      keys:
        "${CLAUDE_API_KEY}":
          name: "Claude Desktop Production"
          permissions:
            can_execute_tasks: true
            allowed_task_patterns: ["safe-*"]
            
  security:
    rate_limiting:
      execute_task_per_minute: 60
      global_per_minute: 300
    audit_log_enabled: true

workers:
  count: 8
  restart_on_failure: true

registry:
  sources:
    - name: "production-tasks"
      uri: "https://registry.company.com/tasks"
      config:
        api_key: "${REGISTRY_API_KEY}"
        cache_duration: 3600
```

### High Availability Setup

```yaml
# ha-config.yaml
database:
  url: "postgresql://ratchet:${DB_PASSWORD}@db-cluster:5432/ratchet"
  max_connections: 100
  connection_timeout: 10

server:
  bind_address: "0.0.0.0"
  port: 8080

# Redis for shared caching
cache:
  type: "redis"
  url: "redis://redis-cluster:6379"
  
# Load balancer health checks
health:
  check_interval: 30
  database_timeout: 5
  worker_timeout: 10

# Distributed task registry
registry:
  sources:
    - name: "shared-tasks"
      uri: "s3://company-tasks/production/"
      config:
        region: "us-west-2"
        cache_duration: 1800
```

## Environment Variables

### Core Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RATCHET_CONFIG` | Configuration file path | `config.yaml` |
| `RATCHET_ENV` | Environment (dev/staging/prod) | `development` |
| `RUST_LOG` | Log level | `info` |

### Server Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RATCHET_SERVER_HOST` | Server bind address | `127.0.0.1` |
| `RATCHET_SERVER_PORT` | Server port | `8080` |
| `RATCHET_DATABASE_URL` | Database connection URL | `sqlite:ratchet.db` |
| `RATCHET_DATABASE_MAX_CONNECTIONS` | Max database connections | `10` |

### MCP Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RATCHET_MCP_ENABLED` | Enable MCP server | `false` |
| `RATCHET_MCP_TRANSPORT` | Transport type (stdio/sse) | `stdio` |
| `RATCHET_MCP_HOST` | MCP server host | `127.0.0.1` |
| `RATCHET_MCP_PORT` | MCP server port | `8090` |
| `MCP_API_KEY` | MCP authentication key | - |

### Example with Environment Variables

```yaml
# config.yaml with environment variable substitution
database:
  url: "${DATABASE_URL:-sqlite:ratchet.db}"
  max_connections: ${DB_MAX_CONNECTIONS:-10}

server:
  bind_address: "${SERVER_HOST:-127.0.0.1}"
  port: ${SERVER_PORT:-8080}

mcp:
  authentication:
    api_key:
      keys:
        "${MCP_API_KEY}":
          name: "${MCP_KEY_NAME:-Default Key}"
          
logging:
  level: "${LOG_LEVEL:-info}"
  sinks:
    - type: file
      path: "${LOG_FILE:-ratchet.log}"
```

## Example Configurations

### Full-Featured Server

```yaml
# full-server-config.yaml
# Complete Ratchet server with all features enabled

# Database configuration
database:
  url: "sqlite:/tmp/ratchet/data/ratchet.db"
  max_connections: 20
  connection_timeout: 30

# Logging configuration
logging:
  level: info
  format: json
  sinks:
    - type: console
      level: info
      use_json: false
    - type: file
      level: debug
      path: "/tmp/ratchet/logs/ratchet.log"
      max_size: 10485760  # 10MB
      max_backups: 5

# Server configuration
server:
  bind_address: "0.0.0.0"
  port: 8080
  
  # API settings
  api:
    enable_graphql: true
    enable_rest: true
    enable_websocket: false
    
  # CORS settings
  cors:
    allowed_origins: ["http://localhost:*", "http://127.0.0.1:*"]
    allowed_methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    allowed_headers: ["Content-Type", "Authorization"]
    max_age: 3600

# MCP Server Configuration
mcp:
  enabled: true
  server:
    transport: "sse"
    bind_address: "0.0.0.0"
    port: 8090
    
  # Authentication
  authentication:
    method: "api_key"
    api_key:
      header_name: "Authorization"
      prefix: "Bearer"
      keys:
        "example-key-2025":
          name: "Example Client"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            allowed_task_patterns: ["*"]

  # Tools configuration
  tools:
    enable_execution: true
    enable_logging: true
    enable_monitoring: true
    enable_batch: true

# Task Registry Configuration
registry:
  sources:
    - name: "local-tasks"
      uri: "file:///tmp/ratchet/tasks"
      config:
        watch_for_changes: true
        auto_reload: true
        scan_interval: 10

# Output delivery configuration
output:
  max_concurrent_deliveries: 20
  default_timeout: 60
  validate_on_startup: true

# Worker processes
workers:
  count: 4
  restart_on_failure: true
  max_restarts: 5
```

### Claude Desktop Integration

```json
// Claude Desktop configuration
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

### Docker Compose Configuration

```yaml
# docker-compose.yml
version: '3.8'

services:
  ratchet:
    image: ratchet:latest
    ports:
      - "8080:8080"
      - "8090:8090"
    environment:
      - DATABASE_URL=postgresql://ratchet:password@postgres:5432/ratchet
      - MCP_API_KEY=your-secret-key
      - LOG_LEVEL=info
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./tasks:/app/tasks
      - logs:/var/log/ratchet
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=ratchet
      - POSTGRES_USER=ratchet
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
  logs:
```

## Security Configuration

### Authentication Configuration

```yaml
mcp:
  authentication:
    method: "api_key"
    api_key:
      header_name: "Authorization"
      prefix: "Bearer"
      keys:
        "prod-key-1":
          name: "Production Client 1"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            can_read_traces: false
            allowed_task_patterns: ["safe-*", "api-*"]
            denied_task_patterns: ["admin-*"]
          rate_limits:
            executions_per_minute: 30
            logs_per_minute: 100
          allowed_ips: ["192.168.1.0/24"]
          
        "dev-key-1":
          name: "Development Client"
          permissions:
            can_execute_tasks: true
            can_read_logs: true
            can_read_traces: true
            allowed_task_patterns: ["*"]
          rate_limits:
            executions_per_minute: 60
```

### TLS Configuration

```yaml
mcp:
  server:
    transport: "sse"
    tls:
      enabled: true
      cert_file: "/etc/ssl/certs/ratchet.crt"
      key_file: "/etc/ssl/private/ratchet.key"
      ca_file: "/etc/ssl/certs/ca.crt"
      verify_client: true
      
server:
  tls:
    enabled: true
    cert_file: "/etc/ssl/certs/server.crt"
    key_file: "/etc/ssl/private/server.key"
```

### Security Policies

```yaml
security:
  # Input validation
  max_request_size: 10485760  # 10MB
  max_response_size: 52428800  # 50MB
  
  # Rate limiting
  rate_limits:
    global_per_minute: 1000
    per_client_per_minute: 100
    
  # Audit logging
  audit:
    enabled: true
    file: "/var/log/ratchet/audit.log"
    include_request_body: false
    include_response_body: false
    
  # Task execution security
  execution:
    sandbox_mode: true
    max_execution_time: 300
    memory_limit: 512MB
    network_access: restricted
```

## Performance Tuning

### Database Optimization

```yaml
database:
  # Connection pooling
  max_connections: 50
  min_connections: 5
  connection_timeout: 30
  idle_timeout: 600
  
  # Query optimization
  statement_cache_size: 1000
  prepared_statement_cache: true
  
  # For PostgreSQL
  postgresql:
    shared_preload_libraries: ["pg_stat_statements"]
    max_connections: 200
    shared_buffers: "256MB"
    effective_cache_size: "1GB"
```

### Worker Process Tuning

```yaml
workers:
  count: 8  # Match CPU cores
  restart_on_failure: true
  max_restarts: 5
  restart_delay: 5
  
  # Resource limits per worker
  memory_limit: 512MB
  cpu_limit: 1.0
  
  # Queue configuration
  queue_size: 1000
  batch_size: 10
  timeout: 300
```

### Caching Configuration

```yaml
cache:
  # Task content caching
  task_content_cache_size: 5000
  task_content_ttl: 3600
  
  # Result caching
  result_cache_size: 10000
  result_cache_ttl: 1800
  
  # Redis configuration (if using Redis)
  redis:
    url: "redis://localhost:6379"
    pool_size: 20
    timeout: 5
    
  # Memory cache configuration
  memory:
    max_size: 1073741824  # 1GB
    cleanup_interval: 300
```

### Network Optimization

```yaml
http:
  # Connection pooling
  connection_pool_size: 100
  connection_timeout: 10
  request_timeout: 30
  keepalive_timeout: 90
  
  # Compression
  enable_compression: true
  compression_level: 6
  min_compression_size: 1024
  
  # HTTP/2 support
  enable_http2: true
```

## Troubleshooting

### Configuration Validation

```bash
# Validate configuration file
ratchet config validate --config-file config.yaml

# Show resolved configuration
ratchet config show --config-file config.yaml

# Test specific components
ratchet config test --component database
ratchet config test --component mcp
```

### Common Configuration Issues

#### Database Connection Issues

```yaml
# Issue: Connection timeout
database:
  connection_timeout: 30  # Increase timeout
  max_connections: 10     # Reduce connections

# Issue: SSL connection problems
database:
  url: "postgresql://user:pass@host:5432/db?sslmode=require"
  ssl:
    ca_file: "/path/to/ca.crt"
    cert_file: "/path/to/client.crt"
    key_file: "/path/to/client.key"
```

#### MCP Authentication Issues

```yaml
# Issue: API key not working
mcp:
  authentication:
    method: "api_key"
    api_key:
      header_name: "Authorization"  # Check header name
      prefix: "Bearer"              # Check prefix
      keys:
        "your-key-here":            # Check key format
          name: "Client Name"
          active: true              # Ensure key is active
```

#### Port Binding Issues

```bash
# Check if ports are in use
netstat -tulpn | grep :8080
netstat -tulpn | grep :8090

# Use different ports if needed
```

```yaml
server:
  port: 8081  # Use alternative port

mcp:
  server:
    port: 8091  # Use alternative port
```

### Debugging Configuration

```yaml
# Enable debug logging for configuration
logging:
  level: debug
  sinks:
    - type: console
      level: debug

# Enable configuration tracing
debug:
  trace_config_loading: true
  validate_config_on_change: true
  log_config_changes: true
```

### Performance Monitoring

```yaml
# Enable performance metrics
performance:
  enable_metrics: true
  metrics_port: 9090
  
  # Database performance
  log_slow_queries: true
  slow_query_threshold: 1000  # 1 second
  
  # Task execution performance
  log_execution_times: true
  execution_time_threshold: 5000  # 5 seconds
```

## Configuration Management

### Environment-Specific Configs

```bash
# Structure for multiple environments
configs/
├── base.yaml           # Common settings
├── development.yaml    # Development overrides
├── staging.yaml       # Staging overrides
└── production.yaml    # Production overrides
```

### Configuration Merging

```yaml
# base.yaml
database:
  max_connections: 10
  connection_timeout: 30

server:
  bind_address: "127.0.0.1"
  port: 8080

# production.yaml (extends base.yaml)
database:
  url: "postgresql://..."
  max_connections: 50

server:
  bind_address: "0.0.0.0"
```

### Secrets Management

```yaml
# Using environment variables for secrets
database:
  url: "${DATABASE_URL}"

mcp:
  authentication:
    api_key:
      keys:
        "${MCP_API_KEY}":
          name: "Production Key"

# External secrets (AWS Secrets Manager, etc.)
secrets:
  provider: "aws"
  region: "us-west-2"
  secret_name: "ratchet/production"
```

For more detailed setup instructions, see:
- [MCP Integration Guide](MCP_INTEGRATION_GUIDE.md)
- [CLI Usage Guide](CLI_USAGE.md)
- [Architecture Overview](../ARCHITECTURE.md)