use anyhow::{Context, Result};
use clap::Parser;
use ratchet_config::{ConfigLoader, RatchetConfig};
use chrono::Utc;

// LibRatchetConfig removed - using RatchetConfig directly

#[cfg(feature = "server")]
use ratchet_execution::{
    CoordinatorMessage, MessageEnvelope, ProcessExecutorConfig, ProcessTaskExecutor,
    TaskExecutionResult, TaskValidationResult, WorkerMessage, WorkerStatus,
    ExecutionBridge,
};

// Use modern alternatives - Task loading handled by ratchet-js and ratchet-cli-tools

#[cfg(feature = "http")]
use ratchet_http::{HttpManager, HttpClient};

#[cfg(feature = "database")]
use ratchet_storage::seaorm::{connection::DatabaseConnection, repositories::RepositoryFactory};
use ratchet_storage::repositories::{BaseRepository, Repository};

use ratchet_registry::RegistryService;

#[cfg(feature = "server")]
use ratchet_server::Server;
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;

#[cfg(feature = "core")]
use ratchet_core::task::Task as CoreTask;

#[cfg(all(feature = "runtime", feature = "core"))]
use ratchet_runtime::{InMemoryTaskExecutor, TaskExecutor};


#[cfg(feature = "javascript")]
use ratchet_js::{FileSystemTask, load_and_execute_task};

mod cli;
mod commands;
use cli::{Cli, Commands, ConfigCommands, GenerateCommands, RepoCommands};

/// Convert ratchet-storage RepositoryFactory to ratchet_lib RepositoryFactory
// Legacy repository factory function removed in 0.5.0 - use ratchet-storage directly

// Legacy config conversion function removed in 0.5.0 - use RatchetConfig directly

// Legacy MCP file logging function removed in 0.5.0 - use ratchet-logging directly

/// Load configuration from file or use defaults
fn load_config(config_path: Option<&PathBuf>) -> Result<RatchetConfig> {
    let loader = ConfigLoader::new();

    let config = match config_path {
        Some(path) => {
            if path.exists() {
                info!("Loading configuration from: {:?}", path);
                loader
                    .from_file(path)
                    .context(format!("Failed to load configuration from {:?}", path))?
            } else {
                warn!("Configuration file not found: {:?}. Using defaults.", path);
                loader
                    .from_env()
                    .context("Failed to load configuration from environment")?
            }
        }
        None => {
            debug!("No configuration file specified. Loading from environment or defaults.");
            let mut config = loader
                .from_env()
                .context("Failed to load configuration from environment")?;

            // If no server config exists and no explicit file was requested, create minimal defaults
            if config.server.is_none() {
                debug!("No server configuration found. Creating default server configuration.");
                config.server = Some(ratchet_config::ServerConfig::default());
            }

            config
        }
    };

    Ok(config)
}

/// Show configuration in YAML format
async fn show_config(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    let yaml = serde_yaml::to_string(&config)
        .context("Failed to serialize configuration to YAML")?;
    println!("{}", yaml);
    Ok(())
}

/// Get configuration value by path
async fn get_config_value(config_path: Option<&PathBuf>, key_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let yaml_value: serde_yaml::Value = serde_yaml::to_value(&config)
        .context("Failed to convert configuration to YAML value")?;
    
    let keys: Vec<&str> = key_path.split('.').collect();
    let mut current = &yaml_value;
    
    for key in keys {
        current = current.get(key)
            .with_context(|| format!("Key '{}' not found in configuration", key))?;
    }
    
    let output = serde_yaml::to_string(current)
        .context("Failed to serialize value to YAML")?;
    print!("{}", output);
    Ok(())
}

/// Set configuration value by path
async fn set_config_value(
    config_path: Option<&PathBuf>,
    key_path: &str,
    value: &str,
) -> Result<()> {
    let config_file = match config_path {
        Some(path) => path.clone(),
        None => {
            return Err(anyhow::anyhow!(
                "Config file path required when setting values"
            ));
        }
    };

    let mut config = if config_file.exists() {
        load_config(Some(&config_file))?
    } else {
        RatchetConfig::default()
    };

    // Parse the value as YAML to handle different types
    let parsed_value: serde_yaml::Value = serde_yaml::from_str(value)
        .context("Failed to parse value as YAML")?;

    // Convert config to mutable YAML value for easier manipulation
    let mut yaml_config: serde_yaml::Value = serde_yaml::to_value(&config)
        .context("Failed to convert configuration to YAML")?;

    // Set the value using the key path
    let keys: Vec<&str> = key_path.split('.').collect();
    set_nested_value(&mut yaml_config, &keys, parsed_value)?;

    // Convert back to config struct
    config = serde_yaml::from_value(yaml_config)
        .context("Failed to convert YAML back to configuration")?;

    // Write the updated config to file
    let yaml_output = serde_yaml::to_string(&config)
        .context("Failed to serialize configuration to YAML")?;
    
    std::fs::write(&config_file, yaml_output)
        .context("Failed to write configuration file")?;
    
    info!("Configuration updated: {} = {}", key_path, value);
    Ok(())
}

