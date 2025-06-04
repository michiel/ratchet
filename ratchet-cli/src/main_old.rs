use anyhow::{Context, Result};
use clap::Parser;
use ratchet_lib::{
    config::RatchetConfig,
    task::Task,
    execution::ipc::{CoordinatorMessage, TaskExecutionResult},
    registry::{TaskSource, DefaultRegistryService, RegistryService},
    services::TaskSyncService,
    recording, task, validation, generate,
};
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use std::path::PathBuf;
use std::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use uuid::Uuid;

mod cli;
use cli::{Cli, Commands};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", global = true)]
    log_level: Option<String>,

    /// Run as worker process (internal use)
    #[arg(long, hide = true)]
    worker: bool,
    
    /// Worker ID (used with --worker)
    #[arg(long, value_name = "ID", hide = true)]
    worker_id: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single task from a file system path
    RunOnce {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,

        /// JSON input for the task (example: --input-json='{"num1":5,"num2":10}')
        #[arg(long, value_name = "JSON")]
        input_json: Option<String>,
        
        /// Record execution to directory with timestamp
        #[arg(long, value_name = "PATH")]
        record: Option<PathBuf>,
    },

    /// Start the Ratchet server with GraphQL API and task execution
    Serve {
        /// Path to configuration file (YAML)
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },

    /// Start the MCP (Model Context Protocol) server for LLM integration
    McpServe {
        /// Path to configuration file (YAML)
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        
        /// Transport type to use (stdio or sse)
        #[arg(long, default_value = "stdio")]
        transport: String,
        
        /// Server host for SSE transport
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        
        /// Server port for SSE transport
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Validate a task's structure and syntax
    Validate {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },

    /// Run tests for a task
    Test {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },

    /// Replay a task using recorded inputs from a previous session
    Replay {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,

        /// Path to the recording directory with input.json, output.json, etc.
        #[arg(long, value_name = "PATH")]
        recording: PathBuf,
    },

    /// Generate task template files
    Generate {
        #[command(subcommand)]
        generate_cmd: GenerateCommands,
    },
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generate a new task template with stub files
    Task {
        /// Path where to create the task directory
        #[arg(long, value_name = "PATH")]
        path: PathBuf,

        /// Task label/name
        #[arg(long, value_name = "STRING")]
        label: Option<String>,

        /// Task description
        #[arg(long, value_name = "STRING")]
        description: Option<String>,

        /// Task version
        #[arg(long, value_name = "STRING")]
        version: Option<String>,
    },
}

/// Load configuration from file or use defaults
fn load_config(config_path: Option<&PathBuf>) -> Result<RatchetConfig> {
    use ratchet_lib::config::{RatchetConfig, ServerConfig};
    use std::time::Duration;
    
    let mut config = match config_path {
        Some(path) => {
            if path.exists() {
                info!("Loading configuration from: {:?}", path);
                RatchetConfig::from_file(path)
                    .context(format!("Failed to load configuration from {:?}", path))?
            } else {
                warn!("Configuration file not found: {:?}. Using defaults.", path);
                RatchetConfig::default()
            }
        }
        None => {
            info!("No configuration file specified. Using default configuration with environment overrides");
            RatchetConfig::default()
        }
    };
    
    // Always apply environment overrides
    config.apply_env_overrides()
        .context("Failed to apply environment overrides")?;
    
    // Ensure server configuration exists for serve command
    if config.server.is_none() {
        info!("No server configuration found, using defaults");
        let mut server_config = ServerConfig::default();
        
        // Apply environment overrides for server config
        if let Ok(host) = std::env::var("RATCHET_SERVER_HOST") {
            server_config.bind_address = host;
        }
        if let Ok(port) = std::env::var("RATCHET_SERVER_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                server_config.port = port_num;
            }
        }
        if let Ok(db_url) = std::env::var("RATCHET_DATABASE_URL") {
            server_config.database.url = db_url;
        }
        if let Ok(max_conn) = std::env::var("RATCHET_DATABASE_MAX_CONNECTIONS") {
            if let Ok(max_conn_num) = max_conn.parse::<u32>() {
                server_config.database.max_connections = max_conn_num;
            }
        }
        if let Ok(timeout) = std::env::var("RATCHET_DATABASE_TIMEOUT") {
            if let Ok(timeout_secs) = timeout.parse::<u64>() {
                server_config.database.connection_timeout = Duration::from_secs(timeout_secs);
            }
        }
        
        config.server = Some(server_config);
    }
    
    // Validate the final configuration
    config.validate()
        .context("Configuration validation failed")?;
    
    Ok(config)
}

