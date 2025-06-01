use super::{LogEvent, ErrorInfo, patterns::ErrorPatternMatcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Configuration for LLM export
#[derive(Debug, Clone)]
pub struct LLMExportConfig {
    /// Include system context (CPU, memory, etc.)
    pub include_system_context: bool,
    
    /// Include similar errors from history
    pub include_similar_errors: bool,
    
    /// Maximum context size in tokens (approximate)
    pub max_context_size: usize,
    
    /// Time window for related errors
    pub related_errors_window: Duration,
    
    /// Include suggested analysis prompts
    pub include_prompts: bool,
}

impl Default for LLMExportConfig {
    fn default() -> Self {
        Self {
            include_system_context: true,
            include_similar_errors: true,
            max_context_size: 8192,
            related_errors_window: Duration::hours(1),
            include_prompts: true,
        }
    }
}

/// LLM-optimized error report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMErrorReport {
    /// Summary of the error
    pub error_summary: ErrorSummary,
    
    /// Execution context
    pub execution_context: ExecutionContext,
    
    /// System state at time of error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_state: Option<SystemState>,
    
    /// Recent operations leading to error
    pub recent_operations: Vec<Operation>,
    
    /// Similar errors for pattern analysis
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub similar_errors: Vec<SimilarError>,
    
    /// Matched error patterns
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matched_patterns: Vec<MatchedPattern>,
    
    /// Relevant code or configuration context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevant_context: Option<RelevantContext>,
    
    /// Suggested analysis prompts
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggested_prompts: Vec<String>,
    
    /// Metadata for LLM processing
    pub metadata: LLMMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub error_type: String,
    pub error_code: String,
    pub message: String,
    pub severity: String,
    pub occurred_at: DateTime<Utc>,
    pub is_retryable: bool,
    pub retry_count: Option<u32>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub task_name: Option<String>,
    pub task_version: Option<String>,
    pub job_id: Option<i32>,
    pub execution_id: Option<i32>,
    pub input_data_summary: Option<serde_json::Value>,
    pub execution_duration_ms: Option<u64>,
    pub execution_phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub hostname: String,
    pub process_id: u32,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub disk_usage_percent: Option<f32>,
    pub active_connections: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub timestamp: DateTime<Utc>,
    pub operation_type: String,
    pub description: String,
    pub duration_ms: Option<u64>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarError {
    pub occurred_at: DateTime<Utc>,
    pub error_type: String,
    pub message: String,
    pub resolution: Option<String>,
    pub time_to_resolve_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub confidence: f64,
    pub category: String,
    pub suggestions: Vec<String>,
    pub common_causes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevantContext {
    pub config_snippets: Vec<ConfigSnippet>,
    pub recent_changes: Vec<RecentChange>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnippet {
    pub file: String,
    pub relevant_section: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentChange {
    pub timestamp: DateTime<Utc>,
    pub change_type: String,
    pub description: String,
    pub affected_components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub health_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMetadata {
    pub report_version: String,
    pub generated_at: DateTime<Utc>,
    pub context_tokens_estimate: usize,
    pub truncated_fields: Vec<String>,
}

/// LLM export formatter
pub struct LLMExporter {
    config: LLMExportConfig,
    pattern_matcher: ErrorPatternMatcher,
}

impl LLMExporter {
    pub fn new(config: LLMExportConfig) -> Self {
        Self {
            config,
            pattern_matcher: ErrorPatternMatcher::with_defaults(),
        }
    }
    
    /// Export error for LLM analysis
    pub fn export_for_analysis(&self, event: &LogEvent) -> Option<LLMErrorReport> {
        let error = event.error.as_ref()?;
        
        let error_summary = self.create_summary(event, error);
        let execution_context = self.extract_execution_context(event);
        let system_state = if self.config.include_system_context {
            Some(self.capture_system_state(event))
        } else {
            None
        };
        
        let recent_operations = self.get_recent_operations(event.trace_id.as_ref());
        let similar_errors = if self.config.include_similar_errors {
            self.find_similar_errors(error)
        } else {
            Vec::new()
        };
        
        let matched_patterns = self.match_patterns(error);
        let relevant_context = self.extract_relevant_context(event, error);
        let suggested_prompts = if self.config.include_prompts {
            self.generate_analysis_prompts(error, &matched_patterns)
        } else {
            Vec::new()
        };
        
        let metadata = LLMMetadata {
            report_version: "1.0".to_string(),
            generated_at: Utc::now(),
            context_tokens_estimate: self.estimate_tokens(&error_summary, &execution_context),
            truncated_fields: Vec::new(),
        };
        
        Some(LLMErrorReport {
            error_summary,
            execution_context,
            system_state,
            recent_operations,
            similar_errors,
            matched_patterns,
            relevant_context,
            suggested_prompts,
            metadata,
        })
    }
    
    fn create_summary(&self, event: &LogEvent, error: &ErrorInfo) -> ErrorSummary {
        ErrorSummary {
            error_type: error.error_type.clone(),
            error_code: error.error_code.clone(),
            message: error.message.clone(),
            severity: format!("{:?}", error.severity),
            occurred_at: event.timestamp,
            is_retryable: error.is_retryable,
            retry_count: event.fields.get("retry_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32),
            trace_id: event.trace_id.clone(),
            span_id: event.span_id.clone(),
        }
    }
    
    fn extract_execution_context(&self, event: &LogEvent) -> ExecutionContext {
        ExecutionContext {
            task_name: event.fields.get("task_name")
                .and_then(|v| v.as_str())
                .map(String::from),
            task_version: event.fields.get("task_version")
                .and_then(|v| v.as_str())
                .map(String::from),
            job_id: event.fields.get("job_id")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            execution_id: event.fields.get("execution_id")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32),
            input_data_summary: event.fields.get("input_data")
                .cloned()
                .map(|v| self.summarize_data(v)),
            execution_duration_ms: event.fields.get("duration_ms")
                .and_then(|v| v.as_u64()),
            execution_phase: event.fields.get("phase")
                .and_then(|v| v.as_str())
                .map(String::from),
        }
    }
    
    fn capture_system_state(&self, event: &LogEvent) -> SystemState {
        SystemState {
            hostname: event.fields.get("hostname")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            process_id: event.fields.get("process_id")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            memory_usage_mb: event.fields.get("memory_usage_mb")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cpu_usage_percent: event.fields.get("cpu_usage_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            disk_usage_percent: event.fields.get("disk_usage_percent")
                .and_then(|v| v.as_f64())
                .map(|n| n as f32),
            active_connections: event.fields.get("active_connections")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32),
        }
    }
    
    fn get_recent_operations(&self, _trace_id: Option<&String>) -> Vec<Operation> {
        // TODO: Implement when we have operation tracking
        Vec::new()
    }
    
    fn find_similar_errors(&self, _error: &ErrorInfo) -> Vec<SimilarError> {
        // TODO: Implement when we have error history storage
        Vec::new()
    }
    
    fn match_patterns(&self, error: &ErrorInfo) -> Vec<MatchedPattern> {
        self.pattern_matcher.match_all(error)
            .into_iter()
            .map(|pattern| MatchedPattern {
                pattern_id: pattern.id.clone(),
                pattern_name: pattern.name.clone(),
                confidence: pattern.match_score(error),
                category: format!("{:?}", pattern.category),
                suggestions: pattern.suggestions.clone(),
                common_causes: pattern.common_causes.clone(),
            })
            .collect()
    }
    
    fn extract_relevant_context(&self, _event: &LogEvent, _error: &ErrorInfo) -> Option<RelevantContext> {
        // TODO: Implement when we have context extraction
        None
    }
    
    fn generate_analysis_prompts(&self, error: &ErrorInfo, patterns: &[MatchedPattern]) -> Vec<String> {
        let mut prompts = vec![
            format!("Analyze this {} error in a task execution system", error.error_type),
            "What are the most likely root causes based on the error context?".to_string(),
        ];
        
        // Add pattern-specific prompts
        for pattern in patterns {
            if pattern.confidence > 0.7 {
                prompts.push(format!(
                    "Given this is likely a {} issue, what specific remediation steps would you recommend?",
                    pattern.pattern_name
                ));
            }
            
            // Add specific prompts for known patterns
            if pattern.pattern_id == "db_connection_timeout" {
                prompts.push("How should we handle database connection timeout errors in a distributed system?".to_string());
            }
        }
        
        // Add severity-specific prompts
        match error.severity {
            crate::errors::ErrorSeverity::Critical => {
                prompts.push("What immediate actions should be taken for this critical error?".to_string());
            }
            crate::errors::ErrorSeverity::High => {
                prompts.push("How can we prevent this high-severity error from recurring?".to_string());
            }
            _ => {}
        }
        
        prompts.push("Based on the execution context, is there a pattern or systemic issue?".to_string());
        
        prompts
    }
    
    fn summarize_data(&self, data: serde_json::Value) -> serde_json::Value {
        // Truncate large data structures for LLM context
        match data {
            serde_json::Value::String(s) if s.len() > 100 => {
                serde_json::json!({
                    "_truncated": true,
                    "preview": &s[..100],
                    "length": s.len()
                })
            }
            serde_json::Value::Array(arr) if arr.len() > 10 => {
                serde_json::json!({
                    "_truncated": true,
                    "preview": &arr[..10],
                    "total_items": arr.len()
                })
            }
            serde_json::Value::Object(obj) if obj.len() > 20 => {
                let preview: HashMap<_, _> = obj.iter()
                    .take(20)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                serde_json::json!({
                    "_truncated": true,
                    "preview": preview,
                    "total_fields": obj.len()
                })
            }
            _ => data,
        }
    }
    
    fn estimate_tokens(&self, summary: &ErrorSummary, context: &ExecutionContext) -> usize {
        // Rough estimation: ~4 characters per token
        let summary_json = serde_json::to_string(summary).unwrap_or_default();
        let context_json = serde_json::to_string(context).unwrap_or_default();
        
        (summary_json.len() + context_json.len()) / 4
    }
}

/// Create a markdown report from LLM error export
pub fn format_markdown_report(report: &LLMErrorReport) -> String {
    let mut output = String::new();
    
    // Header
    output.push_str(&format!("# Error Analysis Report\n\n"));
    output.push_str(&format!("**Generated**: {}\n", report.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
    output.push_str(&format!("**Trace ID**: {}\n\n", report.error_summary.trace_id.as_ref().unwrap_or(&"N/A".to_string())));
    
    // Error Summary
    output.push_str("## Error Summary\n\n");
    output.push_str(&format!("- **Type**: {}\n", report.error_summary.error_type));
    output.push_str(&format!("- **Code**: {}\n", report.error_summary.error_code));
    output.push_str(&format!("- **Message**: {}\n", report.error_summary.message));
    output.push_str(&format!("- **Severity**: {}\n", report.error_summary.severity));
    output.push_str(&format!("- **Retryable**: {}\n", report.error_summary.is_retryable));
    
    // Execution Context
    output.push_str("\n## Execution Context\n\n");
    if let Some(task) = &report.execution_context.task_name {
        output.push_str(&format!("- **Task**: {} ", task));
        if let Some(version) = &report.execution_context.task_version {
            output.push_str(&format!("(v{})", version));
        }
        output.push_str("\n");
    }
    if let Some(job_id) = report.execution_context.job_id {
        output.push_str(&format!("- **Job ID**: {}\n", job_id));
    }
    if let Some(duration) = report.execution_context.execution_duration_ms {
        output.push_str(&format!("- **Duration**: {}ms\n", duration));
    }
    
    // Matched Patterns
    if !report.matched_patterns.is_empty() {
        output.push_str("\n## Matched Error Patterns\n\n");
        for pattern in &report.matched_patterns {
            output.push_str(&format!("### {} ({}% confidence)\n\n", pattern.pattern_name, (pattern.confidence * 100.0) as i32));
            
            if !pattern.suggestions.is_empty() {
                output.push_str("**Suggestions**:\n");
                for suggestion in &pattern.suggestions {
                    output.push_str(&format!("- {}\n", suggestion));
                }
                output.push_str("\n");
            }
            
            if !pattern.common_causes.is_empty() {
                output.push_str("**Common Causes**:\n");
                for cause in &pattern.common_causes {
                    output.push_str(&format!("- {}\n", cause));
                }
                output.push_str("\n");
            }
        }
    }
    
    // System State
    if let Some(system) = &report.system_state {
        output.push_str("\n## System State\n\n");
        output.push_str(&format!("- **Host**: {}\n", system.hostname));
        output.push_str(&format!("- **Memory**: {}MB\n", system.memory_usage_mb));
        output.push_str(&format!("- **CPU**: {:.1}%\n", system.cpu_usage_percent));
    }
    
    // Analysis Prompts
    if !report.suggested_prompts.is_empty() {
        output.push_str("\n## Suggested Analysis Questions\n\n");
        for (i, prompt) in report.suggested_prompts.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, prompt));
        }
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorSeverity;
    
    #[test]
    fn test_llm_export() {
        let error = ErrorInfo::new("TaskExecutionError", "TASK_EXEC_001", "Task failed after 3 retries")
            .with_severity(ErrorSeverity::High)
            .with_retryable(false)
            .with_context_value("task_name", "weather-api")
            .with_context_value("retry_count", 3);
        
        let event = LogEvent::new(super::super::LogLevel::Error, "Task execution failed")
            .with_error(error)
            .with_field("task_name", "weather-api")
            .with_field("job_id", 123)
            .with_field("duration_ms", 5000);
        
        let exporter = LLMExporter::new(LLMExportConfig::default());
        let report = exporter.export_for_analysis(&event);
        
        assert!(report.is_some());
        let report = report.unwrap();
        
        assert_eq!(report.error_summary.error_type, "TaskExecutionError");
        assert_eq!(report.execution_context.task_name, Some("weather-api".to_string()));
        assert!(report.suggested_prompts.len() > 0);
    }
    
    #[test]
    fn test_markdown_formatting() {
        let report = LLMErrorReport {
            error_summary: ErrorSummary {
                error_type: "TestError".to_string(),
                error_code: "TEST_001".to_string(),
                message: "Test error message".to_string(),
                severity: "High".to_string(),
                occurred_at: Utc::now(),
                is_retryable: true,
                retry_count: Some(2),
                trace_id: Some("trace-123".to_string()),
                span_id: None,
            },
            execution_context: ExecutionContext {
                task_name: Some("test-task".to_string()),
                task_version: Some("1.0.0".to_string()),
                job_id: Some(456),
                execution_id: None,
                input_data_summary: None,
                execution_duration_ms: Some(1500),
                execution_phase: None,
            },
            system_state: None,
            recent_operations: Vec::new(),
            similar_errors: Vec::new(),
            matched_patterns: vec![
                MatchedPattern {
                    pattern_id: "test_pattern".to_string(),
                    pattern_name: "Test Pattern".to_string(),
                    confidence: 0.85,
                    category: "TestCategory".to_string(),
                    suggestions: vec!["Try this".to_string()],
                    common_causes: vec!["Common cause".to_string()],
                }
            ],
            relevant_context: None,
            suggested_prompts: vec!["What went wrong?".to_string()],
            metadata: LLMMetadata {
                report_version: "1.0".to_string(),
                generated_at: Utc::now(),
                context_tokens_estimate: 500,
                truncated_fields: Vec::new(),
            },
        };
        
        let markdown = format_markdown_report(&report);
        
        assert!(markdown.contains("# Error Analysis Report"));
        assert!(markdown.contains("Test Pattern (85% confidence)"));
        assert!(markdown.contains("What went wrong?"));
    }
}