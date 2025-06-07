use ratchet_lib::errors::RatchetError;
use ratchet_lib::logging::{
    enrichment::{ProcessEnricher, SystemEnricher},
    init_logger, logger,
    logger::LogSink,
    sinks::{ConsoleSink, FileSink},
    LogContext, LogEvent, LogLevel, LoggerBuilder,
};
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_basic_logging() {
    // Create a logger with console sink
    let console_sink = Arc::new(ConsoleSink::new(LogLevel::Debug));
    let test_logger = LoggerBuilder::new()
        .with_min_level(LogLevel::Debug)
        .add_sink(console_sink)
        .build();

    // Try to initialize global logger, ignore if already initialized
    let _ = init_logger(test_logger);

    // Test basic logging
    let event = LogEvent::new(LogLevel::Info, "Test message")
        .with_field("user_id", 123)
        .with_field("action", "login");

    if let Some(global_logger) = logger() {
        global_logger.log(event);
    }
}

#[test]
fn test_error_logging() {
    // Create logger with file sink
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let file_sink =
        Arc::new(FileSink::new(&log_path, LogLevel::Info).expect("Failed to create file sink"));

    let logger = LoggerBuilder::new()
        .with_min_level(LogLevel::Info)
        .add_sink(file_sink.clone())
        .add_enricher(Box::new(SystemEnricher::new()))
        .add_enricher(Box::new(ProcessEnricher::new()))
        .build();

    // Test error conversion to log event
    let error = RatchetError::TaskNotFound("test-task".to_string());
    let context = LogContext::new()
        .with_field("request_id", "req-123")
        .with_field("task_name", "test-task");

    let log_event = error.to_log_event(&context);
    logger.log(log_event);

    // Flush the logger
    file_sink.flush();

    // Verify file was created and contains data
    assert!(log_path.exists());
    let contents = std::fs::read_to_string(&log_path).unwrap();
    assert!(contents.contains("TaskNotFound"));
    assert!(contents.contains("test-task"));
    assert!(contents.contains("req-123"));
}

#[test]
fn test_context_propagation() {
    let console_sink = Arc::new(ConsoleSink::new(LogLevel::Trace));
    let logger = LoggerBuilder::new()
        .with_min_level(LogLevel::Trace)
        .add_sink(console_sink)
        .build();

    // Create a context
    let parent_context = LogContext::new()
        .with_field("service", "ratchet")
        .with_field("environment", "test");

    // Create logger with context
    let contextualized_logger = logger.with_context(parent_context.clone());

    // Log event should include context fields
    let event = LogEvent::new(LogLevel::Info, "Context test");
    contextualized_logger.log(event);

    // Create child context
    let child_context = parent_context
        .child()
        .with_field("operation", "task_execution");

    let child_logger = logger.with_context(child_context);
    let event = LogEvent::new(LogLevel::Debug, "Child context test");
    child_logger.log(event);
}

#[test]
fn test_structured_error_info() {
    use ratchet_lib::logging::ErrorInfo;

    let error_info = ErrorInfo::new(
        "DatabaseError",
        "DB_CONN_TIMEOUT",
        "Connection to database timed out",
    )
    .with_severity(ratchet_lib::logging::ErrorSeverity::High)
    .with_retryable(true)
    .with_context_value("database", "postgres://localhost:5432/ratchet")
    .with_context_value("timeout_ms", 5000)
    .with_suggestion("Check database server is running")
    .with_suggestion("Verify network connectivity")
    .with_preventive_suggestion("Implement connection pooling")
    .with_preventive_suggestion("Add circuit breaker for database calls");

    // Verify serialization works
    let json = serde_json::to_string_pretty(&error_info).unwrap();
    assert!(json.contains("DB_CONN_TIMEOUT"));
    assert!(json.contains("Check database server is running"));
}

#[tokio::test]
async fn test_async_context_scope() {
    let console_sink = Arc::new(ConsoleSink::new(LogLevel::Debug));
    let test_logger = LoggerBuilder::new()
        .with_min_level(LogLevel::Debug)
        .add_sink(console_sink)
        .build();

    init_logger(test_logger).ok();

    let context = LogContext::new()
        .with_field("async_task", "test_operation")
        .with_field("user_id", 456);

    // Use context scope for async operation
    let result = context
        .scope(async {
            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Log within the context
            if let Some(global_logger) = logger() {
                let event = LogEvent::new(LogLevel::Info, "Async operation completed")
                    .with_field("duration_ms", 10);
                global_logger.log(event);
            }

            42
        })
        .await;

    assert_eq!(result, 42);
}
