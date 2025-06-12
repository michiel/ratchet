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

    /// Configuration management commands
    Config {
        #[command(subcommand)]
        config_cmd: ConfigCommands,
    },

    /// Repository management commands
    Repo {
        #[command(subcommand)]
        repo_cmd: RepoCommands,
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

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Validate a configuration file
    Validate {
        /// Path to the configuration file
        #[arg(long, value_name = "PATH")]
        config_file: PathBuf,
    },

    /// Generate sample configuration files
    Generate {
        /// Configuration type: dev, production, enterprise, minimal, claude
        #[arg(long, value_name = "TYPE", default_value = "dev")]
        config_type: String,

        /// Output file path
        #[arg(long, value_name = "PATH")]
        output: PathBuf,

        /// Overwrite existing file
        #[arg(long)]
        force: bool,
    },

    /// Show current configuration in use
    Show {
        /// Path to configuration file (optional, uses default loading logic)
        #[arg(long, value_name = "PATH")]
        config_file: Option<PathBuf>,

        /// Show MCP configuration only
        #[arg(long)]
        mcp_only: bool,

        /// Output format: yaml, json
        #[arg(long, value_name = "FORMAT", default_value = "yaml")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum RepoCommands {
    /// Initialize a new task repository
    Init {
        /// Directory path where to initialize the repository
        #[arg(value_name = "DIR")]
        directory: PathBuf,

        /// Repository name
        #[arg(long, value_name = "STRING")]
        name: Option<String>,

        /// Repository description
        #[arg(long, value_name = "STRING")]
        description: Option<String>,

        /// Repository version
        #[arg(long, value_name = "STRING", default_value = "1.0.0")]
        version: String,

        /// Minimum required ratchet version
        #[arg(long, value_name = "STRING", default_value = ">=0.6.0")]
        ratchet_version: String,

        /// Force initialization even if directory is not empty
        #[arg(long)]
        force: bool,
    },

    /// Refresh repository metadata and index
    RefreshMetadata {
        /// Directory path of the repository (defaults to current directory)
        #[arg(value_name = "DIR")]
        directory: Option<PathBuf>,

        /// Force regeneration of all metadata
        #[arg(long)]
        force: bool,
    },

    /// Show status of configured task repositories
    Status {
        /// Show detailed status for all repositories
        #[arg(long)]
        detailed: bool,

        /// Show status for specific repository by name
        #[arg(long, value_name = "NAME")]
        repository: Option<String>,

        /// Output format: table, json, yaml
        #[arg(long, value_name = "FORMAT", default_value = "table")]
        format: String,
    },

    /// Verify configured repositories accessibility and list available tasks
    Verify {
        /// Verify specific repository by name
        #[arg(long, value_name = "NAME")]
        repository: Option<String>,

        /// Output format: table, json, yaml
        #[arg(long, value_name = "FORMAT", default_value = "table")]
        format: String,

        /// Show detailed verification information
        #[arg(long)]
        detailed: bool,

        /// List all available tasks in each repository
        #[arg(long)]
        list_tasks: bool,

        /// Skip connectivity tests (only validate configuration)
        #[arg(long)]
        offline: bool,
    },
}