/// Helper function to set nested values in YAML
fn set_nested_value(
    yaml: &mut serde_yaml::Value,
    keys: &[&str],
    value: serde_yaml::Value,
) -> Result<()> {
    if keys.is_empty() {
        return Err(anyhow::anyhow!("Empty key path"));
    }

    if keys.len() == 1 {
        if let serde_yaml::Value::Mapping(ref mut map) = yaml {
            map.insert(
                serde_yaml::Value::String(keys[0].to_string()),
                value,
            );
        } else {
            return Err(anyhow::anyhow!("Cannot set key on non-object value"));
        }
    } else {
        if let serde_yaml::Value::Mapping(ref mut map) = yaml {
            let key = serde_yaml::Value::String(keys[0].to_string());
            let entry = map
                .entry(key.clone())
                .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
            set_nested_value(entry, &keys[1..], value)?;
        } else {
            return Err(anyhow::anyhow!("Cannot traverse non-object value"));
        }
    }

    Ok(())
}

/// List available repositories
async fn list_repositories(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    
    if let Some(registry_config) = &config.registry {
        println!("Available repositories:");
        for source in &registry_config.sources {
            println!("  - {} ({})", source.name, source.uri);
            println!("    Type: {:?}", source.source_type);
            // Note: description moved to source-specific config
            println!();
        }
    } else {
        println!("No registry configuration found.");
    }
    Ok(())
}

