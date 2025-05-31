# Error Logging Improvement Plan

## Overview

This plan outlines the implementation of an enhanced error logging system for Ratchet that provides structured, contextual logging optimized for both human debugging and LLM-assisted error resolution.

## Goals

1. **Structured Logging**: Implement consistent JSON-formatted logs with semantic fields
2. **Contextual Information**: Capture comprehensive execution context for each error
3. **LLM Optimization**: Structure logs to facilitate automated analysis and resolution suggestions
4. **Traceability**: Enable end-to-end request tracing across distributed components
5. **Performance**: Minimize logging overhead while maximizing diagnostic value

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Logging Architecture                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────┐  │
│  │   Logger    │───▶│  Enrichment  │───▶│    Sinks        │  │
│  │  Interface  │    │   Pipeline   │    │ (File/Console/  │  │
│  └─────────────┘    └──────────────┘    │  Remote/etc)    │  │
│         │                   │            └─────────────────┘  │
│         ▼                   ▼                                  │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────┐  │
│  │   Context   │    │   Sampling   │    │   LLM Export    │  │
│  │   Capture   │    │   Strategy   │    │   Formatter     │  │
│  └─────────────┘    └──────────────┘    └─────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Log Structure

```json
{
  "timestamp": "2024-01-06T12:34:56.789Z",
  "level": "error",
  "logger": "ratchet.execution.worker",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "span_id": "e29b41d4a716",
  
  "error": {
    "type": "TaskExecutionError",
    "message": "Task execution failed after 3 retries",
    "code": "TASK_EXEC_001",
    "severity": "high",
    "is_retryable": false,
    "stack_trace": "...",
    
    "context": {
      "task_id": 123,
      "task_name": "weather-api",
      "task_version": "1.0.0",
      "job_id": 456,
      "execution_id": 789,
      "retry_count": 3,
      "input_data_sample": {"city": "London"},
      "execution_duration_ms": 5234
    },
    
    "system": {
      "hostname": "worker-01",
      "process_id": 12345,
      "memory_usage_mb": 256,
      "cpu_usage_percent": 45.2
    },
    
    "related_errors": [
      {
        "timestamp": "2024-01-06T12:34:51.123Z",
        "type": "HttpRequestError",
        "message": "Connection timeout to api.weather.com"
      }
    ],
    
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
  },
  
  "llm_context": {
    "error_pattern": "external_api_timeout",
    "relevant_config": {
      "http_timeout": 5000,
      "max_retries": 3
    },
    "recent_changes": [],
    "similar_errors_last_24h": 15
  }
}
```

## Implementation Phases

### Phase 1: Core Logging Infrastructure (Week 1-2)

#### 1.1 Logger Trait and Implementation

```rust
// ratchet-lib/src/logging/mod.rs
pub trait StructuredLogger: Send + Sync {
    fn log(&self, event: LogEvent);
    fn with_context(&self, context: LogContext) -> Box<dyn StructuredLogger>;
}

pub struct LogEvent {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub error: Option<ErrorInfo>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

pub struct ErrorInfo {
    pub error_type: String,
    pub error_code: String,
    pub message: String,
    pub severity: ErrorSeverity,
    pub is_retryable: bool,
    pub stack_trace: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub suggestions: ErrorSuggestions,
}
```

#### 1.2 Context Propagation

```rust
// ratchet-lib/src/logging/context.rs
pub struct LogContext {
    trace_id: String,
    span_id: String,
    fields: HashMap<String, serde_json::Value>,
}

impl LogContext {
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: generate_span_id(),
            fields: HashMap::new(),
        }
    }
    
    pub fn with_field(mut self, key: &str, value: impl Serialize) -> Self {
        self.fields.insert(key.to_string(), json!(value));
        self
    }
}
```

#### 1.3 Integration with Existing Error Types

```rust
// Enhance existing RatchetError
impl RatchetError {
    pub fn to_log_event(&self, context: &LogContext) -> LogEvent {
        LogEvent {
            timestamp: Utc::now(),
            level: self.severity().to_log_level(),
            message: self.to_string(),
            error: Some(self.to_error_info()),
            trace_id: Some(context.trace_id.clone()),
            span_id: Some(context.span_id.clone()),
            fields: self.get_context_fields(),
        }
    }
    
    fn to_error_info(&self) -> ErrorInfo {
        ErrorInfo {
            error_type: self.error_type(),
            error_code: self.error_code(),
            message: self.to_string(),
            severity: self.severity(),
            is_retryable: self.is_retryable(),
            stack_trace: self.backtrace(),
            context: self.get_error_context(),
            suggestions: self.get_suggestions(),
        }
    }
}
```

### Phase 2: LLM-Optimized Features (Week 3-4)

#### 2.1 Error Pattern Recognition

