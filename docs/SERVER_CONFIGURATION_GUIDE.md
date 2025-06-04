# Ratchet Server Configuration Guide

This guide explains how to configure and deploy Ratchet's different server modes for various use cases.

## Server Modes Overview

Ratchet provides two distinct server modes:

### 1. **Regular Server** (`ratchet serve`)
- **Purpose:** Full-featured HTTP/GraphQL API server for web applications
- **Protocols:** HTTP REST API, GraphQL API, WebSocket subscriptions
- **Use Cases:** Web applications, mobile apps, direct API integration
- **Ports:** Default 8080 (configurable)

### 2. **MCP Server** (`ratchet mcp-serve`)
- **Purpose:** Specialized Model Context Protocol server for LLM integration
- **Protocols:** MCP over stdio, SSE (future: WebSocket)
- **Use Cases:** Claude Desktop, AI assistants, LLM-powered automation
- **Transport:** stdio (no ports), SSE (default 8090)

## Configuration Architecture

Both servers share the same core configuration but use different sections:

```yaml
# config.yaml - Unified configuration file

# Shared configuration
database:
  url: "sqlite:ratchet.db"

logging:
  level: info
  sinks:
    - type: console
    - type: file
      path: ratchet.log

# Regular server configuration
server:
  host: "0.0.0.0"
  port: 8080
  cors:
    enabled: true
    origins: ["*"]

# REST API configuration  
rest:
  enabled: true
  prefix: "/api/v1"
  rate_limit:
    requests_per_minute: 1000

# GraphQL configuration
graphql:
  enabled: true
  endpoint: "/graphql"
  playground: true

# MCP server configuration
mcp:
  enabled: true
  transport: stdio
  auth_type: none
  max_connections: 10
  rate_limit_per_minute: 100
```

## Deployment Scenarios

### Scenario 1: Web Application with AI Assistant

Run both servers simultaneously for maximum flexibility:

```bash
# Terminal 1: Start regular server for web app
ratchet serve --config config.yaml

# Terminal 2: Start MCP server for Claude
ratchet mcp-serve --config config.yaml
```

**Use Cases:**
- Web dashboard at `http://localhost:8080`
- Claude Desktop integration via MCP
- Mobile app using REST API
- Real-time updates via GraphQL subscriptions

### Scenario 2: Claude-Only Integration

Run only the MCP server for LLM-focused usage:

```bash
# Start MCP server only
ratchet mcp-serve --config config.yaml --transport stdio
```

**Use Cases:**
- Claude Desktop as primary interface
- AI-powered task automation
- Minimal resource usage
- Command-line AI assistance

### Scenario 3: Production Web Service

Run the regular server with full production features:

```bash
# Production web server
ratchet serve --config production-config.yaml
```

**Configuration:**
```yaml
# production-config.yaml
server:
  host: "0.0.0.0"
  port: 8080

auth:
  jwt:
    secret: "${JWT_SECRET}"
    expires_in: "24h"

security:
  rate_limit:
    requests_per_minute: 1000
    burst_size: 100
  
database:
  url: "${DATABASE_URL}"
  pool_size: 20

logging:
  level: warn
  sinks:
    - type: file
      path: /var/log/ratchet.log
      level: info
      rotation:
        size: "100MB"
        keep: 10
```

### Scenario 4: Development Environment

Use both servers with development-friendly settings:

```yaml
# dev-config.yaml
database:
  url: "sqlite:dev.db"

logging:
  level: debug
  sinks:
    - type: console
      level: debug

server:
  host: "localhost"
  port: 8080

mcp:
  enabled: true
  transport: stdio
  auth_type: none  # No auth for development

# Enable all debugging features
graphql:
  playground: true
  introspection: true

rest:
  cors:
    origins: ["http://localhost:3000", "http://localhost:8080"]
```

## Server Management

### Process Management

#### Using systemd (Linux)

**Regular Server Service:**
```ini
# /etc/systemd/system/ratchet-server.service
[Unit]
Description=Ratchet Task Execution Server
After=network.target

[Service]
Type=simple
User=ratchet
WorkingDirectory=/opt/ratchet
ExecStart=/usr/local/bin/ratchet serve --config /etc/ratchet/config.yaml
Restart=always
RestartSec=5

Environment=RUST_LOG=info
Environment=DATABASE_URL=postgresql://user:pass@localhost/ratchet

[Install]
WantedBy=multi-user.target
```

**MCP Server Service (if needed separately):**
```ini
# /etc/systemd/system/ratchet-mcp.service
[Unit]
Description=Ratchet MCP Server
After=network.target

[Service]
Type=simple
User=ratchet
WorkingDirectory=/opt/ratchet
ExecStart=/usr/local/bin/ratchet mcp-serve --config /etc/ratchet/config.yaml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

#### Using Docker

**Multi-service Docker Compose:**
```yaml
# docker-compose.yml
version: '3.8'

services:
  ratchet-server:
    image: ratchet:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./tasks:/app/tasks
    command: ["serve", "--config", "config.yaml"]
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://postgres:password@db:5432/ratchet

  ratchet-mcp:
    image: ratchet:latest
    volumes:
      - ./config.yaml:/app/config.yaml
      - ./tasks:/app/tasks
    command: ["mcp-serve", "--config", "config.yaml"]
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://postgres:password@db:5432/ratchet

  db:
    image: postgres:15
    environment:
      POSTGRES_DB: ratchet
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

### Health Monitoring

Both servers provide health endpoints:

**Regular Server:**
```bash
# Health check
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/api/v1/health
```