/// Synchronize repositories to database
async fn sync_repositories(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    
    let registry_config = config.registry
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No registry configuration found"))?;
    
    if registry_config.sources.is_empty() {
        info!("No repositories configured for synchronization");
        return Ok(());
    }
    
    info!("Starting repository synchronization...");
    
    // Convert config sources to registry task sources
    let mut sources = Vec::new();
    for source_config in &registry_config.sources {
        let task_source = match source_config.source_type {
            ratchet_config::domains::registry::RegistrySourceType::Filesystem => {
                ratchet_registry::config::TaskSource::Filesystem {
                    path: source_config.uri.clone(),
                    recursive: true, // Default value
                    watch: source_config.config.filesystem.watch_changes,
                }
            }
            ratchet_config::domains::registry::RegistrySourceType::Http => {
                ratchet_registry::config::TaskSource::Http {
                    url: source_config.uri.clone(),
                    auth: None, // TODO: Map authentication from config
                    polling_interval: source_config.polling_interval
                        .unwrap_or(registry_config.default_polling_interval),
                }
            }
            ratchet_config::domains::registry::RegistrySourceType::Git => {
                ratchet_registry::config::TaskSource::Git {
                    url: source_config.uri.clone(),
                    auth: None, // TODO: Map authentication from config
                    config: ratchet_registry::config::GitConfig {
                        branch: source_config.config.git.branch.clone(),
                        subdirectory: source_config.config.git.subdirectory.clone(),
                        shallow: source_config.config.git.shallow,
                        depth: source_config.config.git.depth,
                        sync_strategy: match source_config.config.git.sync_strategy {
                            ratchet_config::domains::registry::GitSyncStrategy::Clone => 
                                ratchet_registry::config::GitSyncStrategy::Clone,
                            ratchet_config::domains::registry::GitSyncStrategy::Fetch => 
                                ratchet_registry::config::GitSyncStrategy::Fetch,
                            ratchet_config::domains::registry::GitSyncStrategy::Pull => 
                                ratchet_registry::config::GitSyncStrategy::Pull,
                        },
                        cleanup_on_error: source_config.config.git.cleanup_on_error,
                        verify_signatures: source_config.config.git.verify_signatures,
                        allowed_refs: source_config.config.git.allowed_refs.clone(),
                        timeout: source_config.config.git.timeout,
                        max_repo_size: source_config.config.git.max_repo_size.clone(),
                        local_cache_path: source_config.config.git.local_cache_path.clone(),
                        cache_ttl: source_config.config.git.cache_ttl,
                        keep_history: source_config.config.git.keep_history,
                    },
                }
            }
            ratchet_config::domains::registry::RegistrySourceType::S3 => {
                warn!("S3 repositories not yet supported for synchronization: {}", source_config.name);
                continue;
            }
        };
        sources.push(task_source);
    }

    if sources.is_empty() {
        info!("No supported repositories found for synchronization");
        return Ok(());
    }

    // Create registry service configuration
    let registry_service_config = ratchet_registry::config::RegistryConfig {
        sources,
        sync_interval: registry_config.default_polling_interval,
        enable_auto_sync: true,
        enable_validation: true,
        cache_config: ratchet_registry::config::CacheConfig {
            enabled: registry_config.cache.enabled,
            max_size: registry_config.cache.max_entries,
            ttl: registry_config.cache.ttl,
        },
    };

    // Create database connection for sync service using storage-based repository
    let database_url = config.server.as_ref()
        .map(|s| s.database.url.clone())
        .unwrap_or_else(|| "sqlite::memory:".to_string());
    
    // Use the same storage configuration as the main application
    let storage_config = if database_url.starts_with("sqlite:") {
        // Extract the path from the URL
        let path = if database_url == "sqlite::memory:" {
            ":memory:".to_string()
        } else if database_url.starts_with("sqlite://") {
            // Remove sqlite:// prefix
            database_url.strip_prefix("sqlite://").unwrap().to_string()
        } else if database_url.starts_with("sqlite:") {
            // Remove sqlite: prefix and handle relative paths
            let path_part = database_url.strip_prefix("sqlite:").unwrap();
            if path_part.starts_with("//") {
                // Absolute path: sqlite://path -> path
                path_part.strip_prefix("//").unwrap().to_string()
            } else {
                // Relative path or special case: sqlite:path -> path
                path_part.to_string()
            }
        } else {
            // Assume it's already a path
            database_url.clone()
        };
        debug!("Parsed SQLite path for sync: {}", path);
        ratchet_storage::config::StorageConfig::sqlite(path)
    } else {
        return Err(anyhow::anyhow!("Only SQLite databases are currently supported for repository synchronization"));
    };

    // Override connection settings from config
    let mut storage_config = storage_config;
    storage_config.connection.max_connections = config.server.as_ref()
        .map(|s| s.database.max_connections)
        .unwrap_or(10);
    storage_config.connection.connect_timeout = config.server.as_ref()
        .map(|s| s.database.connection_timeout)
        .unwrap_or_else(|| std::time::Duration::from_secs(30));

    // Use the standard storage repository factory instead of SeaORM directly
    
    // For now, use the regular storage repository instead of the adapter to get the build working
    let connection_manager = ratchet_storage::connection::create_connection_manager(&storage_config)
        .await
        .context("Failed to create connection manager for repository synchronization")?;
    let repository_factory = ratchet_storage::repositories::RepositoryFactory::new(connection_manager.clone());
    let task_repo = Arc::new(repository_factory.task_repository());
    let task_repo_for_verification = task_repo.clone();
    
    let sync_service = Arc::new(ratchet_registry::sync::DatabaseSync::new(task_repo));

    // Create registry service with sync capability
    let registry_service = ratchet_registry::service::DefaultRegistryService::new(registry_service_config)
        .with_sync_service(sync_service);

    // Perform synchronization
    info!("🔄 Discovering tasks from repositories...");
    let sync_result = registry_service.sync_to_database().await
        .context("Failed to synchronize repositories to database")?;

    info!("✅ Repository synchronization completed successfully");
    info!("   ➕ Tasks added: {}", sync_result.tasks_added);
    info!("   🔄 Tasks updated: {}", sync_result.tasks_updated);
    
    if !sync_result.errors.is_empty() {
        warn!("   ⚠️  Errors encountered: {}", sync_result.errors.len());
        for error in &sync_result.errors {
            warn!("     - {}: {}", error.task_ref.name, error.error);
        }
    }

    // Add a simple verification: try to count tasks in the actual database
    info!("🔍 Verifying repository synchronization...");
    match task_repo_for_verification.health_check().await {
        Ok(true) => {
            info!("   📊 Repository health check: true");
        }
        Ok(false) => {
            warn!("   ⚠️  Repository health check returned false");
        }
        Err(e) => {
            warn!("   ⚠️  Repository health check failed: {}", e);
        }
    }
    
    // Also verify that tasks were actually persisted to the database
    let query = ratchet_storage::entities::Query::new();
    match task_repo_for_verification.count(&query).await {
        Ok(count) => {
            info!("   📈 Actual tasks in database: {}", count);
            if count == 0 {
                warn!("   ⚠️  No tasks found in database despite sync claiming success");
                warn!("   💡 This suggests the sync process is using a different database or failed to persist");
            }
        }
        Err(e) => {
            warn!("   ⚠️  Failed to count tasks in database: {}", e);
        }
    }

    Ok(())
}

