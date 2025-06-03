---
layout: default
title: Logging & Error Handling
permalink: /logging-error-handling/
---

# Logging & Error Handling

Ratchet provides comprehensive logging and error handling capabilities designed for both human debugging and automated analysis. This guide covers configuration, usage patterns, and best practices.

## Logging System Overview

The Ratchet logging system features:
- Structured logging with contextual information
- Multiple log sinks (console, file, custom)
- Automatic error enrichment
- Pattern matching for common errors
- LLM-ready export formats

## Configuration

### Basic Logging Configuration

```yaml
logging:
  level: "info"           # trace, debug, info, warn, error
  log_to_file: false      
  log_file_path: null     
```

### Advanced Logging Configuration

```yaml
logging:
  level: "info"
  log_to_file: true
  log_file_path: "/var/log/ratchet/ratchet.log"
  
  # File rotation settings
  file_rotation:
    max_size_mb: 100
    max_files: 10
    compress: true
  
  # Structured logging
  format: "json"  # json or text
  
  # Context enrichment
  enrich_with:
    - system_info
    - process_info
    - execution_context
```

## Log Levels

| Level | Usage | Example |
|-------|-------|---------|
| `trace` | Very detailed debugging | Function entry/exit |
| `debug` | Debugging information | Variable values |
| `info` | General information | Task started/completed |
| `warn` | Warning conditions | Deprecated features |
| `error` | Error conditions | Task failures |

## Structured Logging

### Task Execution Logs

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "info",
  "message": "Task execution started",
  "fields": {
    "task_id": "weather-api",
    "job_id": "550e8400-e29b-41d4-a716",
    "input": {
      "city": "London",
      "units": "metric"
    },
    "execution_id": "exec_123",
    "worker_id": "worker_01"
  }
}
```

### Error Logs with Context

```json
{
  "timestamp": "2024-01-15T10:30:46.456Z",
  "level": "error",
  "message": "Task execution failed",
  "fields": {
    "task_id": "weather-api",
    "job_id": "550e8400-e29b-41d4-a716",
    "error_type": "NetworkError",
    "error_message": "Connection timeout",
    "stack_trace": "...",
    "retry_count": 2,
    "max_retries": 3
  }
}
```

## Error Handling

### Error Types

Ratchet defines specific error types for different scenarios:

| Error Type | Description | HTTP Status |
|------------|-------------|-------------|
| `ValidationError` | Invalid input data | 400 |
| `NotFoundError` | Resource not found | 404 |
| `NetworkError` | Network-related failures | 502 |
| `DataError` | Invalid data format | 422 |
| `RuntimeError` | General runtime errors | 500 |
| `TimeoutError` | Operation timeout | 504 |
| `AuthenticationError` | Authentication failed | 401 |
| `AuthorizationError` | Authorization failed | 403 |

### Error Response Format

```json
{
  "error": {
    "type": "ValidationError",
    "message": "Invalid input: city is required",
    "code": "VALIDATION_FAILED",
    "details": {
      "field": "city",
      "constraint": "required"
    },
    "trace_id": "req_123456",
    "timestamp": "2024-01-15T10:30:45.123Z"
  }
}
```

### Task Error Handling

In JavaScript tasks, use typed errors:

```javascript
(function(input) {
    // Validation errors
    if (!input.city) {
        throw new ValidationError("City is required");
    }
    
    try {
        const response = fetch(url);
        
        // Network errors
        if (!response.ok) {
            throw new NetworkError(`API returned ${response.status}`);
        }
        
        // Data errors
        const data = response.body;
        if (!data.weather) {
            throw new DataError("Invalid weather data format");
        }
        
        return data;
        
    } catch (error) {
        // Re-throw typed errors
        if (error.name && error.name.endsWith('Error')) {
            throw error;
        }
        // Wrap unknown errors
        throw new RuntimeError(`Unexpected error: ${error.message}`);
    }
})
```

## Pattern Matching

Ratchet includes built-in pattern matching for common errors:

### Database Patterns

```yaml
patterns:
  - name: "database_locked"
    regex: "database is locked"
    error_type: "DatabaseError"
    suggestion: "Increase connection timeout or use WAL mode"
    
  - name: "connection_pool_exhausted"
    regex: "no connections available"
    error_type: "DatabaseError"
    suggestion: "Increase max_connections in configuration"
```

### Network Patterns

```yaml
patterns:
  - name: "dns_resolution_failed"
    regex: "failed to resolve DNS"
    error_type: "NetworkError"
    suggestion: "Check DNS configuration and network connectivity"
    
  - name: "ssl_certificate_error"
    regex: "certificate verify failed"
    error_type: "NetworkError"
    suggestion: "Update CA certificates or disable SSL verification for development"
```

## Debugging Tools

### Debug Endpoints

Enable debug endpoints for development:

```yaml
development:
  enable_debug_endpoints: true
```

Access debug information:
```bash
# View current configuration
GET /debug/config

# View worker status
GET /debug/workers

# View job queue status
GET /debug/queue

# View recent errors
GET /debug/errors?limit=50
```

### Execution Traces

Enable detailed execution traces:

```yaml
logging:
  level: "trace"
  trace_executions: true