```rust
// ratchet-lib/src/logging/patterns.rs
pub struct ErrorPatternMatcher {
    patterns: Vec<ErrorPattern>,
}

pub struct ErrorPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub matching_rules: Vec<MatchingRule>,
    pub suggestions: Vec<String>,
    pub related_documentation: Vec<String>,
}

impl ErrorPatternMatcher {
    pub fn match_error(&self, error: &ErrorInfo) -> Option<&ErrorPattern> {
        self.patterns.iter()
            .find(|p| p.matches(error))
    }
}
```

#### 2.2 Contextual Information Enrichment

```rust
// ratchet-lib/src/logging/enrichment.rs
pub struct LogEnricher {
    enrichers: Vec<Box<dyn Enricher>>,
}

pub trait Enricher: Send + Sync {
    fn enrich(&self, event: &mut LogEvent);
}

pub struct SystemEnricher;
impl Enricher for SystemEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        event.fields.insert("hostname".to_string(), json!(hostname()));
        event.fields.insert("process_id".to_string(), json!(process::id()));
        event.fields.insert("memory_usage_mb".to_string(), json!(get_memory_usage()));
    }
}

pub struct TaskContextEnricher {
    task_cache: Arc<TaskCache>,
}

impl Enricher for TaskContextEnricher {
    fn enrich(&self, event: &mut LogEvent) {
        if let Some(task_id) = event.fields.get("task_id") {
            if let Some(task) = self.task_cache.get(task_id) {
                event.fields.insert("task_name".to_string(), json!(task.name));
                event.fields.insert("task_version".to_string(), json!(task.version));
            }
        }
    }
}
```

#### 2.3 LLM Export Format

```rust
// ratchet-lib/src/logging/llm_export.rs
pub struct LLMExporter {
    include_system_context: bool,
    include_similar_errors: bool,
    max_context_size: usize,
}

impl LLMExporter {
    pub fn export_for_analysis(&self, error: &LogEvent) -> LLMErrorReport {
        LLMErrorReport {
            error_summary: self.create_summary(error),
            execution_context: self.extract_execution_context(error),
            system_state: self.capture_system_state(),
            recent_operations: self.get_recent_operations(error.trace_id.as_ref()),
            similar_errors: self.find_similar_errors(error),
            relevant_code_context: self.extract_code_context(error),
            suggested_prompts: self.generate_analysis_prompts(error),
        }
    }
    
    fn generate_analysis_prompts(&self, error: &LogEvent) -> Vec<String> {
        vec![
            format!("Analyze this {} error in a task execution system", error.error.as_ref().map(|e| &e.error_type).unwrap_or(&"Unknown".to_string())),
            "What are the most likely root causes?".to_string(),
            "Suggest specific code changes to prevent this error".to_string(),
            "Is this error part of a larger pattern?".to_string(),
        ]
    }
}
```

### Phase 3: Storage and Query (Week 5-6)

#### 3.1 Log Storage Backend

```rust
// ratchet-lib/src/logging/storage.rs
pub trait LogStorage: Send + Sync {
    async fn store(&self, event: LogEvent) -> Result<(), LogStorageError>;
    async fn query(&self, query: LogQuery) -> Result<Vec<LogEvent>, LogStorageError>;
    async fn aggregate(&self, aggregation: LogAggregation) -> Result<AggregationResult, LogStorageError>;
}

pub struct SqliteLogStorage {
    connection: Arc<DatabaseConnection>,
}

pub struct LogQuery {
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub trace_id: Option<String>,
    pub error_type: Option<String>,
    pub severity: Option<ErrorSeverity>,
    pub task_id: Option<i32>,
    pub limit: usize,
}
```

#### 3.2 Log Aggregation for Patterns

```rust
// ratchet-lib/src/logging/aggregation.rs
pub struct ErrorAggregator {
    storage: Arc<dyn LogStorage>,
}

impl ErrorAggregator {
    pub async fn get_error_trends(&self, window: Duration) -> Result<ErrorTrends> {
        let aggregation = LogAggregation {
            group_by: vec!["error_type".to_string()],
            time_bucket: Some(Duration::from_secs(3600)), // 1 hour buckets
            metrics: vec![
                AggregationMetric::Count,
                AggregationMetric::UniqueTraceIds,
            ],
        };
        
        let results = self.storage.aggregate(aggregation).await?;
        Ok(self.analyze_trends(results))
    }
    
    pub async fn find_correlated_errors(&self, error: &LogEvent) -> Result<Vec<CorrelatedError>> {
        // Find errors that frequently occur together
        let query = LogQuery {
            time_range: Some((error.timestamp - Duration::from_secs(300), error.timestamp)),
            trace_id: error.trace_id.clone(),
            ..Default::default()
        };
        
        let related = self.storage.query(query).await?;
        Ok(self.analyze_correlations(error, related))
    }
}
```

### Phase 4: Integration and Tooling (Week 7-8)

#### 4.1 REST API Endpoints

