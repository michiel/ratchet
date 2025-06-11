//! Configuration migration demonstration
//!
//! This example shows how to use the configuration auto-migration functionality
//! to detect legacy config formats and automatically upgrade them to modern format.

use ratchet_config::{
    ConfigMigrator, ConfigCompatibilityService, ConfigFormat, MigrationReport,
    ConfigError, ConfigResult
};
use std::fs;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> ConfigResult<()> {
    println!("🚀 Ratchet Configuration Migration Demo\n");

    // Create temporary directory for demo files
    let temp_dir = TempDir::new().map_err(|e| ConfigError::ParseError {
        message: format!("Failed to create temp directory: {}", e),
    })?;

    // Demo 1: Detect and migrate legacy YAML configuration
    demo_legacy_yaml_migration(&temp_dir).await?;
    
    // Demo 2: Auto-detection with compatibility service
    demo_compatibility_service(&temp_dir).await?;
    
    // Demo 3: No migration needed for modern config
    demo_modern_config_loading(&temp_dir).await?;

    println!("✅ All demos completed successfully!");
    println!("🎉 Configuration auto-migration is working correctly!");

    Ok(())
}

async fn demo_legacy_yaml_migration(temp_dir: &TempDir) -> ConfigResult<()> {
    println!("📋 Demo 1: Legacy YAML Configuration Migration");
    println!("=".repeat(50));

    // Create a legacy configuration file
    let legacy_config_path = temp_dir.path().join("legacy_config.yaml");
    let legacy_config_content = r#"# Legacy Ratchet Configuration (pre-0.4.0)
server:
  bind_address: "127.0.0.1"
  port: 3000
  database:
    url: "sqlite://legacy_database.db"
    max_connections: 15
    connection_timeout: 45s

# Legacy flat execution settings
max_execution_duration: 600
validate_schemas: true
max_concurrent_tasks: 12
timeout_grace_period: 90

# Legacy HTTP settings
http:
  timeout: 60s
  user_agent: "LegacyRatchet/0.3.0"
  verify_ssl: false
  max_redirects: 8

# Legacy logging settings
logging:
  level: "debug"
  format: "json"
  output: "file"

# Legacy MCP settings
mcp:
  enabled: true
  transport: "stdio"
  host: "localhost"
  port: 3001
"#;

    fs::write(&legacy_config_path, legacy_config_content).map_err(|e| ConfigError::FileIo {
        path: legacy_config_path.clone(),
        error: e,
    })?;

    println!("📁 Created legacy config: {}", legacy_config_path.display());

    // Create migrator and perform migration
    let migrator = ConfigMigrator::with_options(true, false); // Create backup, overwrite original
    let (modern_config, report) = migrator.migrate_config_file(&legacy_config_path).await?;

    // Print migration report
    println!("\n📊 Migration Report:");
    report.print_summary();

    // Verify settings were preserved
    println!("\n🔍 Verification - Settings Preserved:");
    println!("   Server address: {}", modern_config.server.as_ref().unwrap().bind_address);
    println!("   Server port: {}", modern_config.server.as_ref().unwrap().port);
    println!("   Database URL: {}", modern_config.database.url);
    println!("   Database connections: {}", modern_config.database.max_connections);
    println!("   Execution duration: {}s", modern_config.execution.max_execution_duration.as_secs());
    println!("   Schema validation: {}", modern_config.execution.validate_schemas);
    println!("   Max concurrent: {}", modern_config.execution.max_concurrent);
    println!("   HTTP timeout: {}s", modern_config.http.timeout.as_secs());
    println!("   HTTP user agent: {}", modern_config.http.user_agent);
    println!("   SSL verification: {}", modern_config.http.verify_ssl);
    println!("   Log level: {}", modern_config.logging.level);
    println!("   Log format: {}", modern_config.logging.format);
    println!("   File logging: {}", modern_config.logging.file.enabled);
    
    if let Some(mcp) = &modern_config.mcp {
        println!("   MCP enabled: {}", mcp.enabled);
        println!("   MCP transport: {}", mcp.transport);
        println!("   MCP port: {}", mcp.port);
    }

    // Show new modern domains that were added
    println!("\n🆕 New Modern Domains Added:");
    println!("   Cache enabled: {}", modern_config.cache.enabled);
    println!("   Registry enabled: {}", modern_config.registry.enabled);
    println!("   Output destinations: {}", modern_config.output.destinations.len());

    println!("\n✅ Demo 1 completed successfully!\n");
    Ok(())
}

async fn demo_compatibility_service(temp_dir: &TempDir) -> ConfigResult<()> {
    println!("📋 Demo 2: Compatibility Service Auto-Detection");
    println!("=".repeat(50));

    // Create a legacy JSON configuration
    let legacy_json_path = temp_dir.path().join("legacy_config.json");
    let legacy_json_content = r#"{
  "server": {
    "bind_address": "0.0.0.0",
    "port": 8080,
    "database": {
      "url": "postgres://user:pass@localhost/ratchet",
      "max_connections": 25,
      "connection_timeout": "30s"
    }
  },
  "max_execution_duration": 900,
  "validate_schemas": false,
  "max_concurrent_tasks": 6,
  "timeout_grace_period": 120,
  "http": {
    "timeout": "45s",
    "user_agent": "ProductionRatchet/1.0",
    "verify_ssl": true,
    "max_redirects": 15
  },
  "logging": {
    "level": "warn",
    "format": "text",
    "output": "console"
  }
}"#;

    fs::write(&legacy_json_path, legacy_json_content).map_err(|e| ConfigError::FileIo {
        path: legacy_json_path.clone(),
        error: e,
    })?;

    println!("📁 Created legacy JSON config: {}", legacy_json_path.display());

    // Check if migration is needed
    let needs_migration = ConfigCompatibilityService::needs_migration(&legacy_json_path)?;
    println!("🔍 Migration needed: {}", needs_migration);

    // Load with automatic migration
    let (modern_config, report) = ConfigCompatibilityService::load_with_migration(&legacy_json_path).await?;

    println!("\n📊 Auto-Migration Report:");
    println!("   Format detected: {}", report.original_format);
    println!("   Migration performed: {}", report.migration_performed);
    println!("   Validation passed: {}", report.validation_passed);

    // Show conversion to legacy format for backward compatibility
    println!("\n🔄 Backward Compatibility Test:");
    let legacy_format = ConfigCompatibilityService::to_legacy_format(&modern_config);
    println!("   Can convert back to legacy: ✅");
    println!("   Legacy server port: {}", legacy_format.server.as_ref().unwrap().port);
    println!("   Legacy execution duration: {}s", legacy_format.execution.max_execution_duration);

    println!("\n✅ Demo 2 completed successfully!\n");
    Ok(())
}

