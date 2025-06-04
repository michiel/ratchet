use anyhow::{Context, Result};
use clap::Parser;
use ratchet_lib::{
    config::RatchetConfig,
    task::Task,
    execution::ipc::{WorkerMessage, CoordinatorMessage, TaskExecutionResult, MessageEnvelope},
    registry::{TaskSource, DefaultRegistryService, RegistryService},
    services::TaskSyncService,
    recording, task, validation,
    js_executor::execute_task,
    http::HttpManager,
};
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use std::path::PathBuf;
use std::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use uuid::Uuid;

mod cli;
use cli::{Cli, Commands, GenerateCommands};

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
            debug!("No configuration file specified. Using defaults.");
            RatchetConfig::default()
        }
    };
    
    // Ensure server config is set
    if config.server.is_none() {
        debug!("No server configuration found. Adding default server config.");
        config.server = Some(ServerConfig::default());
    }

    // Apply environment variable overrides
    config.apply_env_overrides()
        .context("Failed to apply environment variable overrides")?;

    // Validate the configuration
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
    task_executor.start().await.context("Failed to start worker processes")?;
    
    // Start MCP service if enabled
    let mcp_service_handle = if let Some(mcp_config) = &config.mcp {
        if mcp_config.enabled {
            info!("Starting MCP service integration");
            let handle = tokio::spawn(async {
                // MCP service integration would go here
                // For now, just a placeholder
            });
            Some(handle)
        } else {
            None
        }
    } else {
        None
    };
    
    // Create the application
    let app = create_app(repositories, job_queue, task_executor.clone(), None, None);
    
    // Bind to address
    let addr_str = format!("{}:{}", server_config.bind_address, server_config.port);
    let addr: std::net::SocketAddr = addr_str.parse()
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
        McpServer, SimpleTransportType,
        server::{
            adapter::RatchetMcpAdapter, 
            config::{McpServerConfig, McpServerTransport, CorsConfig},
            tools::RatchetToolRegistry,
        },
        security::{McpAuthManager, McpAuth, AuditLogger},
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
            .unwrap_or_default();
        (db_config, mcp_config.clone())
    } else if let Some(server_config) = &ratchet_config.server {
        info!("Using server configuration for MCP server");
        let default_mcp = ratchet_lib::config::McpServerConfig::default();
        (server_config.database.clone(), default_mcp)
    } else {
        info!("No MCP or server configuration found, using defaults");
        let default_db = ratchet_lib::config::DatabaseConfig::default();
        let default_mcp = ratchet_lib::config::McpServerConfig::default();
        (default_db, default_mcp)
    };

    // Initialize database
    info!("Connecting to database: {}", database_config.url);
    let database = DatabaseConnection::new(database_config).await
        .context("Failed to connect to database")?;

    // Run migrations
    info!("Running database migrations");
    database.migrate().await.context("Failed to run database migrations")?;

    // Initialize repositories
    let task_repo = Arc::new(TaskRepository::new(database.clone()));
    let execution_repo = Arc::new(ExecutionRepository::new(database.clone()));

    // Initialize task executor for MCP
    info!("Initializing task executor for MCP");
    let repositories = ratchet_lib::database::repositories::RepositoryFactory::new(database.clone());
    let task_executor = Arc::new(
        ProcessTaskExecutor::new(repositories, ratchet_config).await
            .context("Failed to initialize process task executor")?
    );

    // Start worker processes for the executor
    task_executor.start().await.context("Failed to start worker processes")?;

    // Create MCP adapter
    let adapter = RatchetMcpAdapter::new(
        task_executor.clone(),
        task_repo.clone(),
        execution_repo.clone(),
    );

    // Build MCP server configuration
    let server_config = match transport_type {
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
                    allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
                    allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
                    allow_credentials: false,
                },
                timeout: std::time::Duration::from_secs(mcp_server_config.request_timeout),
            },
            security: ratchet_mcp::security::SecurityConfig::default(),
            bind_address: Some(format!("{}:{}", host, port)),
        },
    };

    // Create tool registry with the adapter as task executor
    let mut tool_registry = RatchetToolRegistry::new();
    tool_registry = tool_registry.with_task_executor(Arc::new(adapter));
    
    // Create auth manager
    let auth_manager = Arc::new(McpAuthManager::new(
        McpAuth::default() // TODO: Configure from mcp_server_config
    ));
    
    // Create audit logger
    let audit_logger = Arc::new(ratchet_mcp::security::AuditLogger::new(true));

    // Create and start MCP server
    let mut server = McpServer::new(server_config, Arc::new(tool_registry), auth_manager, audit_logger);

    info!("Starting MCP server with {} transport on {}:{}", transport, host, port);

    // Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("Received shutdown signal");
    };

    // Start server and wait for shutdown
    tokio::select! {
        result = server.start() => {
            match result {
                Ok(_) => info!("MCP server completed successfully"),
                Err(e) => {
                    error!("MCP server error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = shutdown_signal => {
            info!("Shutting down MCP server");
        }
    }

    // Stop task executor
    info!("Stopping task executor");
    task_executor.stop().await.context("Failed to stop task executor")?;
    
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
            EnvFilter::try_new(level).unwrap_or_else(|_| {
                eprintln!("Invalid log level '{}', falling back to 'info'", level);
                EnvFilter::new("info")
            })
        }
        None => {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        }
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
        Some(json_str) => {
            from_str(json_str).context("Failed to parse input JSON")
        }
        None => Ok(json!({})),
    }
}

