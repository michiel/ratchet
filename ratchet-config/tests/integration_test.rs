//! Integration tests for ratchet-config

use ratchet_config::*;
use ratchet_config::domains::logging::{LogLevel, LogFormat};
use std::time::Duration;
use temp_env::with_vars;

#[test]
fn test_default_config_validation() {
    let config = RatchetConfig::default();
    assert!(config.validate_all().is_ok());
}

#[test]
fn test_config_loader_from_env() {
    let vars = vec![
        ("RATCHET_HTTP_TIMEOUT", Some("60")),
        ("RATCHET_CACHE_SIZE", Some("200")),
        ("RATCHET_LOG_LEVEL", Some("debug")),
        ("RATCHET_MAX_EXECUTION_SECONDS", Some("600")),
    ];
    
    with_vars(vars, || {
        let loader = ConfigLoader::new();
        let config = loader.from_env().unwrap();
        
        assert_eq!(config.http.timeout, Duration::from_secs(60));
        assert_eq!(config.cache.task_cache.task_content_cache_size, 200);
        assert_eq!(config.logging.level, LogLevel::Debug);
        assert_eq!(config.execution.max_execution_duration, Duration::from_secs(600));
    });
}

#[test]
fn test_yaml_config_serialization() {
    let config = RatchetConfig::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    
    // Parse it back
    let parsed: RatchetConfig = serde_yaml::from_str(&yaml).unwrap();
    assert!(parsed.validate_all().is_ok());
}

#[test]
fn test_comprehensive_config() {
    let yaml = r#"
execution:
  max_execution_duration: 300
  validate_schemas: true
  max_concurrent_tasks: 5

http:
  timeout: 45
  max_redirects: 5
  user_agent: "Test Agent"
  verify_ssl: false

cache:
  enabled: true
  task_cache:
    cache_type: "lru"
    task_content_cache_size: 50
    memory_limit_bytes: 33554432
    ttl: 1800

logging:
  level: warn
  format: json
  structured: true
  targets:
    - type: console
    - type: file
      path: "/var/log/ratchet.log"
      max_size_bytes: 10485760
      max_files: 5

output:
  max_concurrent_deliveries: 5
  default_timeout: 60
  validate_on_startup: true
  default_retry_policy:
    max_attempts: 5
    initial_delay_ms: 2000
    max_delay_ms: 60000
    backoff_multiplier: 2.5

server:
  bind_address: "0.0.0.0"
  port: 9090
  database:
    url: "sqlite:///tmp/test.db"
    max_connections: 20
  cors:
    allowed_origins: ["http://localhost:3000"]
    allowed_methods: ["GET", "POST"]
    allow_credentials: true

# Registry configuration commented out for now
# registry:
#   sources:
#     - name: "local-tasks"
#       uri: "file://./tasks"
#       source_type: filesystem
#       enabled: true
#   default_polling_interval: 600
"#;

    let config: RatchetConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate_all().is_ok());
    
    // Verify specific values
    assert_eq!(config.execution.max_concurrent_tasks, 5);
    assert_eq!(config.http.timeout, Duration::from_secs(45));
    assert!(!config.http.verify_ssl);
    assert_eq!(config.cache.task_cache.task_content_cache_size, 50);
    assert_eq!(config.logging.level, LogLevel::Warn);
    assert_eq!(config.logging.format, LogFormat::Json);
    assert_eq!(config.output.max_concurrent_deliveries, 5);
    assert_eq!(config.output.default_retry_policy.max_attempts, 5);
    
    if let Some(server) = config.server {
        assert_eq!(server.bind_address, "0.0.0.0");
        assert_eq!(server.port, 9090);
        assert_eq!(server.database.max_connections, 20);
        assert!(server.cors.allow_credentials);
    }
    
    // Registry tests commented out
    // if let Some(registry) = config.registry {
    //     assert_eq!(registry.sources.len(), 1);
    //     assert_eq!(registry.sources[0].name, "local-tasks");
    //     assert_eq!(registry.default_polling_interval, Duration::from_secs(600));
    //     assert_eq!(registry.cache.max_entries, 500);
    // }
}