/// Start the Ratchet server
async fn serve_command(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    serve_command_with_config(config).await
}

async fn serve_command_with_config(config: RatchetConfig) -> Result<()> {
    use ratchet_lib::{
        database::DatabaseConnection,
        database::repositories::RepositoryFactory,
        execution::{JobQueueManager, ProcessTaskExecutor},
        server::create_app,
    };
    use std::sync::Arc;
    use tokio::signal;

    info!("Starting Ratchet server");
    
    // Get server configuration (guaranteed to exist from load_config)
    let server_config = config.server.as_ref().unwrap();

    info!("Server configuration loaded: {}:{}", server_config.bind_address, server_config.port);

    // Initialize database
    info!("Connecting to database: {}", server_config.database.url);
    let database = DatabaseConnection::new(server_config.database.clone()).await
        .context("Failed to connect to database")?;
    
    // Run migrations
    info!("Running database migrations");
    database.migrate().await.context("Failed to run database migrations")?;
    
    // Initialize repositories
    let repositories = RepositoryFactory::new(database);
    
    // Initialize job queue
    let job_queue = Arc::new(JobQueueManager::with_default_config(repositories.clone()));
    
    // Initialize process task executor
    info!("Initializing process task executor");
    let task_executor = Arc::new(
        ProcessTaskExecutor::new(repositories.clone(), config.clone()).await
            .context("Failed to initialize process task executor")?
    );
    
    // Start worker processes
    info!("Starting worker processes");
    task_executor.start().await.context("Failed to start worker processes")?;
    
    // Initialize registry if configured
    let registry = if let Some(registry_config) = &config.registry {
        info!("Initializing task registry");
        
        // Convert config sources to TaskSource
        let mut sources = Vec::new();
        let mut valid_configs = Vec::new();
        for source_config in &registry_config.sources {
            match TaskSource::from_config(source_config) {
                Ok(source) => {
                    info!("Added registry source: {} ({})", source_config.name, source_config.uri);
                    sources.push(source);
                    valid_configs.push(source_config.clone());
                },
                Err(e) => {
                    error!("Failed to parse registry source {}: {}", source_config.name, e);
                }
            }
        }
        
        // Create registry service with configs
        let mut registry_service = DefaultRegistryService::new_with_configs(sources, valid_configs);
        
        // Get the registry reference first
        let registry = registry_service.registry().await;
        
        // Create sync service for auto-registration
        let sync_service = Arc::new(TaskSyncService::new(
            repositories.task_repo.clone(),
            registry.clone(),
        ));
        
        // Set the sync service on the existing registry service
        registry_service = registry_service.with_sync_service(sync_service.clone());
        
        // Load all sources (this will auto-sync to database)
        if let Err(e) = registry_service.load_all_sources().await {
            error!("Failed to load registry sources: {}", e);
        }
        
        // Start watching filesystem sources if configured
        if let Err(e) = registry_service.start_watching().await {
            warn!("Failed to start filesystem watcher: {}", e);
            // Continue anyway - watching is optional
        }
        
        // Return both registry and sync service
        Some((registry, Some(sync_service)))
    } else {
        info!("No registry configuration found");
        None
    };
    
    // Extract registry and sync service
    let (registry, sync_service) = match registry {
        Some((reg, sync)) => (Some(reg), sync),
        None => (None, None),
    };
    
    // Create the application
    let app = create_app(
        repositories.clone(),
        job_queue,
        task_executor.clone(),
        registry,
        sync_service,
    );
    
    // Start MCP service if configured
    let mcp_service_handle = if let Some(mcp_config) = &config.mcp {
        if mcp_config.enabled {
            info!("Starting MCP service");
            
            use ratchet_mcp::server::McpService;
            
            // Determine log file path from logging config
            let log_file_path = config.logging.sinks.iter()
                .find_map(|sink| match sink {
                    ratchet_lib::logging::config::SinkConfig::File { path, .. } => Some(path.clone()),
                    _ => None,
                });
            
            // Create MCP service
            let mcp_service = match McpService::from_ratchet_config(
                mcp_config,
                task_executor.clone(),
                Arc::new(repositories.task_repo.clone()),
                Arc::new(repositories.execution_repo.clone()),
                log_file_path,
            ).await {
                Ok(service) => Arc::new(service),
                Err(e) => {
                    error!("Failed to create MCP service: {}", e);
                    return Err(anyhow::anyhow!("Failed to create MCP service: {}", e));
                }
            };
            
            // Start MCP service in background task if using SSE transport
            if mcp_config.transport == "sse" {
                let service = mcp_service.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = service.start().await {
                        error!("MCP service error: {}", e);
                    }
                }))
            } else {
                // For stdio transport, we'll need to handle it differently
                warn!("MCP stdio transport should be started separately using 'ratchet mcp-serve'");
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    
    // Create server address
    let addr = format!("{}:{}", server_config.bind_address, server_config.port);
    let addr: std::net::SocketAddr = addr.parse()
        .context(format!("Failed to parse address: {}", addr))?;
    
    info!("🚀 Ratchet server starting on http://{}", addr);
    info!("📊 GraphQL playground available at http://{}/playground", addr);
    info!("🏥 Health check available at http://{}/health", addr);
    info!("📖 REST API documentation available at http://{}/api-docs", addr);
    
    if let Some(mcp_config) = &config.mcp {
        if mcp_config.enabled && mcp_config.transport == "sse" {
            info!("🤖 MCP server available at http://{}:{}", mcp_config.host, mcp_config.port);
        }
    }
    
    // Create shutdown signal
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown");
    };
    
    // Start the server with graceful shutdown (axum 0.6 style)
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal);
    
    // Run server and handle shutdown
    match server.await {
        Ok(_) => {
            info!("Server stopped gracefully");
        }
        Err(e) => {
            error!("Server error: {}", e);
            return Err(e.into());
        }
    }
    
    // Stop MCP service if running
    if let Some(handle) = mcp_service_handle {
        info!("Stopping MCP service");
        handle.abort();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
    }
    
    // Stop worker processes
    info!("Stopping worker processes");
    task_executor.stop().await.context("Failed to stop worker processes")?;
    
    info!("Ratchet server shutdown complete");
    Ok(())
}

