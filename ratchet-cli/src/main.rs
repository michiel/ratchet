use anyhow::{Context, Result};
use clap::Parser;
use ratchet_config::{ConfigLoader, RatchetConfig};

#[cfg(feature = "server")]
use ratchet_lib::config::RatchetConfig as LibRatchetConfig;

#[cfg(feature = "server")]
use ratchet_lib::execution::ipc::{
    CoordinatorMessage, MessageEnvelope, TaskExecutionResult, WorkerMessage,
};

use ratchet_lib::{http::HttpManager, js_executor::execute_task, task::Task};

#[cfg(feature = "database")]
use ratchet_storage::seaorm::{connection::DatabaseConnection, repositories::RepositoryFactory};
use serde_json::{from_str, json, Value as JsonValue};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[cfg(feature = "core")]
use ratchet_core::task::Task as CoreTask;

mod cli;
use cli::{Cli, Commands, ConfigCommands, GenerateCommands};

/// Convert ratchet-storage RepositoryFactory to ratchet_lib RepositoryFactory
#[cfg(all(feature = "server", feature = "database"))]
async fn convert_to_legacy_repository_factory(
    storage_config: ratchet_storage::seaorm::config::DatabaseConfig,
) -> Result<ratchet_lib::database::repositories::RepositoryFactory> {
    // Convert storage config to legacy config
    let legacy_config = ratchet_lib::config::DatabaseConfig {
        url: storage_config.url.clone(),
        max_connections: storage_config.max_connections,
        connection_timeout: storage_config.connection_timeout,
    };

    // Create legacy database connection using the same configuration
    let legacy_db = ratchet_lib::database::DatabaseConnection::new(legacy_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create legacy database connection: {}", e))?;

    // Create legacy repository factory
    let legacy_repos = ratchet_lib::database::repositories::RepositoryFactory::new(legacy_db);

    Ok(legacy_repos)
}

/// Convert config format to legacy format for backward compatibility when server features are enabled
#[cfg(feature = "server")]
fn convert_to_legacy_config(new_config: RatchetConfig) -> Result<LibRatchetConfig> {
    use ratchet_lib::config::*;

    // Convert database config from server.database
    let database_config = if let Some(server_config) = &new_config.server {
        DatabaseConfig {
            url: server_config.database.url.clone(),
            max_connections: server_config.database.max_connections,
            connection_timeout: server_config.database.connection_timeout,
        }
    } else {
        DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            connection_timeout: std::time::Duration::from_secs(30),
        }
    };

    // Convert server config
    let server_config = ServerConfig {
        bind_address: new_config
            .server
            .as_ref()
            .map(|s| s.bind_address.clone())
            .unwrap_or_else(|| "0.0.0.0".to_string()),
        port: new_config.server.as_ref().map(|s| s.port).unwrap_or(8080),
        database: database_config,
    };

    // Convert execution config
    let execution_config = ExecutionConfig {
        max_execution_duration: new_config.execution.max_execution_duration.as_secs(),
        validate_schemas: new_config.execution.validate_schemas,
        max_concurrent_tasks: new_config.execution.max_concurrent_tasks,
        timeout_grace_period: new_config.execution.timeout_grace_period.as_secs(),
    };

    // Convert HTTP config
    let http_config = HttpConfig {
        timeout: new_config.http.timeout,
        user_agent: new_config.http.user_agent,
        verify_ssl: new_config.http.verify_ssl,
        max_redirects: new_config.http.max_redirects,
    };

    // Convert logging config - use lib's format
    let logging_config = ratchet_lib::logging::LoggingConfig::default();

    // Convert MCP config
    let mcp_config = new_config.mcp.as_ref().map(|mcp| McpServerConfig {
        enabled: mcp.enabled,
        transport: mcp.transport.clone(),
        host: mcp.host.clone(),
        port: mcp.port,
    });

    Ok(LibRatchetConfig {
        server: Some(server_config),
        execution: execution_config,
        http: http_config,
        logging: logging_config,
        mcp: mcp_config,
        cache: ratchet_lib::config::CacheConfig::default(),
        output: ratchet_lib::config::OutputConfig::default(),
        registry: None,
    })
}

/// Load configuration from file or use defaults
fn load_config(config_path: Option<&PathBuf>) -> Result<RatchetConfig> {
    let loader = ConfigLoader::new();

    match config_path {
        Some(path) => {
            if path.exists() {
                info!("Loading configuration from: {:?}", path);
                loader
                    .from_file(path)
                    .context(format!("Failed to load configuration from {:?}", path))
            } else {
                warn!("Configuration file not found: {:?}. Using defaults.", path);
                loader
                    .from_env()
                    .context("Failed to load configuration from environment")
            }
        }
        None => {
            debug!("No configuration file specified. Loading from environment or defaults.");
            loader
                .from_env()
                .context("Failed to load configuration from environment")
        }
    }
}

/// Start the Ratchet server
#[cfg(feature = "server")]
async fn serve_command(config_path: Option<&PathBuf>) -> Result<()> {
    let config = load_config(config_path)?;
    let lib_config = convert_to_legacy_config(config)?;
    serve_command_with_config(lib_config).await
}

#[cfg(not(feature = "server"))]
async fn serve_command(_config_path: Option<&PathBuf>) -> Result<()> {
    Err(anyhow::anyhow!(
        "Server functionality not available. Build with --features=server"
    ))
}

