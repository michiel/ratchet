---
layout: default
title: Server Configuration
permalink: /server-configuration/
---

# Server Configuration

This guide covers all configuration options for deploying and running a Ratchet server in different environments.

## Configuration File

Ratchet uses YAML configuration files. Create a `config.yaml` file with your desired settings:

```yaml
# Basic server configuration
server:
  bind_address: "127.0.0.1"
  port: 8080
  
  database:
    url: "sqlite://data/ratchet.db"
    max_connections: 10
```

## Configuration Sections

### Execution Configuration

Control how tasks are executed:

```yaml
execution:
  max_execution_duration: 300  # Maximum execution time in seconds
  validate_schemas: true       # Enable JSON schema validation
  
  # JavaScript variable names for HTTP operations
  fetch_variables:
    url_var: "__fetch_url"
    params_var: "__fetch_params"
    body_var: "__fetch_body"
    result_var: "__http_result"
    temp_result_var: "__temp_result"
```

### HTTP Client Configuration

Configure the HTTP client used by tasks:

```yaml
http:
  timeout: 30              # Request timeout in seconds
  max_redirects: 10        # Maximum redirects to follow
  user_agent: "Ratchet/1.0"
  verify_ssl: true         # SSL certificate verification
```

### Caching Configuration

Enable and configure caching:

```yaml
cache:
  task_content_cache_size: 100  # LRU cache size
  enabled: true                 # Enable/disable caching
```

### Logging Configuration

Control logging behavior:

```yaml
logging:
  level: "info"           # trace, debug, info, warn, error
  log_to_file: false      # Write logs to file
  log_file_path: null     # Log file path
```

### Output Destinations

Configure where task outputs are delivered:

```yaml
output:
  max_concurrent_deliveries: 10
  default_timeout: 30
  validate_on_startup: true
  
  default_retry_policy:
    max_attempts: 3
    initial_delay_ms: 1000
    max_delay_ms: 30000
    backoff_multiplier: 2.0
  
  global_destinations:
    - name: "production_logs"
      destination:
        type: filesystem
        path: "/var/log/ratchet/{{task_name}}_{{job_uuid}}.json"
        format: json
        permissions: "644"
        create_dirs: true
```

### Server Configuration

Core server settings:

```yaml
server:
  bind_address: "127.0.0.1"
  port: 8080
  
  database:
    url: "sqlite://data/ratchet.db"
    max_connections: 10
    connection_timeout: 30
  
  rate_limit:
    requests_per_minute: 60
    burst_size: 10
    profile: "default"  # default, permissive, strict
```

### Worker Configuration

Configure worker processes:

```yaml
server:
  workers:
    worker_count: 4
    restart_on_crash: true
    max_restart_attempts: 5
    restart_delay_seconds: 10
    health_check_interval_seconds: 30
```

### Job Queue Configuration

Configure the job queue:

```yaml
server:
  job_queue:
    max_dequeue_batch_size: 10
    max_queue_size: 1000
    default_retry_delay: 60
    default_max_retries: 3
```

### Task Registry

Configure task sources:

```yaml
registry:
  sources:
    - name: "local-tasks"
      uri: "file://./sample/js-tasks"
      config:
        watch: true
        debounce_ms: 1000
        ignore_patterns:
          - "*.tmp"
          - "node_modules/**"
```

## Environment Variables

Override configuration using environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `RATCHET_HTTP_TIMEOUT` | HTTP timeout in seconds | `60` |
| `RATCHET_CACHE_SIZE` | Task cache size | `200` |
| `RATCHET_LOG_LEVEL` | Logging level | `debug` |
| `RATCHET_DATABASE_URL` | Database connection | `sqlite://ratchet.db` |
| `RATCHET_SERVER_PORT` | Server port | `3000` |
| `RATCHET_RATE_LIMIT_REQUESTS_PER_MINUTE` | Rate limit | `120` |

## Deployment Configurations

### Development Configuration

```yaml
# development.yaml
execution:
  max_execution_duration: 60
  validate_schemas: true

logging:
  level: "debug"
  log_to_file: false

server:
  bind_address: "127.0.0.1"
  port: 8080
  database:
    url: "sqlite::memory:"
  
  workers:
    worker_count: 2

development:
  enable_debug_endpoints: true
  mock_external_services: false
  validate_on_startup: true
```

### Production Configuration