/// Start the MCP (Model Context Protocol) server
async fn mcp_serve_command(
    config_path: Option<&PathBuf>,
    transport: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    let ratchet_config = load_config(config_path)?;
    mcp_serve_command_with_config(ratchet_config, transport, host, port).await
}

async fn mcp_serve_command_with_config(
    ratchet_config: RatchetConfig,
    transport: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    use ratchet_mcp::{
        McpServer, McpConfig, SimpleTransportType,
        server::adapter::RatchetMcpAdapter,
    };
    use ratchet_lib::{
        database::DatabaseConnection,
        database::repositories::{
            task_repository::TaskRepository,
            execution_repository::ExecutionRepository,
        },
        execution::ProcessTaskExecutor,
    };
    use std::sync::Arc;
    use tokio::signal;

    info!("Starting Ratchet MCP server");

    // Parse transport type
    let transport_type = match transport.to_lowercase().as_str() {
        "stdio" => SimpleTransportType::Stdio,
        "sse" => SimpleTransportType::Sse,
        _ => {
            return Err(anyhow::anyhow!("Invalid transport type: {}. Use 'stdio' or 'sse'", transport));
        }
    };
    
    // Get MCP configuration (use dedicated MCP config if available, otherwise fall back to server config)
    let (database_config, mcp_server_config) = if let Some(mcp_config) = &ratchet_config.mcp {
        info!("Using dedicated MCP server configuration");
        // Use server database config if available, otherwise default
        let db_config = ratchet_config.server.as_ref()
            .map(|s| s.database.clone())
            .unwrap_or_else(ratchet_lib::config::DatabaseConfig::default);
        (db_config, Some(mcp_config.clone()))
    } else if let Some(server_config) = &ratchet_config.server {
        info!("Using server configuration for MCP server");
        (server_config.database.clone(), None)
    } else {
        info!("Using default configuration for MCP server");
        (ratchet_lib::config::DatabaseConfig::default(), None)
    };

    info!("MCP server configuration loaded");

    // Initialize database
    info!("Connecting to database: {}", database_config.url);
    let database = DatabaseConnection::new(database_config).await
        .context("Failed to connect to database")?;
    
    // Run migrations
    info!("Running database migrations");
    database.migrate().await.context("Failed to run database migrations")?;
    
    // Initialize repositories
    let task_repository = Arc::new(TaskRepository::new(database.clone()));
    let execution_repository = Arc::new(ExecutionRepository::new(database.clone()));
    
    // Initialize task executor
    info!("Initializing process task executor");
    let executor = Arc::new(
        ProcessTaskExecutor::new(
            ratchet_lib::database::repositories::RepositoryFactory::new(database),
            ratchet_config.clone()
        ).await.context("Failed to initialize process task executor")?
    );
    
    // Start worker processes
    info!("Starting worker processes");
    executor.start().await.context("Failed to start worker processes")?;
    
    // Create MCP adapter
    let adapter = RatchetMcpAdapter::new(
        executor.clone(),
        task_repository,
        execution_repository,
    );
    
    // Create MCP server config (prioritize file config, then CLI args, then defaults)
    let mcp_config = if let Some(file_config) = mcp_server_config {
        McpConfig {
            transport_type: if file_config.enabled { 
                match file_config.transport.as_str() {
                    "sse" => SimpleTransportType::Sse,
                    _ => SimpleTransportType::Stdio,
                }
            } else { 
                transport_type.clone() 
            },
            host: if file_config.host != "127.0.0.1" { 
                file_config.host 
            } else { 
                host.to_string() 
            },
            port: if file_config.port != 3000 { 
                file_config.port 
            } else { 
                port 
            },
            ..Default::default()
        }
    } else {
        McpConfig {
            transport_type: transport_type.clone(),
            host: host.to_string(),
            port,
            ..Default::default()
        }
    };
    
    info!("MCP server starting with transport: {:?}, host: {}, port: {}", 
          mcp_config.transport_type, mcp_config.host, mcp_config.port);
    
    // Create MCP server
    let mut server = McpServer::with_adapter(mcp_config, adapter).await
        .context("Failed to create MCP server")?;
    
    info!("🚀 Ratchet MCP server starting with {} transport", transport);
    
    match transport_type {
        SimpleTransportType::Stdio => {
            info!("MCP server ready on stdio - waiting for LLM connections");
            info!("Use this with LLM clients that support MCP over stdio");
        }
        SimpleTransportType::Sse => {
            info!("MCP server starting on http://{}:{}", host, port);
            info!("LLM clients can connect via Server-Sent Events");
        }
    }
    
    // Create shutdown signal
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown");
    };
    
    // Start server based on transport type
    let result = match transport_type {
        SimpleTransportType::Stdio => {
            tokio::select! {
                result = server.run_stdio() => result,
                _ = shutdown_signal => {
                    info!("Graceful shutdown initiated");
                    Ok(())
                }
            }
        }
        SimpleTransportType::Sse => {
            tokio::select! {
                result = server.run_sse() => result,
                _ = shutdown_signal => {
                    info!("Graceful shutdown initiated");
                    Ok(())
                }
            }
        }
    };
    
    // Handle any server errors
    if let Err(e) = result {
        error!("MCP server error: {}", e);
        return Err(e.into());
    }
    
    // Stop worker processes
    info!("Stopping worker processes");
    executor.stop().await.context("Failed to stop worker processes")?;
    
    info!("Ratchet MCP server shutdown complete");
    Ok(())
}

