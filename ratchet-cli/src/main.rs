use anyhow::{Context, Result};
use clap::Parser;
use ratchet_config::{ConfigLoader, RatchetConfig};

// LibRatchetConfig removed - using RatchetConfig directly

// Use modern alternatives - Task loading handled by ratchet-js and ratchet-cli-tools

#[cfg(feature = "http")]
use ratchet_http::{HttpClient, HttpManager};

// Legacy repository imports removed - functionality temporarily disabled
// use ratchet_storage::repositories::{BaseRepository, Repository};

// use ratchet_registry::service::DefaultRegistryService; // Temporarily disabled

#[cfg(feature = "server")]
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

#[cfg(feature = "core")]
use ratchet_core::task::Task as CoreTask;

#[cfg(all(feature = "runtime", feature = "core"))]
use ratchet_runtime::{InMemoryTaskExecutor, TaskExecutor};

#[cfg(feature = "javascript")]
use ratchet_js::load_and_execute_task;

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
    let yaml = serde_yaml::to_string(&config).context("Failed to serialize configuration to YAML")?;
    println!("{}", yaml);
    Ok(())
}

/// Get configuration value by path
async fn get_config_value(config_path: Option<&PathBuf>, key_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let yaml_value: serde_yaml::Value =
        serde_yaml::to_value(&config).context("Failed to convert configuration to YAML value")?;

    let keys: Vec<&str> = key_path.split('.').collect();
    let mut current = &yaml_value;

    for key in keys {
        current = current
            .get(key)
            .with_context(|| format!("Key '{}' not found in configuration", key))?;
    }

    let output = serde_yaml::to_string(current).context("Failed to serialize value to YAML")?;
    print!("{}", output);
    Ok(())
}

/// Set configuration value by path
async fn set_config_value(config_path: Option<&PathBuf>, key_path: &str, value: &str) -> Result<()> {
    let config_file = match config_path {
        Some(path) => path.clone(),
        None => {
            return Err(anyhow::anyhow!("Config file path required when setting values"));
        }
    };

    let mut config = if config_file.exists() {
        load_config(Some(&config_file))?
    } else {
        RatchetConfig::default()
    };

    // Parse the value as YAML to handle different types
    let parsed_value: serde_yaml::Value = serde_yaml::from_str(value).context("Failed to parse value as YAML")?;

    // Convert config to mutable YAML value for easier manipulation
    let mut yaml_config: serde_yaml::Value =
        serde_yaml::to_value(&config).context("Failed to convert configuration to YAML")?;

    // Set the value using the key path
    let keys: Vec<&str> = key_path.split('.').collect();
    set_nested_value(&mut yaml_config, &keys, parsed_value)?;

    // Convert back to config struct
    config = serde_yaml::from_value(yaml_config).context("Failed to convert YAML back to configuration")?;

    // Write the updated config to file
    let yaml_output = serde_yaml::to_string(&config).context("Failed to serialize configuration to YAML")?;

    std::fs::write(&config_file, yaml_output).context("Failed to write configuration file")?;

    info!("Configuration updated: {} = {}", key_path, value);
    Ok(())
}