/// Run a task from a file system path
async fn run_task(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {}", from_fs);
    
    // Load the task
    let mut task = Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;
    
    // Validate the task
    task.validate().context("Task validation failed")?;
    
    // Execute the task
    info!("Executing task: {}", task.metadata.label);
    
    // Create HTTP manager for the task execution
    let http_manager = HttpManager::new();
    
    // Execute the task
    let result = execute_task(&mut task, input.clone(), &http_manager).await
        .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;
    
    Ok(result)
}

/// Validate a task
fn validate_task(from_fs: &str) -> Result<()> {
    info!("Validating task from: {}", from_fs);
    
    // Load the task
    let mut task = Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;
    
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
async fn test_task(from_fs: &str) -> Result<()> {
    info!("Testing task from: {}", from_fs);
    
    // Load the task
    let mut task = Task::from_fs(from_fs).map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;
    
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
async fn run_single_test(task: &mut Task, test_file: &std::path::Path) -> Result<()> {
    use serde_json::Value;
    
    // Load test data
    let test_content = fs::read_to_string(test_file)
        .context("Failed to read test file")?;
    let test_data: Value = from_str(&test_content)
        .context("Failed to parse test JSON")?;
    
    // Extract input and expected output
    let input = test_data.get("input").unwrap_or(&json!({})).clone();
    let expected = test_data.get("expected_output").ok_or_else(|| {
        anyhow::anyhow!("Test file missing 'expected_output' field")
    })?;
    
    // Create HTTP manager for the task execution
    let http_manager = HttpManager::new();
    
    // Execute the task
    let actual = execute_task(task, input, &http_manager).await
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
async fn replay_task(from_fs: &str, recording: &Option<PathBuf>) -> Result<JsonValue> {
    let recording_path = recording.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Recording path is required for replay")
    })?;
    
    info!("Replaying task from: {} with recording: {:?}", from_fs, recording_path);
    
    // Load recorded input
    let input_file = recording_path.join("input.json");
    if !input_file.exists() {
        return Err(anyhow::anyhow!("Recording input file not found: {:?}", input_file));
    }
    
    let input_content = fs::read_to_string(&input_file)
        .context("Failed to read recorded input")?;
    let input: JsonValue = from_str(&input_content)
        .context("Failed to parse recorded input JSON")?;
    
    info!("Using recorded input: {}", to_string_pretty(&input)?);
    
    // Set up recording replay context
    recording::set_recording_dir(recording_path.clone())?;
    
    // Run the task with recorded input
    let result = run_task(from_fs, &input).await?;
    
    // Compare with recorded output if available
    let output_file = recording_path.join("output.json");
    if output_file.exists() {
        let recorded_output_content = fs::read_to_string(&output_file)
            .context("Failed to read recorded output")?;
        let recorded_output: JsonValue = from_str(&recorded_output_content)
            .context("Failed to parse recorded output JSON")?;
        
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
                match serde_json::from_str::<MessageEnvelope<WorkerMessage>>(&line.trim()) {
                    Ok(envelope) => {
                        let result = process_worker_message(envelope.message).await;
                        
                        let response_envelope = MessageEnvelope::new(result);
                        
                        let response_json = serde_json::to_string(&response_envelope)
                            .context("Failed to serialize response")?;
                        
                        stdout.write_all(response_json.as_bytes()).await
                            .context("Failed to write response")?;
                        stdout.write_all(b"\n").await
                            .context("Failed to write newline")?;
                        stdout.flush().await
                            .context("Failed to flush stdout")?;
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
async fn process_worker_message(msg: WorkerMessage) -> CoordinatorMessage {
    use chrono::Utc;
    use uuid::Uuid;
    
    match msg {
        WorkerMessage::ExecuteTask { job_id, task_id, task_path, input_data, execution_context, correlation_id } => {
            info!("Worker executing task: {} (Job ID: {}, Task ID: {})", task_path, job_id, task_id);
            
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
        WorkerMessage::ValidateTask { task_path, correlation_id } => {
            info!("Worker validating task: {}", task_path);
            
            let result = match Task::from_fs(&task_path) {
                Ok(mut task) => {
                    match task.validate() {
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
                    }
                }
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
        .with_description(description.as_deref().unwrap_or("A description of what this task does"))
        .with_version(version.as_deref().unwrap_or("1.0.0"));
    
    let result = ratchet_lib::generate::generate_task(config)
        .context("Failed to generate task template")?;
    
    println!("✅ Task template generated at: {:?}", path);
    println!("📝 Edit the files to customize your task:");
    println!("   - main.js: Task implementation");
    println!("   - metadata.json: Task metadata and configuration");
    println!("   - input.schema.json: Input validation schema");
    println!("   - output.schema.json: Output validation schema");
    println!("   - tests/: Test cases");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle worker mode first (before any logging setup to avoid conflicts)
    if cli.worker {
        let worker_id = cli.worker_id.unwrap_or_else(|| {
            Uuid::new_v4().to_string()
        });
        
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
                    runtime.block_on(handle_generate_task(path, label, description, version))
                }
            }
        }
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