```yaml
# production.yaml
execution:
  max_execution_duration: 300
  validate_schemas: true

http:
  timeout: 30
  verify_ssl: true

logging:
  level: "info"
  log_to_file: true
  log_file_path: "/var/log/ratchet/ratchet.log"

server:
  bind_address: "0.0.0.0"  # Listen on all interfaces
  port: 8080
  
  database:
    url: "sqlite:///var/lib/ratchet/ratchet.db"
    max_connections: 50
  
  rate_limit:
    profile: "strict"
  
  workers:
    worker_count: 8
    restart_on_crash: true
    max_restart_attempts: 10
```

### High-Performance Configuration

```yaml
# high-performance.yaml
cache:
  task_content_cache_size: 500
  enabled: true

server:
  database:
    max_connections: 100
  
  job_queue:
    max_dequeue_batch_size: 50
    max_queue_size: 10000
  
  workers:
    worker_count: 16
    health_check_interval_seconds: 60
  
  rate_limit:
    profile: "permissive"

output:
  max_concurrent_deliveries: 50
```

## Security Considerations

### Network Security

1. **Bind Address**: In production, be careful with `0.0.0.0`
   ```yaml
   server:
     bind_address: "10.0.0.5"  # Specific interface
   ```

2. **SSL/TLS**: Always enable SSL verification
   ```yaml
   http:
     verify_ssl: true
   ```

3. **Rate Limiting**: Configure appropriate limits
   ```yaml
   server:
     rate_limit:
       requests_per_minute: 30
       profile: "strict"
   ```

### Database Security

1. **File Permissions**: Secure database files
   ```bash
   chmod 600 /var/lib/ratchet/ratchet.db
   chown ratchet:ratchet /var/lib/ratchet/ratchet.db
   ```

2. **Backups**: Regular database backups
   ```bash
   # Add to crontab
   0 2 * * * sqlite3 /var/lib/ratchet/ratchet.db ".backup /backup/ratchet-$(date +\%Y\%m\%d).db"
   ```

### Secrets Management

Use environment variables for sensitive data:

```yaml
output:
  global_destinations:
    - name: "webhook"
      destination:
        auth:
          type: bearer
          token: "${WEBHOOK_TOKEN}"  # From environment
```

## Monitoring and Health Checks

### Health Check Endpoint

The server provides health check endpoints:

```bash
# Basic health check
curl http://localhost:8080/health

# Detailed health check
curl http://localhost:8080/health/detailed
```

### Metrics and Monitoring

Configure logging for monitoring:

```yaml
logging:
  level: "info"
  log_to_file: true
  log_file_path: "/var/log/ratchet/ratchet.log"
```

Parse logs for metrics:
- Task execution times
- Error rates
- Worker health
- Queue sizes

## Performance Tuning

### Database Performance

1. **Connection Pool**: Size based on worker count
   ```yaml
   server:
     database:
       max_connections: 50  # 2-3x worker count
   ```

2. **SQLite Optimizations**:
   ```sql
   PRAGMA journal_mode = WAL;
   PRAGMA synchronous = NORMAL;
   PRAGMA cache_size = -64000;  -- 64MB
   ```

### Worker Tuning

1. **Worker Count**: Based on CPU cores
   ```yaml
   server:
     workers:
       worker_count: 8  # 2x CPU cores for I/O-bound tasks
   ```

2. **Queue Sizes**: Based on memory
   ```yaml
   server:
     job_queue:
       max_queue_size: 5000  # Adjust based on RAM
   ```

### Caching Strategy

1. **Task Cache**: Reduce file I/O
   ```yaml
   cache:
     task_content_cache_size: 200  # Popular tasks
   ```

2. **HTTP Caching**: For external APIs
   ```yaml
   http:
     enable_cache: true
     cache_ttl: 300  # 5 minutes
   ```

## Troubleshooting

### Common Issues

1. **Database Locked Errors**
   - Increase connection timeout
   - Check for long-running queries
   - Use WAL mode for SQLite

2. **Worker Crashes**
   - Check task memory usage
   - Review error logs
   - Increase health check interval

3. **Performance Issues**
   - Monitor queue sizes
   - Check database query performance
   - Review task execution times

### Debug Mode

Enable debug mode for troubleshooting:

```yaml
logging:
  level: "debug"

development:
  enable_debug_endpoints: true
  validate_on_startup: true
```

## Next Steps

- Explore [Integrations]({{ "/integrations" | relative_url }}) for connecting with external systems
- Learn about [Logging & Error Handling]({{ "/logging-error-handling" | relative_url }}) for monitoring