/// Helper function to set nested values in YAML
fn set_nested_value(yaml: &mut serde_yaml::Value, keys: &[&str], value: serde_yaml::Value) -> Result<()> {
    if keys.is_empty() {
        return Err(anyhow::anyhow!("Empty key path"));
    }

    if keys.len() == 1 {
        if let serde_yaml::Value::Mapping(ref mut map) = yaml {
            map.insert(serde_yaml::Value::String(keys[0].to_string()), value);
        } else {
            return Err(anyhow::anyhow!("Cannot set key on non-object value"));
        }
    } else if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let key = serde_yaml::Value::String(keys[0].to_string());
        let entry = map
            .entry(key.clone())
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        set_nested_value(entry, &keys[1..], value)?;
    } else {
        return Err(anyhow::anyhow!("Cannot traverse non-object value"));
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
    use ratchet_storage::seaorm::connection::DatabaseConnection;
    use ratchet_storage::seaorm::repositories::RepositoryFactory;

    info!("Starting repository synchronization");

    // Load configuration
    let config = load_config(config_path)?;

    // Ensure we have server configuration with database
    let server_config = config.server.as_ref().ok_or_else(|| {
        anyhow::anyhow!("No server configuration found. Database connection required for repository sync.")
    })?;

    // Create database connection using SeaORM
    info!("Connecting to database at: {}", server_config.database.url);
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: server_config.database.url.clone(),
        max_connections: server_config.database.max_connections,
        connection_timeout: server_config.database.connection_timeout,
    };

    let connection = DatabaseConnection::new(storage_db_config)
        .await
        .context("Failed to connect to database")?;
    let factory = RepositoryFactory::new(connection);

    // Create registry service if we have registry configuration
    if let Some(_registry_config) = &config.registry {
        info!("Registry sync functionality temporarily disabled while SeaORM integration is completed");

        // TODO: Fix config mapping between ratchet_config::RegistryConfig and ratchet_registry::RegistryConfig
        // The two config types have incompatible structures that need proper conversion

        println!("Repository synchronization is temporarily disabled.");
        println!("The feature is being updated to work with the new SeaORM backend.");
        println!("Use 'ratchet serve' to access full repository functionality via the server.");
    } else {
        warn!("No registry configuration found. Nothing to synchronize.");
        println!("No registry sources configured. Add registry sources to your configuration file to sync tasks.");
    }

    Ok(())
}

/// Start the MCP (Model Context Protocol) server (legacy wrapper function)
#[cfg(feature = "mcp-server")]
#[allow(dead_code)]
async fn mcp_serve_command(config_path: Option<&PathBuf>, transport: &str, host: &str, port: u16) -> Result<()> {
    let ratchet_config = load_config(config_path)?;
    mcp_serve_command_with_config(ratchet_config, transport, host, port).await
}

#[cfg(not(feature = "mcp-server"))]
async fn mcp_serve_command(_config_path: Option<&PathBuf>, _transport: &str, _host: &str, _port: u16) -> Result<()> {
    Err(anyhow::anyhow!(
        "MCP server feature not enabled. Please compile with --features mcp-server"
    ))
}

/// Start the MCP (Model Context Protocol) server with provided config
#[cfg(feature = "mcp-server")]
async fn mcp_serve_command_with_config(config: RatchetConfig, transport: &str, host: &str, port: u16) -> Result<()> {
    use ratchet_execution::{ExecutionBridge, ProcessExecutorConfig};
    use ratchet_mcp::server::adapter::RatchetMcpAdapterBuilder;
    use ratchet_mcp::{config::McpConfig, config::SimpleTransportType, McpServer};
    use ratchet_storage::seaorm::connection::DatabaseConnection;
    use ratchet_storage::seaorm::repositories::RepositoryFactory;

    info!("Starting MCP server with {} transport", transport);

    // Validate transport type
    if transport.to_lowercase() != "stdio" {
        return Err(anyhow::anyhow!(
            "Only 'stdio' transport is currently supported. Use: ratchet mcp-serve stdio"
        ));
    }

    // Create MCP configuration
    let mcp_config = McpConfig {
        transport_type: SimpleTransportType::Stdio,
        host: host.to_string(),
        port,
        auth: Default::default(),
        limits: Default::default(),
        timeouts: Default::default(),
        tools: Default::default(),
    };

    // Create database connection if configured through server config
    let repositories = if let Some(server_config) = &config.server {
        info!("Connecting to database for MCP server");

        // Convert ratchet-config DatabaseConfig to ratchet-storage DatabaseConfig
        let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
            url: server_config.database.url.clone(),
            max_connections: server_config.database.max_connections,
            connection_timeout: server_config.database.connection_timeout,
        };

        let connection = DatabaseConnection::new(storage_db_config)
            .await
            .context("Failed to connect to database")?;
        let factory = RepositoryFactory::new(connection);
        Some(Arc::new(factory))
    } else {
        warn!("No server configuration found. MCP server will run with limited functionality.");
        None
    };

    // Create execution bridge with default configuration
    let execution_config = ProcessExecutorConfig {
        worker_count: 4,
        task_timeout_seconds: 300,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let execution_bridge = Arc::new(ExecutionBridge::new(execution_config));

    // Create MCP adapter with available components
    let adapter_builder = RatchetMcpAdapterBuilder::new().with_bridge_executor(execution_bridge);

    // Note: CLI integration with unified task service is disabled for now
    // The main implementation is in the server where it's needed most
    if let Some(_repo_factory) = repositories {
        info!("Repositories available but CLI integration with unified task service is pending");
    }

    let adapter = adapter_builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build MCP adapter: {}", e))?;

    // Create and start MCP server
    info!("Creating MCP server with stdio transport");
    let mut mcp_server = McpServer::with_adapter(mcp_config, adapter)
        .await
        .context("Failed to create MCP server")?;

    info!("Starting MCP server stdio transport");
    mcp_server.run_stdio().await.context("MCP server failed to run")?;

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
    let mut ratchet_config = config;
    if let Some(port) = rest_port {
        if let Some(ref mut server_config) = ratchet_config.server {
            server_config.port = port;
        }
    }
    // Note: GraphQL and MCP port configuration would be handled via config file

    // Convert RatchetConfig to ratchet-server ServerConfig
    let server_config = ratchet_server::config::ServerConfig::from_ratchet_config(ratchet_config)
        .context("Failed to convert configuration to server config")?;

    // Create and start the unified server
    info!("Creating Ratchet unified server...");
    let server = ratchet_server::Server::new(server_config)
        .await
        .context("Failed to create server")?;

    info!("Starting server...");
    server.start().await.context("Server failed to start")?;

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
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {}. Use yaml, json, or toml",
                format
            ))
        }
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
    let task = std::fs::read_to_string(task_path).context("Failed to read task file")?;
    let task: CoreTask = from_str(&task).context("Failed to parse task definition")?;

    // Execute the task
    let result = executor.execute(&task, &input).await.context("Task execution failed")?;

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

