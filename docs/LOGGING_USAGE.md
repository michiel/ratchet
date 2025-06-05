# Logging System Usage Guide

## Overview

The Ratchet logging system provides structured, contextual logging optimized for both human debugging and LLM-assisted error resolution. It includes advanced pattern matching, automated error analysis, and AI-ready export formats.

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

## LLM Report Examples

### Generated Error Analysis Report

```markdown
# Error Analysis Report

**Generated**: 2024-01-06 12:34:56 UTC
**Trace ID**: trace-123-456

## Error Summary

- **Type**: DatabaseError
- **Code**: DB_CONN_ERROR
- **Message**: Connection timeout after 5s
- **Severity**: High
- **Retryable**: true

## Execution Context

- **Task**: data-processor (v1.0.0)
- **Job ID**: 789
- **Duration**: 5100ms

## Matched Error Patterns

### Database Connection Timeout (85% confidence)

**Suggestions**:
- Check database server is running and accessible
- Verify network connectivity to database host
- Check firewall rules allow database port

**Common Causes**:
- Database server down or overloaded
- Network issues between application and database
- Incorrect connection string or credentials

## System State

- **Host**: worker-01
- **Memory**: 512MB
- **CPU**: 25.5%

## Suggested Analysis Questions

1. Analyze this DatabaseError error in a task execution system
2. What are the most likely root causes based on the error context?
3. How should we handle database connection timeout errors in a distributed system?
4. How can we prevent this high-severity error from recurring?
5. Based on the execution context, is there a pattern or systemic issue?
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

## LLM-Powered Error Analysis

### Pattern Matching

The logging system includes built-in error patterns for common scenarios:

```rust
use ratchet_lib::logging::{ErrorPatternMatcher, ErrorInfo};

// Create a pattern matcher with default patterns
let matcher = ErrorPatternMatcher::with_defaults();

// Match an error against patterns
let error = ErrorInfo::new("TaskNotFound", "TASK_NOT_FOUND", "Task 'weather-api' not found");
if let Some(pattern) = matcher.match_error(&error) {
    println!("Matched pattern: {}", pattern.name);
    println!("Suggestions: {:?}", pattern.suggestions);
}
```

### Custom Error Patterns

```rust
use ratchet_lib::logging::{ErrorPattern, ErrorCategory, MatchingRule};

let custom_pattern = ErrorPattern {
    id: "api_timeout".to_string(),
    name: "API Timeout".to_string(),
    description: "External API request timeout".to_string(),
    category: ErrorCategory::Network,
    matching_rules: vec![
        MatchingRule::All { rules: vec![
            MatchingRule::ErrorType { value: "NetworkError".to_string() },
            MatchingRule::MessagePattern { pattern: r"(?i)timeout".to_string() },
        ]},
    ],
    suggestions: vec![
        "Check API endpoint health".to_string(),
        "Increase timeout configuration".to_string(),
    ],
    preventive_measures: vec![
        "Implement circuit breaker".to_string(),
        "Add retry with exponential backoff".to_string(),
    ],
    related_documentation: vec![],
    common_causes: vec![
        "Network latency".to_string(),
        "Overloaded API server".to_string(),
    ],
    llm_prompts: vec![
        "How to handle API timeouts in microservices?".to_string(),
    ],
};
```

### LLM Export for AI Analysis

```rust
use ratchet_lib::logging::{LLMExporter, LLMExportConfig, format_markdown_report};

// Configure LLM export
let config = LLMExportConfig {
    include_system_context: true,
    include_similar_errors: true,
    max_context_size: 8192,
    include_prompts: true,
    ..Default::default()
};

let exporter = LLMExporter::new(config);

