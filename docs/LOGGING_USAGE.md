# Logging System Usage Guide

## Overview

The Ratchet logging system provides structured, contextual logging optimized for both human debugging and LLM-assisted error resolution.

## Basic Usage

### Initialize the Logger

```rust
use ratchet_lib::logging::{
    init_logger, LoggerBuilder, LogLevel,
    sinks::{ConsoleSink, FileSink},
    enrichment::{SystemEnricher, ProcessEnricher},
};
use std::sync::Arc;

// Create a console sink for development
let console_sink = Arc::new(ConsoleSink::new(LogLevel::Info));

// Create a file sink for production
let file_sink = Arc::new(
    FileSink::new("/var/log/ratchet/app.log", LogLevel::Info)
        .expect("Failed to create file sink")
        .with_rotation(100 * 1024 * 1024) // 100MB rotation
);

// Build the logger with enrichers
let logger = LoggerBuilder::new()
    .with_min_level(LogLevel::Info)
    .add_sink(console_sink)
    .add_sink(file_sink)
    .add_enricher(Box::new(SystemEnricher::new()))
    .add_enricher(Box::new(ProcessEnricher::new()))
    .build();

// Initialize global logger
init_logger(logger).expect("Failed to initialize logger");
```

### Basic Logging

```rust
use ratchet_lib::{log_event, logging::{LogLevel}};

// Simple log message
log_event!(LogLevel::Info, "Application started");

// Log with structured fields
log_event!(
    LogLevel::Info,
    "User logged in",
    "user_id" => 123,
    "ip_address" => "192.168.1.1",
    "user_agent" => "Mozilla/5.0..."
);
```

### Error Logging

```rust
use ratchet_lib::{log_error, errors::RatchetError};

// Automatically log errors with context
let error = RatchetError::TaskNotFound("weather-api".to_string());
log_error!(error);

// Log error with additional context
log_error!(
    error,
    "request_id" => "req-123",
    "user_id" => 456,
    "retry_count" => 3
);
```

### Context Propagation

```rust
use ratchet_lib::logging::{LogContext, LogEvent, LogLevel};

// Create a context for a request
let context = LogContext::new()
    .with_field("request_id", "req-789")
    .with_field("user_id", 123)
    .with_field("endpoint", "/api/tasks");

// Use context scope for async operations
let result = context.scope(async {
    // All logs within this scope will include context fields
    log_event!(LogLevel::Info, "Processing request");
    
    // Do some work...
    process_request().await
}).await;
```

## Log Output Examples

### Console Output (Development)

```
2024-01-06 12:34:56.789 INFO  User logged in [user_id=123] trace=550e8400 span=e29b41d4
2024-01-06 12:34:57.123 ERROR Task execution failed after 3 retries
  Error: Task 'weather-api' not found (TASK_NOT_FOUND)
  Type: TaskNotFound
  Retryable: false
  Suggestions:
    - Check if task 'weather-api' exists in the registry
    - Run 'ratchet list' to see available tasks
```

### JSON Output (Production)

```json
{
  "timestamp": "2024-01-06T12:34:56.789Z",
  "level": "error",
  "logger": "ratchet.execution.worker",
  "message": "Task execution failed after 3 retries",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "span_id": "e29b41d4a716",
  "fields": {
    "task_id": 123,
    "job_id": 456,
    "execution_id": 789,
    "hostname": "worker-01",
    "process_id": 12345,
    "memory_usage_mb": 256
  },
  "error": {
    "error_type": "TaskExecutionError",
    "error_code": "TASK_EXEC_001",
    "message": "Task execution failed after 3 retries",
    "severity": "high",
    "is_retryable": false,
    "context": {
      "task_name": "weather-api",
      "retry_count": 3
    },
    "suggestions": {
      "immediate": [
        "Check network connectivity to api.weather.com",
        "Verify API credentials are valid"
      ],
      "preventive": [
        "Implement circuit breaker for external API calls",
        "Add retry with exponential backoff"
      ]
    }
  }
}
```

## Advanced Features

### Custom Enrichers

```rust
use ratchet_lib::logging::{Enricher, LogEvent};

struct RequestEnricher {
    request_id: String,
}

impl Enricher for RequestEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        event.fields.insert(
            "request_id".to_string(),
            serde_json::json!(self.request_id)
        );
    }
}
```

### LLM-Friendly Error Export

```rust
use ratchet_lib::logging::ErrorInfo;

// Create detailed error information
let error_info = ErrorInfo::new(
    "DatabaseError",
    "DB_CONN_TIMEOUT",
    "Connection to database timed out"
)
.with_severity(ErrorSeverity::High)
.with_retryable(true)
.with_context_value("database", "postgres://localhost:5432/ratchet")
.with_context_value("timeout_ms", 5000)
.with_suggestion("Check database server is running")
.with_suggestion("Verify network connectivity")
.with_preventive_suggestion("Implement connection pooling");

// The error will be automatically formatted for LLM analysis
```

## Best Practices

1. **Use Structured Fields**: Instead of embedding data in messages, use fields
   ```rust
   // Bad
   log_event!(LogLevel::Info, format!("User {} logged in", user_id));
   
   // Good
   log_event!(LogLevel::Info, "User logged in", "user_id" => user_id);
   ```

2. **Add Context Early**: Set up context at the beginning of operations
   ```rust
   let context = LogContext::new()
       .with_field("operation", "task_execution")
       .with_field("task_id", task_id);
   ```

3. **Use Appropriate Log Levels**:
   - `Trace`: Very detailed debugging information
   - `Debug`: Debugging information
   - `Info`: General informational messages
   - `Warn`: Warning messages for potentially harmful situations
   - `Error`: Error messages for serious problems

4. **Include Error Context**: When logging errors, include relevant context
   ```rust
   log_error!(
       error,
       "task_id" => task_id,
       "input_size" => input.len(),
       "execution_time_ms" => duration.as_millis()
   );
   ```

5. **Use Enrichers**: Let enrichers add common fields automatically instead of manually adding them everywhere

## Configuration

```yaml
# config.yaml
logging:
  level: info
  format: json
  
  sinks:
    - type: console
      level: warn
      
    - type: file
      path: /var/log/ratchet/app.log
      rotation:
        max_size: 100MB
        max_age: 7d
  
  enrichment:
    system_info: true
    task_context: true
    
  sampling:
    error_rate: 1.0  # Log all errors
    info_rate: 0.1   # Sample 10% of info logs
```