/// Validate a task definition and optionally fix missing files
async fn validate_task(task_path: &str, fix: bool) -> Result<()> {
    use std::path::Path;

    info!("Validating task: {}", task_path);

    let path = Path::new(task_path);

    if path.is_dir() {
        validate_task_directory(path, fix).await
    } else {
        validate_task_file(path, fix).await
    }
}

/// Validate a task directory structure
async fn validate_task_directory(task_dir: &Path, fix: bool) -> Result<()> {
    info!("Validating task directory: {:?}", task_dir);

    let mut issues = Vec::new();
    let mut fixed_issues = Vec::new();

    // Check required files
    let metadata_path = task_dir.join("metadata.json");
    let main_js_path = task_dir.join("main.js");
    let input_schema_path = task_dir.join("input.schema.json");
    let output_schema_path = task_dir.join("output.schema.json");

    // Validate metadata.json
    if !metadata_path.exists() {
        issues.push("Missing metadata.json file".to_string());
        if fix {
            generate_metadata_stub(&metadata_path, task_dir)?;
            fixed_issues.push("Generated metadata.json stub".to_string());
        }
    } else {
        validate_metadata_file(&metadata_path, fix)?;
    }

    // Validate main.js
    if !main_js_path.exists() {
        issues.push("Missing main.js file".to_string());
        if fix {
            generate_main_js_stub(&main_js_path)?;
            fixed_issues.push("Generated main.js stub".to_string());
        }
    } else {
        validate_js_file(&main_js_path)?;
    }

    // Validate input.schema.json
    if !input_schema_path.exists() {
        issues.push("Missing input.schema.json file".to_string());
        if fix {
            generate_input_schema_stub(&input_schema_path)?;
            fixed_issues.push("Generated input.schema.json stub".to_string());
        }
    } else {
        validate_schema_file(&input_schema_path, "input")?;
    }

    // Validate output.schema.json
    if !output_schema_path.exists() {
        issues.push("Missing output.schema.json file".to_string());
        if fix {
            generate_output_schema_stub(&output_schema_path)?;
            fixed_issues.push("Generated output.schema.json stub".to_string());
        }
    } else {
        validate_schema_file(&output_schema_path, "output")?;
    }

    // Create tests directory if it doesn't exist
    let tests_dir = task_dir.join("tests");
    if !tests_dir.exists() {
        issues.push("Missing tests/ directory".to_string());
        if fix {
            std::fs::create_dir_all(&tests_dir)?;
            generate_test_stub(&tests_dir.join("basic.test.js"))?;
            fixed_issues.push("Generated tests/ directory with basic.test.js".to_string());
        }
    }

    // Report results
    if !issues.is_empty() {
        warn!("Found {} validation issue(s):", issues.len());
        for issue in &issues {
            warn!("  ❌ {}", issue);
        }
    }

    if !fixed_issues.is_empty() {
        info!("Fixed {} issue(s):", fixed_issues.len());
        for fix in &fixed_issues {
            info!("  🔧 {}", fix);
        }
    }

    if issues.is_empty() || (fix && !fixed_issues.is_empty()) {
        info!("✅ Task directory validation completed successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Task validation failed with {} issues. Use --fix to automatically resolve missing files",
            issues.len()
        ))
    }
}

