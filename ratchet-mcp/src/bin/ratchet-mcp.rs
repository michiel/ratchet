use anyhow::Result;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use ratchet_lib::{config::RatchetConfig, execution::ProcessTaskExecutor};
use ratchet_mcp::{
    config::McpConfig,
    server::{adapter::RatchetMcpAdapter, McpServer},
};
use ratchet_storage::seaorm::{
    config::DatabaseConfig,
    connection::DatabaseConnection,
    repositories::{task_repository::TaskRepository, RepositoryFactory},
};

#[derive(Parser)]
#[command(name = "ratchet-mcp")]
#[command(about = "Ratchet Model Context Protocol (MCP) server")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server
    Serve {
        /// Transport type to use
        #[arg(short, long, default_value = "stdio")]
        transport: TransportChoice,

        /// Server address for SSE transport
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port for SSE transport
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// List available tools
    Tools,

    /// Test connection to Ratchet backend
    Test,

    /// Validate configuration file
    ValidateConfig,
}

#[derive(Clone, ValueEnum)]
enum TransportChoice {
    /// Standard input/output (for LLM integration)
    Stdio,
    /// Server-sent events over HTTP
    Sse,
}

impl From<TransportChoice> for ratchet_mcp::SimpleTransportType {
    fn from(choice: TransportChoice) -> Self {
        match choice {
            TransportChoice::Stdio => ratchet_mcp::SimpleTransportType::Stdio,
            TransportChoice::Sse => ratchet_mcp::SimpleTransportType::Sse,
        }
    }
}

/// Convert ratchet-storage RepositoryFactory to ratchet_lib RepositoryFactory
async fn convert_to_legacy_repository_factory(
    storage_config: DatabaseConfig,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();

    match cli.command {
        Commands::Serve {
            transport,
            host,
            port,
        } => serve_command(cli.config.as_deref(), transport.into(), &host, port).await,
        Commands::Tools => tools_command().await,
        Commands::Test => test_command(cli.config.as_deref()).await,
        Commands::ValidateConfig => validate_config_command(cli.config.as_deref()).await,
    }
}

async fn serve_command(
    config_path: Option<&str>,
    transport: ratchet_mcp::SimpleTransportType,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Ratchet MCP server with {:?} transport", transport);

    // Load configuration
    let config = load_config(config_path).await?;

    // Initialize database connection (using default since RatchetConfig doesn't have database field)
    let db_config = DatabaseConfig::default();
    let db = DatabaseConnection::new(db_config.clone()).await?;

    // Run migrations
    db.migrate().await?;

    // Initialize repositories using repository factory
    let repo_factory = RepositoryFactory::new(db.clone());
    let task_repository = Arc::new(repo_factory.task_repository());
    let execution_repository = Arc::new(repo_factory.execution_repository());

    // Initialize task executor (still needs legacy format)
    let legacy_repo_factory = convert_to_legacy_repository_factory(db_config.clone()).await?;
    let executor = Arc::new(ProcessTaskExecutor::new(legacy_repo_factory, config.clone()).await?);

    // Create MCP adapter
    let adapter = RatchetMcpAdapter::new(executor, task_repository, execution_repository);

    // Create MCP server
    let mcp_config = McpConfig {
        transport_type: transport.clone(),
        host: host.to_string(),
        port,
        ..Default::default()
    };

    let mut server = McpServer::with_adapter(mcp_config, adapter).await?;

    // Start server based on transport type
    match transport {
        ratchet_mcp::SimpleTransportType::Stdio => {
            tracing::info!("MCP server ready on stdio - waiting for LLM connections");
            server.run_stdio().await?;
        }
        ratchet_mcp::SimpleTransportType::Sse => {
            tracing::info!("MCP server starting on http://{}:{}", host, port);
            server.run_sse().await?;
        }
    }

    tracing::info!("MCP server stopped");
    Ok(())
}