#[cfg(feature = "server")]
async fn serve_command_with_config(config: LibRatchetConfig) -> Result<()> {
    use ratchet_lib::{
        execution::{JobQueueManager, ProcessTaskExecutor},
        server::create_app,
    };
    use std::sync::Arc;
    use tokio::signal;

    info!("Starting Ratchet server");

    // Get server configuration (guaranteed to exist from load_config)
    let server_config = config.server.as_ref().unwrap();

    info!(
        "Server configuration loaded: {}:{}",
        server_config.bind_address, server_config.port
    );

    // Initialize database
    info!("Connecting to database: {}", server_config.database.url);

    // Convert lib database config to storage database config
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: server_config.database.url.clone(),
        max_connections: server_config.database.max_connections,
        connection_timeout: server_config.database.connection_timeout,
    };

    let database = DatabaseConnection::new(storage_db_config.clone())
        .await
        .context("Failed to connect to database")?;

    // Run migrations
    info!("Running database migrations");
    database
        .migrate()
        .await
        .context("Failed to run database migrations")?;

    // Initialize repositories using legacy factory for backward compatibility
    let storage_config = storage_db_config;
    let legacy_repositories = convert_to_legacy_repository_factory(storage_config.clone()).await?;
    
    // Create storage repository factory for MCP service
    let storage_repositories = RepositoryFactory::new(database.clone());

    // Initialize job queue
    let job_queue = Arc::new(JobQueueManager::with_default_config(
        legacy_repositories.clone(),
    ));

    // Initialize process task executor
    info!("Initializing process task executor");
    let task_executor = Arc::new(
        ProcessTaskExecutor::new(legacy_repositories.clone(), config.clone())
            .await
            .context("Failed to initialize process task executor")?,
    );

    // Start worker processes
    task_executor
        .start()
        .await
        .context("Failed to start worker processes")?;

    // Start MCP service if enabled
    let mcp_service_handle = if let Some(mcp_config) = &config.mcp {
        if mcp_config.enabled {
            info!("Starting MCP service integration");
            
            #[cfg(feature = "mcp-server")]
            {
                use ratchet_mcp::server::service::McpService;
                
                // Create repositories for MCP service using storage types
                let mcp_task_repo = Arc::new(storage_repositories.task_repository());
                let mcp_execution_repo = Arc::new(storage_repositories.execution_repository());
                
                // Create MCP service with legacy repository factory conversion
                let mcp_service_result = McpService::from_legacy_ratchet_config(
                    mcp_config,
                    task_executor.clone(),
                    mcp_task_repo,
                    mcp_execution_repo,
                    None, // No special log file for MCP service
                ).await;
                
                match mcp_service_result {
                    Ok(mcp_service) => {
                        let mcp_service = Arc::new(mcp_service);
                        
                        // Start MCP service in background task
                        let mcp_service_for_task = mcp_service.clone();
                        let handle = tokio::spawn(async move {
                            info!("MCP service starting...");
                            if let Err(e) = mcp_service_for_task.start().await {
                                error!("MCP service failed to start: {}", e);
                            } else {
                                info!("MCP service started successfully");
                                
                                // Keep the service running until shutdown
                                loop {
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                    if !mcp_service_for_task.is_running().await {
                                        warn!("MCP service stopped running");
                                        break;
                                    }
                                }
                            }
                        });
                        
                        Some(handle)
                    }
                    Err(e) => {
                        error!("Failed to create MCP service: {}", e);
                        None
                    }
                }
            }
            
            #[cfg(not(feature = "mcp-server"))]
            {
                warn!("MCP service requested but mcp-server feature not enabled");
                None
            }
        } else {
            info!("MCP service disabled in configuration");
            None
        }
    } else {
        None
    };

    // Create the application
    let app = create_app(
        legacy_repositories,
        job_queue,
        task_executor.clone(),
        None,
        None,
    );

    // Bind to address
    let addr_str = format!("{}:{}", server_config.bind_address, server_config.port);
    let addr: std::net::SocketAddr = addr_str
        .parse()
        .context(format!("Invalid bind address: {}", addr_str))?;
    info!("Server listening on: {}", addr);

    // Graceful shutdown signal
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
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
        
        #[cfg(feature = "mcp-server")]
        {
            // Gracefully abort the MCP service task
            handle.abort();
            
            // Wait for the task to complete with timeout
            match tokio::time::timeout(std::time::Duration::from_secs(10), handle).await {
                Ok(_) => {
                    info!("MCP service stopped gracefully");
                }
                Err(_) => {
                    warn!("MCP service shutdown timed out");
                }
            }
        }
        
        #[cfg(not(feature = "mcp-server"))]
        {
            handle.abort();
        }
    }

    // Stop worker processes
    info!("Stopping worker processes");
    task_executor
        .stop()
        .await
        .context("Failed to stop worker processes")?;

    info!("Ratchet server shutdown complete");
    Ok(())
}

/// Start the MCP (Model Context Protocol) server
#[cfg(feature = "mcp-server")]
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
        "MCP server functionality not available. Build with --features=mcp-server"
    ))
}

