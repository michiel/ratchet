# Ratchet MCP Configuration Guide

This guide provides comprehensive information about configuring the Ratchet MCP (Model Context Protocol) server for different deployment scenarios.

## Table of Contents

1. [Overview](#overview)
2. [Configuration Structure](#configuration-structure)
3. [Example Configurations](#example-configurations)
4. [Server Settings](#server-settings)
5. [Authentication](#authentication)
6. [Security Configuration](#security-configuration)
7. [Performance Settings](#performance-settings)
8. [Tool Configuration](#tool-configuration)
9. [Audit and Logging](#audit-and-logging)
10. [Environment Variables](#environment-variables)
11. [Best Practices](#best-practices)
12. [Troubleshooting](#troubleshooting)

## Overview

The Ratchet MCP server provides a standardized interface for Large Language Models (LLMs) to interact with the Ratchet task execution system. The configuration system supports multiple deployment scenarios from development to enterprise production environments.

## Configuration Structure

The MCP configuration is nested under the `mcp` key in the main Ratchet configuration file:

```yaml
mcp:
  enabled: true
  server: {...}
  authentication: {...}
  security: {...}
  performance: {...}
  tools: {...}
  audit: {...}
```

## Example Configurations

### Available Examples

1. **Development** (`example-mcp-dev.yaml`)
   - Minimal security for local development
   - Maximum debugging capabilities
   - High resource limits
   - stdio transport for simplicity

2. **Production** (`example-mcp-production.yaml`)
   - Comprehensive security settings
   - API key authentication
   - Network transport with TLS
   - Structured logging and monitoring

3. **Enterprise** (`example-mcp-enterprise.yaml`)
   - Multi-tenant support
   - Multiple authentication methods (JWT, OAuth2, API keys)
   - Comprehensive compliance features
   - High availability configuration

4. **Minimal** (`example-mcp-minimal.yaml`)
   - Bare minimum configuration
   - Suitable for simple deployments
   - Reduced feature set

5. **Claude Integration** (`example-mcp-claude-integration.yaml`)
   - Optimized for Claude Desktop
   - stdio transport configuration
   - LLM-specific rate limits and settings

## Server Settings

### Transport Types

- **stdio**: Standard input/output for direct integration with LLM clients
- **sse**: Server-Sent Events over HTTP/HTTPS
- **websocket**: WebSocket transport (future enhancement)

```yaml
server:
  transport: "stdio"  # or "sse"
  host: "127.0.0.1"   # ignored for stdio
  port: 3000          # ignored for stdio
  enable_cors: true
  cors_origins:
    - "https://your-domain.com"
```

### TLS Configuration

For production deployments using network transports:

```yaml
server:
  tls:
    cert_file: "/path/to/server.crt"
    key_file: "/path/to/server.key"
    ca_file: "/path/to/ca.crt"        # optional
    require_client_cert: false        # mutual TLS
```

## Authentication

### None (Development Only)

```yaml
authentication:
  method: "none"
```

**⚠️ Warning**: Only use for trusted local environments.

### API Key Authentication

```yaml
authentication:
  method: "api_key"
  api_key:
    header_name: "Authorization"
    prefix: "Bearer"
    keys:
      "your-secure-api-key-here":
        name: "Client Name"
        description: "Purpose of this key"
        permissions:
          can_execute_tasks: true
          can_read_logs: true
          can_read_traces: false
          allowed_task_patterns:
            - "allowed-*"
            - "safe-*"
          denied_task_patterns:
            - "dangerous-*"
        created_at: "2024-01-01T00:00:00Z"
        expires_at: "2024-12-31T23:59:59Z"  # optional
        active: true
        allowed_ips:                         # optional
          - "10.0.0.0/8"
```

### JWT Authentication

```yaml
authentication:
  method: "jwt"
  jwt:
    secret_or_key_file: "/path/to/public.pem"
    algorithm: "RS256"
    issuer: "https://your-sso.com"
    audience: "ratchet-mcp"
    expiration_seconds: 3600
    clock_skew_seconds: 60
```

### OAuth2 Authentication

```yaml
authentication:
  method: "oauth2"
  oauth2:
    issuer_url: "https://login.provider.com/tenant/v2.0"
    client_id: "${OAUTH_CLIENT_ID}"
    client_secret: "${OAUTH_CLIENT_SECRET}"
    required_scopes:
      - "openid"
      - "profile"
      - "ratchet.execute"
    jwks_uri: "https://login.provider.com/keys"
```

## Security Configuration

### Rate Limiting

```yaml
security:
  rate_limiting:
    global_per_minute: 1000
    execute_task_per_minute: 100
    get_logs_per_minute: 500
    get_traces_per_minute: 100
    algorithm: "token_bucket"  # or "sliding_window"
    burst_allowance: 50
```

### Request Limits

```yaml
security:
  request_limits:
    max_request_size_bytes: 10485760    # 10MB
    max_response_size_bytes: 52428800   # 50MB
    max_connections_per_ip: 100
    max_concurrent_executions_per_client: 10
    max_execution_time_seconds: 300
```

### IP Filtering

```yaml
security:
  ip_filtering:
    enabled: true
    default_policy: "deny"
    allowed_ranges:
      - "10.0.0.0/8"
      - "172.16.0.0/12"
      - "192.168.0.0/16"
    blocked_ranges:
      - "169.254.0.0/16"
    trusted_proxies:
      - "10.0.1.10"
```

### Security Headers

```yaml
security:
  headers:
    enabled: true
    content_security_policy: "default-src 'self'"
    x_frame_options: "DENY"
    x_content_type_options: "nosniff"
    strict_transport_security: "max-age=31536000"
```

### Input Validation

```yaml
security:
  validation:
    strict_schema_validation: true
    sanitize_strings: true
    max_string_length: 65536
    max_array_length: 10000
    max_object_depth: 16
```

## Performance Settings

### Connection Pooling

```yaml
performance:
  connection_pool:
    max_connections: 500
    min_idle_connections: 50
    connection_timeout_seconds: 30
    idle_timeout_seconds: 300
    max_lifetime_seconds: 3600
```

### Caching

```yaml
performance:
  caching:
    enabled: true
    max_size_mb: 1024
    default_ttl_seconds: 3600
    cache_execution_results: true
    cache_log_queries: true
```

### Background Tasks

```yaml
performance:
  background_tasks:
    worker_threads: 8
    queue_size: 10000
    health_check_interval_seconds: 30
    cleanup_interval_seconds: 300
```

### Monitoring and Alerts

```yaml
performance:
  monitoring:
    enabled: true
    collection_interval_seconds: 60
    export_enabled: true
    export_endpoint: "http://prometheus:9090/metrics"
    alerts:
      cpu_threshold: 80.0
      memory_threshold: 85.0
      connection_threshold: 400
      error_rate_threshold: 5.0
```

## Tool Configuration

```yaml
tools:
  enable_execution: true
  enable_logging: true
  enable_monitoring: true
  enable_debugging: false    # Disable in production
  enable_filesystem: false   # Disable for security
  custom_tools:
    "my_custom_tool":
      enabled: true
      some_config: "value"
  tool_rate_limits:
    execute_task: 50
    get_logs: 200
    my_custom_tool: 10
```

## Audit and Logging

### Basic Audit Configuration

```yaml
audit:
  enabled: true
  level: "info"
  log_all_requests: false
  log_auth_events: true
  log_permission_checks: false
  log_performance: true
```

### Log Rotation

```yaml
audit:
  rotation:
    max_size_mb: 100
    max_files: 30
    compress: true
```

### External Audit Destinations

```yaml
audit:
  external_destinations:
    # Syslog
    - type: "syslog"
      address: "syslog.company.com:514"
      facility: "local0"
    
    # Webhook
    - type: "webhook"
      url: "https://audit.company.com/api/logs"
      headers:
        "Content-Type": "application/json"
      auth:
        type: "bearer"
        token: "${AUDIT_TOKEN}"
    
    # Database
    - type: "database"
      connection_string: "postgresql://user:pass@host:5432/audit"
      table_name: "mcp_audit_log"
```

## Environment Variables

The configuration supports environment variable substitution using `${VAR_NAME}` syntax:

### Common Environment Variables

- `MCP_HOST`: Override server host
- `MCP_PORT`: Override server port
- `MCP_TRANSPORT`: Override transport type
- `JWT_SECRET`: JWT signing secret
- `DB_PASSWORD`: Database password
- `OAUTH_CLIENT_ID`: OAuth2 client ID
- `OAUTH_CLIENT_SECRET`: OAuth2 client secret
- `AUDIT_TOKEN`: Token for audit webhooks

### Environment Variable Overrides

Specific environment variables can override configuration values:

```bash
export MCP_HOST="0.0.0.0"
export MCP_PORT="8443"
export MCP_TRANSPORT="sse"
```

## Best Practices

### Security

1. **Never use `method: "none"` in production**
2. **Always use TLS for network transports**
3. **Implement proper IP filtering**
4. **Use strong API keys (minimum 32 characters)**
5. **Regularly rotate API keys and certificates**
6. **Enable comprehensive audit logging**

### Performance

1. **Tune connection pool sizes based on load**
2. **Enable caching for read-heavy workloads**
3. **Set appropriate rate limits**
4. **Monitor resource usage and adjust thresholds**
5. **Use appropriate worker thread counts**

### Monitoring

1. **Enable metrics export for production**
2. **Set up alerting on key thresholds**
3. **Monitor audit logs for security events**
4. **Track performance metrics over time**
5. **Implement health checks**

### Configuration Management

1. **Use version control for configuration files**
2. **Validate configurations before deployment**
3. **Use environment-specific configurations**
4. **Document any custom settings**
5. **Test configuration changes in staging**

## Troubleshooting

### Common Issues

#### Connection Refused
- Check if the correct transport is configured
- For network transports, verify host and port settings
- Check firewall rules and network connectivity

#### Authentication Failures
- Verify API key format and validity
- Check JWT token expiration and signature
- Ensure correct authentication method is configured

#### High Memory Usage
- Reduce cache size in performance settings
- Lower connection pool limits
- Check for memory leaks in custom tools

#### Rate Limiting Issues
- Adjust rate limits in security configuration
- Check burst allowance settings
- Monitor actual usage patterns

### Debug Configuration

For debugging configuration issues, enable debug logging:

```yaml
logging:
  level: debug

mcp:
  audit:
    level: "debug"
    log_all_requests: true
    log_permission_checks: true
```

### Validation

To validate your configuration before deployment:

```bash
# Check configuration syntax
ratchet config validate /path/to/config.yaml

# Test MCP server startup
ratchet mcp test-config /path/to/config.yaml
```

## Configuration Schema

For detailed schema validation and IDE support, see the JSON schema documentation at `docs/openapi.yaml`.

## Support

For additional support:

1. Check the troubleshooting section above
2. Review the example configurations
3. Consult the API documentation
4. File an issue in the project repository