/// Start the MCP (Model Context Protocol) server (legacy wrapper function)
#[cfg(feature = "mcp-server")]
#[allow(dead_code)]
async fn mcp_serve_command(
    config_path: Option<&PathBuf>,
    transport: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    let ratchet_config = load_config(config_path)?;
    mcp_serve_command_with_config(ratchet_config, transport, host, port).await
}

#[cfg(not(feature = "mcp-server"))]
async fn mcp_serve_command(
    _config_path: Option<&PathBuf>,
    _transport: &str,
    _host: &str,
    _port: u16,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "MCP server feature not enabled. Please compile with --features mcp-server"
    ))
}

/// Start the MCP (Model Context Protocol) server with provided config
#[cfg(feature = "mcp-server")]
async fn mcp_serve_command_with_config(
    config: RatchetConfig,
    transport: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    use ratchet_mcp::McpServer;
    use ratchet_mcp::transport::{McpTransport, StdioTransport};
    
    info!("Starting MCP server with {} transport", transport);
    
    // Create the MCP server - simplified for now
    // TODO: Properly configure McpServer with registry, auth, and audit logger
    info!("MCP server creation temporarily simplified - full implementation needed");
    
    // Choose transport based on the argument
    let transport: Box<dyn McpTransport> = match transport.to_lowercase().as_str() {
        "stdio" => {
            let stdio_transport = StdioTransport::new(
                "ratchet".to_string(),
                vec!["mcp-serve".to_string()],
                HashMap::new(),
                None,
            )?;
            Box::new(stdio_transport)
        }
        "http" => {
            // HTTP transport not available in current version
            return Err(anyhow::anyhow!("HTTP transport not available"));
        }
        _ => return Err(anyhow::anyhow!("Unsupported transport: {}. Use 'stdio' or 'http'", transport)),
    };
    
    // Run the server - temporarily disabled
    info!("MCP server would run with transport");
    Ok(())
}

#[cfg(not(feature = "mcp-server"))]
async fn mcp_serve_command_with_config(
    _config: RatchetConfig,
    _transport: &str,
    _host: &str,
    _port: u16,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "MCP server feature not enabled. Please compile with --features mcp-server"
    ))
}

/// Start the unified server
#[cfg(feature = "server")]
async fn server_command(
    config_path: Option<&PathBuf>,
    rest_port: Option<u16>,
    graphql_port: Option<u16>,
    mcp_port: Option<u16>,
) -> Result<()> {
    let config = load_config(config_path)?;
    
    // Override ports from command line if provided
    let mut server_config = config.server.clone().unwrap_or_default();
    if let Some(port) = rest_port {
        server_config.port = port;
    }
    // Note: GraphQL and MCP port configuration moved to dedicated config sections
    
    // Start the unified server
    info!("Starting Ratchet unified server...");
    info!("  Server: http://{}:{}", server_config.bind_address, server_config.port);
    
    // Create a new config with the updated server config
    let mut updated_config = config;
    updated_config.server = Some(server_config);
    
    // TODO: Convert RatchetConfig to ratchet-server ServerConfig
    info!("Server functionality temporarily disabled due to config conversion needs");
    Ok(())
}

#[cfg(not(feature = "server"))]
async fn server_command(
    _config_path: Option<&PathBuf>,
    _rest_port: Option<u16>,
    _graphql_port: Option<u16>,
    _mcp_port: Option<u16>,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "Server feature not enabled. Please compile with --features server"
    ))
}

