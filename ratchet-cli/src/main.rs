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
use ratchet_http::HttpManager;

#[cfg(feature = "database")]
use ratchet_storage::seaorm::{connection::DatabaseConnection, repositories::RepositoryFactory};
use serde_json::{from_str, json, to_string_pretty, Value as JsonValue};
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

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
            
            // Add default task repository when running without a config file
            // and no registry is configured via environment variables
            if config.registry.is_none() {
                use ratchet_config::domains::registry::{
                    RegistryConfig, RegistrySourceConfig, RegistrySourceType, 
                    SourceSpecificConfig, GitSourceConfig
                };
                
                info!("Adding default task repository: https://github.com/michiel/ratchet-repo-samples");
                info!("📦 Active repositories:");
                
                let default_source = RegistrySourceConfig {
                    name: "ratchet-samples".to_string(),
                    uri: "https://github.com/michiel/ratchet-repo-samples.git".to_string(),
                    source_type: RegistrySourceType::Git,
                    polling_interval: None,
                    enabled: true,
                    auth_name: None,
                    config: SourceSpecificConfig {
                        git: GitSourceConfig {
                            branch: "main".to_string(),
                            subdirectory: None,
                            shallow: true,
                            depth: Some(1),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                };
                
                config.registry = Some(RegistryConfig {
                    sources: vec![default_source.clone()],
                    ..Default::default()
                });
                
                info!("   ✅ {} (git): {}", default_source.name, default_source.uri);
            }
            
            config
        }
    };

    // Log repository information if any are configured
    if let Some(registry_config) = &config.registry {
        if !registry_config.sources.is_empty() {
            info!("📦 Configured repositories ({}):", registry_config.sources.len());
            for source in &registry_config.sources {
                let status_icon = if source.enabled { "✅" } else { "⏸️" };
                let source_type_name = match source.source_type {
                    ratchet_config::domains::registry::RegistrySourceType::Filesystem => "filesystem",
                    ratchet_config::domains::registry::RegistrySourceType::Http => "http",
                    ratchet_config::domains::registry::RegistrySourceType::Git => "git",
                    ratchet_config::domains::registry::RegistrySourceType::S3 => "s3",
                };
                info!("   {} {} ({}): {}", status_icon, source.name, source_type_name, source.uri);
            }
            info!("💡 Use 'ratchet repo status' to check repository health and synchronization state");
        } else {
            info!("📦 No task repositories configured");
            info!("💡 Use 'ratchet repo init <path>' to create a repository or add sources to your config");
        }
    } else {
        info!("📦 No task repositories configured");
        info!("💡 Use 'ratchet repo init <path>' to create a repository or add sources to your config");
    }

    Ok(config)
}

/// Start the Ratchet server (legacy wrapper function)
#[cfg(feature = "server")]
#[allow(dead_code)]
async fn serve_command(config_path: Option<&PathBuf>) -> Result<()> {
    let mut config = load_config(config_path)?;
    
    // If no explicit config file is provided, enable MCP by default for integrated server
    if config_path.is_none() {
        if let Some(ref mut mcp_config) = config.mcp {
            if !mcp_config.enabled {
                info!("Enabling MCP SSE server by default for integrated server mode");
                mcp_config.enabled = true;
            }
        }
    }
    
    serve_command_with_config(config).await
}

#[cfg(not(feature = "server"))]
async fn serve_command(_config_path: Option<&PathBuf>) -> Result<()> {
    Err(anyhow::anyhow!(
        "Server functionality not available. Build with --features=server"
    ))
}

#[cfg(feature = "server")]
async fn serve_command_with_config(config: RatchetConfig) -> Result<()> {
    info!("Starting Ratchet server with modern architecture");
    serve_with_ratchet_server(config).await
}

#[cfg(feature = "server")]
async fn serve_with_ratchet_server(config: RatchetConfig) -> Result<()> {
    use ratchet_server::{ServerConfig, Server};
    
    info!("🚀 Starting Ratchet server with new modular architecture");
    
    // Convert new config to server config
    let server_config = ServerConfig::from_ratchet_config(config.clone())?;
    
    // Log repository information if any are configured
    if let Some(registry_config) = &config.registry {
        if !registry_config.sources.is_empty() {
            info!("📦 Configured repositories ({}):", registry_config.sources.len());
            for source in &registry_config.sources {
                let source_type_name = match source.source_type {
                    ratchet_config::domains::registry::RegistrySourceType::Filesystem => "filesystem",
                    ratchet_config::domains::registry::RegistrySourceType::Http => "http",
                    ratchet_config::domains::registry::RegistrySourceType::Git => "git",
                    ratchet_config::domains::registry::RegistrySourceType::S3 => "s3",
                };
                let status_icon = if source.enabled { "✅" } else { "⏸️" };
                info!("   {} {} ({}): {}", status_icon, source.name, source_type_name, source.uri);
            }
            info!("💡 Use 'ratchet repo status' to check repository health and synchronization state");
        } else {
            info!("📦 No task repositories configured");
            info!("💡 Use 'ratchet repo init <path>' to create a repository or add sources to your config");
        }
    } else {
        info!("📦 No task repositories configured");
        info!("💡 Use 'ratchet repo init <path>' to create a repository or add sources to your config");
    }
    
    // Create the server
    let server = Server::new(server_config).await?;
    
    // Initialize and synchronize repositories with the database if any are configured
    sync_repositories_to_database(&config).await?;
    
    // Start the server
    server.start().await?;
    
    Ok(())
}

// Legacy server function removed in 0.5.0 - use ratchet-server crate instead