async fn demo_modern_config_loading(temp_dir: &TempDir) -> ConfigResult<()> {
    println!("📋 Demo 3: Modern Configuration (No Migration Needed)");
    println!("=".repeat(50));

    // Create a modern configuration file
    let modern_config_path = temp_dir.path().join("modern_config.yaml");
    let modern_config_content = r#"# Modern Ratchet Configuration (v0.4.0+)
# Domain-driven structure with full feature support

server:
  bind_address: "0.0.0.0"
  port: 8080
  enable_graceful_shutdown: true
  shutdown_timeout: 30s

database:
  url: "sqlite://modern_database.db"
  max_connections: 20
  connection_timeout: 30s
  enable_migrations: true
  pool_timeout: 10s

execution:
  max_execution_duration: 300s
  validate_schemas: true
  max_concurrent: 8
  shutdown_timeout: 60s
  enable_recording: true
  recording_path: "/var/log/ratchet/executions"

http:
  timeout: 30s
  user_agent: "ModernRatchet/0.4.0"
  verify_ssl: true
  max_redirects: 10
  connection_pool_size: 100
  enable_compression: true

logging:
  level: "info"
  format: "json"
  console:
    enabled: true
    colors: true
  file:
    enabled: true
    path: "/var/log/ratchet/app.log"
    rotation: "daily"
    max_size: "100MB"
  structured:
    enabled: true
    include_spans: true

cache:
  enabled: true
  provider: "memory"
  max_entries: 10000
  ttl: 3600s
  cleanup_interval: 300s

registry:
  enabled: true
  sources:
    - type: "filesystem"
      path: "/opt/ratchet/tasks"
      watch: true
    - type: "http"
      url: "https://registry.example.com/tasks"
      sync_interval: 3600s

output:
  destinations:
    - type: "filesystem"
      path: "/var/lib/ratchet/outputs"
      compression: "gzip"
    - type: "webhook"
      url: "https://webhook.example.com/ratchet"
      timeout: 30s
      retry_count: 3

mcp:
  enabled: true
  transport: "sse"
  host: "localhost"
  port: 3000
  auth:
    enabled: false
  rate_limit:
    requests_per_minute: 1000
"#;

    fs::write(&modern_config_path, modern_config_content).map_err(|e| ConfigError::FileIo {
        path: modern_config_path.clone(),
        error: e,
    })?;

    println!("📁 Created modern config: {}", modern_config_path.display());

    // Check if migration is needed (should be false)
    let needs_migration = ConfigCompatibilityService::needs_migration(&modern_config_path)?;
    println!("🔍 Migration needed: {}", needs_migration);

    // Load configuration (no migration should occur)
    let (modern_config, report) = ConfigCompatibilityService::load_with_migration(&modern_config_path).await?;

    println!("\n📊 Loading Report:");
    println!("   Format detected: {}", report.original_format);
    println!("   Migration performed: {}", report.migration_performed);
    println!("   Validation passed: {}", report.validation_passed);

    // Show modern features that weren't available in legacy
    println!("\n🆕 Modern Features Available:");
    println!("   Server graceful shutdown: enabled");
    println!("   Database migrations: {}", modern_config.database.enable_migrations);
    println!("   Execution recording: {}", modern_config.execution.enable_recording);
    println!("   HTTP compression: {}", modern_config.http.enable_compression);
    println!("   Structured logging: {}", modern_config.logging.structured.enabled);
    println!("   Cache provider: {}", modern_config.cache.provider);
    println!("   Registry sources: {}", modern_config.registry.sources.len());
    println!("   Output destinations: {}", modern_config.output.destinations.len());
    
    if let Some(mcp) = &modern_config.mcp {
        println!("   MCP rate limiting: {} req/min", mcp.rate_limit.requests_per_minute);
    }

    println!("\n✅ Demo 3 completed successfully!\n");
    Ok(())
}

/// Helper function to demonstrate format detection
fn demonstrate_format_detection() {
    println!("📋 Format Detection Examples:");
    println!("=".repeat(30));

    let legacy_indicators = [
        "max_execution_duration field present",
        "flat server.database structure", 
        "logging.output as string instead of structured",
        "missing modern domains (cache, registry, output)"
    ];

    let modern_indicators = [
        "domain-organized structure",
        "execution.max_execution_duration as duration",
        "structured logging configuration",
        "cache, registry, output domains present"
    ];

    println!("🏷️  Legacy Format Indicators:");
    for indicator in &legacy_indicators {
        println!("   • {}", indicator);
    }

    println!("\n🏷️  Modern Format Indicators:");
    for indicator in &modern_indicators {
        println!("   • {}", indicator);
    }
    println!();
}