/// Generate shell completions
async fn generate_completions(shell: clap_complete::Shell, output_dir: Option<&PathBuf>) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::generate_to;
    use clap_complete::Shell;

    let mut cmd = Cli::command();
    let bin_name = "ratchet";

    let output_path = match output_dir {
        Some(dir) => {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
            dir.clone()
        }
        None => std::env::current_dir()?,
    };

    let completion_file = generate_to(shell, &mut cmd, bin_name, &output_path)?;
    
    info!("Generated {} completion file: {:?}", shell, completion_file);
    Ok(())
}

/// Generate configuration file template
async fn generate_config(output_path: Option<&PathBuf>, format: &str) -> Result<()> {
    let config = RatchetConfig::default();
    
    let content = match format.to_lowercase().as_str() {
        "yaml" | "yml" => serde_yaml::to_string(&config)?,
        "json" => serde_json::to_string_pretty(&config)?,
        "toml" => serde_yaml::to_string(&config) // TOML support would need toml crate
            .context("Failed to serialize config to TOML")?,
        _ => return Err(anyhow::anyhow!("Unsupported format: {}. Use yaml, json, or toml", format)),
    };
    
    match output_path {
        Some(path) => {
            std::fs::write(path, content)?;
            info!("Generated configuration file: {:?}", path);
        }
        None => {
            println!("{}", content);
        }
    }
    
    Ok(())
}

/// Execute a task from the command line
#[cfg(all(feature = "runtime", feature = "core"))]
async fn execute_task(
    _config_path: Option<&PathBuf>,
    task_path: &str,
    input_data: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Executing task: {}", task_path);
    
    // Parse input data
    let input: JsonValue = match input_data {
        Some(data) => from_str(data).context("Failed to parse input JSON")?,
        None => json!({}),
    };
    
    // Create a simple in-memory task executor for CLI usage
    let executor = InMemoryTaskExecutor::new();
    
    // Load the task
    let task = std::fs::read_to_string(task_path)
        .context("Failed to read task file")?;
    let task: CoreTask = from_str(&task)
        .context("Failed to parse task definition")?;
    
    // Execute the task
    let result = executor.execute(&task, &input).await
        .context("Task execution failed")?;
    
    // Format and display output
    match output_format.to_lowercase().as_str() {
        "json" => {
            println!("{}", to_string_pretty(&result)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&result)?);
        }
        "pretty" => {
            println!("Task execution completed:");
            println!("  Result: {}", to_string_pretty(&result)?);
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported output format: {}. Use json, yaml, or pretty",
                output_format
            ));
        }
    }
    
    Ok(())
}

#[cfg(not(all(feature = "runtime", feature = "core")))]
async fn execute_task(
    _config_path: Option<&PathBuf>,
    _task_path: &str,
    _input_data: Option<&str>,
    _output_format: &str,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "Task execution requires both 'runtime' and 'core' features to be enabled"
    ))
}

/// Validate a task definition
async fn validate_task(task_path: &str) -> Result<()> {
    info!("Validating task: {}", task_path);
    
    // Read task file
    let task_content = std::fs::read_to_string(task_path)
        .context("Failed to read task file")?;
    
    // Parse as JSON first
    let task_json: JsonValue = from_str(&task_content)
        .context("Task file is not valid JSON")?;
    
    // Validate required fields
    let required_fields = ["name", "version", "path"];
    for field in &required_fields {
        if !task_json.get(field).is_some() {
            return Err(anyhow::anyhow!("Missing required field: {}", field));
        }
    }
    
    // Schema validation would require jsonschema crate - skipping for now
    if task_json.get("input_schema").is_some() {
        debug!("Input schema present (validation skipped)");
    }
    
    if task_json.get("output_schema").is_some() {
        debug!("Output schema present (validation skipped)");
    }
    
    info!("✅ Task definition is valid");
    Ok(())
}