/// Validate a single task file (legacy format)
async fn validate_task_file(task_path: &Path, _fix: bool) -> Result<()> {
    info!("Validating task file: {:?}", task_path);

    // Read task file
    let task_content = std::fs::read_to_string(task_path).context("Failed to read task file")?;

    // Parse as JSON first
    let task_json: JsonValue = from_str(&task_content).context("Task file is not valid JSON")?;

    // Validate required fields
    let required_fields = ["name", "version", "path"];
    for field in &required_fields {
        if task_json.get(field).is_none() {
            return Err(anyhow::anyhow!("Missing required field: {}", field));
        }
    }

    // Check schema fields
    if task_json.get("input_schema").is_some() {
        debug!("Input schema present");
    }

    if task_json.get("output_schema").is_some() {
        debug!("Output schema present");
    }

    info!("✅ Task file validation completed");
    Ok(())
}

/// Generate a metadata.json stub
fn generate_metadata_stub(metadata_path: &Path, task_dir: &Path) -> Result<()> {
    use chrono::Utc;

    let task_name = task_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed_task");

    let metadata = serde_json::json!({
        "name": task_name,
        "uuid": uuid::Uuid::new_v4().to_string(),
        "version": "1.0.0",
        "label": format!("{} Task", task_name.replace(['_', '-'], " ")),
        "description": format!("TODO: Add description for {} task", task_name),
        "tags": ["stub", "TODO"],
        "category": "TODO",
        "author": "TODO: Add author name",
        "license": "MIT",
        "ratchet_version": "0.4.6",
        "created_at": Utc::now().to_rfc3339(),
        "updated_at": Utc::now().to_rfc3339()
    });

    std::fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}

/// Generate a main.js stub
fn generate_main_js_stub(main_js_path: &Path) -> Result<()> {
    let js_content = r#"/**
 * TODO: Implement your task logic here
 * 
 * This function should accept input parameters and return output
 * that matches the defined JSON schemas.
 */
function main(input) {
    // TODO: Add your implementation here
    console.log("Processing input:", JSON.stringify(input));
    
    // Example: return a simple result
    return {
        status: "success",
        message: "Task completed successfully",
        input_received: input
    };
}

// Export the main function
module.exports = { main };
"#;

    std::fs::write(main_js_path, js_content)?;
    Ok(())
}

/// Generate an input.schema.json stub
fn generate_input_schema_stub(input_schema_path: &Path) -> Result<()> {
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Task Input Schema",
        "description": "TODO: Define the expected input structure for this task",
        "properties": {
            "example_field": {
                "type": "string",
                "description": "TODO: Replace with actual input fields"
            }
        },
        "required": [],
        "additionalProperties": true
    });

    std::fs::write(input_schema_path, serde_json::to_string_pretty(&schema)?)?;
    Ok(())
}

/// Generate an output.schema.json stub
fn generate_output_schema_stub(output_schema_path: &Path) -> Result<()> {
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Task Output Schema",
        "description": "TODO: Define the expected output structure for this task",
        "properties": {
            "status": {
                "type": "string",
                "description": "Task execution status",
                "enum": ["success", "error"]
            },
            "message": {
                "type": "string",
                "description": "Human-readable status message"
            }
        },
        "required": ["status"],
        "additionalProperties": true
    });

    std::fs::write(output_schema_path, serde_json::to_string_pretty(&schema)?)?;
    Ok(())
}