**MCP Server:**
```bash
# MCP server doesn't expose HTTP endpoints in stdio mode
# Monitor via logs or process status
ps aux | grep "ratchet mcp-serve"
tail -f /var/log/ratchet.log
```

### Load Balancing

For high-availability deployments:

```nginx
# nginx.conf
upstream ratchet_servers {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
}

server {
    listen 80;
    server_name ratchet.example.com;

    location / {
        proxy_pass http://ratchet_servers;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    location /graphql {
        proxy_pass http://ratchet_servers;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## Configuration Reference

### Database Configuration

```yaml
database:
  # SQLite (development)
  url: "sqlite:ratchet.db"
  
  # PostgreSQL (production)
  url: "postgresql://user:pass@localhost/ratchet"
  pool_size: 20
  max_connections: 100
  connection_timeout: 30
  idle_timeout: 600
```

### Logging Configuration

```yaml
logging:
  level: info  # trace, debug, info, warn, error
  sinks:
    - type: console
      level: info
      colored: true
    
    - type: file
      path: /var/log/ratchet.log
      level: debug
      rotation:
        size: "100MB"
        keep: 10
      buffered:
        size: 1000
        flush_interval: "5s"
```

### Authentication Configuration

```yaml
auth:
  jwt:
    secret: "${JWT_SECRET}"
    algorithm: "HS256"
    expires_in: "24h"
    issuer: "ratchet"
    
  api_keys:
    - key: "${API_KEY_1}"
      name: "service-account"
      permissions: ["read", "write"]
    
    - key: "${API_KEY_2}"
      name: "monitoring"
      permissions: ["read"]
```

### Rate Limiting Configuration

```yaml
rate_limiting:
  # Global limits
  global:
    requests_per_minute: 10000
    burst_size: 1000
  
  # Per-endpoint limits
  endpoints:
    "/api/v1/tasks/execute":
      requests_per_minute: 100
      burst_size: 10
    
    "/graphql":
      requests_per_minute: 1000
      burst_size: 100
  
  # Per-user limits (requires authentication)
  per_user:
    requests_per_minute: 1000
    burst_size: 50
```

### MCP-Specific Configuration

```yaml
mcp:
  enabled: true
  transport: stdio  # stdio, sse
  
  # SSE transport settings
  sse:
    host: "0.0.0.0"
    port: 8090
    cors:
      enabled: true
      origins: ["*"]
  
  # Authentication
  auth_type: api_key  # none, api_key, jwt
  api_keys:
    - key: "${MCP_API_KEY}"
      name: "claude-desktop"
      permissions:
        can_execute_tasks: true
        can_read_logs: true
        allowed_task_patterns: ["*"]
        rate_limits:
          executions_per_minute: 60
          logs_per_minute: 200
  
  # Performance settings
  performance:
    max_concurrent_executions_per_client: 5
    connection_pool_size: 20
    request_timeout: 30
    max_execution_time: 300
  
  # Security settings
  security:
    input_sanitization: true
    audit_log_enabled: true
    max_input_size: "10MB"
    max_output_size: "50MB"
```

## Environment Variables

Key environment variables for configuration:

```bash
# Database
export DATABASE_URL="postgresql://user:pass@localhost/ratchet"

# Authentication
export JWT_SECRET="your-jwt-secret-key"
export API_KEY_1="your-api-key-here"
export MCP_API_KEY="mcp-specific-key"

# Logging
export RUST_LOG="ratchet=debug,ratchet_mcp=trace"

# Server settings
export RATCHET_HOST="0.0.0.0"
export RATCHET_PORT="8080"
export MCP_PORT="8090"

# Feature flags
export RATCHET_ENABLE_GRAPHQL="true"
export RATCHET_ENABLE_MCP="true"
```

## Security Best Practices

### 1. Network Security
- Use HTTPS in production
- Configure proper CORS policies
- Implement rate limiting
- Use firewalls to restrict access

### 2. Authentication
- Always use authentication in production
- Rotate API keys regularly
- Use strong JWT secrets
- Implement proper session management

### 3. Input Validation
- Enable input sanitization
- Set reasonable size limits
- Validate all user inputs
- Use schema validation

### 4. Monitoring
- Enable audit logging
- Monitor resource usage
- Set up alerting for failures
- Track execution patterns

## Troubleshooting

### Common Issues

#### "Database connection failed"
```bash
# Check database connectivity
psql postgresql://user:pass@localhost/ratchet

# Verify configuration
ratchet serve --config config.yaml --help
```

#### "Port already in use"
```bash
# Find process using port
lsof -i :8080

# Kill conflicting process
kill $(lsof -t -i:8080)
```

#### "Permission denied"
```bash
# Check file permissions
ls -la config.yaml

# Fix permissions
chmod 644 config.yaml
chown ratchet:ratchet config.yaml
```

#### High memory usage
```bash
# Monitor resource usage
htop

# Check database connections
netstat -an | grep :5432

# Review configuration
grep -i pool config.yaml
```

### Performance Tuning

#### Database Optimization
```yaml
database:
  pool_size: 20
  max_connections: 100
  connection_timeout: 10
  idle_timeout: 300
  prepared_statements: true
```

#### Server Optimization
```yaml
server:
  worker_threads: 8
  max_blocking_threads: 512
  keep_alive: 60
  request_timeout: 30
```

#### Memory Management
```yaml
limits:
  max_request_size: "10MB"
  max_response_size: "50MB"
  max_concurrent_connections: 1000
  task_execution_memory_limit: "1GB"
```

This configuration guide provides comprehensive coverage of Ratchet's server deployment options and should help users understand how to properly configure both the regular server and MCP server for their specific use cases.