/// List available tasks
async fn list_tasks(config_path: Option<&PathBuf>, format: &str) -> Result<()> {
    let config = load_config(config_path)?;
    
    // Check if we have storage configuration
    if let Some(server_config) = &config.server {
        info!("Listing tasks from database...");
        
        // Create storage connection
        let storage_config = ratchet_storage::config::StorageConfig::sqlite(&server_config.database.url);
        let connection_manager = ratchet_storage::connection::create_connection_manager(&storage_config)
            .await
            .context("Failed to create connection manager")?;
        let factory = ratchet_storage::repositories::RepositoryFactory::new(connection_manager);
        let task_repo = factory.task_repository();
        
        // Load tasks
        let query = ratchet_storage::entities::Query::new();
        let tasks = task_repo.find_all(&query).await
            .context("Failed to load tasks from database")?;
        
        // Format output
        match format.to_lowercase().as_str() {
            "json" => {
                println!("{}", to_string_pretty(&tasks)?);
            }
            "yaml" => {
                println!("{}", serde_yaml::to_string(&tasks)?);
            }
            "table" => {
                println!("{:<20} {:<10} {:<30} {:<10}", "Name", "Version", "Path", "Enabled");
                println!("{}", "-".repeat(80));
                for task in tasks {
                    println!("{:<20} {:<10} {:<30} {:<10}", 
                        task.name, 
                        task.version, 
                        task.path,
                        task.enabled
                    );
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported format: {}. Use json, yaml, or table",
                    format
                ));
            }
        }
    } else {
        info!("No database configuration found. Listing from registry sources...");
        
        if let Some(registry_config) = &config.registry {
            for source in &registry_config.sources {
                println!("Source: {} ({})", source.name, source.uri);
                // TODO: Actually scan the source for tasks
            }
        } else {
            println!("No registry configuration found.");
        }
    }
    
    Ok(())
}

/// Display status information
async fn status_command(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    
    println!("Ratchet Status");
    println!("==============");
    
    // Server configuration
    if let Some(server_config) = &config.server {
        println!("Server:");
        println!("  Database: {}", server_config.database.url);
        println!("  Server: {}:{}", server_config.bind_address, server_config.port);
        
        // Test database connection
        print!("  Database connection: ");
        match test_database_connection(&server_config.database.url).await {
            Ok(_) => println!("✅ Connected"),
            Err(e) => println!("❌ Failed: {}", e),
        }
    } else {
        println!("Server: Not configured");
    }
    
    // Registry configuration
    if let Some(registry_config) = &config.registry {
        println!("Registry:");
        println!("  Sources: {}", registry_config.sources.len());
        for source in &registry_config.sources {
            println!("    - {} ({})", source.name, source.uri);
        }
        println!("  Cache enabled: {}", registry_config.cache.enabled);
    } else {
        println!("Registry: Not configured");
    }
    
    // Logging configuration
    let logging_config = &config.logging;
    println!("Logging:");
    println!("  Level: {:?}", logging_config.level);
    println!("  Format: {:?}", logging_config.format);
    for target in &logging_config.targets {
        match target {
            ratchet_config::domains::logging::LogTarget::File { path, .. } => {
                println!("  File: {}", path);
            }
            _ => {}
        }
    }
    
    Ok(())
}

/// Test database connection
async fn test_database_connection(database_url: &str) -> Result<()> {
    let storage_config = ratchet_storage::config::StorageConfig::sqlite(database_url);
    let connection_manager = ratchet_storage::connection::create_connection_manager(&storage_config)
        .await
        .context("Failed to create connection manager")?;
    
    connection_manager.health_check().await
        .context("Database health check failed")?;
    
    Ok(())
}

/// Create shell command completions
async fn create_completion_command(
    shell: clap_complete::Shell,
    output_dir: Option<&PathBuf>,
) -> Result<()> {
    generate_completions(shell, output_dir).await
}

/// HTTP Management functions
#[cfg(feature = "http")]
async fn test_http_request(
    url: &str,
    method: &str,
    headers: Option<&str>,
    body: Option<&str>,
) -> Result<()> {
    let http_manager = HttpManager::new();
    
    info!("Testing HTTP request: {} {}", method, url);
    
    // Parse headers if provided
    let headers_map = if let Some(headers_str) = headers {
        let headers_json: JsonValue = from_str(headers_str)
            .context("Failed to parse headers as JSON")?;
        Some(headers_json)
    } else {
        None
    };
    
    // Parse body if provided
    let body_json = if let Some(body_str) = body {
        Some(from_str::<JsonValue>(body_str)
            .context("Failed to parse body as JSON")?)
    } else {
        None
    };
    
    // Execute request
    let params = json!({
        "method": method.to_uppercase(),
        "headers": headers_map.unwrap_or_default()
    });
    
    let response = http_manager.call_http(url, Some(&params), body_json.as_ref()).await
        .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;
    
    println!("Response:");
    println!("{}", to_string_pretty(&response)?);
    
    Ok(())
}