/// Generate a test stub
fn generate_test_stub(test_path: &Path) -> Result<()> {
    let test_content = r#"/**
 * Basic tests for the task
 * TODO: Add comprehensive test cases
 */

const { main } = require('../main.js');

describe('Task Tests', () => {
    test('should execute successfully with valid input', () => {
        const input = {
            example_field: "test_value"
        };
        
        const result = main(input);
        
        expect(result).toBeDefined();
        expect(result.status).toBe("success");
    });
    
    test('should handle empty input', () => {
        const result = main({});
        
        expect(result).toBeDefined();
        expect(result.status).toBeDefined();
    });
    
    // TODO: Add more specific test cases based on your task requirements
});
"#;

    std::fs::write(test_path, test_content)?;
    Ok(())
}

/// Validate metadata.json file
fn validate_metadata_file(metadata_path: &Path, fix: bool) -> Result<()> {
    let content = std::fs::read_to_string(metadata_path)?;
    let mut metadata: JsonValue = serde_json::from_str(&content).context("metadata.json is not valid JSON")?;

    let required_fields = ["name", "version", "label"];
    let mut missing_fields = Vec::new();
    let mut fixed_fields = Vec::new();

    for field in &required_fields {
        if metadata.get(field).is_none() {
            missing_fields.push(*field);
        }
    }

    if !missing_fields.is_empty() {
        if fix {
            // Fix missing metadata fields
            let task_name = metadata_path
                .parent()
                .and_then(|dir| dir.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("unnamed_task");

            let metadata_obj = metadata
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("metadata.json is not a valid JSON object"))?;

            for field in &missing_fields {
                match *field {
                    "name" => {
                        metadata_obj.insert("name".to_string(), serde_json::Value::String(task_name.to_string()));
                        fixed_fields.push("name");
                    }
                    "version" => {
                        metadata_obj.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
                        fixed_fields.push("version");
                    }
                    "label" => {
                        let label = format!("{} Task", task_name.replace(['_', '-'], " "));
                        metadata_obj.insert("label".to_string(), serde_json::Value::String(label));
                        fixed_fields.push("label");
                    }
                    _ => {}
                }
            }

            // Add other useful fields if they don't exist
            if !metadata_obj.contains_key("uuid") {
                metadata_obj.insert(
                    "uuid".to_string(),
                    serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                );
                fixed_fields.push("uuid");
            }

            if !metadata_obj.contains_key("description") {
                let description = format!("TODO: Add description for {} task", task_name);
                metadata_obj.insert("description".to_string(), serde_json::Value::String(description));
                fixed_fields.push("description");
            }

            if !metadata_obj.contains_key("created_at") {
                metadata_obj.insert(
                    "created_at".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                fixed_fields.push("created_at");
            }

            if !metadata_obj.contains_key("updated_at") {
                metadata_obj.insert(
                    "updated_at".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                fixed_fields.push("updated_at");
            }

            // Write the updated metadata back to file
            std::fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;

            info!("🔧 Fixed metadata.json - added missing fields: {:?}", fixed_fields);
        } else {
            return Err(anyhow::anyhow!(
                "Missing required metadata fields: {:?}",
                missing_fields
            ));
        }
    }

    debug!("✅ metadata.json is valid");
    Ok(())
}

/// Validate JavaScript file
fn validate_js_file(js_path: &Path) -> Result<()> {
    let content = std::fs::read_to_string(js_path)?;

    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("JavaScript file is empty"));
    }

    // Basic check for function definition
    if !content.contains("function") && !content.contains("=>") && !content.contains("module.exports") {
        warn!("JavaScript file may not contain a proper function definition");
    }

    debug!("✅ main.js contains content");
    Ok(())
}

/// Validate JSON schema file
fn validate_schema_file(schema_path: &Path, schema_type: &str) -> Result<()> {
    let content = std::fs::read_to_string(schema_path)?;
    let schema: JsonValue =
        serde_json::from_str(&content).with_context(|| format!("{}.schema.json is not valid JSON", schema_type))?;

    // Basic schema validation
    if !schema.is_object() {
        return Err(anyhow::anyhow!("{} schema must be a JSON object", schema_type));
    }

    debug!("✅ {}.schema.json is valid", schema_type);
    Ok(())
}

