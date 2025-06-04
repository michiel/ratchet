//! CLI argument parsing definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,

    /// Set the log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", global = true)]
    pub log_level: Option<String>,

    /// Run as worker process (internal use)
    #[arg(long, hide = true)]
    pub worker: bool,
    
    /// Worker ID (used with --worker)
    #[arg(long, value_name = "ID", hide = true)]
    pub worker_id: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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
    
    /// Start the Ratchet server
    Serve {
        /// Path to configuration file
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
    
    /// Start the MCP (Model Context Protocol) server
    McpServe {
        /// Path to configuration file
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        
        /// Transport type: stdio, sse
        #[arg(long, value_name = "TYPE", default_value = "stdio")]
        transport: String,
        
        /// Host to bind to (for SSE transport)
        #[arg(long, value_name = "HOST", default_value = "127.0.0.1")]
        host: String,
        
        /// Port to bind to (for SSE transport)
        #[arg(long, value_name = "PORT", default_value = "8090")]
        port: u16,
    },
    
    /// Validate a task
    Validate {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },
    
    /// Test a task
    Test {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
    },
    
    /// Replay a recorded task execution
    Replay {
        /// Path to the file system resource
        #[arg(long, value_name = "STRING")]
        from_fs: String,
        
        /// Path to the recording directory
        #[arg(long, value_name = "PATH")]
        recording: Option<PathBuf>,
    },
    
    /// Generate code templates
    Generate {
        #[command(subcommand)]
        generate_cmd: GenerateCommands,
    },
}

#[derive(Subcommand)]
pub enum GenerateCommands {
    /// Generate a new task template
    Task {
        /// Directory path where to generate the task
        #[arg(long, value_name = "PATH")]
        path: PathBuf,
        
        /// Task label
        #[arg(long, value_name = "STRING")]
        label: Option<String>,
        
        /// Task description
        #[arg(long, value_name = "STRING")]
        description: Option<String>,
        
        /// Task version
        #[arg(long, value_name = "STRING", default_value = "1.0.0")]
        version: Option<String>,
    },
}