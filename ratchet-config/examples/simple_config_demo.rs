//! Simple configuration demo
//!
//! A minimal example showing basic configuration loading

use ratchet_config::{ConfigError, ConfigResult, loader::ConfigLoader};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> ConfigResult<()> {
    println!("🚀 Simple Ratchet Configuration Demo");

    // Create temporary directory for demo files
    let temp_dir = TempDir::new().map_err(|e| ConfigError::ValidationError(
        format!("Failed to create temp directory: {}", e)
    ))?;

    // Create a simple config file
    let config_path = temp_dir.path().join("config.yaml");
    let config_content = r#"
server:
  bind_address: "127.0.0.1"
  port: 8080
  database:
    url: "sqlite:///tmp/ratchet.db"
    max_connections: 10

execution:
  max_execution_duration: "300s"
  validate_schemas: true
  max_concurrent_tasks: 4

http:
  timeout: "30s"
  user_agent: "Ratchet/1.0"
  verify_ssl: true

logging:
  level: "info"
  format: "json"
"#;

    fs::write(&config_path, config_content).map_err(ConfigError::FileReadError)?;

    // Load the configuration
    let loader = ConfigLoader::new();
    let config = loader.from_file(&config_path)?;

    println!("✅ Configuration loaded successfully!");
    println!("   Server: {}:{}", config.server.as_ref().unwrap().bind_address, config.server.as_ref().unwrap().port);
    println!("   Max concurrent tasks: {}", config.execution.max_concurrent_tasks);
    println!("   HTTP timeout: {}s", config.http.timeout.as_secs());
    println!("   Logging level: {:?}", config.logging.level);

    Ok(())
}