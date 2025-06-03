use std::io::{self, BufRead};
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use ratchet_mcp::{
    server::{McpServer, adapter::RatchetMcpAdapter}, 
    transport::{TransportType, stdio::StdioTransport},
    config::McpConfig,
    error::McpResult,
};
use ratchet_lib::{
    execution::ProcessTaskExecutor,
    database::{
        connection::DatabaseConnection,
        repositories::{
            task_repository::TaskRepository,
            execution_repository::ExecutionRepository,
        },
    },
    config::RatchetConfig,
};
use tokio::io::{AsyncBufReadExt, BufReader};

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
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
        
        /// Server port for SSE transport
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    
    /// List available tools
    Tools,
    
    /// Test connection to Ratchet backend
    Test,
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
        Commands::Serve { transport, host, port } => {
            serve_command(cli.config.as_deref(), transport.into(), &host, port).await
        }
        Commands::Tools => {
            tools_command().await
        }
        Commands::Test => {
            test_command(cli.config.as_deref()).await
        }
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
    
    // Initialize database connection
    let db = DatabaseConnection::new(&config.database).await?;
    
    // Initialize repositories
    let task_repository = Arc::new(TaskRepository::new(db.clone()));
    let execution_repository = Arc::new(ExecutionRepository::new(db.clone()));
    
    // Initialize task executor
    let executor = Arc::new(ProcessTaskExecutor::new(config.execution.clone())?);
    
    // Create MCP adapter
    let adapter = RatchetMcpAdapter::new(
        executor,
        task_repository,
        execution_repository,
    );
    
    // Create MCP server
    let mcp_config = McpConfig {
        transport_type: transport,
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
        ("ratchet.execute_task", "Execute a Ratchet task with input data"),
        ("ratchet.list_tasks", "List all available tasks"),
        ("ratchet.get_task_info", "Get detailed information about a specific task"),
        ("ratchet.get_execution_status", "Get status of a task execution"),
        ("ratchet.get_execution_logs", "Get logs from a task execution"),
        ("ratchet.cancel_execution", "Cancel a running task execution"),
        ("ratchet.list_executions", "List recent task executions"),
        ("ratchet.get_execution_trace", "Get detailed execution trace for debugging"),
        ("ratchet.analyze_execution_error", "Analyze execution errors with suggestions"),
        ("ratchet.validate_task_input", "Validate input data against task schema"),
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
    let db = match DatabaseConnection::new(&config.database).await {
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
    match task_repository.count_all().await {
        Ok(count) => {
            tracing::info!("✓ Task repository accessible ({} tasks found)", count);
        }
        Err(e) => {
            tracing::error!("✗ Task repository access failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Test executor initialization
    match ProcessTaskExecutor::new(config.execution.clone()) {
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

async fn load_config(config_path: Option<&str>) -> Result<RatchetConfig, Box<dyn std::error::Error>> {
    match config_path {
        Some(path) => {
            tracing::info!("Loading configuration from: {}", path);
            RatchetConfig::from_file(path).await.map_err(Into::into)
        }
        None => {
            tracing::info!("Using default configuration");
            Ok(RatchetConfig::default())
        }
    }
}