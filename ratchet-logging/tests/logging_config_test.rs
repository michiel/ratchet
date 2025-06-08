use ratchet_logging::{init_logger, LogLevel, LoggingConfig};
use tempfile::tempdir;

#[test]
fn test_logging_config_integration() {
    let yaml_config = r#"
level: debug
format: json
sinks:
  - type: console
    level: info
    use_json: true
  - type: file
    path: /tmp/ratchet.log
    level: debug
    rotation:
      max_size: 50MB
enrichment:
  system_info: true
  process_info: true
sampling:
  error_rate: 1.0
  info_rate: 0.5
  debug_rate: 0.1
"#;

    let config: LoggingConfig = serde_yaml::from_str(yaml_config).unwrap();

    // Verify config was parsed correctly
    assert_eq!(config.level, LogLevel::Debug);
    assert_eq!(config.sinks.len(), 2);
    assert_eq!(config.sampling.info_rate, 0.5);

    // Build and initialize logger from config
    let built_logger = config.build_logger().unwrap();
    init_logger(built_logger).ok();

    // Verify logger is available
    assert!(ratchet_logging::logger().is_some());
}

#[test]
fn test_minimal_logging_config() {
    // Test with empty config - should use all defaults
    let config = LoggingConfig::default();

    // Should use defaults
    assert_eq!(config.level, LogLevel::Info);
    assert_eq!(config.sinks.len(), 1); // Default console sink
    assert!(config.enrichment.system_info);
    assert!(config.enrichment.process_info);
}

#[tokio::test]
async fn test_config_with_buffered_file_sink() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let yaml_config = format!(
        r#"
level: trace
sinks:
  - type: file
    path: {:?}
    level: info
    buffered:
      size: 500
      flush_interval: 2s
    rotation:
      max_size: 10MB
"#,
        log_path
    );

    let config: LoggingConfig = serde_yaml::from_str(&yaml_config).unwrap();

    assert_eq!(config.level, LogLevel::Trace);
    assert_eq!(config.sinks.len(), 1);

    // Build logger should succeed
    let built_logger = config.build_logger().unwrap();
    assert_eq!(built_logger.min_level(), LogLevel::Trace);
}
