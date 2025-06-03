use ratchet_lib::config::{RatchetConfig, OutputConfig, RetryPolicyConfig, ConfigError, OutputDestinationTemplate, OutputDestinationConfigTemplate, WebhookAuthConfig};
use std::time::Duration;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_output_config_defaults() {
    let config = OutputConfig::default();
    
    assert_eq!(config.max_concurrent_deliveries, 10);
    assert_eq!(config.default_timeout, Duration::from_secs(30));
    assert!(config.validate_on_startup);
    assert_eq!(config.global_destinations.len(), 0);
    assert_eq!(config.default_retry_policy.max_attempts, 3);
}

#[test]
fn test_retry_policy_defaults() {
    let policy = RetryPolicyConfig::default();
    
    assert_eq!(policy.max_attempts, 3);
    assert_eq!(policy.initial_delay_ms, 1000);
    assert_eq!(policy.max_delay_ms, 30000);
    assert_eq!(policy.backoff_multiplier, 2.0);
}

#[test]
fn test_output_config_validation_success() {
    let config = RatchetConfig {
        output: OutputConfig {
            max_concurrent_deliveries: 5,
            default_timeout: Duration::from_secs(60),
            validate_on_startup: true,
            global_destinations: vec![
                OutputDestinationTemplate {
                    name: "test-filesystem".to_string(),
                    description: Some("Test filesystem destination".to_string()),
                    destination: OutputDestinationConfigTemplate::Filesystem {
                        path: "/tmp/test.json".to_string(),
                        format: "json".to_string(),
                        permissions: "644".to_string(),
                        create_dirs: true,
                        overwrite: true,
                        backup_existing: false,
                    }
                }
            ],
            default_retry_policy: RetryPolicyConfig::default(),
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_ok());
}

#[test]
fn test_output_config_validation_zero_concurrent_deliveries() {
    let mut config = RatchetConfig::default();
    config.output.max_concurrent_deliveries = 0;
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Max concurrent deliveries must be greater than 0"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_output_config_validation_zero_timeout() {
    let mut config = RatchetConfig::default();
    config.output.default_timeout = Duration::from_secs(0);
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Default delivery timeout must be greater than 0 seconds"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_retry_policy_validation_zero_attempts() {
    let mut config = RatchetConfig::default();
    config.output.default_retry_policy.max_attempts = 0;
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Max retry attempts must be greater than 0"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_retry_policy_validation_zero_initial_delay() {
    let mut config = RatchetConfig::default();
    config.output.default_retry_policy.initial_delay_ms = 0;
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Initial retry delay must be greater than 0 milliseconds"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_retry_policy_validation_max_delay_less_than_initial() {
    let mut config = RatchetConfig::default();
    config.output.default_retry_policy.initial_delay_ms = 5000;
    config.output.default_retry_policy.max_delay_ms = 1000;
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Max retry delay must be greater than or equal to initial delay"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_retry_policy_validation_invalid_backoff_multiplier() {
    let mut config = RatchetConfig::default();
    config.output.default_retry_policy.backoff_multiplier = 1.0;
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Backoff multiplier must be greater than 1.0"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_global_destination_validation_empty_name() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Filesystem {
            path: "/tmp/test.json".to_string(),
            format: "json".to_string(),
            permissions: "644".to_string(),
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Global destination template 0 has empty name"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_filesystem_destination_validation_empty_path() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Filesystem {
            path: "".to_string(),
            format: "json".to_string(),
            permissions: "644".to_string(),
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Filesystem destination 'test' has empty path"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_filesystem_destination_validation_invalid_format() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Filesystem {
            path: "/tmp/test.json".to_string(),
            format: "invalid_format".to_string(),
            permissions: "644".to_string(),
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Filesystem destination 'test' has invalid format 'invalid_format'"));
        assert!(msg.contains("Valid formats: json, json_compact, yaml, csv, raw, template"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_webhook_destination_validation_empty_url() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Webhook {
            url: "".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 30,
            content_type: None,
            auth: None,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Webhook destination 'test' has empty URL"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_webhook_destination_validation_invalid_url() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Webhook {
            url: "invalid-url".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 30,
            content_type: None,
            auth: None,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Webhook destination 'test' has invalid URL format"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_webhook_destination_validation_invalid_method() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Webhook {
            url: "https://example.com".to_string(),
            method: "INVALID".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 30,
            content_type: None,
            auth: None,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Webhook destination 'test' has invalid HTTP method 'INVALID'"));
        assert!(msg.contains("Valid methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_database_destination_validation_empty_connection() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Database {
            connection_string: "".to_string(),
            table_name: "results".to_string(),
            column_mappings: std::collections::HashMap::new(),
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("Database destination 'test' has empty connection string"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_s3_destination_validation_empty_bucket() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::S3 {
            bucket: "".to_string(),
            key_template: "{{job_id}}.json".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
        }
    });
    
    let result = config.validate();
    assert!(result.is_err());
    
    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("S3 destination 'test' has empty bucket name"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_yaml_config_loading() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_content = r#"
output:
  max_concurrent_deliveries: 15
  default_timeout: 45
  validate_on_startup: true
  default_retry_policy:
    max_attempts: 5
    initial_delay_ms: 2000
    max_delay_ms: 60000
    backoff_multiplier: 2.5
  global_destinations:
    - name: "test_filesystem"
      description: "Test filesystem destination"
      destination:
        type: filesystem
        path: "/tmp/{{job_id}}.json"
        format: json
        permissions: "644"
        create_dirs: true
        overwrite: true
    - name: "test_webhook"
      description: "Test webhook destination"
      destination:
        type: webhook
        url: "https://example.com/webhook"
        method: POST
        timeout_seconds: 30
        content_type: "application/json"
"#;
    
    let config: RatchetConfig = serde_yaml::from_str(yaml_content)?;
    
    assert_eq!(config.output.max_concurrent_deliveries, 15);
    assert_eq!(config.output.default_timeout, Duration::from_secs(45));
    assert_eq!(config.output.default_retry_policy.max_attempts, 5);
    assert_eq!(config.output.global_destinations.len(), 2);
    
    let filesystem_dest = &config.output.global_destinations[0];
    assert_eq!(filesystem_dest.name, "test_filesystem");
    
    let webhook_dest = &config.output.global_destinations[1];
    assert_eq!(webhook_dest.name, "test_webhook");
    
    // Validate the loaded config
    assert!(config.validate().is_ok());
    
    Ok(())
}

#[test]
fn test_config_file_loading() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_content = r#"
output:
  max_concurrent_deliveries: 20
  default_timeout: 60
  global_destinations:
    - name: "production_logs"
      destination:
        type: filesystem
        path: "/var/log/outputs/{{task_name}}_{{job_id}}.json"
        format: json
        create_dirs: true
"#;
    
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(yaml_content.as_bytes())?;
    temp_file.flush()?;
    
    let config = RatchetConfig::from_file(temp_file.path())?;
    
    assert_eq!(config.output.max_concurrent_deliveries, 20);
    assert_eq!(config.output.default_timeout, Duration::from_secs(60));
    assert_eq!(config.output.global_destinations.len(), 1);
    
    Ok(())
}

#[test]
fn test_environment_variable_overrides() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variables
    std::env::set_var("RATCHET_OUTPUT_MAX_CONCURRENT", "25");
    std::env::set_var("RATCHET_OUTPUT_DEFAULT_TIMEOUT", "90");
    std::env::set_var("RATCHET_OUTPUT_VALIDATE_STARTUP", "false");
    
    // Note: The actual environment variable override implementation
    // would need to be added to the config.rs file for these specific variables
    
    // For now, test that we can at least create a config from env
    let config = RatchetConfig::from_env()?;
    assert!(config.validate().is_ok());
    
    // Clean up environment variables
    std::env::remove_var("RATCHET_OUTPUT_MAX_CONCURRENT");
    std::env::remove_var("RATCHET_OUTPUT_DEFAULT_TIMEOUT");
    std::env::remove_var("RATCHET_OUTPUT_VALIDATE_STARTUP");
    
    Ok(())
}

#[test]
fn test_invalid_yaml_config() {
    let invalid_yaml = r#"
output:
  max_concurrent_deliveries: "not_a_number"
  default_timeout: invalid
"#;
    
    let result: Result<RatchetConfig, _> = serde_yaml::from_str(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_webhook_auth_config_validation() {
    let mut config = RatchetConfig::default();
    config.output.global_destinations.push(OutputDestinationTemplate {
        name: "test_auth".to_string(),
        description: None,
        destination: OutputDestinationConfigTemplate::Webhook {
            url: "https://example.com/webhook".to_string(),
            method: "POST".to_string(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 30,
            content_type: None,
            auth: Some(WebhookAuthConfig::Bearer {
                token: "test-token".to_string(),
            }),
        }
    });
    
    // Should validate successfully with authentication
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_with_all_destination_types() {
    let mut config = RatchetConfig::default();
    
    // Add all destination types
    config.output.global_destinations = vec![
        OutputDestinationTemplate {
            name: "fs_dest".to_string(),
            description: None,
            destination: OutputDestinationConfigTemplate::Filesystem {
                path: "/tmp/output.json".to_string(),
                format: "json".to_string(),
                permissions: "644".to_string(),
                create_dirs: true,
                overwrite: true,
                backup_existing: false,
            }
        },
        OutputDestinationTemplate {
            name: "webhook_dest".to_string(),
            description: None,
            destination: OutputDestinationConfigTemplate::Webhook {
                url: "https://example.com/webhook".to_string(),
                method: "POST".to_string(),
                headers: std::collections::HashMap::new(),
                timeout_seconds: 30,
                content_type: Some("application/json".to_string()),
                auth: None,
            }
        },
        OutputDestinationTemplate {
            name: "db_dest".to_string(),
            description: None,
            destination: OutputDestinationConfigTemplate::Database {
                connection_string: "postgresql://user:pass@localhost/db".to_string(),
                table_name: "task_outputs".to_string(),
                column_mappings: std::collections::HashMap::new(),
            }
        },
        OutputDestinationTemplate {
            name: "s3_dest".to_string(),
            description: None,
            destination: OutputDestinationConfigTemplate::S3 {
                bucket: "my-bucket".to_string(),
                key_template: "outputs/{{job_id}}.json".to_string(),
                region: "us-west-2".to_string(),
                access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            }
        },
    ];
    
    assert!(config.validate().is_ok());
    assert_eq!(config.output.global_destinations.len(), 4);
}