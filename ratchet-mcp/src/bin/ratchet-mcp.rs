use anyhow::Result;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use ratchet_execution::{ProcessTaskExecutor, ProcessExecutorConfig};
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
    _config_path: Option<&str>,
    transport: ratchet_mcp::SimpleTransportType,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Ratchet MCP server with {:?} transport", transport);

    // We don't need RatchetConfig for the new executor

    // Initialize database connection (using default since RatchetConfig doesn't have database field)
    let db_config = DatabaseConfig::default();
    let db = DatabaseConnection::new(db_config.clone()).await?;

    // Run migrations
    db.migrate().await?;

    // Initialize repositories using repository factory
    let repo_factory = RepositoryFactory::new(db.clone());
    let task_repository = Arc::new(repo_factory.task_repository());
    let execution_repository = Arc::new(repo_factory.execution_repository());

    // Initialize task executor using the new API from ratchet-execution
    let executor_config = ProcessExecutorConfig {
        worker_count: 4,
        task_timeout_seconds: 300,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let executor = Arc::new(ProcessTaskExecutor::new(executor_config));

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

async fn test_command(_config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Testing connection to Ratchet backend");

    // Configuration is not needed for the new executor
    tracing::info!("✓ Using default executor configuration");

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
    let _repositories = RepositoryFactory::new(db.clone());
    let executor_config = ProcessExecutorConfig {
        worker_count: 2,
        task_timeout_seconds: 60,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let _executor = ProcessTaskExecutor::new(executor_config);
    tracing::info!("✓ Task executor initialized successfully");

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