async fn tools_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available MCP Tools:");
    println!();

    // List all available tools that the MCP server exposes
    let tools = [
        (
            "ratchet.execute_task",
            "Execute a Ratchet task with input data",
        ),
        ("ratchet.list_tasks", "List all available tasks"),
        (
            "ratchet.get_task_info",
            "Get detailed information about a specific task",
        ),
        (
            "ratchet.get_execution_status",
            "Get status of a task execution",
        ),
        (
            "ratchet.get_execution_logs",
            "Get logs from a task execution",
        ),
        (
            "ratchet.cancel_execution",
            "Cancel a running task execution",
        ),
        ("ratchet.list_executions", "List recent task executions"),
        (
            "ratchet.get_execution_trace",
            "Get detailed execution trace for debugging",
        ),
        (
            "ratchet.analyze_execution_error",
            "Analyze execution errors with suggestions",
        ),
        (
            "ratchet.validate_task_input",
            "Validate input data against task schema",
        ),
    ];

    for (name, description) in tools {
        println!("  {} - {}", name, description);
    }

    println!();
    println!("Use these tools in your LLM to interact with Ratchet task execution.");

    Ok(())
}

async fn test_command(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Testing connection to Ratchet backend");

    // Load configuration
    let config = match load_config(config_path).await {
        Ok(config) => {
            tracing::info!("✓ Configuration loaded successfully");
            config
        }
        Err(e) => {
            tracing::error!("✗ Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    // Test database connection
    let db_config = DatabaseConfig::default();
    let db = match DatabaseConnection::new(db_config.clone()).await {
        Ok(db) => {
            tracing::info!("✓ Database connection successful");
            db
        }
        Err(e) => {
            tracing::error!("✗ Database connection failed: {}", e);
            return Err(e.into());
        }
    };

    // Test repository access
    let task_repository = TaskRepository::new(db.clone());
    match task_repository.count().await {
        Ok(count) => {
            tracing::info!("✓ Task repository accessible ({} tasks found)", count);
        }
        Err(e) => {
            tracing::error!("✗ Task repository access failed: {}", e);
            return Err(e.into());
        }
    }

    // Test executor initialization
    let repositories = RepositoryFactory::new(db.clone());
    let legacy_repo_factory_test = convert_to_legacy_repository_factory(db_config.clone()).await?;
    match ProcessTaskExecutor::new(legacy_repo_factory_test, config.clone()).await {
        Ok(_) => {
            tracing::info!("✓ Task executor initialized successfully");
        }
        Err(e) => {
            tracing::error!("✗ Task executor initialization failed: {}", e);
            return Err(e.into());
        }
    }

    tracing::info!("All systems operational - MCP server ready to start");
    Ok(())
}

async fn validate_config_command(
    config_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Validating MCP configuration");

    let config_path = config_path.ok_or("Configuration file path is required for validation")?;

    // Load and validate MCP config
    match McpConfig::from_file(config_path).await {
        Ok(config) => {
            tracing::info!("✓ Configuration file loaded successfully");
            tracing::info!("  Transport: {:?}", config.transport_type);
            tracing::info!("  Host: {}", config.host);
            tracing::info!("  Port: {}", config.port);
            tracing::info!("  Auth: {:?}", config.auth);

            match config.validate() {
                Ok(()) => {
                    tracing::info!("✓ Configuration is valid");
                    println!("Configuration validation successful!");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("✗ Configuration validation failed: {}", e);
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            tracing::error!("✗ Failed to load configuration: {}", e);
            Err(e.into())
        }
    }
}

async fn load_config(
    config_path: Option<&str>,
) -> Result<RatchetConfig, Box<dyn std::error::Error>> {
    match config_path {
        Some(path) => {
            tracing::info!("Loading configuration from: {}", path);
            RatchetConfig::from_file(path).map_err(Into::into)
        }
        None => {
            tracing::info!("Using default configuration");
            Ok(RatchetConfig::default())
        }
    }
}