#[test]
fn test_validation_errors() {
    // Test invalid HTTP timeout
    let mut config = RatchetConfig::default();
    config.http.timeout = Duration::from_secs(0);
    assert!(config.validate_all().is_err());
    
    // Test invalid cache size
    config = RatchetConfig::default();
    config.cache.task_cache.task_content_cache_size = 0;
    assert!(config.validate_all().is_err());
    
    // Test invalid execution duration
    config = RatchetConfig::default();
    config.execution.max_execution_duration = Duration::from_secs(0);
    assert!(config.validate_all().is_err());
}

#[test]
fn test_custom_prefix_loader() {
    let vars = vec![
        ("CUSTOM_HTTP_TIMEOUT", Some("120")),
        ("CUSTOM_CACHE_SIZE", Some("300")),
    ];
    
    with_vars(vars, || {
        let loader = ConfigLoader::with_prefix("CUSTOM");
        let config = loader.from_env().unwrap();
        
        assert_eq!(config.http.timeout, Duration::from_secs(120));
        assert_eq!(config.cache.task_cache.task_content_cache_size, 300);
    });
}

#[test]
fn test_domain_specific_validation() {
    use domains::{
        cache::CacheConfig,
        http::HttpConfig,
        logging::LoggingConfig,
    };
    use validation::Validatable;
    
    // Test cache config validation
    let mut cache = CacheConfig::default();
    assert!(cache.validate().is_ok());
    
    cache.task_cache.cache_type = "invalid".to_string();
    assert!(cache.validate().is_err());
    
    // Test HTTP config validation
    let mut http = HttpConfig::default();
    assert!(http.validate().is_ok());
    
    http.user_agent = String::new();
    assert!(http.validate().is_err());
    
    // Test logging config validation
    let mut logging = LoggingConfig::default();
    assert!(logging.validate().is_ok());
    
    logging.targets.clear();
    assert!(logging.validate().is_err());
}

#[test]
fn test_generate_sample_config() {
    let sample = RatchetConfig::generate_sample();
    assert!(!sample.is_empty());
    assert!(sample.contains("execution:"));
    assert!(sample.contains("http:"));
    assert!(sample.contains("cache:"));
    assert!(sample.contains("logging:"));
    assert!(sample.contains("output:"));
    
    // Verify the sample is valid YAML
    let parsed: RatchetConfig = serde_yaml::from_str(&sample).unwrap();
    assert!(parsed.validate_all().is_ok());
}

#[cfg(test)]
mod output_destination_tests {
    use super::*;
    use domains::output::*;
    
    #[test]
    fn test_output_destination_template_validation() {
        let template = OutputDestinationTemplate {
            name: "test-webhook".to_string(),
            description: Some("Test webhook destination".to_string()),
            destination: OutputDestinationConfigTemplate::Webhook {
                url: "https://api.example.com/webhook".to_string(),
                method: "POST".to_string(),
                headers: std::collections::HashMap::new(),
                timeout_seconds: 30,
                content_type: Some("application/json".to_string()),
                auth: Some(WebhookAuthConfig::Bearer {
                    token: "test-token".to_string(),
                }),
            },
        };
        
        assert!(template.validate_with_context("test").is_ok());
        
        // Test invalid webhook URL
        let invalid = OutputDestinationTemplate {
            name: "test-webhook".to_string(),
            description: None,
            destination: OutputDestinationConfigTemplate::Webhook {
                url: "invalid-url".to_string(),
                method: "POST".to_string(),
                headers: std::collections::HashMap::new(),
                timeout_seconds: 30,
                content_type: None,
                auth: None,
            },
        };
        
        assert!(invalid.validate_with_context("test").is_err());
    }
}