/// Validate a configuration file
async fn validate_config_file(config_path: &Path) -> Result<()> {
    info!("Validating configuration file: {:?}", config_path);

    let content = std::fs::read_to_string(config_path).context("Failed to read configuration file")?;

    // Try to parse as YAML first, then JSON
    let _config: serde_json::Value = if config_path.extension().and_then(|s| s.to_str()) == Some("yaml")
        || config_path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content).context("Configuration file is not valid YAML")?
    } else {
        serde_json::from_str(&content).context("Configuration file is not valid JSON")?
    };

    info!("✅ Configuration file is valid");
    Ok(())
}

/// List available tasks
async fn list_tasks(config_path: Option<&PathBuf>, format: &str) -> Result<()> {
    use ratchet_storage::seaorm::connection::DatabaseConnection;
    use ratchet_storage::seaorm::repositories::RepositoryFactory;

    info!("Listing tasks from database");

    // Load configuration
    let config = load_config(config_path)?;

    // Ensure we have server configuration with database
    let server_config = config.server.as_ref().ok_or_else(|| {
        anyhow::anyhow!("No server configuration found. Database connection required for task listing.")
    })?;

    // Create database connection using SeaORM
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: server_config.database.url.clone(),
        max_connections: server_config.database.max_connections,
        connection_timeout: server_config.database.connection_timeout,
    };

    let connection = DatabaseConnection::new(storage_db_config)
        .await
        .context("Failed to connect to database")?;
    let factory = RepositoryFactory::new(connection);
    let task_repo = factory.task_repository();

    // List all tasks from the database
    let tasks = task_repo
        .find_all()
        .await
        .context("Failed to list tasks from database")?;

    // Format and display tasks
    match format.to_lowercase().as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&tasks)?);
        }
        "table" | "pretty" => {
            if tasks.is_empty() {
                println!("No tasks found in database.");
            } else {
                println!("Tasks in database ({} total):", tasks.len());
                println!(
                    "{:<20} {:<10} {:<15} {:<10} {}",
                    "Name", "Version", "Status", "Enabled", "Path"
                );
                println!("{}", "=".repeat(80));
                for task in &tasks {
                    let status = if task.validated_at.is_some() {
                        "Validated"
                    } else {
                        "Pending"
                    };
                    let enabled = if task.enabled { "Yes" } else { "No" };
                    println!(
                        "{:<20} {:<10} {:<15} {:<10} {}",
                        task.name, task.version, status, enabled, task.path.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {}. Use json, yaml, or table",
                format
            ));
        }
    }

    info!("Listed {} tasks", tasks.len());
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
        if let ratchet_config::domains::logging::LogTarget::File { path, .. } = target {
            println!("  File: {}", path);
        }
    }

    Ok(())
}

/// Test database connection
async fn test_database_connection(database_url: &str) -> Result<()> {
    use ratchet_storage::seaorm::connection::DatabaseConnection;
    use ratchet_storage::seaorm::repositories::RepositoryFactory;

    // Create database connection using SeaORM
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: database_url.to_string(),
        max_connections: 1,
        connection_timeout: std::time::Duration::from_secs(5),
    };

    let connection = DatabaseConnection::new(storage_db_config)
        .await
        .context("Failed to connect to database")?;
    let factory = RepositoryFactory::new(connection);
    let task_repo = factory.task_repository();

    // Test the connection by performing a health check
    task_repo
        .health_check_send()
        .await
        .context("Database health check failed")?;

    Ok(())
}

/// Create shell command completions
async fn create_completion_command(shell: clap_complete::Shell, output_dir: Option<&PathBuf>) -> Result<()> {
    generate_completions(shell, output_dir).await
}