/// Initialize logging from configuration with fallback to simple tracing
fn init_logging_with_config(config: &RatchetConfig, log_level: Option<&String>, record_dir: Option<&PathBuf>) -> Result<()> {
    // If CLI log level is provided, override config level
    let mut logging_config = config.logging.clone();
    if let Some(level_str) = log_level {
        if let Ok(level) = level_str.parse::<ratchet_lib::logging::LogLevel>() {
            logging_config.level = level;
        }
    }
    
    // For recording mode, ensure we have file output
    if let Some(record_path) = record_dir {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let session_dir = record_path.join(format!("ratchet_session_{}", timestamp));
        fs::create_dir_all(&session_dir).context("Failed to create recording directory")?;
        
        // Add file sink for recording
        let log_file_path = session_dir.join("ratchet.log");
        logging_config.sinks.push(ratchet_lib::logging::config::SinkConfig::File {
            path: log_file_path,
            level: logging_config.level,
            rotation: Some(ratchet_lib::logging::config::RotationConfig {
                max_size: "100MB".to_string(),
                max_age: None,
                max_files: Some(5),
            }),
            buffered: None,
        });
        
        // Store the session directory for use by other components
        ratchet_lib::recording::set_recording_dir(session_dir)?;
        
        info!("Recording session to: {:?}", record_path.join(format!("ratchet_session_{}", timestamp)));
    }
    
    // Try to initialize structured logging
    match ratchet_lib::logging::init_logging_from_config(&logging_config) {
        Ok(()) => {
            debug!("Structured logging initialized");
        }
        Err(e) => {
            // Fall back to simple tracing if structured logging fails
            eprintln!("Failed to initialize structured logging: {}, falling back to simple tracing", e);
            init_simple_tracing(log_level)?;
        }
    }

    Ok(())
}

