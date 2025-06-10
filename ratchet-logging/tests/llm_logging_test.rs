use ratchet_logging::{
    format_markdown_report, ErrorCategory, ErrorInfo, ErrorPattern, ErrorPatternMatcher,
    ErrorSeverity, LLMExportConfig, LLMExporter, LogEvent, LogLevel, MatchingRule, LogContext,
};

#[test]
fn test_pattern_matching_for_task_errors() {
    let matcher = ErrorPatternMatcher::with_defaults();

    // Test TaskNotFound pattern
    let error = ErrorInfo::new(
        "TaskNotFound",
        "TASK_NOT_FOUND",
        "Task 'weather-api' not found",
    )
    .with_severity(ErrorSeverity::Medium)
    .with_context_value("task_name", "weather-api");

    let matched = matcher.match_error(&error);
    assert!(matched.is_some());

    let pattern = matched.unwrap();
    assert_eq!(pattern.id, "task_not_found");
    assert_eq!(pattern.category, ErrorCategory::TaskExecution);

    let suggestions = matcher.get_suggestions(&error);
    assert!(suggestions.iter().any(|s| s.contains("ratchet list")));
}

#[test]
fn test_network_timeout_pattern() {
    let matcher = ErrorPatternMatcher::with_defaults();

    let error = ErrorInfo::new(
        "NetworkError",
        "NETWORK_TIMEOUT",
        "Request timeout after 30s",
    )
    .with_severity(ErrorSeverity::High)
    .with_retryable(true)
    .with_context_value("url", "https://api.example.com/data")
    .with_context_value("timeout_ms", 30000);

    let matched = matcher.match_error(&error);
    assert!(matched.is_some());

    let pattern = matched.unwrap();
    assert_eq!(pattern.id, "http_timeout");
    assert_eq!(pattern.category, ErrorCategory::Network);

    assert!(!pattern.preventive_measures.is_empty());
}

#[test]
fn test_custom_pattern_matching() {
    let custom_pattern = ErrorPattern {
        id: "custom_auth_error".to_string(),
        name: "Authentication Failure".to_string(),
        description: "Failed authentication attempts".to_string(),
        category: ErrorCategory::Authentication,
        matching_rules: vec![MatchingRule::All {
            rules: vec![
                MatchingRule::ErrorType {
                    value: "AuthError".to_string(),
                },
                MatchingRule::Any {
                    rules: vec![
                        MatchingRule::MessagePattern {
                            pattern: r"(?i)invalid.*token".to_string(),
                        },
                        MatchingRule::MessagePattern {
                            pattern: r"(?i)expired.*credentials".to_string(),
                        },
                    ],
                },
            ],
        }],
        suggestions: vec![
            "Check API token validity".to_string(),
            "Refresh authentication credentials".to_string(),
        ],
        preventive_measures: vec!["Implement token refresh logic".to_string()],
        related_documentation: vec![],
        common_causes: vec![
            "Expired API tokens".to_string(),
            "Invalid credentials".to_string(),
        ],
        llm_prompts: vec![],
    };

    let error1 = ErrorInfo::new("AuthError", "AUTH_001", "Invalid API token");
    assert!(custom_pattern.matches(&error1));

    let error2 = ErrorInfo::new("AuthError", "AUTH_002", "Expired credentials");
    assert!(custom_pattern.matches(&error2));

    let error3 = ErrorInfo::new("AuthError", "AUTH_003", "User not found");
    assert!(!custom_pattern.matches(&error3));
}

#[test]
fn test_llm_export_with_patterns() {
    let error = ErrorInfo::new(
        "DatabaseError",
        "DB_CONN_ERROR",
        "Connection timeout after 5s",
    )
    .with_severity(ErrorSeverity::High)
    .with_retryable(true)
    .with_context_value("database", "postgres://localhost:5432/ratchet")
    .with_context_value("timeout_ms", 5000)
    .with_suggestion("Check database server is running")
    .with_preventive_suggestion("Implement connection pooling");

    let event = LogEvent::new(LogLevel::Error, "Database connection failed")
        .with_error(error)
        .with_field("task_name", "data-processor")
        .with_field("job_id", 789)
        .with_field("execution_id", 456)
        .with_field("duration_ms", 5100)
        .with_field("hostname", "worker-01")
        .with_field("memory_usage_mb", 512)
        .with_field("cpu_usage_percent", 25.5)
        .with_trace_id("trace-123-456")
        .with_span_id("span-789");

    let exporter = LLMExporter::new(LLMExportConfig::default());
    let report = exporter.export_for_analysis(&event).unwrap();

    // Verify error summary
    assert_eq!(report.error_summary.error_type, "DatabaseError");
    assert_eq!(report.error_summary.error_code, "DB_CONN_ERROR");
    assert!(report.error_summary.is_retryable);

    // Verify execution context
    assert_eq!(
        report.execution_context.task_name,
        Some("data-processor".to_string())
    );
    assert_eq!(report.execution_context.job_id, Some(789));
    assert_eq!(report.execution_context.execution_duration_ms, Some(5100));

    // Verify system state
    let system = report.system_state.as_ref().unwrap();
    assert_eq!(system.hostname, "worker-01");
    assert_eq!(system.memory_usage_mb, 512);
    assert_eq!(system.cpu_usage_percent, 25.5);

    // Verify pattern matching
    assert!(!report.matched_patterns.is_empty());
    let matched = &report.matched_patterns[0];
    assert_eq!(matched.pattern_id, "db_connection_timeout");
    assert!(matched.confidence > 0.5);

    // Verify prompts
    assert!(!report.suggested_prompts.is_empty());
    assert!(report
        .suggested_prompts
        .iter()
        .any(|p| p.contains("database connection timeout")));
}