/// HTTP Management functions
#[cfg(feature = "http")]
async fn test_http_request(url: &str, method: &str, headers: Option<&str>, body: Option<&str>) -> Result<()> {
    let http_manager = HttpManager::new();

    info!("Testing HTTP request: {} {}", method, url);

    // Parse headers if provided
    let headers_map = if let Some(headers_str) = headers {
        let headers_json: JsonValue = from_str(headers_str).context("Failed to parse headers as JSON")?;
        Some(headers_json)
    } else {
        None
    };

    // Parse body if provided
    let body_json = if let Some(body_str) = body {
        Some(from_str::<JsonValue>(body_str).context("Failed to parse body as JSON")?)
    } else {
        None
    };

    // Execute request
    let params = json!({
        "method": method.to_uppercase(),
        "headers": headers_map.unwrap_or_default()
    });

    let response = http_manager
        .call_http(url, Some(&params), body_json.as_ref())
        .await
        .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

    println!("Response:");
    println!("{}", to_string_pretty(&response)?);

    Ok(())
}

#[cfg(not(feature = "http"))]
async fn test_http_request(_url: &str, _method: &str, _headers: Option<&str>, _body: Option<&str>) -> Result<()> {
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
    let result = load_and_execute_task(script_path, input)
        .await
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
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle subcommands
    match cli.command {
        Some(Commands::Config { config_cmd }) => match config_cmd {
            ConfigCommands::Show {
                config_file,
                mcp_only: _,
                format: _,
            } => {
                show_config(config_file.as_ref()).await?;
            }
            ConfigCommands::Generate {
                config_type,
                output,
                force: _,
            } => {
                generate_config(Some(&output), &config_type).await?;
            }
            ConfigCommands::Validate { config_file } => {
                validate_config_file(&config_file).await?;
            }
        },
        Some(Commands::Repo { repo_cmd }) => match repo_cmd {
            RepoCommands::Init {
                directory,
                name: _,
                description: _,
                version: _,
                ratchet_version: _,
                force: _,
            } => {
                info!("Repository initialization not yet implemented for: {:?}", directory);
            }
            RepoCommands::RefreshMetadata { directory: _, force: _ } => {
                info!("Repository refresh metadata not yet implemented");
            }
            RepoCommands::Status {
                detailed: _,
                repository: _,
                format: _,
            } => {
                info!("Repository status not yet implemented");
            }
            RepoCommands::Verify {
                repository: _,
                format: _,
                detailed: _,
                list_tasks: _,
                offline: _,
            } => {
                info!("Repository verify not yet implemented");
            }
        },
        Some(Commands::Generate { generate_cmd }) => match generate_cmd {
            GenerateCommands::Task {
                path,
                label: _,
                description: _,
                version: _,
            } => {
                info!("Task generation not yet implemented for: {:?}", path);
            }
            GenerateCommands::McpserversJson {
                name: _,
                command: _,
                args: _,
                config: _,
                transport: _,
                host: _,
                port: _,
                env: _,
                format: _,
                pretty: _,
            } => {
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
        Some(Commands::RunOnce {
            from_fs,
            input_json,
            record: _,
        }) => {
            execute_js_task(None, &from_fs, input_json.as_deref(), "json").await?;
        }
        Some(Commands::Validate { from_fs, fix }) => {
            validate_task(&from_fs, fix).await?;
        }
        Some(Commands::Test { from_fs }) => {
            execute_js_task(None, &from_fs, None, "json").await?;
        }
        Some(Commands::Replay { from_fs, recording: _ }) => {
            execute_js_task(None, &from_fs, None, "json").await?;
        }
        Some(Commands::Console {
            config,
            connect,
            transport,
            host,
            port,
            auth_token,
            history_file,
            script,
        }) => {
            let console_config = commands::console::ConsoleConfig {
                config_file: config,
                connect_url: connect,
                transport,
                host,
                port,
                auth_token,
                history_file,
                script_file: script,
            };
            commands::console::run_console(console_config).await?;
        }
        Some(Commands::Update {
            check_only,
            force,
            pre_release,
            version,
            install_dir,
            backup,
            rollback,
            dry_run,
            skip_verify,
        }) => {
            let update_cmd = commands::update::command::UpdateCommand {
                check_only,
                force,
                pre_release,
                version,
                install_dir,
                backup,
                rollback,
                dry_run,
                skip_verify,
            };
            update_cmd.execute().await?;
        }
        None => {
            // No command provided, show help
            info!("No command provided. Use --help for usage information.");
        }
    }

    Ok(())
}