/// Initialize simple tracing with environment variable override support (fallback)
fn init_simple_tracing(log_level: Option<&String>) -> Result<()> {
    let env_filter = match log_level {
        Some(level) => {
            EnvFilter::try_new(level).unwrap_or_else(|_| {
                eprintln!("Invalid log level '{}', falling back to 'info'", level);
                EnvFilter::new("info")
            })
        }
        None => {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
    };
    
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    debug!("Simple tracing initialized");
    Ok(())
}

/// Initialize tracing for worker processes (output to stderr to avoid IPC conflicts)
fn init_worker_tracing(log_level: Option<&String>) -> Result<()> {
    let env_filter = match log_level {
        Some(level) => {
            // Use provided log level
            EnvFilter::try_new(level).unwrap_or_else(|_| {
                eprintln!("Invalid log level '{}', falling back to 'info'", level);
                EnvFilter::new("info")
            })
        }
        None => {
            // Try environment variable first, then default to info
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
    };
    
    // Configure tracing to output to stderr only (stdout is used for IPC)
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    Ok(())
}

/// Parse JSON input string into a JsonValue
fn parse_input_json(input: Option<&String>) -> Result<JsonValue> {
    match input {
        Some(json_str) => {
            debug!("Parsing input JSON: {}", json_str);
            from_str(json_str).context("Failed to parse input JSON")
        }
        None => {
            debug!("No input JSON provided, using empty object");
            // Default empty JSON object if no input provided
            Ok(json!({}))
        }
    }
}

/// Run a task with the given input
async fn run_task(task_path: &str, input_json: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {}", task_path);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Execute the task with the provided input
    info!("Executing task with input");
    let http_manager = ratchet_lib::http::HttpManager::new();
    let result = ratchet_lib::js_executor::execute_task(&mut task, input_json.clone(), &http_manager)
        .await
        .context("Failed to execute task")?;

    info!("Task execution completed successfully");
    Ok(result)
}

/// Validate a task's structure and syntax
fn validate_task(task_path: &str) -> Result<()> {
    info!("Validating task at: {}", task_path);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Validate the task
    task.validate().context("Task validation failed")?;

    println!("✓ Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);
    println!("  Description: {}", task.metadata.description);

    info!("Task validation completed successfully");
    Ok(())
}

/// Run tests for a task
async fn test_task(task_path: &str) -> Result<()> {
    info!("Running tests for task at: {}", task_path);

    // First validate the task
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    task.validate().context("Task validation failed")?;

    println!("Task validated successfully!");
    println!("  UUID: {}", task.uuid());
    println!("  Label: {}", task.metadata.label);
    println!("  Version: {}", task.metadata.version);

    // Run tests
    info!("Starting test execution");
    match ratchet_lib::test::run_tests(task_path).await {
        Ok(summary) => {
            info!(
                "Tests completed - Total: {}, Passed: {}, Failed: {}",
                summary.total, summary.passed, summary.failed
            );

            println!("\nTest Results:");
            println!("-------------");
            println!("Total tests: {}", summary.total);
            println!("Passed: {}", summary.passed);
            println!("Failed: {}", summary.failed);
            println!("-------------");

            // Print details of failed tests
            if summary.failed > 0 {
                warn!("Found {} failed tests", summary.failed);
                println!("\nFailed Tests:");
                for (i, result) in summary.results.iter().enumerate() {
                    if !result.passed {
                        let file_name = result.file_path.file_name().unwrap().to_string_lossy();
                        warn!("Test failed: {}", file_name);
                        println!("\n{}. Test: {}", i + 1, file_name);

                        if let Some(actual) = &result.actual_output {
                            // Get the expected output from the test file
                            let test_file_content = std::fs::read_to_string(&result.file_path)
                                .context(format!(
                                    "Failed to read test file: {:?}",
                                    result.file_path
                                ))?;
                            let test_json: JsonValue = serde_json::from_str(&test_file_content)
                                .context(format!(
                                    "Failed to parse test file: {:?}",
                                    result.file_path
                                ))?;
                            let expected = test_json.get("expected_output").unwrap();

                            println!("   Expected: {}", serde_json::to_string_pretty(expected)?);
                            println!("   Actual: {}", serde_json::to_string_pretty(actual)?);
                        } else if let Some(error) = &result.error_message {
                            error!("Test error: {}", error);
                            println!("   Error: {}", error);
                        }
                    }
                }

                // Return non-zero exit code for CI/CD pipelines
                error!("Tests failed, exiting with code 1");
                std::process::exit(1);
            } else if summary.total == 0 {
                warn!("No tests found");
                println!("\nNo tests found. Create test files in the 'tests' directory.");
            } else {
                info!("All tests passed successfully");
                println!("\nAll tests passed! ✓");
            }

            Ok(())
        }
        Err(err) => match err {
            ratchet_lib::test::TestError::NoTestsDirectory => {
                info!("No tests directory found");
                println!("\nNo tests directory found.");
                println!("Create a 'tests' directory with JSON test files to run tests.");
                println!("Each test file should contain 'input' and 'expected_output' fields.");
                println!("Example: {{ \"input\": {{ \"num1\": 5, \"num2\": 10 }}, \"expected_output\": {{ \"sum\": 15 }} }}");
                Ok(())
            }
            _ => {
                error!("Test execution failed: {:?}", err);
                Err(err).context("Test execution failed")
            }
        },
    }
}

/// Replay a task using recorded inputs from a previous session
async fn replay_task(task_path: &str, recording_dir: &PathBuf) -> Result<JsonValue> {
    info!("Replaying task from: {} with recording: {:?}", task_path, recording_dir);

    // Load the recorded input
    let input_file = recording_dir.join("input.json");
    if !input_file.exists() {
        return Err(anyhow::anyhow!("No input.json found in recording directory: {:?}", recording_dir));
    }

    let input_content = fs::read_to_string(&input_file)
        .context(format!("Failed to read input file: {:?}", input_file))?;
    let input_json: JsonValue = from_str(&input_content)
        .context("Failed to parse input JSON from recording")?;

    info!("Loaded recorded input from: {:?}", input_file);
    debug!("Input data: {}", to_string_pretty(&input_json)?);

    // Load the task from the filesystem
    let mut task = Task::from_fs(task_path)
        .context(format!("Failed to load task from path: {}", task_path))?;

    debug!("Task loaded: {} ({})", task.metadata.label, task.uuid());

    // Execute the task with the recorded input
    info!("Executing task with recorded input");
    let http_manager = ratchet_lib::http::HttpManager::new();
    let result = ratchet_lib::js_executor::execute_task(&mut task, input_json.clone(), &http_manager)
        .await
        .context("Failed to execute task")?;

    info!("Task replay completed successfully");
    
    // Compare with recorded output if available
    let output_file = recording_dir.join("output.json");
    if output_file.exists() {
        let recorded_output_content = fs::read_to_string(&output_file)
            .context(format!("Failed to read output file: {:?}", output_file))?;
        let recorded_output: JsonValue = from_str(&recorded_output_content)
            .context("Failed to parse recorded output JSON")?;

        if result == recorded_output {
            println!("✓ Output matches recorded output");
            info!("Output matches recorded output");
        } else {
            println!("⚠ Output differs from recorded output");
            warn!("Output differs from recorded output");
            println!("\nRecorded output:");
            println!("{}", to_string_pretty(&recorded_output)?);
            println!("\nActual output:");
            println!("{}", to_string_pretty(&result)?);
        }
    } else {
        warn!("No recorded output found for comparison at: {:?}", output_file);
    }

    Ok(result)
}

/// Generate a new task template with stub files
fn generate_task(
    path: &PathBuf,
    label: Option<&String>,
    description: Option<&String>,
    version: Option<&String>,
) -> Result<()> {
    info!("Generating task template at: {:?}", path);

    // Build configuration using the builder pattern
    let mut config = ratchet_lib::generate::TaskGenerationConfig::new(path.clone());
    
    if let Some(label) = label {
        config = config.with_label(label);
    }
    if let Some(description) = description {
        config = config.with_description(description);
    }
    if let Some(version) = version {
        config = config.with_version(version);
    }

    // Generate the task using ratchet-lib
    let generated_info = ratchet_lib::generate::generate_task(config)
        .context("Failed to generate task template")?;

    // Display success information
    println!("✓ Task template created successfully!");
    println!("  Path: {:?}", generated_info.path);
    println!("  UUID: {}", generated_info.uuid);
    println!("  Label: {}", generated_info.label);
    println!("  Version: {}", generated_info.version);
    println!("  Description: {}", generated_info.description);
    println!("\nFiles created:");
    for file in &generated_info.files_created {
        println!("  - {}        ({})", file, get_file_description(file));
    }
    println!("\nNext steps:");
    println!("  1. Edit main.js to implement your task logic");
    println!("  2. Update input.schema.json and output.schema.json as needed");
    println!("  3. Add more test cases in the tests/ directory");
    println!("  4. Validate: ratchet validate --from-fs={}", generated_info.path.display());
    println!("  5. Test: ratchet test --from-fs={}", generated_info.path.display());

    info!("Task template generation completed successfully");
    Ok(())
}

/// Get a human-readable description for a file type
fn get_file_description(file: &str) -> &'static str {
    match file {
        "metadata.json" => "task metadata",
        "input.schema.json" => "input validation schema",
        "output.schema.json" => "output validation schema", 
        "main.js" => "task implementation",
        "tests/test-001.json" => "sample test case",
        _ => "generated file",
    }
}

/// Run as a worker process that handles IPC messages
async fn run_worker_process(worker_id: String) -> Result<()> {
    use ratchet_lib::execution::ipc::{
        WorkerMessage, CoordinatorMessage, MessageEnvelope, 
        WorkerError,
    };
    
    info!("Worker process {} starting", worker_id);
    
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut line = String::new();
    
    // Send ready message
    let ready_msg = CoordinatorMessage::Ready {
        worker_id: worker_id.clone(),
    };
    send_message(&mut stdout, &ready_msg).await?;
    
    info!("Worker {} ready for tasks", worker_id);
    
    // Process messages
    loop {
        line.clear();
        
        match reader.read_line(&mut line).await {
            Ok(0) => {
                info!("Worker {} received EOF, shutting down", worker_id);
                break;
            }
            Ok(_) => {
                // Remove newline
                line.truncate(line.trim_end().len());
                
                if line.is_empty() {
                    continue;
                }
                
                // Parse message
                match serde_json::from_str::<MessageEnvelope<WorkerMessage>>(&line) {
                    Ok(envelope) => {
                        debug!("Worker {} received message: {:?}", worker_id, envelope.message);
                        
                        let response = match envelope.message {
                            WorkerMessage::ExecuteTask { job_id, task_id, task_path, input_data, correlation_id, .. } => {
                                execute_task_worker(job_id, task_id, &task_path, &input_data, correlation_id).await
                            }
                            WorkerMessage::ValidateTask { task_path, correlation_id } => {
                                validate_task_worker(&task_path, correlation_id).await
                            }
                            WorkerMessage::Ping { correlation_id } => {
                                handle_ping_worker(&worker_id, correlation_id).await
                            }
                            WorkerMessage::Shutdown => {
                                info!("Worker {} received shutdown signal", worker_id);
                                break;
                            }
                        };
                        
                        if let Err(e) = send_message(&mut stdout, &response).await {
                            error!("Worker {} failed to send response: {}", worker_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Worker {} failed to parse message: {} - line: {}", worker_id, e, line);
                        
                        let error_msg = CoordinatorMessage::Error {
                            correlation_id: None,
                            error: WorkerError::MessageParseError(e.to_string()),
                        };
                        
                        if let Err(e) = send_message(&mut stdout, &error_msg).await {
                            error!("Worker {} failed to send error response: {}", worker_id, e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Worker {} failed to read from stdin: {}", worker_id, e);
                break;
            }
        }
    }
    
    info!("Worker {} shutting down", worker_id);
    Ok(())
}

/// Send a message to stdout
async fn send_message(stdout: &mut tokio::io::Stdout, message: &CoordinatorMessage) -> Result<()> {
    let envelope = ratchet_lib::execution::ipc::MessageEnvelope::new(message.clone());
    let json = serde_json::to_string(&envelope)?;
    let line = format!("{}\n", json);
    
    stdout.write_all(line.as_bytes()).await?;
    stdout.flush().await?;
    
    Ok(())
}

/// Execute a task in the worker process
async fn execute_task_worker(
    _job_id: i32,
    _task_id: i32,
    task_path: &str,
    input_data: &JsonValue,
    correlation_id: Uuid,
) -> CoordinatorMessage {
    use ratchet_lib::execution::ipc::CoordinatorMessage;
    
    let started_at = chrono::Utc::now();
    
    // Execute the task
    match run_task(task_path, input_data).await {
        Ok(output) => {
            let completed_at = chrono::Utc::now();
            let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
            
            CoordinatorMessage::TaskResult {
                job_id: _job_id,
                correlation_id,
                result: TaskExecutionResult {
                    success: true,
                    output: Some(output),
                    error_message: None,
                    error_details: None,
                    started_at,
                    completed_at,
                    duration_ms,
                },
            }
        }
        Err(e) => {
            let completed_at = chrono::Utc::now();
            let duration_ms = (completed_at - started_at).num_milliseconds() as i32;
            
            CoordinatorMessage::TaskResult {
                job_id: _job_id,
                correlation_id,
                result: TaskExecutionResult {
                    success: false,
                    output: None,
                    error_message: Some(e.to_string()),
                    error_details: None,
                    started_at,
                    completed_at,
                    duration_ms,
                },
            }
        }
    }
}

/// Validate a task in the worker process
async fn validate_task_worker(
    task_path: &str,
    correlation_id: Uuid,
) -> CoordinatorMessage {
    use ratchet_lib::execution::ipc::{TaskValidationResult, CoordinatorMessage};
    
    match validate_task(task_path) {
        Ok(_) => CoordinatorMessage::ValidationResult {
            correlation_id,
            result: TaskValidationResult {
                valid: true,
                error_message: None,
                error_details: None,
            },
        },
        Err(e) => CoordinatorMessage::ValidationResult {
            correlation_id,
            result: TaskValidationResult {
                valid: false,
                error_message: Some(e.to_string()),
                error_details: None,
            },
        },
    }
}

/// Handle ping message in the worker process
async fn handle_ping_worker(
    worker_id: &str,
    correlation_id: Uuid,
) -> CoordinatorMessage {
    use ratchet_lib::execution::ipc::{WorkerStatus, CoordinatorMessage};
    
    CoordinatorMessage::Pong {
        correlation_id,
        worker_id: worker_id.to_string(),
        status: WorkerStatus {
            worker_id: worker_id.to_string(),
            pid: std::process::id(),
            started_at: chrono::Utc::now(), // TODO: Track actual start time
            last_activity: chrono::Utc::now(),
            tasks_executed: 0, // TODO: Track task count
            tasks_failed: 0, // TODO: Track failure count
            memory_usage_mb: None,
            cpu_usage_percent: None,
        },
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if running as worker process
    if cli.worker {
        let worker_id = cli.worker_id.unwrap_or_else(|| "unknown".to_string());
        
        // Initialize tracing for worker (stderr only to avoid IPC conflicts)
        init_worker_tracing(cli.log_level.as_ref())?;
        
        // Create a tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        
        // Run worker process
        return runtime.block_on(run_worker_process(worker_id));
    }

    // Load configuration first
    let config = load_config(cli.config.as_ref())?;
    
    // Initialize logging from config
    init_logging_with_config(&config, cli.log_level.as_ref(), cli.command.as_ref().and_then(|cmd| {
        match cmd {
            Commands::RunOnce { record, .. } => record.as_ref(),
            _ => None,
        }
    }))?;

    info!("Ratchet CLI starting");

    // Create a tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    match &cli.command {
        Some(Commands::RunOnce {
            from_fs,
            input_json,
            record,
        }) => {
            info!("Running task from file system path: {}", from_fs);

            // Parse input JSON
            let input = parse_input_json(input_json.as_ref())?;

            if input_json.is_some() {
                info!("Using provided input: {}", input_json.as_ref().unwrap());
            }

            // Run the task
            let result = runtime.block_on(run_task(from_fs, &input))?;

            // Pretty-print the result
            let formatted = to_string_pretty(&result).context("Failed to format result as JSON")?;

            println!("Result: {}", formatted);
            info!("Task execution completed");
            
            // Finalize recording if it was enabled
            if record.is_some() {
                if let Err(e) = ratchet_lib::recording::finalize_recording() {
                    warn!("Failed to finalize recording: {}", e);
                } else if let Some(dir) = ratchet_lib::recording::get_recording_dir() {
                    println!("Recording saved to: {:?}", dir);
                }
            }
            
            Ok(())
        }
        Some(Commands::Serve { config: config_override }) => {
            info!("Starting Ratchet server");
            // Use config override if provided, otherwise use loaded config
            let server_config = if config_override.is_some() {
                load_config(config_override.as_ref())?
            } else {
                config
            };
            runtime.block_on(serve_command_with_config(server_config))
        }
        Some(Commands::McpServe { config: config_override, transport, host, port }) => {
            info!("Starting MCP server");
            // Use config override if provided, otherwise use loaded config
            let mcp_config = if config_override.is_some() {
                load_config(config_override.as_ref())?
            } else {
                config
            };
            runtime.block_on(mcp_serve_command_with_config(mcp_config, transport, host, *port))
        }
        Some(Commands::Validate { from_fs }) => validate_task(from_fs),
        Some(Commands::Test { from_fs }) => runtime.block_on(test_task(from_fs)),
        Some(Commands::Replay { from_fs, recording }) => {
            info!("Replaying task from file system path: {} with recording: {:?}", from_fs, recording);

            // Run the replay
            let result = runtime.block_on(replay_task(from_fs, recording))?;

            // Pretty-print the result
            let formatted = to_string_pretty(&result).context("Failed to format result as JSON")?;

            println!("Replay Result: {}", formatted);
            info!("Task replay completed");
            
            Ok(())
        }
        Some(Commands::Generate { generate_cmd }) => {
            match generate_cmd {
                GenerateCommands::Task { path, label, description, version } => {
                    info!("Generating task template at: {:?}", path);
                    generate_task(path, label.as_ref(), description.as_ref(), version.as_ref())
                }
            }
        }
        None => {
            warn!("No command specified");
            println!("No command specified. Use --help to see available commands.");
            Ok(())
        }
    }
}