// Export error for LLM analysis
if let Some(report) = exporter.export_for_analysis(&log_event) {
    // Generate JSON for API
    let json = serde_json::to_string_pretty(&report)?;
    
    // Generate Markdown for human reading
    let markdown = format_markdown_report(&report);
    
    println!("Suggested analysis questions:");
    for prompt in &report.suggested_prompts {
        println!("- {}", prompt);
    }
}
```

### Built-in Error Patterns

The system includes patterns for:

- **Database Connection Timeouts**: Matches DB connection failures
- **Task Not Found**: Identifies missing task references
- **HTTP Timeouts**: Detects network request timeouts
- **Rate Limiting**: Recognizes rate limit exceeded errors

Each pattern provides:
- Immediate action suggestions
- Preventive measures
- Common root causes
- LLM-specific analysis prompts

## Configuration

### Complete Configuration Example

The logging system can be fully configured via YAML configuration files with environment variable overrides. Here's a comprehensive example:

```yaml
# sample/configs/example-config.yaml - Complete logging configuration
logging:
  # Global logging level (trace, debug, info, warn, error)
  level: info
  
  # Configure multiple sinks for different output targets
  sinks:
    # Console sink for development and debugging
    - type: console
      level: debug
      format: colored  # Options: colored, plain, json
      enabled: true
      
    # File sink for persistent logging
    - type: file
      level: info
      path: logs/ratchet.log
      format: json  # Always JSON for file sinks
      enabled: true
      rotation:
        # Rotate when file reaches 100MB
        max_size: 100MB
        # Keep maximum 10 rotated files
        max_files: 10
        # Optional: rotate daily regardless of size
        # max_age: 24h
      
    # Buffered sink for high-performance logging
    - type: buffer
      # Buffer wraps another sink (usually file)
      inner_sink:
        type: file
        path: logs/ratchet-buffered.log
        level: info
        format: json
      # Buffer configuration
      buffer_size: 10000        # Buffer up to 10k events
      flush_interval: 5s        # Flush every 5 seconds
      flush_on_error: true      # Immediately flush on errors
      enabled: true
      
    # Future: Database sink for centralized logging
    # - type: database
    #   connection_string: postgres://user:pass@localhost/logs
    #   table_name: log_events
    #   level: warn
    #   enabled: false
  
  # Log enrichment configuration
  enrichment:
    enabled: true
    # Add timestamp to all events (always enabled)
    add_timestamp: true
    # Add hostname to all events
    add_hostname: true
    # Add process information (PID, thread)
    add_process_info: true
    # Add memory usage information
    add_memory_info: true
    # Add git commit hash if available
    add_git_info: false
    # Custom fields to add to all events
    custom_fields:
      service: ratchet
      environment: production
      version: "1.0.0"
  
  # Error pattern matching configuration
  patterns:
    enabled: true
    # Confidence threshold for pattern matches (0.0-1.0)
    match_threshold: 0.8
    # Performance: cache pattern match results
    enable_caching: true
    cache_size: 1000
    # Built-in patterns (all enabled by default)
    builtin_patterns:
      database_errors: true
      network_errors: true
      task_failures: true
      auth_failures: true
      rate_limiting: true
      resource_exhaustion: true
    # Custom patterns (see Custom Patterns section)
    custom_patterns: []
  
  # LLM export configuration
  llm_export:
    enabled: true
    # Maximum tokens to include in LLM context
    max_context_tokens: 8000
    # Include system state in exports
    include_system_state: true
    # Include related log events
    include_related_events: true
    max_related_events: 10
    # Include pattern analysis
    include_pattern_analysis: true
    # Include suggested prompts
    include_suggested_prompts: true
    # Export format (json, markdown)
    export_format: markdown
    # Summarization settings
    summarization:
      enabled: true
      # Summarize events older than 1 hour
      age_threshold: 1h
      # Maximum events to include before summarizing
      max_events_before_summary: 100
  
  # Performance and sampling configuration
  performance:
    # Maximum events per second before dropping
    max_events_per_second: 10000
    # Sampling rates by log level (0.0-1.0)
    sampling_rates:
      trace: 0.01    # Sample 1% of trace logs
      debug: 0.1     # Sample 10% of debug logs
      info: 0.5      # Sample 50% of info logs
      warn: 1.0      # Log all warnings
      error: 1.0     # Log all errors
    # Drop logs instead of blocking when buffers are full
    drop_on_full_buffer: true
    # Queue size for async processing
    async_queue_size: 1000
  
  # Context propagation settings
  context:
    # Enable automatic context propagation
    enabled: true
    # Maximum context lifetime
    max_lifetime: 1h
    # Automatically generate trace/span IDs
    auto_generate_ids: true
    # Context fields to propagate
    propagate_fields:
      - request_id
      - user_id
      - session_id
      - operation
    # Maximum context size to prevent memory leaks
    max_context_size: 1024
```

### Environment Variable Overrides

You can override any configuration value using environment variables with the `RATCHET_` prefix:

```bash
# Override global logging level
export RATCHET_LOGGING_LEVEL=debug

# Override console sink level
export RATCHET_LOGGING_SINKS_0_LEVEL=trace

# Override file sink path
export RATCHET_LOGGING_SINKS_1_PATH=/custom/log/path.log

# Disable pattern matching
export RATCHET_LOGGING_PATTERNS_ENABLED=false

# Override LLM export settings
export RATCHET_LOGGING_LLM_EXPORT_MAX_CONTEXT_TOKENS=4000
export RATCHET_LOGGING_LLM_EXPORT_INCLUDE_SYSTEM_STATE=false
```

### Development Configuration

For local development, use this simpler configuration:

```yaml
# sample/configs/dev-config.yaml
logging:
  level: debug
  sinks:
    - type: console
      level: debug
      format: colored
      enabled: true
  enrichment:
    enabled: true
    add_hostname: false  # Skip hostname in dev
    add_process_info: true
  patterns:
    enabled: true
  llm_export:
    enabled: true
    max_context_tokens: 4000