#[cfg(not(feature = "http"))]
async fn test_http_request(
    _url: &str,
    _method: &str,
    _headers: Option<&str>,
    _body: Option<&str>,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "HTTP feature not enabled. Please compile with --features http"
    ))
}

/// JavaScript execution functions  
#[cfg(feature = "javascript")]
async fn execute_js_task(
    _config_path: Option<&PathBuf>,
    script_path: &str,
    input_data: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Executing JavaScript task: {}", script_path);
    
    // Parse input data
    let input: JsonValue = match input_data {
        Some(data) => from_str(data).context("Failed to parse input JSON")?,
        None => json!({}),
    };
    
    // Execute the task directly from filesystem path
    let result = load_and_execute_task(script_path, input).await
        .map_err(|e| anyhow::anyhow!("JavaScript task execution failed: {}", e))?;
    
    // Format and display output
    match output_format.to_lowercase().as_str() {
        "json" => {
            println!("{}", to_string_pretty(&result)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&result)?);
        }
        "pretty" => {
            println!("JavaScript task execution completed:");
            println!("  Result: {}", to_string_pretty(&result)?);
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported output format: {}. Use json, yaml, or pretty",
                output_format
            ));
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "javascript"))]
async fn execute_js_task(
    _config_path: Option<&PathBuf>,
    _script_path: &str,
    _input_data: Option<&str>,
    _output_format: &str,
) -> Result<()> {
    Err(anyhow::anyhow!(
        "JavaScript feature not enabled. Please compile with --features javascript"
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle subcommands
    match cli.command {
        Some(Commands::Config { config_cmd }) => match config_cmd {
            ConfigCommands::Show { config_file, mcp_only: _, format: _ } => {
                show_config(config_file.as_ref()).await?;
            }
            ConfigCommands::Generate { config_type, output, force: _ } => {
                generate_config(Some(&output), &config_type).await?;
            }
            ConfigCommands::Validate { config_file } => {
                validate_task(&config_file.to_string_lossy()).await?;
            }
        },
        Some(Commands::Repo { repo_cmd }) => match repo_cmd {
            RepoCommands::Init { directory, name: _, description: _, version: _, ratchet_version: _, force: _ } => {
                info!("Repository initialization not yet implemented for: {:?}", directory);
            }
            RepoCommands::RefreshMetadata { directory: _, force: _ } => {
                info!("Repository refresh metadata not yet implemented");
            }
            RepoCommands::Status { detailed: _, repository: _, format: _ } => {
                info!("Repository status not yet implemented");
            }
            RepoCommands::Verify { repository: _, format: _, detailed: _, list_tasks: _, offline: _ } => {
                info!("Repository verify not yet implemented");
            }
        },
        Some(Commands::Generate { generate_cmd }) => match generate_cmd {
            GenerateCommands::Task { path, label: _, description: _, version: _ } => {
                info!("Task generation not yet implemented for: {:?}", path);
            }
            GenerateCommands::McpserversJson { name: _, command: _, args: _, config: _, transport: _, host: _, port: _, env: _, format: _, pretty: _ } => {
                info!("MCP servers JSON generation not yet implemented");
            }
        },
        Some(Commands::Mcp {
            config,
            transport,
            host,
            port,
        }) => {
            mcp_serve_command(config.as_ref(), &transport, &host, port).await?;
        }
        Some(Commands::McpServe {
            config,
            transport,
            host,
            port,
        }) => {
            mcp_serve_command(config.as_ref(), &transport, &host, port).await?;
        }
        Some(Commands::Serve { config }) => {
            // Updated to match the new CLI structure - no port arguments
            server_command(config.as_ref(), None, None, None).await?;
        }
        Some(Commands::RunOnce { from_fs, input_json, record: _ }) => {
            execute_js_task(None, &from_fs, input_json.as_deref(), "json").await?;
        }
        Some(Commands::Validate { from_fs }) => {
            validate_task(&from_fs).await?;
        }
        Some(Commands::Test { from_fs }) => {
            execute_js_task(None, &from_fs, None, "json").await?;
        }
        Some(Commands::Replay { from_fs, recording: _ }) => {
            execute_js_task(None, &from_fs, None, "json").await?;
        }
        Some(Commands::Console { config: _, connect: _, transport: _, host: _, port: _, auth_token: _, history_file: _, script: _ }) => {
            info!("Console mode not yet implemented");
        }
        None => {
            // No command provided, show help
            info!("No command provided. Use --help for usage information.");
        }
    }

    Ok(())
}