```rust
// ratchet-lib/src/rest/handlers/logs.rs
pub async fn get_error_analysis(
    Path(error_id): Path<String>,
    State(ctx): State<LogsContext>,
) -> Result<Json<ErrorAnalysis>, RestError> {
    let error = ctx.log_storage.get_by_id(&error_id).await?;
    let exporter = LLMExporter::new();
    let analysis = exporter.export_for_analysis(&error);
    
    Ok(Json(ErrorAnalysis {
        error,
        llm_context: analysis,
        similar_errors: ctx.aggregator.find_similar_errors(&error).await?,
        suggested_fixes: ctx.pattern_matcher.get_suggestions(&error),
    }))
}

pub async fn get_error_trends(
    Query(params): Query<TrendParams>,
    State(ctx): State<LogsContext>,
) -> Result<Json<ErrorTrends>, RestError> {
    let trends = ctx.aggregator.get_error_trends(params.window).await?;
    Ok(Json(trends))
}
```

#### 4.2 CLI Integration

```rust
// ratchet-cli/src/commands/logs.rs
pub struct LogsCommand {
    #[clap(subcommand)]
    command: LogsSubcommand,
}

#[derive(Subcommand)]
pub enum LogsSubcommand {
    /// Analyze a specific error
    Analyze {
        #[clap(help = "Error ID or trace ID")]
        id: String,
        
        #[clap(long, help = "Export for LLM analysis")]
        llm_format: bool,
    },
    
    /// Show error trends
    Trends {
        #[clap(long, default_value = "1h")]
        window: String,
        
        #[clap(long)]
        by_task: bool,
    },
    
    /// Follow logs in real-time
    Tail {
        #[clap(long)]
        error_only: bool,
        
        #[clap(long)]
        task_id: Option<i32>,
    },
}
```

#### 4.3 Development Tools

```rust
// tools/log-analyzer/src/main.rs
/// Standalone tool for log analysis and LLM integration
pub struct LogAnalyzer {
    storage: Arc<dyn LogStorage>,
    llm_client: Option<LLMClient>,
}

impl LogAnalyzer {
    pub async fn analyze_error_pattern(&self, pattern: &str) -> Result<PatternAnalysis> {
        let errors = self.storage.query(LogQuery {
            error_type: Some(pattern.to_string()),
            limit: 100,
            ..Default::default()
        }).await?;
        
        if let Some(llm) = &self.llm_client {
            let context = self.prepare_llm_context(&errors);
            let analysis = llm.analyze(context).await?;
            Ok(PatternAnalysis {
                pattern,
                occurrences: errors.len(),
                llm_insights: Some(analysis),
                suggested_fixes: self.extract_fixes(&analysis),
            })
        } else {
            Ok(self.basic_analysis(&errors))
        }
    }
}
```

## Configuration

### Logging Configuration

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
        
    - type: database
      table: error_logs
      buffer_size: 1000
      flush_interval: 5s
  
  enrichment:
    system_info: true
    task_context: true
    execution_history: true
    
  llm_export:
    enabled: true
    include_system_context: true
    max_context_size: 8192
    
  sampling:
    error_rate: 1.0  # Log all errors
    info_rate: 0.1   # Sample 10% of info logs
    trace_rate: 0.01 # Sample 1% of traces
```

## Migration Strategy

### Step 1: Add New Logging Infrastructure (Non-Breaking)
- Implement new logging traits and structures
- Add parallel logging to existing logs
- No changes to existing error handling

### Step 2: Gradual Migration
- Update error sites one module at a time
- Add context propagation to key paths
- Maintain backward compatibility

### Step 3: Deprecate Old Logging
- Mark old logging methods as deprecated
- Provide migration guide
- Set removal timeline

### Step 4: Full Cutover
- Remove old logging code
- Update all documentation
- Release major version

## Success Metrics

1. **Error Resolution Time**: 50% reduction in MTTR
2. **LLM Integration**: 80% of errors have actionable LLM suggestions
3. **Pattern Detection**: Identify recurring issues within 1 hour
4. **Context Completeness**: 95% of errors have full execution context
5. **Performance Impact**: <1% overhead from enhanced logging

## Security Considerations

1. **PII Handling**: Implement automatic PII detection and redaction
2. **Log Access**: Role-based access control for sensitive logs
3. **Export Controls**: Sanitization before LLM export
4. **Retention**: Configurable retention policies
5. **Encryption**: At-rest encryption for stored logs

## Future Enhancements

1. **Distributed Tracing**: OpenTelemetry integration
2. **ML-Based Anomaly Detection**: Automatic error pattern learning
3. **Auto-Remediation**: Suggested fixes can be automatically applied
4. **Multi-Tenant Support**: Isolated logging per tenant
5. **Real-time Alerting**: Proactive error notification

## Timeline

- **Weeks 1-2**: Core logging infrastructure
- **Weeks 3-4**: LLM optimization features  
- **Weeks 5-6**: Storage and query implementation
- **Weeks 7-8**: Integration and tooling
- **Week 9**: Testing and documentation
- **Week 10**: Rollout and monitoring