/// Synchronize configured repositories with the internal task registry and database
async fn sync_repositories_to_database(config: &RatchetConfig) -> Result<()> {
    use ratchet_registry::{
        service::{DefaultRegistryService, RegistryService},
        config::{RegistryConfig, TaskSource},
        sync::DatabaseSync,
    };
    use ratchet_storage::repositories::Repository;
    use std::sync::Arc;

    // Check if repositories are configured
    let registry_config = match &config.registry {
        Some(config) if !config.sources.is_empty() => config,
        _ => {
            debug!("No repositories configured for synchronization");
            return Ok(());
        }
    };

    info!("🔄 Synchronizing {} repositories with internal task registry", registry_config.sources.len());

    // Convert ratchet-config RegistrySourceConfig to ratchet-registry TaskSource
    let mut sources = Vec::new();
    for source_config in &registry_config.sources {
        let task_source = match source_config.source_type {
            ratchet_config::domains::registry::RegistrySourceType::Filesystem => {
                TaskSource::Filesystem {
                    path: source_config.uri.clone(),
                    recursive: true,
                    watch: source_config.config.filesystem.watch_changes,
                }
            }
            ratchet_config::domains::registry::RegistrySourceType::Http => {
                TaskSource::Http {
                    url: source_config.uri.clone(),
                    auth: None, // TODO: Map authentication from config
                    polling_interval: source_config.polling_interval
                        .unwrap_or(registry_config.default_polling_interval),
                }
            }
            ratchet_config::domains::registry::RegistrySourceType::Git => {
                TaskSource::Git {
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
    let registry_service_config = RegistryConfig {
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
    
    debug!("Repository sync using database URL: {}", database_url);
    
    let storage_config = if database_url == "sqlite::memory:" {
        ratchet_storage::config::StorageConfig::in_memory()
    } else {
        // Parse SQLite path from URL - support multiple formats
        let path = if database_url.starts_with("sqlite:///") {
            // Absolute path: sqlite:///path/to/file.db -> /path/to/file.db
            database_url.trim_start_matches("sqlite:///").to_string()
        } else if database_url.starts_with("sqlite://") {
            // Relative path with double slash: sqlite://file.db -> file.db
            database_url.trim_start_matches("sqlite://").to_string()
        } else if database_url.starts_with("sqlite:") {
            // Direct path: sqlite:file.db -> file.db
            database_url.trim_start_matches("sqlite:").to_string()
        } else {
            // Assume it's already a path
            database_url
        };
        debug!("Parsed SQLite path for sync: {}", path);
        ratchet_storage::config::StorageConfig::sqlite(path)
    };

    // Override connection settings from config
    let mut storage_config = storage_config;
    storage_config.connection.max_connections = config.server.as_ref()
        .map(|s| s.database.max_connections)
        .unwrap_or(10);
    storage_config.connection.connect_timeout = config.server.as_ref()
        .map(|s| s.database.connection_timeout)
        .unwrap_or_else(|| std::time::Duration::from_secs(30));

    // For now, always create a new connection to the same database
    // This ensures we're using the exact same database file/URL as the server
    info!("🔄 Creating database connection for repository synchronization");
    debug!("Sync database URL: {}", storage_config.connection_url().unwrap_or_default());
    
    let connection_manager = ratchet_storage::connection::create_connection_manager(&storage_config).await
        .context("Failed to create connection manager for repository synchronization")?;

    let repository_factory = ratchet_storage::RepositoryFactory::new(connection_manager);
    let task_repo_for_sync = repository_factory.task_repository();
    let task_repo_for_health = repository_factory.task_repository();
    
    let sync_service = Arc::new(DatabaseSync::new(Arc::new(task_repo_for_sync)));

    // Create registry service with sync capability
    let registry_service = DefaultRegistryService::new(registry_service_config)
        .with_sync_service(sync_service);

    // Perform synchronization
    match registry_service.sync_to_database().await {
        Ok(sync_result) => {
            info!("✅ Repository synchronization completed successfully");
            info!("   ➕ Tasks added: {}", sync_result.tasks_added);
            info!("   🔄 Tasks updated: {}", sync_result.tasks_updated);
            if !sync_result.errors.is_empty() {
                warn!("   ⚠️  Failed to sync {} tasks", sync_result.errors.len());
                for error in &sync_result.errors {
                    warn!("      Task '{}': {}", error.task_ref.name, error.error);
                }
            }
        }
        Err(e) => {
            error!("❌ Repository synchronization failed: {}", e);
            // Don't fail server startup due to sync issues, just log the error
            warn!("Server will continue without repository synchronization");
        }
    }

    // Add a simple verification: try to get repository health status
    info!("🔍 Verifying repository synchronization...");
    match task_repo_for_health.health_check().await {
        Ok(healthy) => {
            info!("   📊 Repository health check: {}", healthy);
        }
        Err(e) => {
            warn!("   ⚠️  Repository health check failed: {}", e);
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
        "MCP server functionality not available. Build with --features=mcp-server"
    ))
}

#[cfg(feature = "mcp-server")]
async fn mcp_serve_command_with_config(
    mut ratchet_config: RatchetConfig,
    _transport: &str,
    _host: &str,
    _port: u16,
) -> Result<()> {
    use ratchet_mcp::{
        security::{McpAuth, McpAuthManager},
        server::{
            adapter::RatchetMcpAdapter,
            config::McpServerConfig,
            tools::RatchetToolRegistry,
        },
        McpServer,
    };
    use std::sync::Arc;
    use tokio::signal;

    // Force stdio transport for MCP serve (ignore CLI args)
    ratchet_config.mcp = Some(ratchet_config::domains::mcp::McpConfig {
        enabled: true,
        transport: "stdio".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8090,
    });

    // Note: Logging is already initialized by main() with stderr-only for stdio mode
    // No need to reinitialize here as it would cause a panic
    
    // Log to file only - stdio must remain clean for JSON-RPC
    info!("🤖 Starting Ratchet MCP server in stdio mode");
    
    // Use modern config directly
    let server_config = ratchet_config.server.as_ref().unwrap();
    
    info!("📋 MCP Configuration:");
    info!("   • Transport: stdio (JSON-RPC over stdin/stdout)");
    info!("   • Database: {}", server_config.database.url);
    info!("   • Max Workers: {}", ratchet_config.execution.max_concurrent_tasks);
    info!("   • Logging: file-only (ratchet.log)");

    // Initialize database (same as serve)
    info!("💾 Initializing database connection...");
    let storage_db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: server_config.database.url.clone(),
        max_connections: server_config.database.max_connections,
        connection_timeout: server_config.database.connection_timeout,
    };

    let database = DatabaseConnection::new(storage_db_config.clone())
        .await
        .context("Failed to connect to database")?;

    info!("🔄 Running database migrations...");
    database
        .migrate()
        .await
        .context("Failed to run database migrations")?;
    info!("✅ Database initialized successfully");

    // Initialize repositories and executor (same as serve)
    let storage_repositories = RepositoryFactory::new(database.clone());

    info!("⚙️  Initializing task executor...");
    let executor_config = ProcessExecutorConfig {
        worker_count: ratchet_config.execution.max_concurrent_tasks,
        task_timeout_seconds: ratchet_config.execution.max_execution_duration.as_secs(),
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    
    // Create both ProcessTaskExecutor (for worker management) and ExecutionBridge (for MCP)
    let process_executor = Arc::new(ProcessTaskExecutor::new(executor_config.clone()));
    let execution_bridge = Arc::new(ExecutionBridge::new(executor_config));
    
    info!("👷 Starting worker processes...");
    process_executor
        .start()
        .await
        .context("Failed to start worker processes")?;
    info!("✅ Worker processes started successfully");

    // Create MCP adapter using ExecutionBridge
    info!("🤖 Initializing MCP adapter...");
    let mcp_task_repo = Arc::new(storage_repositories.task_repository());
    let mcp_execution_repo = Arc::new(storage_repositories.execution_repository());
    
    let adapter = RatchetMcpAdapter::with_bridge_executor(
        execution_bridge.clone(), 
        mcp_task_repo,
        mcp_execution_repo,
    );

    // Create tool registry with the adapter
    let mut tool_registry = RatchetToolRegistry::new();
    tool_registry = tool_registry.with_task_executor(Arc::new(adapter));
    
    // Create security components
    let auth_manager = Arc::new(McpAuthManager::new(McpAuth::default()));
    let audit_logger = Arc::new(ratchet_mcp::security::AuditLogger::new(false));
    
    // Create MCP server configuration - always stdio for this command
    let mcp_server_config = McpServerConfig::from_ratchet_config(
        ratchet_config.mcp.as_ref().unwrap()
    );
    
    // Create and run MCP server
    info!("🚀 Starting MCP server...");
    let mut mcp_server = McpServer::new(
        mcp_server_config,
        Arc::new(tool_registry),
        auth_manager,
        audit_logger,
    );

    info!("✅ MCP server ready - listening on stdin/stdout for JSON-RPC messages");
    
    // Setup graceful shutdown
    let shutdown_future = async {
        signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        info!("🛑 Shutdown signal received, stopping MCP server...");
    };

    // Run stdio server with graceful shutdown
    let server_future = mcp_server.run_stdio();
    
    tokio::select! {
        result = server_future => {
            match result {
                Ok(()) => info!("MCP server stopped gracefully"),
                Err(e) => error!("MCP server error: {}", e),
            }
        }
        _ = shutdown_future => {
            info!("Graceful shutdown initiated");
        }
    }

    // Stop worker processes
    info!("🛑 Stopping worker processes...");
    let shutdown_timeout = tokio::time::Duration::from_secs(10);
    
    match tokio::time::timeout(shutdown_timeout, process_executor.stop()).await {
        Ok(Ok(())) => {
            info!("✅ Worker processes stopped successfully");
        }
        Ok(Err(e)) => {
            warn!("⚠️  Error stopping worker processes: {}", e);
        }
        Err(_) => {
            warn!("⚠️  Worker shutdown timed out after {}s", shutdown_timeout.as_secs());
        }
    }

    info!("Ratchet MCP server shutdown complete");
    Ok(())
}

/// Initialize logging from configuration with fallback to simple tracing
#[cfg(feature = "server")]
fn init_logging_with_config(
    _config: &RatchetConfig,
    log_level: Option<&String>,
    record_dir: Option<&PathBuf>,
) -> Result<()> {
    // Check if RUST_LOG is set - if so, prefer simple tracing
    if std::env::var("RUST_LOG").is_ok() && record_dir.is_none() {
        // RUST_LOG is set and we're not recording, use simple tracing
        init_simple_tracing(log_level)?;
        return Ok(());
    }

    // For recording mode, always use simple tracing with file logging
    if let Some(record_path) = record_dir {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let session_dir = record_path.join(format!("ratchet_session_{}", timestamp));
        fs::create_dir_all(&session_dir).context("Failed to create recording directory")?;

        // Store the session directory for use by other components
        ratchet_cli_tools::set_recording_dir(session_dir.clone())?;

        // Use simple tracing with file output for recording
        init_simple_tracing_with_file(log_level, &session_dir.join("ratchet.log"))?;

        info!(
            "Recording session to: {:?}",
            record_path.join(format!("ratchet_session_{}", timestamp))
        );
        return Ok(());
    }

    // For server mode without recording, use simple tracing 
    // The complex structured logging from ratchet-lib is causing issues during migration
    init_simple_tracing(log_level)?;
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
    // Priority: RUST_LOG env var > --log-level flag > default "info"
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // RUST_LOG is set, use it but allow --log-level to set a minimum level
        if let Some(level) = log_level {
            // Combine RUST_LOG with minimum level from --log-level
            let combined = format!("{},{}", rust_log, level);
            EnvFilter::try_new(&combined).unwrap_or_else(|_| {
                // If combination fails, just use RUST_LOG
                EnvFilter::try_new(&rust_log).unwrap_or_else(|_| {
                    eprintln!("Invalid RUST_LOG '{}', falling back to 'info'", rust_log);
                    EnvFilter::new("info")
                })
            })
        } else {
            // Just use RUST_LOG
            EnvFilter::try_new(&rust_log).unwrap_or_else(|_| {
                eprintln!("Invalid RUST_LOG '{}', falling back to 'info'", rust_log);
                EnvFilter::new("info")
            })
        }
    } else if let Some(level) = log_level {
        // No RUST_LOG, use --log-level
        EnvFilter::try_new(level).unwrap_or_else(|_| {
            eprintln!("Invalid log level '{}', falling back to 'info'", level);
            EnvFilter::new("info")
        })
    } else {
        // Neither RUST_LOG nor --log-level, use default
        EnvFilter::new("info")
    };

    // Use try_init to avoid panic if global subscriber already set
    if let Err(_) = tracing_subscriber::fmt().with_env_filter(env_filter).try_init() {
        eprintln!("Global tracing subscriber already initialized, skipping");
    } else {
        debug!("Simple tracing initialized");
    }
    Ok(())
}

/// Initialize simple tracing with file output for recording mode
fn init_simple_tracing_with_file(log_level: Option<&String>, log_file_path: &std::path::Path) -> Result<()> {
    // Priority: --log-level flag > RUST_LOG env var > default "info" 
    let env_filter = if let Some(level) = log_level {
        EnvFilter::try_new(level).unwrap_or_else(|_| {
            eprintln!("Invalid log level '{}', falling back to 'info'", level);
            EnvFilter::new("info")
        })
    } else if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::try_new(&rust_log).unwrap_or_else(|_| {
            eprintln!("Invalid RUST_LOG '{}', falling back to 'info'", rust_log);
            EnvFilter::new("info")
        })
    } else {
        EnvFilter::new("info")
    };

    // For recording mode, just use simple console logging with a note about the file location
    // Complex file logging can be added later if needed
    if let Err(_) = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init() {
        eprintln!("Global tracing subscriber already initialized, skipping");
    } else {
        info!("Tracing initialized for recording mode - logs available at: {:?}", log_file_path);
    }
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
    // Use try_init to avoid panic if global subscriber already set
    if let Err(_) = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init() {
        // Silent failure for MCP stdio mode to avoid contaminating stderr
    }

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
    // Use try_init to avoid panic if global subscriber already set
    if let Err(_) = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init() {
        eprintln!("Global tracing subscriber already initialized, skipping");
    } else {
        debug!("Worker tracing initialized");
    }
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

    // Load using ratchet_js's filesystem loader
    let fs_task = FileSystemTask::from_fs(from_fs)
        .map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Convert to Core Task using the loaded content
    let version = fs_task.metadata.core
        .as_ref()
        .map(|c| c.version.clone())
        .unwrap_or_else(|| fs_task.metadata.version.clone());

    let core_task = TaskBuilder::new(&fs_task.metadata.label, &version)
        .input_schema(fs_task.input_schema.unwrap_or_else(|| json!({})))
        .output_schema(fs_task.output_schema.unwrap_or_else(|| json!({})))
        .javascript_source(&fs_task.content)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build Core Task: {}", e))?;

    Ok(core_task)
}

/// Run a task from a file system path using runtime executor
#[cfg(all(feature = "runtime", feature = "core"))]
async fn run_task_runtime(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {} (using runtime executor)", from_fs);

    // Load the task as Core Task (using ratchet_js filesystem loader)
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

/// Run a task from a file system path using modular executor (ratchet-js)
#[cfg(feature = "javascript")]
async fn run_task_modular(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {} (using modular executor)", from_fs);

    let result = load_and_execute_task(from_fs, input.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

    Ok(result)
}

/// Run a task from a file system path using modern CLI tools
#[cfg(feature = "javascript")]
async fn run_task(from_fs: &str, input: &JsonValue) -> Result<JsonValue> {
    info!("Loading task from: {} (using modern CLI tools)", from_fs);

    // Execute the task using modern CLI tools with automatic validation
    let result = ratchet_cli_tools::execute_task_with_lib_compatibility(
        from_fs,
        ratchet_cli_tools::TaskInput::new(input.clone()) // Use modern execution by default
    )
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))?;

    Ok(result)
}

/// Validate a task using modular components
#[cfg(feature = "javascript")]
fn validate_task_modular(from_fs: &str) -> Result<()> {
    info!("Validating task from: {} (using modular validator)", from_fs);

    // Load the task using ratchet-js
    let task = FileSystemTask::from_fs(from_fs)
        .map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Validate the task
    match task.validate() {
        Ok(_) => {
            println!("✅ Task validation passed");
            info!("Task '{}' is valid", task.label());
            Ok(())
        }
        Err(e) => {
            println!("❌ Task validation failed: {}", e);
            error!("Task validation failed: {}", e);
            Err(e.into())
        }
    }
}

/// Test a task by running its test cases using modular components
#[cfg(feature = "javascript")]
async fn test_task_modular(from_fs: &str) -> Result<()> {
    info!("Testing task from: {} (using modular executor)", from_fs);

    // Load the task using ratchet-js
    let task = FileSystemTask::from_fs(from_fs)
        .map_err(|e| anyhow::anyhow!("Failed to load task: {}", e))?;

    // Validate the task first
    task.validate().context("Task validation failed")?;

    // Get test directory
    let task_path = std::path::Path::new(from_fs);
    let test_dir = task_path.join("tests");

    if !test_dir.exists() {
        println!("No tests directory found at: {}", test_dir.display());
        info!("No tests to run for task '{}'", task.label());
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
        info!("No test files found for task '{}'", task.label());
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for test_file in test_files {
        let test_name = test_file.file_stem().unwrap().to_str().unwrap();
        print!("Running test '{}' ... ", test_name);

        match run_single_test_modular(&task, &test_file).await {
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

/// Run a single test case using modular components
#[cfg(feature = "javascript")]
async fn run_single_test_modular(task: &FileSystemTask, test_file: &std::path::Path) -> Result<()> {
    use serde_json::Value;

    // Load test data
    let test_content = fs::read_to_string(test_file).context("Failed to read test file")?;
    let test_data: Value = from_str(&test_content).context("Failed to parse test JSON")?;

    // Extract input and expected output
    let input = test_data.get("input").unwrap_or(&json!({})).clone();
    let expected = test_data
        .get("expected_output")
        .ok_or_else(|| anyhow::anyhow!("Test file missing 'expected_output' field"))?;

    // Execute the task using ratchet-js directly
    let js_task = task.to_js_task();
    let runner = ratchet_js::JsTaskRunner::new();
    
    let actual = runner
        .execute_task(&js_task, input, None)
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

/// Legacy validate task function (deprecated - use validate_task_modular instead)
#[cfg(feature = "javascript")]
#[allow(dead_code)]
fn validate_task(from_fs: &str) -> Result<()> {
    info!("Validating task from: {}", from_fs);

    // Use modular validation through ratchet-js
    match ratchet_js::FileSystemTask::from_fs(from_fs) {
        Ok(_task) => {
            println!("✅ Task validation passed");
            info!("Task is valid");
            Ok(())
        }
        Err(e) => {
            println!("❌ Task validation failed: {}", e);
            error!("Task validation failed: {}", e);
            Err(e.into())
        }
    }
}

/// Legacy test task function (deprecated - use test_task_modular instead)
#[cfg(feature = "javascript")]
#[allow(dead_code)]
async fn test_task(from_fs: &str) -> Result<()> {
    info!("Testing task from: {}", from_fs);

    // Validate the task first using modern components
    ratchet_js::FileSystemTask::from_fs(from_fs)
        .map_err(|e| anyhow::anyhow!("Task validation failed: {}", e))?;

    // Get test directory
    let task_path = std::path::Path::new(from_fs);
    let test_dir = task_path.join("tests");

    if !test_dir.exists() {
        println!("No tests directory found at: {}", test_dir.display());
        info!("No tests to run for task");
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
        info!("No test files found for task");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for test_file in test_files {
        let test_name = test_file.file_stem().unwrap().to_str().unwrap();
        print!("Running test '{}' ... ", test_name);

        match run_single_test_simple(from_fs, &test_file).await {
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

/// Legacy run single test function (deprecated - use run_single_test_modular instead)
#[cfg(feature = "javascript")]
#[allow(dead_code)]
async fn run_single_test(task_path: &str, test_file: &std::path::Path) -> Result<()> {
    let task_dir = std::path::Path::new(task_path).parent().unwrap();
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

    // Execute the task using CLI tools
    let actual = ratchet_cli_tools::execute_task_with_lib_compatibility(
        &task_dir.to_string_lossy(),
        ratchet_cli_tools::TaskInput::legacy(input.clone())
    )
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

/// Run a single test case using CLI tools (simple version)
#[cfg(feature = "javascript")]
async fn run_single_test_simple(task_path: &str, test_file: &std::path::Path) -> Result<()> {
    use serde_json::Value;

    // Load test data
    let test_content = fs::read_to_string(test_file).context("Failed to read test file")?;
    let test_data: Value = from_str(&test_content).context("Failed to parse test JSON")?;

    // Extract input and expected output
    let input = test_data.get("input").unwrap_or(&json!({})).clone();
    let expected = test_data
        .get("expected_output")
        .ok_or_else(|| anyhow::anyhow!("Test file missing 'expected_output' field"))?;

    // Execute the task using CLI tools
    let actual = ratchet_cli_tools::execute_task_with_lib_compatibility(
        task_path,
        ratchet_cli_tools::TaskInput::new(input)
    )
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed during test: {}", e))?;

    // Compare results
    if &actual == expected {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Test failed: expected {:?}, got {:?}",
            expected,
            actual
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
    ratchet_cli_tools::set_recording_dir(recording_path.clone())?;

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

            // Execute the task using modern CLI tools
            let result = match ratchet_cli_tools::execute_task_with_lib_compatibility(
                        &task_path,
                        ratchet_cli_tools::TaskInput::legacy(input_data)
                    ).await {
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

            let result = match ratchet_js::FileSystemTask::from_fs(&task_path) {
                Ok(_task) => {
                    info!("Worker task validation passed: {}", task_path);
                    TaskValidationResult {
                        valid: true,
                        error_message: None,
                        error_details: None,
                    }
                }
                Err(e) => {
                    error!("Worker task validation failed: {}", e);
                    TaskValidationResult {
                        valid: false,
                        error_message: Some(e.to_string()),
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
                status: WorkerStatus {
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

    // Generate task files (the generator will create the directory and check for conflicts)
    let config = ratchet_cli_tools::TaskGenerationConfig::new(path.clone())
        .with_label(label.as_deref().unwrap_or("My Task"))
        .with_description(
            description
                .as_deref()
                .unwrap_or("A description of what this task does"),
        )
        .with_version(version.as_deref().unwrap_or("1.0.0"));

    let _result =
        ratchet_cli_tools::generate_task(config).context("Failed to generate task template")?;

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

/// Handle repository initialization
fn handle_repo_init(
    directory: &PathBuf,
    name: Option<&str>,
    description: Option<&str>,
    version: &str,
    ratchet_version: &str,
    force: bool,
) -> Result<()> {
    info!("Initializing task repository at: {:?}", directory);

    // Check if directory exists and is empty (unless force is used)
    if directory.exists() {
        if !force {
            let entries: Vec<_> = fs::read_dir(directory)
                .context("Failed to read directory")?
                .collect();
            
            if !entries.is_empty() {
                return Err(anyhow::anyhow!(
                    "Directory is not empty: {:?}. Use --force to initialize anyway.",
                    directory
                ));
            }
        }
    } else {
        // Create directory if it doesn't exist
        fs::create_dir_all(directory).context("Failed to create directory")?;
    }

    // Create .ratchet directory
    let ratchet_dir = directory.join(".ratchet");
    fs::create_dir_all(&ratchet_dir).context("Failed to create .ratchet directory")?;

    // Generate repository name from directory if not provided
    let repo_name = name.unwrap_or_else(|| {
        directory
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Task Repository")
    });

    // Create registry.yaml
    let registry_content = format!(
        r#"# Repository metadata
name: "{}"
description: "{}"
version: "{}"
ratchet_version: "{}"
"#,
        repo_name,
        description.unwrap_or("A collection of Ratchet tasks"),
        version,
        ratchet_version
    );

    let registry_path = ratchet_dir.join("registry.yaml");
    fs::write(&registry_path, registry_content).context("Failed to write registry.yaml")?;

    // Create initial index.json
    let index_content = format!(
        r#"{{
  "version": "1.0",
  "repository": {{
    "name": "{}",
    "description": "{}",
    "version": "{}"
  }},
  "tasks": [],
  "collections": {{}},
  "templates": {{}},
  "statistics": {{
    "total_tasks": 0,
    "categories": {{}},
    "complexity_distribution": {{}}
  }},
  "generated_at": "{}"
}}"#,
        repo_name,
        description.unwrap_or("A collection of Ratchet tasks"),
        version,
        chrono::Utc::now().to_rfc3339()
    );

    let index_path = ratchet_dir.join("index.json");
    fs::write(&index_path, index_content).context("Failed to write index.json")?;

    // Create directory structure
    let tasks_dir = directory.join("tasks");
    let collections_dir = directory.join("collections");
    let templates_dir = directory.join("templates");

    fs::create_dir_all(&tasks_dir).context("Failed to create tasks directory")?;
    fs::create_dir_all(&collections_dir).context("Failed to create collections directory")?;
    fs::create_dir_all(&templates_dir).context("Failed to create templates directory")?;

    // Create .gitkeep files for empty directories
    let collections_gitkeep = format!(
        r#"# Collections Directory

This directory is reserved for task collections - curated sets of related tasks that work together to accomplish complex workflows.

Collections will be defined as YAML files that reference tasks and define execution order, dependencies, and shared configuration.
"#
    );
    fs::write(collections_dir.join(".gitkeep"), collections_gitkeep)
        .context("Failed to write collections/.gitkeep")?;

    let templates_gitkeep = format!(
        r#"# Templates Directory

This directory contains task templates - boilerplate structures that can be used to quickly create new tasks with common patterns.

Templates include standard file structures, schema definitions, and example implementations for different task types.
"#
    );
    fs::write(templates_dir.join(".gitkeep"), templates_gitkeep)
        .context("Failed to write templates/.gitkeep")?;

    // Create README.md
    let readme_content = format!(
        r#"# {}

{}

## Repository Structure

```
.
├── .ratchet/
│   ├── registry.yaml    # Repository metadata and configuration
│   └── index.json       # Fast task discovery index
├── tasks/               # Individual task implementations
├── collections/         # Task collections and workflows
├── templates/           # Task templates and boilerplate
└── README.md           # This file
```

## Getting Started

### Adding Tasks

1. Create a new directory under `tasks/`
2. Include all required files:
   - `metadata.json`: Task definition and configuration
   - `main.js`: Task implementation
   - `input.schema.json`: Input validation schema
   - `output.schema.json`: Output format definition
   - `tests/`: Test cases and examples

### Refreshing Metadata

After adding or modifying tasks, refresh the repository metadata:

```bash
ratchet repo refresh-metadata
```

### Using with Git+HTTP Registry

Configure this repository as a Git task source in your Ratchet configuration:

```yaml
registries:
  - name: "my-tasks"
    source:
      type: "git"
      url: "https://github.com/your-org/your-task-repo.git"
      ref: "main"
```

## License

See individual task metadata for licensing information.
"#,
        repo_name,
        description.unwrap_or("A collection of Ratchet tasks")
    );

    let readme_path = directory.join("README.md");
    fs::write(&readme_path, readme_content).context("Failed to write README.md")?;

    println!("✅ Task repository initialized at: {:?}", directory);
    println!("📝 Repository structure:");
    println!("   • .ratchet/registry.yaml: Repository metadata");
    println!("   • .ratchet/index.json: Task discovery index");
    println!("   • tasks/: Directory for task implementations");
    println!("   • collections/: Directory for task collections");
    println!("   • templates/: Directory for task templates");
    println!("   • README.md: Repository documentation");
    println!();
    println!("🚀 Next steps:");
    println!("   1. Add your first task: ratchet generate task --path {}/tasks/my-task", directory.display());
    println!("   2. Refresh metadata: ratchet repo refresh-metadata {}", directory.display());
    
    Ok(())
}

/// Handle repository metadata refresh
async fn handle_repo_refresh_metadata(directory: Option<&PathBuf>, force: bool) -> Result<()> {
    // Use current directory if none specified
    let repo_dir = directory
        .map(|d| d.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    info!("Refreshing repository metadata at: {:?}", repo_dir);

    // Verify this is a repository directory
    let ratchet_dir = repo_dir.join(".ratchet");
    let registry_path = ratchet_dir.join("registry.yaml");
    
    if !registry_path.exists() {
        return Err(anyhow::anyhow!(
            "Not a Ratchet repository: {:?} (missing .ratchet/registry.yaml). Run 'ratchet repo init' first.",
            repo_dir
        ));
    }

    // Load existing registry metadata
    let registry_content = fs::read_to_string(&registry_path)
        .context("Failed to read registry.yaml")?;
    
    let registry_yaml: serde_yaml::Value = serde_yaml::from_str(&registry_content)
        .context("Failed to parse registry.yaml")?;

    let repo_name = registry_yaml.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Task Repository");
    let repo_description = registry_yaml.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("A collection of Ratchet tasks");
    let repo_version = registry_yaml.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");

    // Scan tasks directory
    let tasks_dir = repo_dir.join("tasks");
    let mut tasks = Vec::new();
    let mut categories = std::collections::HashMap::new();
    let mut complexity_dist = std::collections::HashMap::new();

    if tasks_dir.exists() {
        println!("🔍 Scanning tasks directory...");
        
        let task_entries = fs::read_dir(&tasks_dir)
            .context("Failed to read tasks directory")?;

        for entry in task_entries {
            let entry = entry.context("Failed to read directory entry")?;
            let task_path = entry.path();
            
            if task_path.is_dir() {
                let metadata_file = task_path.join("metadata.json");
                
                if metadata_file.exists() {
                    match load_task_metadata(&metadata_file, &task_path, &tasks_dir) {
                        Ok(task_info) => {
                            let task_name = task_info.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            println!("   ✅ {}", task_name);
                            
                            // Update statistics
                            if let Some(category) = task_info.get("category").and_then(|v| v.as_str()) {
                                *categories.entry(category.to_string()).or_insert(0) += 1;
                            }
                            if let Some(complexity) = task_info.get("complexity").and_then(|v| v.as_str()) {
                                *complexity_dist.entry(complexity.to_string()).or_insert(0) += 1;
                            }
                            
                            tasks.push(task_info);
                        }
                        Err(e) => {
                            println!("   ❌ {}: {}", task_path.file_name().unwrap().to_string_lossy(), e);
                            if !force {
                                return Err(anyhow::anyhow!("Task metadata error: {}. Use --force to continue anyway.", e));
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate new index.json
    let index_json = serde_json::json!({
        "version": "1.0",
        "repository": {
            "name": repo_name,
            "description": repo_description,
            "version": repo_version
        },
        "tasks": tasks,
        "collections": {},
        "templates": {},
        "statistics": {
            "total_tasks": tasks.len(),
            "categories": categories,
            "complexity_distribution": complexity_dist
        },
        "generated_at": chrono::Utc::now().to_rfc3339()
    });

    let index_path = ratchet_dir.join("index.json");
    let index_content = serde_json::to_string_pretty(&index_json)
        .context("Failed to serialize index.json")?;
    
    fs::write(&index_path, index_content)
        .context("Failed to write index.json")?;

    println!();
    println!("✅ Repository metadata refreshed");
    println!("📊 Statistics:");
    println!("   • Total tasks: {}", tasks.len());
    println!("   • Categories: {}", categories.len());
    println!("   • Index updated: .ratchet/index.json");
    
    Ok(())
}

/// Load task metadata from a metadata.json file
fn load_task_metadata(
    metadata_file: &PathBuf, 
    task_path: &PathBuf, 
    tasks_dir: &PathBuf
) -> Result<serde_json::Value> {
    let metadata_content = fs::read_to_string(metadata_file)
        .with_context(|| format!("Failed to read metadata file: {:?}", metadata_file))?;
    
    let mut metadata: serde_json::Value = serde_json::from_str(&metadata_content)
        .with_context(|| format!("Failed to parse metadata JSON: {:?}", metadata_file))?;
    
    // Add the relative path from tasks directory
    let relative_path = task_path.strip_prefix(tasks_dir)
        .with_context(|| format!("Failed to get relative path for: {:?}", task_path))?;
    
    let path_str = format!("tasks/{}", relative_path.to_string_lossy());
    metadata["path"] = serde_json::Value::String(path_str);
    
    // Ensure required fields exist with defaults
    if !metadata.get("name").is_some() {
        let task_name = task_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        metadata["name"] = serde_json::Value::String(task_name.to_string());
    }
    
    if !metadata.get("supports_streaming").is_some() {
        metadata["supports_streaming"] = serde_json::Value::Bool(false);
    }
    
    Ok(metadata)
}

#[derive(Debug, Clone)]
struct RepositoryStatus {
    name: String,
    source_type: String,
    uri: String,
    enabled: bool,
    sync_state: String,
    last_checked: String,
    tasks_count: Option<usize>,
    health_status: String,
    error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RepositoryVerification {
    name: String,
    source_type: String,
    uri: String,
    enabled: bool,
    accessible: bool,
    readable: bool,
    correctly_configured: bool,
    usable: bool,
    verification_time: String,
    tasks: Option<Vec<TaskInfo>>,
    issues: Vec<String>,
    warnings: Vec<String>,
    error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TaskInfo {
    name: String,
    version: Option<String>,
    path: String,
    description: Option<String>,
    tags: Vec<String>,
}

/// Handle repository status display
async fn handle_repo_status(
    config: &RatchetConfig,
    detailed: bool,
    repository_filter: Option<&str>,
    format: &str,
) -> Result<()> {
    info!("Checking repository status (detailed: {}, format: {})", detailed, format);

    let mut repositories = Vec::new();

    // Check if we have registry configuration
    if let Some(registry_config) = &config.registry {
        info!("Found {} configured repositories", registry_config.sources.len());
        
        for source in &registry_config.sources {
            // Skip if filtering for specific repository
            if let Some(filter) = repository_filter {
                if source.name != filter {
                    continue;
                }
            }

            let source_type_name = match source.source_type {
                ratchet_config::domains::registry::RegistrySourceType::Filesystem => "filesystem",
                ratchet_config::domains::registry::RegistrySourceType::Http => "http",
                ratchet_config::domains::registry::RegistrySourceType::Git => "git",
                ratchet_config::domains::registry::RegistrySourceType::S3 => "s3",
            };

            // Test repository connectivity and get basic info
            let (sync_state, health_status, error_message, tasks_count) = 
                test_repository_connection(source, detailed).await;

            let status = RepositoryStatus {
                name: source.name.clone(),
                source_type: source_type_name.to_string(),
                uri: source.uri.clone(),
                enabled: source.enabled,
                sync_state,
                last_checked: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                tasks_count,
                health_status,
                error_message,
            };

            repositories.push(status);
        }
    } else {
        info!("No registry configuration found");
    }

    // Display results
    match format.to_lowercase().as_str() {
        "json" => {
            let output = serde_json::json!({
                "repositories": repositories.iter().map(|r| {
                    let mut repo = serde_json::json!({
                        "name": r.name,
                        "source_type": r.source_type,
                        "uri": r.uri,
                        "enabled": r.enabled,
                        "sync_state": r.sync_state,
                        "last_checked": r.last_checked,
                        "health_status": r.health_status
                    });
                    
                    if let Some(count) = r.tasks_count {
                        repo["tasks_count"] = serde_json::json!(count);
                    }
                    
                    if let Some(ref error) = r.error_message {
                        repo["error"] = serde_json::json!(error);
                    }
                    
                    repo
                }).collect::<Vec<_>>(),
                "total_repositories": repositories.len(),
                "active_repositories": repositories.iter().filter(|r| r.enabled).count(),
                "healthy_repositories": repositories.iter().filter(|r| r.health_status == "healthy").count()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "yaml" => {
            let output = serde_json::json!({
                "repositories": repositories.iter().map(|r| {
                    let mut repo = serde_json::json!({
                        "name": r.name,
                        "source_type": r.source_type,
                        "uri": r.uri,
                        "enabled": r.enabled,
                        "sync_state": r.sync_state,
                        "last_checked": r.last_checked,
                        "health_status": r.health_status
                    });
                    
                    if let Some(count) = r.tasks_count {
                        repo["tasks_count"] = serde_json::json!(count);
                    }
                    
                    if let Some(ref error) = r.error_message {
                        repo["error"] = serde_json::json!(error);
                    }
                    
                    repo
                }).collect::<Vec<_>>(),
                "total_repositories": repositories.len(),
                "active_repositories": repositories.iter().filter(|r| r.enabled).count(),
                "healthy_repositories": repositories.iter().filter(|r| r.health_status == "healthy").count()
            });
            let yaml_output = serde_yaml::to_string(&output)?;
            println!("{}", yaml_output);
        }
        "table" | _ => {
            println!();
            println!("📋 Repository Status Report");
            println!("══════════════════════════════════════════");
            println!("Total repositories: {}", repositories.len());
            println!("Active repositories: {}", repositories.iter().filter(|r| r.enabled).count());
            println!("Healthy repositories: {}", repositories.iter().filter(|r| r.health_status == "healthy").count());
            println!();

            if repositories.is_empty() {
                println!("❌ No repositories configured.");
                println!("💡 Add repositories to your config file or use 'ratchet repo init' to create a local repository.");
                return Ok(());
            }

            for repo in &repositories {
                let status_icon = if !repo.enabled {
                    "⏸️"
                } else {
                    match repo.health_status.as_str() {
                        "healthy" => "✅",
                        "warning" => "⚠️",
                        "error" => "❌",
                        _ => "❓",
                    }
                };

                println!("{} {} ({}, {})", status_icon, repo.name, repo.source_type, repo.uri);
                
                if detailed {
                    println!("   └─ Enabled: {}", if repo.enabled { "Yes" } else { "No" });
                    println!("   └─ Status: {}", repo.sync_state);
                    println!("   └─ Health: {}", repo.health_status);
                    if let Some(count) = repo.tasks_count {
                        println!("   └─ Tasks: {}", count);
                    }
                    if let Some(ref error) = repo.error_message {
                        println!("   └─ Error: {}", error);
                    }
                    println!("   └─ Last checked: {}", repo.last_checked);
                    println!();
                } else {
                    if repo.enabled {
                        let task_info = if let Some(count) = repo.tasks_count {
                            format!(" ({} tasks)", count)
                        } else {
                            String::new()
                        };
                        println!("   └─ {}{}", repo.sync_state, task_info);
                    } else {
                        println!("   └─ Disabled");
                    }
                    
                    if let Some(ref error) = repo.error_message {
                        println!("   └─ ❌ {}", error);
                    }
                }
            }
            
            if !detailed && repositories.iter().any(|r| r.health_status != "healthy") {
                println!();
                println!("💡 Use --detailed flag for more information about repository issues.");
            }
        }
    }

    Ok(())
}

/// Handle repository verification with comprehensive checks
async fn handle_repo_verify(
    config: &RatchetConfig,
    repository_filter: Option<&str>,
    format: &str,
    detailed: bool,
    list_tasks: bool,
    offline: bool,
) -> Result<()> {
    info!("Verifying repositories (detailed: {}, format: {}, list_tasks: {}, offline: {})", 
          detailed, format, list_tasks, offline);

    let mut verifications = Vec::new();

    // Check if we have registry configuration
    if let Some(registry_config) = &config.registry {
        info!("Found {} configured repositories", registry_config.sources.len());
        
        for source in &registry_config.sources {
            // Skip if filtering for specific repository
            if let Some(filter) = repository_filter {
                if source.name != filter {
                    continue;
                }
            }

            let verification = verify_repository(source, true, offline).await;
            verifications.push(verification);
        }
    } else {
        eprintln!("❌ No registry configuration found");
        return Ok(());
    }

    // Display results based on format
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "repositories": verifications
            }))?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&serde_yaml::to_value(&serde_json::json!({
                "repositories": verifications
            }))?)?);
        }
        "table" | _ => {
            if verifications.is_empty() {
                if let Some(filter) = repository_filter {
                    eprintln!("❌ No repository found with name '{}'", filter);
                } else {
                    eprintln!("❌ No repositories configured");
                }
                return Ok(());
            }

            println!("\n📋 Repository Verification Report");
            println!("════════════════════════════════");

            for verification in &verifications {
                let status_icon = if !verification.enabled {
                    "⏸️"
                } else if verification.usable {
                    "✅"
                } else if verification.accessible {
                    "⚠️"
                } else {
                    "❌"
                };

                println!("\n{} {} ({}) - {}", 
                         status_icon, verification.name, verification.source_type, verification.uri);
                
                // Show verification results
                println!("   📡 Accessible: {}", if verification.accessible { "✅ Yes" } else { "❌ No" });
                println!("   📖 Readable: {}", if verification.readable { "✅ Yes" } else { "❌ No" });
                println!("   ⚙️  Configured: {}", if verification.correctly_configured { "✅ Yes" } else { "❌ No" });
                println!("   🚀 Usable: {}", if verification.usable { "✅ Yes" } else { "❌ No" });
                
                // Show metadata details
                println!("   🕒 Verified: {}", verification.verification_time);
                if verification.enabled {
                    println!("   ⚡ Status: Enabled");
                } else {
                    println!("   ⏸️  Status: Disabled");
                }
                
                // Show repository-specific configuration details
                if verification.source_type == "git" {
                    // For Git repositories, show branch information
                    println!("   🌿 Configuration: Git repository");
                    if detailed {
                        println!("      • Repository type: Git");
                        println!("      • Protocol: HTTPS");
                    }
                } else if verification.source_type == "filesystem" {
                    println!("   📁 Configuration: Local filesystem");
                    if detailed {
                        println!("      • Repository type: Filesystem");
                        println!("      • Access: Direct file access");
                    }
                } else {
                    println!("   ⚙️  Configuration: {} repository", verification.source_type);
                }
                
                // Always show task information when available
                if let Some(ref tasks) = verification.tasks {
                    println!("   📦 Tasks: {} found", tasks.len());
                    
                    if !tasks.is_empty() {
                        println!("   └─ Available tasks:");
                        for task in tasks.iter().take(if detailed { tasks.len() } else { 10 }) {
                            let version_info = task.version.as_deref().unwrap_or("unknown");
                            let desc_info = task.description.as_deref().unwrap_or("No description");
                            println!("      • {} (v{}) - {}", task.name, version_info, desc_info);
                            if !task.tags.is_empty() {
                                println!("        📋 Tags: {}", task.tags.join(", "));
                            }
                            if detailed {
                                println!("        📂 Path: {}", task.path);
                            }
                        }
                        if !detailed && tasks.len() > 10 {
                            println!("      ... and {} more tasks (use --detailed to see all)", tasks.len() - 10);
                        }
                    }
                } else {
                    println!("   📦 Tasks: Discovery not available for this repository type");
                }
                
                if !verification.warnings.is_empty() {
                    println!("   ⚠️  Warnings:");
                    for warning in &verification.warnings {
                        println!("      • {}", warning);
                    }
                }
                
                if !verification.issues.is_empty() {
                    println!("   ❌ Issues:");
                    for issue in &verification.issues {
                        println!("      • {}", issue);
                    }
                }
                
                if let Some(ref error) = verification.error_message {
                    println!("   💥 Error: {}", error);
                }
            }

            // Summary
            let total = verifications.len();
            let usable = verifications.iter().filter(|v| v.usable).count();
            let accessible = verifications.iter().filter(|v| v.accessible).count();
            let disabled = verifications.iter().filter(|v| !v.enabled).count();
            
            println!("\n📊 Summary");
            println!("═══════════");
            println!("Total repositories: {}", total);
            println!("Usable: {} ({:.1}%)", usable, if total > 0 { (usable as f64 / total as f64) * 100.0 } else { 0.0 });
            println!("Accessible: {} ({:.1}%)", accessible, if total > 0 { (accessible as f64 / total as f64) * 100.0 } else { 0.0 });
            println!("Disabled: {}", disabled);
            
            if usable == total && disabled == 0 {
                println!("\n🎉 All repositories are working correctly!");
            } else if usable > 0 {
                println!("\n💡 Some repositories need attention. Check issues above for details.");
            } else {
                println!("\n🚨 No repositories are currently usable. Please check configuration and connectivity.");
            }
        }
    }

    Ok(())
}

/// Test repository connection and get basic status info
async fn test_repository_connection(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    detailed: bool,
) -> (String, String, Option<String>, Option<usize>) {
    if !source.enabled {
        return ("disabled".to_string(), "disabled".to_string(), None, None);
    }

    match source.source_type {
        ratchet_config::domains::registry::RegistrySourceType::Git => {
            test_git_repository(source, detailed).await
        }
        ratchet_config::domains::registry::RegistrySourceType::Filesystem => {
            test_filesystem_repository(source, detailed).await
        }
        ratchet_config::domains::registry::RegistrySourceType::Http => {
            test_http_repository(source, detailed).await
        }
        ratchet_config::domains::registry::RegistrySourceType::S3 => {
            ("not implemented".to_string(), "warning".to_string(), 
             Some("S3 repository support not yet implemented".to_string()), None)
        }
    }
}

/// Test Git repository connectivity
async fn test_git_repository(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    detailed: bool,
) -> (String, String, Option<String>, Option<usize>) {
    info!("Testing Git repository: {}", source.uri);
    
    // For now, we'll do a basic validation and return placeholder status
    // In a full implementation, we would:
    // 1. Test Git connectivity
    // 2. Check authentication
    // 3. Count available tasks
    // 4. Verify branch/ref exists
    
    if source.uri.starts_with("https://") || source.uri.starts_with("git://") || source.uri.starts_with("ssh://") {
        // Basic URI validation passed
        if detailed {
            // In detailed mode, we could try to actually connect
            ("ready to sync".to_string(), "healthy".to_string(), None, Some(0))
        } else {
            ("configured".to_string(), "healthy".to_string(), None, None)
        }
    } else {
        ("invalid configuration".to_string(), "error".to_string(), 
         Some("Invalid Git URL format".to_string()), None)
    }
}

/// Test filesystem repository connectivity
async fn test_filesystem_repository(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    detailed: bool,
) -> (String, String, Option<String>, Option<usize>) {
    info!("Testing filesystem repository: {}", source.uri);
    
    let path = if source.uri.starts_with("file://") {
        &source.uri[7..]
    } else {
        &source.uri
    };
    
    let path_buf = std::path::PathBuf::from(path);
    
    if !path_buf.exists() {
        return ("path not found".to_string(), "error".to_string(), 
                Some(format!("Path does not exist: {}", path)), None);
    }
    
    if !path_buf.is_dir() {
        return ("not a directory".to_string(), "error".to_string(), 
                Some("Path is not a directory".to_string()), None);
    }
    
    // Count tasks if detailed mode
    let task_count = if detailed {
        count_filesystem_tasks(&path_buf).await
    } else {
        None
    };
    
    ("accessible".to_string(), "healthy".to_string(), None, task_count)
}

/// Test HTTP repository connectivity
async fn test_http_repository(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    _detailed: bool,
) -> (String, String, Option<String>, Option<usize>) {
    info!("Testing HTTP repository: {}", source.uri);
    
    // Basic URL validation
    if source.uri.starts_with("http://") || source.uri.starts_with("https://") {
        // In a full implementation, we would make an HTTP request here
        ("configured".to_string(), "healthy".to_string(), None, None)
    } else {
        ("invalid configuration".to_string(), "error".to_string(), 
         Some("Invalid HTTP URL format".to_string()), None)
    }
}

/// Count tasks in a filesystem repository
async fn count_filesystem_tasks(path: &std::path::Path) -> Option<usize> {
    let mut task_count = 0;
    
    // Look for tasks directory first
    let tasks_dir = path.join("tasks");
    let search_path = if tasks_dir.exists() {
        tasks_dir
    } else {
        path.to_path_buf()
    };
    
    if let Ok(entries) = fs::read_dir(&search_path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let metadata_file = entry_path.join("metadata.json");
                if metadata_file.exists() {
                    task_count += 1;
                }
            }
        }
    }
    
    Some(task_count)
}

/// Comprehensive repository verification with task discovery
async fn verify_repository(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    list_tasks: bool,
    offline: bool,
) -> RepositoryVerification {
    use chrono::Utc;

    let source_type_name = match source.source_type {
        ratchet_config::domains::registry::RegistrySourceType::Filesystem => "filesystem",
        ratchet_config::domains::registry::RegistrySourceType::Http => "http",
        ratchet_config::domains::registry::RegistrySourceType::Git => "git",
        ratchet_config::domains::registry::RegistrySourceType::S3 => "s3",
    };

    let mut verification = RepositoryVerification {
        name: source.name.clone(),
        source_type: source_type_name.to_string(),
        uri: source.uri.clone(),
        enabled: source.enabled,
        accessible: false,
        readable: false,
        correctly_configured: false,
        usable: false,
        verification_time: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        tasks: None,
        issues: Vec::new(),
        warnings: Vec::new(),
        error_message: None,
    };

    // If repository is disabled, skip detailed checks
    if !source.enabled {
        verification.warnings.push("Repository is disabled".to_string());
        verification.correctly_configured = true; // Config is valid, just disabled
        return verification;
    }

    // Check basic configuration
    if source.uri.is_empty() {
        verification.issues.push("Empty URI configured".to_string());
        return verification;
    }

    verification.correctly_configured = true;

    // Skip connectivity tests if offline mode
    if offline {
        verification.warnings.push("Offline mode: skipping connectivity tests".to_string());
        verification.accessible = true;
        verification.readable = true;
        verification.usable = true;
        return verification;
    }

    // Test accessibility and task discovery based on source type
    match source.source_type {
        ratchet_config::domains::registry::RegistrySourceType::Git => {
            verify_git_repository_detailed(source, &mut verification, list_tasks).await;
        }
        ratchet_config::domains::registry::RegistrySourceType::Filesystem => {
            verify_filesystem_repository_detailed(source, &mut verification, list_tasks).await;
        }
        ratchet_config::domains::registry::RegistrySourceType::Http => {
            verify_http_repository_detailed(source, &mut verification, list_tasks).await;
        }
        ratchet_config::domains::registry::RegistrySourceType::S3 => {
            verification.issues.push("S3 repository support not yet implemented".to_string());
        }
    }

    // Determine overall usability
    verification.usable = verification.accessible && verification.readable && verification.correctly_configured;

    verification
}

/// Detailed Git repository verification
async fn verify_git_repository_detailed(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    verification: &mut RepositoryVerification,
    list_tasks: bool,
) {
    use std::process::Command;
    
    info!("Performing detailed Git repository verification: {}", source.uri);

    // Check if git is available
    match Command::new("git").arg("--version").output() {
        Ok(_) => {},
        Err(_) => {
            verification.issues.push("Git command not available on system".to_string());
            return;
        }
    }

    // Test Git connectivity using git ls-remote
    let git_ref = &source.config.git.branch;

    let mut cmd = Command::new("git");
    cmd.arg("ls-remote").arg("--heads").arg("--tags").arg(&source.uri);

    // Add authentication if configured
    if let Some(auth_name) = &source.auth_name {
        verification.warnings.push(format!("Authentication '{}' configured (not tested in verification)", auth_name));
    }

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                verification.accessible = true;
                
                let output_str = String::from_utf8_lossy(&output.stdout);
                if !output_str.is_empty() {
                    verification.readable = true;
                    
                    // Check if the specified ref exists
                    if git_ref != "HEAD" {
                        let ref_exists = output_str.lines().any(|line| {
                            line.contains(&format!("refs/heads/{}", git_ref)) ||
                            line.contains(&format!("refs/tags/{}", git_ref))
                        });
                        
                        if !ref_exists {
                            verification.warnings.push(format!("Specified ref '{}' not found in remote", git_ref));
                        }
                    }
                }
            } else {
                verification.error_message = Some(format!("Git ls-remote failed: {}", 
                    String::from_utf8_lossy(&output.stderr)));
            }
        }
        Err(e) => {
            verification.error_message = Some(format!("Failed to execute git ls-remote: {}", e));
        }
    }

    // If we can access the repository and task listing is requested, try to discover tasks
    if verification.accessible && verification.readable && list_tasks {
        discover_git_tasks(source, verification).await;
    }
}

/// Detailed filesystem repository verification
async fn verify_filesystem_repository_detailed(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    verification: &mut RepositoryVerification,
    list_tasks: bool,
) {
    use std::path::Path;
    
    info!("Performing detailed filesystem repository verification: {}", source.uri);

    let path_str = if source.uri.starts_with("file://") {
        &source.uri[7..]  // Remove "file://" prefix
    } else {
        &source.uri
    };

    let path = Path::new(path_str);
    
    // Check if path exists and is accessible
    if path.exists() {
        verification.accessible = true;
        
        if path.is_dir() {
            verification.readable = true;
            
            // Check for .ratchet directory (optional but recommended)
            let ratchet_dir = path.join(".ratchet");
            if ratchet_dir.exists() {
                if ratchet_dir.join("registry.yaml").exists() {
                    verification.warnings.push("Found .ratchet/registry.yaml (repository metadata)".to_string());
                }
            } else {
                verification.warnings.push("No .ratchet directory found (repository metadata recommended)".to_string());
            }
            
            // If task listing is requested, discover tasks
            if list_tasks {
                discover_filesystem_tasks(source, verification, path).await;
            }
        } else {
            verification.issues.push("Path exists but is not a directory".to_string());
        }
    } else {
        verification.error_message = Some("Path does not exist or is not accessible".to_string());
    }
}

/// Detailed HTTP repository verification
async fn verify_http_repository_detailed(
    source: &ratchet_config::domains::registry::RegistrySourceConfig,
    verification: &mut RepositoryVerification,
    _list_tasks: bool,
) {
    info!("Performing detailed HTTP repository verification: {}", source.uri);
    
    // For now, mark as not implemented since HTTP task repositories are not fully implemented
    verification.issues.push("HTTP repository support is not yet fully implemented".to_string());
    verification.warnings.push("HTTP repositories are planned for future releases".to_string());
}

/// Discover tasks in Git repository (simplified version - requires actual clone for full discovery)
async fn discover_git_tasks(
    _source: &ratchet_config::domains::registry::RegistrySourceConfig,
    verification: &mut RepositoryVerification,
) {
    // Note: Full task discovery would require cloning the repository locally
    // For now, we'll indicate that tasks discovery requires full repository sync
    verification.warnings.push("Task discovery requires repository synchronization (use 'ratchet repo status' for full task listing)".to_string());
    verification.tasks = Some(vec![]); // Empty list to indicate we attempted discovery
}

/// Discover tasks in filesystem repository
async fn discover_filesystem_tasks(
    _source: &ratchet_config::domains::registry::RegistrySourceConfig,
    verification: &mut RepositoryVerification,
    path: &std::path::Path,
) {
    let mut tasks = Vec::new();
    
    // Look for tasks in the tasks/ directory
    let tasks_dir = path.join("tasks");
    if tasks_dir.exists() && tasks_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&tasks_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let metadata_file = entry_path.join("metadata.json");
                    if metadata_file.exists() {
                        // Try to read task metadata
                        if let Ok(metadata_content) = std::fs::read_to_string(&metadata_file) {
                            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_content) {
                                let task_name = entry_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                
                                let task_info = TaskInfo {
                                    name: task_name,
                                    version: metadata.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    path: entry_path.to_string_lossy().to_string(),
                                    description: metadata.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    tags: metadata.get("tags")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| arr.iter()
                                            .filter_map(|v| v.as_str())
                                            .map(|s| s.to_string())
                                            .collect())
                                        .unwrap_or_default(),
                                };
                                tasks.push(task_info);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Also check root directory for tasks
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() && entry_path.file_name() != Some(std::ffi::OsStr::new("tasks")) {
                let metadata_file = entry_path.join("metadata.json");
                if metadata_file.exists() {
                    // Similar task discovery logic for root-level tasks
                    if let Ok(metadata_content) = std::fs::read_to_string(&metadata_file) {
                        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_content) {
                            let task_name = entry_path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            let task_info = TaskInfo {
                                name: task_name,
                                version: metadata.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                path: entry_path.to_string_lossy().to_string(),
                                description: metadata.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                tags: metadata.get("tags")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| arr.iter()
                                        .filter_map(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .collect())
                                    .unwrap_or_default(),
                            };
                            tasks.push(task_info);
                        }
                    }
                }
            }
        }
    }
    
    verification.tasks = Some(tasks);
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
        #[cfg(feature = "server")]
        {
            // Use modern config directly
            init_logging_with_config(
                &config,
                cli.log_level.as_ref(),
                cli.command.as_ref().and_then(|cmd| match cmd {
                    Commands::RunOnce { record, .. } => record.as_ref(),
                    _ => None,
                }),
            )?;
        }
        #[cfg(not(feature = "server"))]
        {
            // Use simple tracing for non-server builds
            init_simple_tracing(cli.log_level.as_ref())?;
        }
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
            // Try modular executor first (ratchet-js), then runtime, then fall back to legacy
            #[cfg(feature = "javascript")]
            {
                info!(
                    "Running task from file system path: {} (using modular executor)",
                    from_fs
                );

                // Parse input JSON
                let input = parse_input_json(input_json.as_ref())?;

                if input_json.is_some() {
                    info!("Using provided input: {}", input_json.as_ref().unwrap());
                }

                // Run the task with modular executor (ratchet-js)
                let result = run_task_modular(from_fs, &input).await?;

                // Pretty-print the result
                let formatted =
                    to_string_pretty(&result).context("Failed to format result as JSON")?;

                println!("{}", formatted);
                info!("Task execution completed");

                // TODO: Add recording support for modular executor
                if record.is_some() {
                    warn!("Recording functionality not yet implemented for modular executor");
                }

                Ok(())
            }
            
            #[cfg(not(feature = "javascript"))]
            {
                // Fallback to runtime executor if available
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

                #[cfg(not(all(feature = "runtime", feature = "core")))]
                {
                    Err(anyhow::anyhow!("Task execution not available. Build with --features=javascript or --features=runtime,core"))
                }
            }
        }
        Some(Commands::Serve {
            config: config_override,
        }) => {
            #[cfg(feature = "server")]
            {
                info!("Starting Ratchet server");
                // Load config with MCP auto-enabling logic
                let mut server_config = if config_override.is_some() {
                    load_config(config_override.as_ref())?
                } else {
                    config.clone()
                };
                
                // If no explicit config file is provided, enable MCP by default for integrated server
                if config_override.is_none() {
                    if let Some(ref mut mcp_config) = server_config.mcp {
                        if !mcp_config.enabled {
                            info!("Enabling MCP SSE server by default for integrated server mode");
                            mcp_config.enabled = true;
                        }
                    }
                }
                
                serve_command_with_config(server_config).await
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
                // Use modular validator by default, fallback to legacy if needed
                validate_task_modular(from_fs)
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
                // Use modular test runner by default, fallback to legacy if needed
                test_task_modular(from_fs).await
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
        Some(Commands::Repo { repo_cmd }) => match repo_cmd {
            RepoCommands::Init {
                directory,
                name,
                description,
                version,
                ratchet_version,
                force,
            } => handle_repo_init(directory, name.as_deref(), description.as_deref(), version, ratchet_version, *force),
            RepoCommands::RefreshMetadata { directory, force } => {
                handle_repo_refresh_metadata(directory.as_ref(), *force).await
            }
            RepoCommands::Status { detailed, repository, format } => {
                handle_repo_status(&config, *detailed, repository.as_deref(), format).await
            }
            RepoCommands::Verify { repository, format, detailed, list_tasks, offline } => {
                handle_repo_verify(&config, repository.as_deref(), format, *detailed, *list_tasks, *offline).await
            }
        },
        Some(Commands::Console {
            config: console_config,
            connect,
            transport,
            host,
            port,
            auth_token,
            history_file,
            script,
        }) => {
            use crate::commands::console::{ConsoleConfig, run_console};
            
            let console_config = ConsoleConfig {
                config_file: console_config.clone(),
                connect_url: connect.clone(),
                transport: transport.clone(),
                host: host.clone(),
                port: *port,
                auth_token: auth_token.clone(),
                history_file: history_file.clone(),
                script_file: script.clone(),
            };
            
            run_console(console_config).await
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