```

### Production Configuration

For production environments:

```yaml
# sample/configs/prod-config.yaml
logging:
  level: info
  sinks:
    # Minimal console output in production
    - type: console
      level: error
      format: json
      enabled: true
      
    # Primary structured logging to file
    - type: buffer
      inner_sink:
        type: file
        path: /var/log/ratchet/app.log
        level: info
        format: json
        rotation:
          max_size: 500MB
          max_files: 20
      buffer_size: 50000
      flush_interval: 2s
      flush_on_error: true
      enabled: true
      
    # Error-only file for quick troubleshooting
    - type: file
      path: /var/log/ratchet/errors.log
      level: error
      format: json
      rotation:
        max_size: 100MB
        max_files: 10
      enabled: true
  
  enrichment:
    enabled: true
    add_hostname: true
    add_process_info: true
    add_memory_info: true
    custom_fields:
      service: ratchet
      environment: production
      datacenter: us-east-1
  
  patterns:
    enabled: true
    match_threshold: 0.9  # Higher threshold for production
    enable_caching: true
    cache_size: 5000
  
  llm_export:
    enabled: true
    max_context_tokens: 8000
    include_system_state: true
  
  performance:
    max_events_per_second: 50000
    sampling_rates:
      trace: 0.001  # Very low sampling for trace
      debug: 0.01   # Low sampling for debug
      info: 0.2     # 20% of info logs
      warn: 1.0     # All warnings
      error: 1.0    # All errors
    drop_on_full_buffer: true
```

### Custom Patterns Configuration

Add custom error patterns to enhance pattern matching:

```yaml
logging:
  patterns:
    enabled: true
    custom_patterns:
      # Custom pattern for API timeouts
      - id: custom_api_timeout
        name: "Custom API Timeout"
        description: "Timeout calling external APIs"
        category: network
        matching_rules:
          - type: message_regex
            pattern: "(?i)api.*timeout|external.*service.*timeout"
          - type: field_contains
            field: error_code
            value: "TIMEOUT"
        suggestions:
          - "Check API endpoint health"
          - "Review timeout configuration"
          - "Implement circuit breaker pattern"
        severity_multiplier: 1.3
        auto_resolve: true
        
      # Pattern for business logic errors
      - id: business_rule_violation
        name: "Business Rule Violation"
        description: "Business logic validation failure"
        category: business
        matching_rules:
          - type: error_type
            value: "BusinessRuleViolation"
        suggestions:
          - "Review business rule implementation"
          - "Check input data validation"
          - "Verify business rule configuration"
        severity_multiplier: 0.8
        auto_resolve: false
```

### Container Configuration

For Docker/Kubernetes deployments:

```yaml
# sample/configs/container-config.yaml
logging:
  level: info
  sinks:
    # Log to stdout for container log collection
    - type: console
      level: info
      format: json  # Structured logs for log aggregation
      enabled: true
  
  enrichment:
    enabled: true
    add_hostname: true
    add_process_info: true
    custom_fields:
      service: ratchet
      # Container environment variables
      container_id: "${HOSTNAME}"
      pod_name: "${POD_NAME}"
      namespace: "${POD_NAMESPACE}"
      node_name: "${NODE_NAME}"
  
  # Optimized for container environments
  performance:
    max_events_per_second: 20000
    drop_on_full_buffer: true
    async_queue_size: 2000
  
  patterns:
    enabled: true
    # Reduced caching in containers
    enable_caching: true
    cache_size: 1000
  
  llm_export:
    enabled: true
    # Smaller context for container environments
    max_context_tokens: 6000
```

### Using Configuration in Code

```rust
use ratchet_lib::{RatchetConfig, logging::init_from_config};

// Load configuration from file
let config = RatchetConfig::from_file("config.yaml")?;

// Initialize logging from configuration
init_from_config(&config.logging)?;

// Alternatively, load with environment overrides
let config = RatchetConfig::from_file_with_env("config.yaml")?;
init_from_config(&config.logging)?;
```

### Configuration Validation

The system validates configuration at startup and provides helpful error messages:

```rust
// This will fail with a clear error message
let invalid_config = r#"
logging:
  level: invalid_level  # Error: invalid log level
  sinks:
    - type: file
      path: ""  # Error: empty file path
"#;

match RatchetConfig::from_str(invalid_config) {
    Ok(_) => println!("Configuration valid"),
    Err(e) => {
        eprintln!("Configuration error: {}", e);
        // Error: Invalid log level 'invalid_level'. Valid levels are: trace, debug, info, warn, error
        // Error: File sink path cannot be empty
    }
}
```