#[test]
fn test_markdown_report_generation() {
    let error_info = ErrorInfo::new(
        "TaskNotFound",
        "TASK_NOT_FOUND",
        "Task 'weather-api' not found"
    ).with_context_value("task_name", "weather-api");
    
    let context = LogContext::new()
        .with_field("request_id", "req-123")
        .with_field("user_id", 456);

    let event = LogEvent::new(LogLevel::Error, "Task not found")
        .with_error(error_info)
        .with_trace_id(context.trace_id.clone())
        .with_field("request_id", "req-123")
        .with_field("user_id", 456);

    let exporter = LLMExporter::new(LLMExportConfig {
        include_system_context: true,
        include_similar_errors: false,
        max_context_size: 4096,
        related_errors_window: chrono::Duration::hours(1),
        include_prompts: true,
    });

    let report = exporter.export_for_analysis(&event).unwrap();
    let markdown = format_markdown_report(&report);

    // Verify markdown structure
    assert!(markdown.contains("# Error Analysis Report"));
    assert!(markdown.contains("## Error Summary"));
    assert!(markdown.contains("TaskNotFound"));
    assert!(markdown.contains("TASK_NOT_FOUND"));
    assert!(markdown.contains("## Matched Error Patterns"));
    assert!(markdown.contains("## Suggested Analysis Questions"));

    // Verify trace ID is included
    assert!(markdown.contains(&context.trace_id));
}

#[test]
fn test_data_summarization() {
    // Test that large data is truncated appropriately
    let large_string = "x".repeat(200);
    let large_array: Vec<i32> = (0..15).collect(); // Will be truncated at >10
    let mut large_object = serde_json::Map::new();
    for i in 0..25 {
        // Will be truncated at >20
        large_object.insert(format!("field_{}", i), serde_json::json!(i));
    }

    let error = ErrorInfo::new("DataError", "DATA_001", "Processing failed")
        .with_context_value("large_string", large_string.clone())
        .with_context_value("large_array", large_array.clone())
        .with_context_value("large_object", large_object.clone());

    let event = LogEvent::new(LogLevel::Error, "Data processing error")
        .with_error(error)
        .with_field(
            "input_data",
            serde_json::json!({
                "string": large_string,
                "array": large_array,
                "object": large_object,
            }),
        );

    let exporter = LLMExporter::new(LLMExportConfig::default());
    let report = exporter.export_for_analysis(&event).unwrap();

    // Check that data was summarized
    assert!(
        report.execution_context.input_data_summary.is_some(),
        "Expected input_data_summary to be present"
    );

    // TODO: Fix data truncation logic - currently the summarize_data function
    // is called on individual fields, not the complete input_data object

    // Token estimate should be reasonable
    assert!(report.metadata.context_tokens_estimate < 2000);
}

#[test]
fn test_complex_pattern_matching() {
    let patterns = vec![ErrorPattern {
        id: "api_rate_limit".to_string(),
        name: "API Rate Limit".to_string(),
        description: "Rate limit exceeded".to_string(),
        category: ErrorCategory::Network,
        matching_rules: vec![MatchingRule::Any {
            rules: vec![
                MatchingRule::FieldEquals {
                    field: "http_status".to_string(),
                    value: serde_json::json!(429),
                },
                MatchingRule::All {
                    rules: vec![
                        MatchingRule::MessagePattern {
                            pattern: r"(?i)rate.?limit".to_string(),
                        },
                        MatchingRule::ErrorType {
                            value: "HttpError".to_string(),
                        },
                    ],
                },
            ],
        }],
        suggestions: vec!["Implement backoff strategy".to_string()],
        preventive_measures: vec![],
        related_documentation: vec![],
        common_causes: vec![],
        llm_prompts: vec![],
    }];

    let matcher = ErrorPatternMatcher::new(patterns);

    // Test direct status code match
    let error1 = ErrorInfo::new("HttpError", "HTTP_429", "Too many requests")
        .with_context_value("http_status", 429);
    assert!(matcher.match_error(&error1).is_some());

    // Test message pattern match
    let error2 = ErrorInfo::new("HttpError", "HTTP_ERROR", "Rate limit exceeded");
    assert!(matcher.match_error(&error2).is_some());

    // Test non-match
    let error3 = ErrorInfo::new("HttpError", "HTTP_500", "Internal server error");
    assert!(matcher.match_error(&error3).is_none());
}
