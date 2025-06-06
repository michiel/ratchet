use ratchet_lib::config::{ConfigError, OutputConfig, RatchetConfig};

#[test]
fn test_output_config_defaults() {
    let config = OutputConfig::default();

    assert_eq!(config.max_concurrent_deliveries, 10);
    assert_eq!(config.default_timeout, 30); // u64 seconds
    assert!(config.validate_on_startup);
}

#[test]
fn test_output_config_validation_success() {
    let config = RatchetConfig {
        output: OutputConfig {
            max_concurrent_deliveries: 5,
            default_timeout: 60,
            validate_on_startup: true,
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
        assert!(msg.contains("max_concurrent_deliveries must be greater than 0"));
    } else {
        panic!("Expected ValidationError");
    }
}

#[test]
fn test_output_config_validation_zero_timeout() {
    let mut config = RatchetConfig::default();
    config.output.default_timeout = 0;

    let result = config.validate();
    assert!(result.is_err());

    if let Err(ConfigError::ValidationError(msg)) = result {
        assert!(msg.contains("default_timeout must be greater than 0"));
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
"#;

    let config: RatchetConfig = serde_yaml::from_str(yaml_content)?;

    assert_eq!(config.output.max_concurrent_deliveries, 15);
    assert_eq!(config.output.default_timeout, 45);
    assert!(config.output.validate_on_startup);

    // Validate the loaded config
    assert!(config.validate().is_ok());

    Ok(())
}

#[test]
fn test_config_from_env() -> Result<(), Box<dyn std::error::Error>> {
    // Test that we can create a config from env (even with defaults)
    let config = RatchetConfig::from_env()?;
    assert!(config.validate().is_ok());

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