```

Example trace output:
```
[TRACE] Task execution pipeline started
[TRACE] Loading task: weather-api
[TRACE] Validating input schema
[TRACE] Input validation passed
[TRACE] Creating worker process
[TRACE] Sending IPC message to worker
[TRACE] Worker acknowledged task
[TRACE] Executing JavaScript code
[TRACE] HTTP request: GET https://api.weather.com
[TRACE] HTTP response: 200 OK (245ms)
[TRACE] Validating output schema
[TRACE] Task completed successfully
```

## Log Analysis

### Searching Logs

Use grep or ripgrep for log analysis:

```bash
# Find all errors for a specific task
rg "task_id.*weather-api.*level.*error" /var/log/ratchet/

# Find slow executions (>5 seconds)
rg "execution_duration_ms.*[5-9][0-9]{3,}" /var/log/ratchet/

# Find all network errors
rg "error_type.*NetworkError" /var/log/ratchet/
```

### Log Aggregation

Parse JSON logs for analysis:

```python
import json
import sys

# Count errors by type
error_counts = {}
for line in sys.stdin:
    try:
        log = json.loads(line)
        if log.get('level') == 'error':
            error_type = log.get('fields', {}).get('error_type', 'Unknown')
            error_counts[error_type] = error_counts.get(error_type, 0) + 1
    except:
        pass

for error_type, count in sorted(error_counts.items()):
    print(f"{error_type}: {count}")
```

## LLM Integration

### Export Logs for Analysis

Generate LLM-ready reports:

```bash
# Export recent errors
ratchet-cli logs export --format llm --since "1 hour ago" > error_report.md
```

Example LLM report format:
```markdown
# Ratchet Error Analysis Report

## Summary
- Time Range: 2024-01-15 09:30:00 - 10:30:00
- Total Errors: 23
- Affected Tasks: weather-api (15), user-sync (8)

## Critical Errors

### NetworkError in weather-api
- Count: 15
- Pattern: Connection timeout after 30s
- Suggested Fix: Increase HTTP timeout or implement retry logic
- Example Stack Trace:
  ```
  NetworkError: Connection timeout
    at fetch (internal)
    at handler (weather-api/main.js:15:20)
  ```

### DataError in user-sync
- Count: 8
- Pattern: Invalid JSON response
- Suggested Fix: Add response validation before parsing
```

### AI-Assisted Debugging

Use the structured logs with AI tools:

```bash
# Generate debugging suggestions
cat recent_errors.json | llm "Analyze these Ratchet errors and suggest fixes"

# Pattern analysis
ratchet-cli logs analyze --ai --pattern "timeout|connection"
```

## Monitoring Integration

### Prometheus Metrics

Export log-based metrics:

```prometheus
# Error rate by task
ratchet_errors_total{task="weather-api",error_type="NetworkError"} 15
ratchet_errors_total{task="user-sync",error_type="DataError"} 8

# Execution duration histogram
ratchet_execution_duration_seconds_bucket{task="weather-api",le="0.1"} 45
ratchet_execution_duration_seconds_bucket{task="weather-api",le="0.5"} 89
ratchet_execution_duration_seconds_bucket{task="weather-api",le="1.0"} 95
```

### Alert Rules

Example Prometheus alert rules:

```yaml
groups:
  - name: ratchet_alerts
    rules:
    - alert: HighErrorRate
      expr: rate(ratchet_errors_total[5m]) > 0.1
      annotations:
        summary: "High error rate for task {{ $labels.task }}"
        
    - alert: SlowExecution
      expr: ratchet_execution_duration_seconds{quantile="0.95"} > 5
      annotations:
        summary: "Slow execution for task {{ $labels.task }}"
```

## Best Practices

### 1. Use Structured Logging

Always include relevant context:

```javascript
console.log("Processing user", {
    user_id: user.id,
    action: "update",
    fields_changed: ["email", "name"]
});
```

### 2. Handle Errors Gracefully

Provide meaningful error messages:

```javascript
if (!config.api_key) {
    throw new ValidationError(
        "API key is required. Set it in the configuration or environment variables."
    );
}
```

### 3. Use Appropriate Log Levels

- `trace`: Function entry/exit, detailed debugging
- `debug`: Variable values, state changes
- `info`: Normal operations, milestones
- `warn`: Recoverable issues, deprecations
- `error`: Failures requiring attention

### 4. Include Request Context

Track requests across the system:

```javascript
const context = {
    request_id: generateRequestId(),
    user_id: getUserId(),
    timestamp: new Date().toISOString()
};

console.log("Starting request", context);
```

### 5. Monitor Key Metrics

Track important metrics:
- Task execution time
- Error rates by type
- Queue sizes
- Worker health

## Troubleshooting Common Issues

### High Memory Usage

Check for memory leaks in logs:
```bash
rg "memory_usage_mb.*[0-9]{4,}" /var/log/ratchet/
```

### Database Lock Errors

Look for concurrent access patterns:
```bash
rg "database is locked" /var/log/ratchet/ -B 5 -A 5
```

### Network Timeouts

Analyze timeout patterns:
```bash
rg "timeout|timed out" /var/log/ratchet/ | \
  jq -r '.fields.task_id' | sort | uniq -c
```

## Next Steps

- Configure [Server Settings]({{ "/server-configuration" | relative_url }}) for optimal logging
- Set up [Integrations]({{ "/integrations" | relative_url }}) with monitoring tools
- Review [Architecture]({{ "/architecture" | relative_url }}) for system observability