#[cfg(feature = "mcp-server")]
async fn mcp_serve_command_with_config(
    ratchet_config: RatchetConfig,
    transport: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    let is_stdio = transport == "stdio";
    use ratchet_lib::execution::ProcessTaskExecutor;
    use ratchet_mcp::{
        security::{McpAuth, McpAuthManager},
        server::{
            adapter::RatchetMcpAdapter,
            config::{CorsConfig, McpServerConfig, McpServerTransport},
            tools::RatchetToolRegistry,
        },
        McpServer, SimpleTransportType,
    };
    use std::sync::Arc;
    use tokio::signal;

    if !is_stdio {
        info!("Starting Ratchet MCP server");
    }

    // Parse transport type
    let transport_type = match transport.to_lowercase().as_str() {
        "stdio" => SimpleTransportType::Stdio,
        "sse" => SimpleTransportType::Sse,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid transport type: {}. Use 'stdio' or 'sse'",
                transport
            ));
        }
    };

    // Get database configuration (use server config if available, otherwise default)
    let database_config = if let Some(server_config) = &ratchet_config.server {
        if !is_stdio {
            info!("Using server database configuration for MCP");
        }
        &server_config.database
    } else {
        if !is_stdio {
            info!("No server configuration found, using default database config");
        }
        // Use default from server config
        &ratchet_config::domains::server::ServerConfig::default().database
    };

    // Convert new database config to storage database config
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: database_config.url.clone(),
        max_connections: database_config.max_connections,
        connection_timeout: database_config.connection_timeout,
    };

    // Initialize database
    if !is_stdio {
        info!("Connecting to database: {}", storage_db_config.url);
    }
    let database = DatabaseConnection::new(storage_db_config)
        .await
        .context("Failed to connect to database")?;

    // Run migrations
    if !is_stdio {
        info!("Running database migrations");
    }
    database
        .migrate()
        .await
        .context("Failed to run database migrations")?;

    // Initialize repositories using ratchet-storage
    let repositories = RepositoryFactory::new(database.clone());
    let task_repo = Arc::new(repositories.task_repository());
    let execution_repo = Arc::new(repositories.execution_repository());

    // For legacy ProcessTaskExecutor, convert to legacy repository factory
    let storage_config = database.get_config().clone();
    let legacy_repositories = convert_to_legacy_repository_factory(storage_config).await?;

    // Initialize task executor for MCP - convert to legacy config for executor
    if !is_stdio {
        info!("Initializing task executor for MCP");
    }
    let legacy_config = convert_to_legacy_config(ratchet_config.clone())?;
    let task_executor = Arc::new(
        ProcessTaskExecutor::new(legacy_repositories, legacy_config)
            .await
            .context("Failed to initialize process task executor")?,
    );

    // Start worker processes for the executor
    if !is_stdio {
        // Only log worker startup for non-stdio modes to avoid stderr noise
        task_executor
            .start()
            .await
            .context("Failed to start worker processes")?;
    } else {
        // For stdio mode, start workers silently
        task_executor
            .start()
            .await
            .context("Failed to start worker processes")?;
    }

    // Create MCP adapter
    let adapter = RatchetMcpAdapter::new(
        task_executor.clone(),
        task_repo.clone(),
        execution_repo.clone(),
    );

    // Build MCP server configuration - use MCP config from ratchet_config if available, otherwise use CLI args
    let server_config = if let Some(mcp_config) = &ratchet_config.mcp {
        // Use config from file/environment
        McpServerConfig::from_ratchet_config(mcp_config)
    } else {
        // Use command line arguments
        match transport_type {
            SimpleTransportType::Stdio => McpServerConfig {
                transport: McpServerTransport::Stdio,
                security: ratchet_mcp::security::SecurityConfig::default(),
                bind_address: None,
            },
            SimpleTransportType::Sse => McpServerConfig {
                transport: McpServerTransport::Sse {
                    port,
                    host: host.to_string(),
                    tls: false,
                    cors: CorsConfig {
                        allowed_origins: vec!["*".to_string()],
                        allowed_methods: vec![
                            "GET".to_string(),
                            "POST".to_string(),
                            "OPTIONS".to_string(),
                        ],
                        allowed_headers: vec![
                            "Content-Type".to_string(),
                            "Authorization".to_string(),
                        ],
                        allow_credentials: false,
                    },
                    timeout: std::time::Duration::from_secs(300), // Default 5 minutes
                },
                security: ratchet_mcp::security::SecurityConfig::default(),
                bind_address: Some(format!("{}:{}", host, port)),
            },
        }
    };

    // Create tool registry with the adapter as task executor
    let mut tool_registry = RatchetToolRegistry::new();
    tool_registry = tool_registry.with_task_executor(Arc::new(adapter));

    // Create auth manager
    let auth_manager = Arc::new(McpAuthManager::new(
        McpAuth::default(), // TODO: Configure from mcp_server_config
    ));

    // Create audit logger
    let audit_logger = Arc::new(ratchet_mcp::security::AuditLogger::new(true));

    // Create and start MCP server
    let server = McpServer::new(
        server_config,
        Arc::new(tool_registry),
        auth_manager,
        audit_logger,
    );

    if !is_stdio {
        info!(
            "Starting MCP server with {} transport on {}:{}",
            transport, host, port
        );
    }

    // Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        if !is_stdio {
            info!("Received shutdown signal");
        }
    };

    // Start server and wait for shutdown
    tokio::select! {
        result = server.start() => {
            match result {
                Ok(_) => {
                    if !is_stdio {
                        info!("MCP server completed successfully");
                    }
                }
                Err(e) => {
                    error!("MCP server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_signal => {
            if !is_stdio {
                info!("Shutting down MCP server");
            }
        }
    }

    // Stop task executor
    if !is_stdio {
        info!("Stopping task executor");
    }
    task_executor
        .stop()
        .await
        .context("Failed to stop task executor")?;

    if !is_stdio {
        info!("Ratchet MCP server shutdown complete");
    }
    Ok(())
}

/// Initialize logging from configuration with fallback to simple tracing
#[cfg(feature = "server")]
fn init_logging_with_config(
    config: &LibRatchetConfig,
    log_level: Option<&String>,
    record_dir: Option<&PathBuf>,
) -> Result<()> {
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
        logging_config
            .sinks
            .push(ratchet_lib::logging::config::SinkConfig::File {
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

        info!(
            "Recording session to: {:?}",
            record_path.join(format!("ratchet_session_{}", timestamp))
        );
    }

    // Try to initialize structured logging
    match ratchet_lib::logging::init_logging_from_config(&logging_config) {
        Ok(()) => {
            debug!("Structured logging initialized");
        }
        Err(e) => {
            // Fall back to simple tracing if structured logging fails
            eprintln!(
                "Failed to initialize structured logging: {}, falling back to simple tracing",
                e
            );
            init_simple_tracing(log_level)?;
        }
    }

    Ok(())
}

/// Initialize simple logging when server features are disabled
#[cfg(not(feature = "server"))]
fn init_logging_with_config(
    _config: &RatchetConfig,
    log_level: Option<&String>,
    _record_dir: Option<&PathBuf>,
) -> Result<()> {
    init_simple_tracing(log_level)
}

/// Initialize simple tracing with environment variable override support (fallback)
fn init_simple_tracing(log_level: Option<&String>) -> Result<()> {
    let env_filter = match log_level {
        Some(level) => EnvFilter::try_new(level).unwrap_or_else(|_| {
            eprintln!("Invalid log level '{}', falling back to 'info'", level);
            EnvFilter::new("info")
        }),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    debug!("Simple tracing initialized");
    Ok(())
}

/// Initialize minimal stderr-only logging for MCP stdio mode
fn init_mcp_stdio_logging(log_level: Option<&String>) -> Result<()> {
    // For MCP stdio mode, use error-level logging to stderr only
    // This ensures stdout is reserved exclusively for JSON-RPC responses
    let default_level = "error".to_string(); // Only show errors by default
    let level_str = log_level.unwrap_or(&default_level);

    let env_filter = EnvFilter::try_new(level_str).unwrap_or_else(|_| {
        eprintln!("Invalid log level '{}', falling back to 'error'", level_str);
        EnvFilter::new("error")
    });

    // Force output to stderr only for MCP stdio mode
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    Ok(())
}

/// Initialize tracing for worker processes (output to stderr to avoid IPC conflicts)
fn init_worker_tracing(log_level: Option<&String>) -> Result<()> {
    let env_filter = match log_level {
        Some(level) => EnvFilter::try_new(level).unwrap_or_else(|_| {
            eprintln!("Invalid log level '{}', falling back to 'info'", level);
            EnvFilter::new("info")
        }),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    // Worker processes output to stderr to avoid conflicts with IPC on stdout
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    debug!("Worker tracing initialized");
    Ok(())
}

/// Parse input JSON or use empty object if none provided
fn parse_input_json(input_json: Option<&String>) -> Result<JsonValue> {
    match input_json {
        Some(json_str) => from_str(json_str).context("Failed to parse input JSON"),
        None => Ok(json!({})),
    }
}

/// Load task from filesystem and convert to Core Task
#[cfg(all(feature = "runtime", feature = "core", feature = "javascript"))]
fn load_task_as_core_task(from_fs: &str) -> Result<CoreTask> {
    use ratchet_core::task::TaskBuilder;
    use ratchet_lib::task::Task as LibTask;

    // Load using ratchet_lib's filesystem loader
    let mut lib_task =
        LibTask::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Ensure JavaScript content is loaded
    lib_task
        .ensure_content_loaded()
        .map_err(|e| anyhow::anyhow!("Failed to load task content: {}", e))?;

    // Get the JavaScript content
    let js_content = lib_task
        .get_js_content()
        .map_err(|e| anyhow::anyhow!("Failed to get JS content: {}", e))?;

    // Convert to Core Task
    let core_task = TaskBuilder::new(&lib_task.metadata.label, &lib_task.metadata.version)
        .input_schema(lib_task.input_schema.clone())
        .output_schema(lib_task.output_schema.clone())
        .javascript_source(js_content.as_str())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build Core Task: {}", e))?;

    Ok(core_task)
}

/// Run a task from a file system path using runtime executor
#[cfg(all(feature = "runtime", feature = "core"))]
async fn run_task_runtime(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {} (using runtime executor)", from_fs);

    // Load the task as Core Task (with conversion from ratchet_lib if available)
    #[cfg(feature = "javascript")]
    let task = load_task_as_core_task(from_fs)?;

    #[cfg(not(feature = "javascript"))]
    {
        return Err(anyhow::anyhow!("Task loading requires javascript feature. Please build with --features=javascript or use legacy executor."));
    }

    // Execute the task
    info!(
        "Executing task: {} (using runtime executor)",
        task.metadata.name
    );

    // Create in-memory executor for CLI usage (simpler than full worker process management)
    let executor = InMemoryTaskExecutor::new();

    // Execute the task
    let result = executor
        .execute_task(&task, input.clone(), None)
        .await
        .map_err(|e| anyhow::anyhow!("Runtime task execution failed: {}", e))?;

    Ok(result)
}

/// Run a task from a file system path using legacy executor
#[cfg(feature = "javascript")]
async fn run_task(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {} (using legacy executor)", from_fs);

    // Load the task
    let mut task =
        Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Validate the task
    task.validate().context("Task validation failed")?;

    // Execute the task
    info!("Executing task: {}", task.metadata.label);

    // Create HTTP manager for the task execution
    let http_manager = HttpManager::new();

    // Execute the task
    let result = execute_task(&mut task, input.clone(), &http_manager)
        .await
        .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

    Ok(result)
}

/// Validate a task
#[cfg(feature = "javascript")]
fn validate_task(from_fs: &str) -> Result<()> {
    info!("Validating task from: {}", from_fs);

    // Load the task
    let mut task =
        Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Validate the task
    match task.validate() {
        Ok(_) => {
            println!("✅ Task validation passed");
            info!("Task '{}' is valid", task.metadata.label);
            Ok(())
        }
        Err(e) => {
            println!("❌ Task validation failed: {}", e);
            error!("Task validation failed: {}", e);
            Err(e.into())
        }
    }
}

/// Test a task by running its test cases
#[cfg(feature = "javascript")]
async fn test_task(from_fs: &str) -> Result<()> {
    info!("Testing task from: {}", from_fs);

    // Load the task
    let mut task =
        Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Validate the task first
    task.validate().context("Task validation failed")?;

    // Get test directory
    let task_path = std::path::Path::new(from_fs);
    let test_dir = task_path.join("tests");

    if !test_dir.exists() {
        println!("No tests directory found at: {}", test_dir.display());
        info!("No tests to run for task '{}'", task.metadata.label);
        return Ok(());
    }

    // Find test files
    let test_files: Vec<_> = fs::read_dir(&test_dir)
        .context("Failed to read tests directory")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "json" && path.file_name()?.to_str()?.starts_with("test-") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if test_files.is_empty() {
        println!("No test files found in: {}", test_dir.display());
        info!("No test files found for task '{}'", task.metadata.label);
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for test_file in test_files {
        let test_name = test_file.file_stem().unwrap().to_str().unwrap();
        print!("Running test '{}' ... ", test_name);

        match run_single_test(&mut task.clone(), &test_file).await {
            Ok(_) => {
                println!("✅ PASSED");
                passed += 1;
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
                failed += 1;
            }
        }
    }

    println!("\nTest Results:");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Total:  {}", passed + failed);

    if failed > 0 {
        Err(anyhow::anyhow!("{} test(s) failed", failed))
    } else {
        Ok(())
    }
}

/// Run a single test case
#[cfg(feature = "javascript")]
async fn run_single_test(task: &mut Task, test_file: &std::path::Path) -> Result<()> {
    use serde_json::Value;

    // Load test data
    let test_content = fs::read_to_string(test_file).context("Failed to read test file")?;
    let test_data: Value = from_str(&test_content).context("Failed to parse test JSON")?;

    // Extract input and expected output
    let input = test_data.get("input").unwrap_or(&json!({})).clone();
    let expected = test_data
        .get("expected_output")
        .ok_or_else(|| anyhow::anyhow!("Test file missing 'expected_output' field"))?;

    // Create HTTP manager for the task execution
    let http_manager = HttpManager::new();

    // Execute the task
    let actual = execute_task(task, input, &http_manager)
        .await
        .map_err(|e| anyhow::anyhow!("Task execution failed during test: {}", e))?;

    // Compare results
    if &actual == expected {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Output mismatch.\nExpected: {}\nActual: {}",
            to_string_pretty(expected)?,
            to_string_pretty(&actual)?
        ))
    }
}

/// Replay a task execution using recorded session data
#[cfg(feature = "javascript")]
async fn replay_task(from_fs: &str, recording: &Option<PathBuf>) -> Result<JsonValue> {
    let recording_path = recording
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Recording path is required for replay"))?;

    info!(
        "Replaying task from: {} with recording: {:?}",
        from_fs, recording_path
    );

    // Load recorded input
    let input_file = recording_path.join("input.json");
    if !input_file.exists() {
        return Err(anyhow::anyhow!(
            "Recording input file not found: {:?}",
            input_file
        ));
    }

    let input_content = fs::read_to_string(&input_file).context("Failed to read recorded input")?;
    let input: JsonValue =
        from_str(&input_content).context("Failed to parse recorded input JSON")?;

    info!("Using recorded input: {}", to_string_pretty(&input)?);

    // Set up recording replay context
    recording::set_recording_dir(recording_path.clone())?;

    // Run the task with recorded input
    let result = run_task(from_fs, &input).await?;

    // Compare with recorded output if available
    let output_file = recording_path.join("output.json");
    if output_file.exists() {
        let recorded_output_content =
            fs::read_to_string(&output_file).context("Failed to read recorded output")?;
        let recorded_output: JsonValue =
            from_str(&recorded_output_content).context("Failed to parse recorded output JSON")?;

        if result == recorded_output {
            info!("✅ Replay output matches recorded output");
        } else {
            warn!("⚠️  Replay output differs from recorded output");
            info!("Recorded: {}", to_string_pretty(&recorded_output)?);
            info!("Replayed: {}", to_string_pretty(&result)?);
        }
    }

    Ok(result)
}

/// Run as worker process
#[cfg(feature = "server")]
async fn run_worker_process(worker_id: String) -> Result<()> {
    info!("Starting worker process with ID: {}", worker_id);

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();

    loop {
        let mut line = String::new();
        match stdin.read_line(&mut line).await {
            Ok(0) => {
                // EOF reached, parent process has closed stdin
                info!("Worker {} received EOF, shutting down", worker_id);
                break;
            }
            Ok(_) => {
                // Parse the message envelope
                match serde_json::from_str::<MessageEnvelope<WorkerMessage>>(line.trim()) {
                    Ok(envelope) => {
                        let result = process_worker_message(envelope.message).await;

                        let response_envelope = MessageEnvelope::new(result);

                        let response_json = serde_json::to_string(&response_envelope)
                            .context("Failed to serialize response")?;

                        stdout
                            .write_all(response_json.as_bytes())
                            .await
                            .context("Failed to write response")?;
                        stdout
                            .write_all(b"\n")
                            .await
                            .context("Failed to write newline")?;
                        stdout.flush().await.context("Failed to flush stdout")?;
                    }
                    Err(e) => {
                        error!("Worker {} failed to parse worker message: {}", worker_id, e);
                        error!("Invalid message: {}", line.trim());
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

/// Process a worker message
#[cfg(feature = "server")]
async fn process_worker_message(msg: WorkerMessage) -> CoordinatorMessage {
    use chrono::Utc;

    match msg {
        WorkerMessage::ExecuteTask {
            job_id,
            task_id,
            task_path,
            input_data,
            execution_context: _,
            correlation_id,
        } => {
            info!(
                "Worker executing task: {} (Job ID: {}, Task ID: {})",
                task_path, job_id, task_id
            );

            let started_at = Utc::now();

            // Load and execute the task
            let result = match Task::from_fs(&task_path) {
                Ok(mut task) => {
                    // Create HTTP manager for the task execution
                    let http_manager = HttpManager::new();

                    match execute_task(&mut task, input_data, &http_manager).await {
                        Ok(output) => {
                            let completed_at = Utc::now();
                            let duration_ms = (completed_at - started_at).num_milliseconds() as i32;

                            info!("Worker completed task: {} (Job ID: {})", task_path, job_id);
                            TaskExecutionResult {
                                success: true,
                                output: Some(output),
                                error_message: None,
                                error_details: None,
                                started_at,
                                completed_at,
                                duration_ms,
                            }
                        }
                        Err(e) => {
                            let completed_at = Utc::now();
                            let duration_ms = (completed_at - started_at).num_milliseconds() as i32;

                            error!("Worker task execution failed: {}", e);
                            TaskExecutionResult {
                                success: false,
                                output: None,
                                error_message: Some(e.to_string()),
                                error_details: None,
                                started_at,
                                completed_at,
                                duration_ms,
                            }
                        }
                    }
                }
                Err(e) => {
                    let completed_at = Utc::now();
                    let duration_ms = (completed_at - started_at).num_milliseconds() as i32;

                    error!("Worker failed to load task: {}", e);
                    TaskExecutionResult {
                        success: false,
                        output: None,
                        error_message: Some(format!("Failed to load task: {}", e)),
                        error_details: None,
                        started_at,
                        completed_at,
                        duration_ms,
                    }
                }
            };

            CoordinatorMessage::TaskResult {
                job_id,
                correlation_id,
                result,
            }
        }
        WorkerMessage::ValidateTask {
            task_path,
            correlation_id,
        } => {
            info!("Worker validating task: {}", task_path);

            let result = match Task::from_fs(&task_path) {
                Ok(mut task) => match task.validate() {
                    Ok(_) => {
                        info!("Worker task validation passed: {}", task_path);
                        ratchet_lib::execution::ipc::TaskValidationResult {
                            valid: true,
                            error_message: None,
                            error_details: None,
                        }
                    }
                    Err(e) => {
                        error!("Worker task validation failed: {}", e);
                        ratchet_lib::execution::ipc::TaskValidationResult {
                            valid: false,
                            error_message: Some(e.to_string()),
                            error_details: None,
                        }
                    }
                },
                Err(e) => {
                    error!("Worker failed to load task for validation: {}", e);
                    ratchet_lib::execution::ipc::TaskValidationResult {
                        valid: false,
                        error_message: Some(format!("Failed to load task: {}", e)),
                        error_details: None,
                    }
                }
            };

            CoordinatorMessage::ValidationResult {
                correlation_id,
                result,
            }
        }
        WorkerMessage::Ping { correlation_id } => {
            debug!("Worker received ping");
            CoordinatorMessage::Pong {
                correlation_id,
                worker_id: "worker".to_string(), // TODO: Use actual worker ID
                status: ratchet_lib::execution::ipc::WorkerStatus {
                    worker_id: "worker".to_string(),
                    pid: std::process::id(),
                    started_at: Utc::now(),
                    last_activity: Utc::now(),
                    tasks_executed: 0,
                    tasks_failed: 0,
                    memory_usage_mb: None,
                    cpu_usage_percent: None,
                },
            }
        }
        WorkerMessage::Shutdown => {
            info!("Worker received shutdown signal");
            // This message doesn't require a response according to the protocol
            // We'll exit the worker loop after sending this
            CoordinatorMessage::Ready {
                worker_id: "worker".to_string(),
            }
        }
    }
}

/// Handle task generation
#[cfg(feature = "javascript")]
async fn handle_generate_task(
    path: &PathBuf,
    label: &Option<String>,
    description: &Option<String>,
    version: &Option<String>,
) -> Result<()> {
    info!("Generating task template at: {:?}", path);

    // Create directory if it doesn't exist
    if path.exists() {
        return Err(anyhow::anyhow!("Directory already exists: {:?}", path));
    }

    fs::create_dir_all(path).context("Failed to create task directory")?;

    // Generate task files
    let config = ratchet_lib::generate::TaskGenerationConfig::new(path.clone())
        .with_label(label.as_deref().unwrap_or("My Task"))
        .with_description(
            description
                .as_deref()
                .unwrap_or("A description of what this task does"),
        )
        .with_version(version.as_deref().unwrap_or("1.0.0"));

    let _result =
        ratchet_lib::generate::generate_task(config).context("Failed to generate task template")?;

    println!("✅ Task template generated at: {:?}", path);
    println!("📝 Edit the files to customize your task:");
    println!("   - main.js: Task implementation");
    println!("   - metadata.json: Task metadata and configuration");
    println!("   - input.schema.json: Input validation schema");
    println!("   - output.schema.json: Output validation schema");
    println!("   - tests/: Test cases");

    Ok(())
}

/// Handle configuration validation
fn handle_config_validate(config_file: &PathBuf) -> Result<()> {
    info!("Validating configuration file: {:?}", config_file);

    // Check if file exists
    if !config_file.exists() {
        return Err(anyhow::anyhow!(
            "Configuration file not found: {:?}",
            config_file
        ));
    }

    // Try to load and validate configuration
    match load_config(Some(config_file)) {
        Ok(_config) => {
            println!("✅ Configuration file is valid");
            info!("Configuration validation passed");
            Ok(())
        }
        Err(e) => {
            println!("❌ Configuration validation failed: {}", e);
            error!("Configuration validation failed: {}", e);
            Err(e)
        }
    }
}

/// Handle configuration generation
fn handle_config_generate(config_type: &str, output: &PathBuf, force: bool) -> Result<()> {
    info!("Generating {} configuration at: {:?}", config_type, output);

    // Check if file exists and force is not set
    if output.exists() && !force {
        return Err(anyhow::anyhow!(
            "Output file already exists: {:?}. Use --force to overwrite.",
            output
        ));
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    // Get the appropriate configuration content
    let config_content = match config_type.to_lowercase().as_str() {
        "dev" | "development" => {
            info!("Generating development configuration");
            include_str!("../../sample/configs/example-mcp-dev.yaml")
        }
        "prod" | "production" => {
            info!("Generating production configuration");
            include_str!("../../sample/configs/example-mcp-production.yaml")
        }
        "enterprise" => {
            info!("Generating enterprise configuration");
            include_str!("../../sample/configs/example-mcp-enterprise.yaml")
        }
        "minimal" => {
            info!("Generating minimal configuration");
            include_str!("../../sample/configs/example-mcp-minimal.yaml")
        }
        "claude" => {
            info!("Generating Claude integration configuration");
            include_str!("../../sample/configs/example-mcp-claude-integration.yaml")
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown configuration type: {}. Valid types: dev, production, enterprise, minimal, claude",
                config_type
            ));
        }
    };

    // Write configuration to file
    fs::write(output, config_content).context("Failed to write configuration file")?;

    println!(
        "✅ {} configuration generated at: {:?}",
        config_type, output
    );
    println!("📝 Edit the file to customize settings for your environment");
    println!(
        "🔧 Validate with: ratchet config validate --config-file {:?}",
        output
    );

    Ok(())
}

/// Handle configuration display
fn handle_config_show(config_file: Option<&PathBuf>, mcp_only: bool, format: &str) -> Result<()> {
    info!(
        "Showing configuration (MCP only: {}, format: {})",
        mcp_only, format
    );

    // Load configuration
    let config = load_config(config_file)?;

    // Prepare output based on what to show
    let output_value = if mcp_only {
        // Show only MCP configuration
        if let Some(mcp_config) = &config.mcp {
            serde_json::to_value(mcp_config).context("Failed to serialize MCP config")?
        } else {
            serde_json::json!({
                "mcp": null,
                "note": "No MCP configuration found"
            })
        }
    } else {
        // Show full configuration
        serde_json::to_value(&config).context("Failed to serialize config")?
    };

    // Format and print output
    match format.to_lowercase().as_str() {
        "yaml" | "yml" => {
            let yaml_output =
                serde_yaml::to_string(&output_value).context("Failed to serialize to YAML")?;
            println!("{}", yaml_output);
        }
        "json" => {
            let json_output = serde_json::to_string_pretty(&output_value)
                .context("Failed to serialize to JSON")?;
            println!("{}", json_output);
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown output format: {}. Valid formats: yaml, json",
                format
            ));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle worker mode first (before any logging setup to avoid conflicts)
    if cli.worker {
        #[cfg(feature = "server")]
        {
            let worker_id = cli.worker_id.unwrap_or_else(|| Uuid::new_v4().to_string());

            // Initialize tracing for worker (stderr only to avoid IPC conflicts)
            init_worker_tracing(cli.log_level.as_ref())?;

            // Run worker process
            return run_worker_process(worker_id).await;
        }
        #[cfg(not(feature = "server"))]
        {
            return Err(anyhow::anyhow!(
                "Worker functionality not available. Build with --features=server"
            ));
        }
    }

    // Check if this is MCP stdio mode - if so, use minimal stderr-only logging
    let is_mcp_stdio =
        matches!(&cli.command, Some(Commands::McpServe { transport, .. }) if transport == "stdio");

    // Load configuration first
    let config = load_config(cli.config.as_ref())?;

    // Initialize logging appropriately for the command
    if is_mcp_stdio {
        // For MCP stdio mode, use minimal stderr-only logging to avoid interfering with JSON-RPC on stdout
        init_mcp_stdio_logging(cli.log_level.as_ref())?;
    } else {
        // Initialize logging from config
        let legacy_config = convert_to_legacy_config(config.clone())?;
        init_logging_with_config(
            &legacy_config,
            cli.log_level.as_ref(),
            cli.command.as_ref().and_then(|cmd| match cmd {
                Commands::RunOnce { record, .. } => record.as_ref(),
                _ => None,
            }),
        )?;
    }

    if !is_mcp_stdio {
        info!("Ratchet CLI starting");
    }

    match &cli.command {
        Some(Commands::RunOnce {
            from_fs,
            input_json,
            record,
        }) => {
            // Try runtime executor first if available, then fall back to legacy
            #[cfg(all(feature = "runtime", feature = "core"))]
            {
                info!(
                    "Running task from file system path: {} (using runtime executor)",
                    from_fs
                );

                // Parse input JSON
                let input = parse_input_json(input_json.as_ref())?;

                if input_json.is_some() {
                    info!("Using provided input: {}", input_json.as_ref().unwrap());
                }

                // Run the task with runtime executor
                let result = run_task_runtime(from_fs, &input).await?;

                // Pretty-print the result
                let formatted =
                    to_string_pretty(&result).context("Failed to format result as JSON")?;

                println!("{}", formatted);
                return Ok(());
            }

            #[cfg(all(
                feature = "javascript",
                not(all(feature = "runtime", feature = "core"))
            ))]
            {
                info!(
                    "Running task from file system path: {} (using legacy executor)",
                    from_fs
                );

                // Parse input JSON
                let input = parse_input_json(input_json.as_ref())?;

                if input_json.is_some() {
                    info!("Using provided input: {}", input_json.as_ref().unwrap());
                }

                // Run the task with legacy executor
                let result = run_task(from_fs, &input).await?;

                // Pretty-print the result
                let formatted =
                    to_string_pretty(&result).context("Failed to format result as JSON")?;

                println!("Result: {}", formatted);
                info!("Task execution completed");

                // Finalize recording if it was enabled
                if record.is_some() {
                    #[cfg(feature = "server")]
                    {
                        if let Err(e) = ratchet_lib::recording::finalize_recording() {
                            warn!("Failed to finalize recording: {}", e);
                        } else if let Some(dir) = ratchet_lib::recording::get_recording_dir() {
                            println!("Recording saved to: {:?}", dir);
                        }
                    }
                    #[cfg(not(feature = "server"))]
                    {
                        warn!("Recording functionality not available without server features");
                    }
                }

                Ok(())
            }
            #[cfg(not(any(feature = "javascript", all(feature = "runtime", feature = "core"))))]
            {
                Err(anyhow::anyhow!("Task execution not available. Build with --features=javascript or --features=runtime,core"))
            }
        }
        Some(Commands::Serve {
            config: config_override,
        }) => {
            #[cfg(feature = "server")]
            {
                info!("Starting Ratchet server");
                // Use config override if provided, otherwise use loaded config
                let server_config = if config_override.is_some() {
                    load_config(config_override.as_ref())?
                } else {
                    config
                };
                let lib_config = convert_to_legacy_config(server_config)?;
                serve_command_with_config(lib_config).await
            }
            #[cfg(not(feature = "server"))]
            {
                Err(anyhow::anyhow!(
                    "Server functionality not available. Build with --features=server"
                ))
            }
        }
        Some(Commands::McpServe {
            config: config_override,
            transport,
            host,
            port,
        }) => {
            #[cfg(feature = "mcp-server")]
            {
                if !is_mcp_stdio {
                    info!("Starting MCP server");
                }
                // Use config override if provided, otherwise use loaded config
                let mcp_config = if config_override.is_some() {
                    load_config(config_override.as_ref())?
                } else {
                    config
                };
                mcp_serve_command_with_config(mcp_config, transport, host, *port).await
            }
            #[cfg(not(feature = "mcp-server"))]
            {
                Err(anyhow::anyhow!(
                    "MCP server functionality not available. Build with --features=mcp-server"
                ))
            }
        }
        Some(Commands::Validate { from_fs }) => {
            #[cfg(feature = "javascript")]
            {
                validate_task(from_fs)
            }
            #[cfg(not(feature = "javascript"))]
            {
                Err(anyhow::anyhow!(
                    "Task validation not available. Build with --features=javascript"
                ))
            }
        }
        Some(Commands::Test { from_fs }) => {
            #[cfg(feature = "javascript")]
            {
                test_task(from_fs).await
            }
            #[cfg(not(feature = "javascript"))]
            {
                Err(anyhow::anyhow!(
                    "Task testing not available. Build with --features=javascript"
                ))
            }
        }
        Some(Commands::Replay { from_fs, recording }) => {
            #[cfg(feature = "javascript")]
            {
                info!(
                    "Replaying task from file system path: {} with recording: {:?}",
                    from_fs, recording
                );

                // Run the replay
                let result = replay_task(from_fs, recording).await?;

                // Pretty-print the result
                let formatted =
                    to_string_pretty(&result).context("Failed to format result as JSON")?;

                println!("Replay Result: {}", formatted);
                info!("Task replay completed");

                Ok(())
            }
            #[cfg(not(feature = "javascript"))]
            {
                Err(anyhow::anyhow!(
                    "Task replay not available. Build with --features=javascript"
                ))
            }
        }
        Some(Commands::Generate { generate_cmd }) => match generate_cmd {
            GenerateCommands::Task {
                path,
                label,
                description,
                version,
            } => {
                #[cfg(feature = "javascript")]
                {
                    handle_generate_task(path, label, description, version).await
                }
                #[cfg(not(feature = "javascript"))]
                {
                    Err(anyhow::anyhow!(
                        "Task generation not available. Build with --features=javascript"
                    ))
                }
            }
        },
        Some(Commands::Config { config_cmd }) => match config_cmd {
            ConfigCommands::Validate { config_file } => handle_config_validate(config_file),
            ConfigCommands::Generate {
                config_type,
                output,
                force,
            } => handle_config_generate(config_type, output, *force),
            ConfigCommands::Show {
                config_file,
                mcp_only,
                format,
            } => handle_config_show(config_file.as_ref(), *mcp_only, format),
        },
        None => {
            // If no subcommand is provided, print help
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            cmd.print_help().context("Failed to print help")?;
            println!();
            Ok(())
        }
    }
}
