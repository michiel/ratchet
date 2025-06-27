//! Base command trait for unified console command handling

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use futures::Stream;
use tokio::sync::mpsc;

use super::enhanced_mcp_client::EnhancedMcpClient;

/// Command arguments parsed from user input
#[derive(Debug, Clone)]
pub struct CommandArgs {
    pub action: String,
    pub positional: Vec<String>,
    pub flags: HashMap<String, Option<String>>,
    pub raw_args: Vec<String>,
}

/// Rich output formatting options
pub enum CommandOutput {
    /// Simple text output
    Text(String),
    /// JSON output with optional pretty printing
    Json(Value),
    /// Table output with headers and rows
    Table(Table),
    /// Streaming output for real-time updates
    Stream(Pin<Box<dyn Stream<Item = CommandOutput> + Send>>),
    /// Interactive dashboard mode
    Dashboard(DashboardState),
    /// Success message with optional data
    Success { message: String, data: Option<Value> },
    /// Error message with context
    Error { message: String, context: Option<Value> },
    /// Progress indicator
    Progress { message: String, percentage: Option<f64> },
}

impl std::fmt::Debug for CommandOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandOutput::Text(text) => f.debug_tuple("Text").field(text).finish(),
            CommandOutput::Json(value) => f.debug_tuple("Json").field(value).finish(),
            CommandOutput::Table(table) => f.debug_tuple("Table").field(table).finish(),
            CommandOutput::Stream(_) => f.debug_tuple("Stream").field(&"<stream>").finish(),
            CommandOutput::Dashboard(dash) => f.debug_tuple("Dashboard").field(dash).finish(),
            CommandOutput::Success { message, data } => f.debug_struct("Success")
                .field("message", message)
                .field("data", data)
                .finish(),
            CommandOutput::Error { message, context } => f.debug_struct("Error")
                .field("message", message)
                .field("context", context)
                .finish(),
            CommandOutput::Progress { message, percentage } => f.debug_struct("Progress")
                .field("message", message)
                .field("percentage", percentage)
                .finish(),
        }
    }
}

/// Table structure for tabular output
#[derive(Debug, Clone)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub title: Option<String>,
}

/// Dashboard state for interactive monitoring
#[derive(Debug, Clone)]
pub struct DashboardState {
    pub execution_panel: ExecutionPanel,
    pub metrics_panel: MetricsPanel,
    pub logs_panel: LogsPanel,
    pub workers_panel: WorkersPanel,
}

#[derive(Debug, Clone)]
pub struct ExecutionPanel {
    pub active_executions: Vec<Value>,
    pub recent_executions: Vec<Value>,
    pub total_executions: u64,
}

#[derive(Debug, Clone)]
pub struct MetricsPanel {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub queue_size: u64,
    pub worker_count: u64,
}

#[derive(Debug, Clone)]
pub struct LogsPanel {
    pub recent_logs: Vec<String>,
    pub log_level: String,
    pub error_count: u64,
}

#[derive(Debug, Clone)]
pub struct WorkersPanel {
    pub active_workers: u64,
    pub idle_workers: u64,
    pub total_workers: u64,
    pub worker_details: Vec<Value>,
}

/// Base trait for all console commands
#[async_trait]
pub trait ConsoleCommand: Send + Sync {
    /// Execute the command with MCP client integration
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput>;

    /// Get completion hints for partial input
    fn completion_hints(&self, partial: &str) -> Vec<String>;

    /// Get help text for the command
    fn help_text(&self) -> &'static str;

    /// Get command usage examples
    fn usage_examples(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Check if command requires MCP connection
    fn requires_connection(&self) -> bool {
        true
    }

    /// Get command category for organization
    fn category(&self) -> &'static str {
        "general"
    }

    /// Get command aliases
    fn aliases(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Validate command arguments before execution
    fn validate_args(&self, _args: &CommandArgs) -> Result<()> {
        Ok(())
    }
}

/// Command wrapper enum to avoid dyn trait issues with async traits
#[derive(Clone)]
pub enum BoxedCommand {
    EnhancedTask(std::sync::Arc<crate::commands::console::commands::EnhancedTaskCommand>),
    Template(std::sync::Arc<crate::commands::console::commands::TemplateCommand>),
}

impl BoxedCommand {
    /// Execute the command
    pub async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.execute(args, mcp_client).await,
            BoxedCommand::Template(cmd) => cmd.execute(args, mcp_client).await,
        }
    }

    /// Get completion hints
    pub fn completion_hints(&self, partial: &str) -> Vec<String> {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.completion_hints(partial),
            BoxedCommand::Template(cmd) => cmd.completion_hints(partial),
        }
    }

    /// Get help text
    pub fn help_text(&self) -> &'static str {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.help_text(),
            BoxedCommand::Template(cmd) => cmd.help_text(),
        }
    }

    /// Get usage examples
    pub fn usage_examples(&self) -> Vec<&'static str> {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.usage_examples(),
            BoxedCommand::Template(cmd) => cmd.usage_examples(),
        }
    }

    /// Check if command requires connection
    pub fn requires_connection(&self) -> bool {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.requires_connection(),
            BoxedCommand::Template(cmd) => cmd.requires_connection(),
        }
    }

    /// Get command category
    pub fn category(&self) -> &'static str {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.category(),
            BoxedCommand::Template(cmd) => cmd.category(),
        }
    }

    /// Get command aliases
    pub fn aliases(&self) -> Vec<&'static str> {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.aliases(),
            BoxedCommand::Template(cmd) => cmd.aliases(),
        }
    }

    /// Validate command arguments
    pub fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        match self {
            BoxedCommand::EnhancedTask(cmd) => cmd.validate_args(args),
            BoxedCommand::Template(cmd) => cmd.validate_args(args),
        }
    }
}

/// Helper trait for commands that support streaming output
#[async_trait]
pub trait StreamingCommand: ConsoleCommand {
    /// Execute command with streaming output
    async fn execute_stream(
        &self,
        args: CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<impl Stream<Item = CommandOutput> + Send>;
}

/// Helper trait for commands that support interactive modes
#[async_trait]
pub trait InteractiveCommand: ConsoleCommand {
    /// Start interactive mode for the command
    async fn interactive_mode(
        &self,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput>;

    /// Handle interactive input
    async fn handle_interactive_input(
        &self,
        input: &str,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput>;
}

/// Utility functions for command output formatting
impl CommandOutput {
    /// Create a simple text output
    pub fn text(message: impl Into<String>) -> Self {
        CommandOutput::Text(message.into())
    }

    /// Create a JSON output
    pub fn json(value: Value) -> Self {
        CommandOutput::Json(value)
    }

    /// Create a success message
    pub fn success(message: impl Into<String>) -> Self {
        CommandOutput::Success {
            message: message.into(),
            data: None,
        }
    }

    /// Create a success message with data
    pub fn success_with_data(message: impl Into<String>, data: Value) -> Self {
        CommandOutput::Success {
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create an error message
    pub fn error(message: impl Into<String>) -> Self {
        CommandOutput::Error {
            message: message.into(),
            context: None,
        }
    }

    /// Create an error message with context
    pub fn error_with_context(message: impl Into<String>, context: Value) -> Self {
        CommandOutput::Error {
            message: message.into(),
            context: Some(context),
        }
    }

    /// Create a progress indicator
    pub fn progress(message: impl Into<String>, percentage: Option<f64>) -> Self {
        CommandOutput::Progress {
            message: message.into(),
            percentage,
        }
    }

    /// Create a table output
    pub fn table(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        CommandOutput::Table(Table {
            headers,
            rows,
            title: None,
        })
    }

    /// Create a table output with title
    pub fn table_with_title(
        title: impl Into<String>,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    ) -> Self {
        CommandOutput::Table(Table {
            headers,
            rows,
            title: Some(title.into()),
        })
    }

    /// Create a streaming output
    pub fn stream(stream: Pin<Box<dyn Stream<Item = CommandOutput> + Send>>) -> Self {
        CommandOutput::Stream(stream)
    }
}

impl Table {
    /// Create a new table
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            headers,
            rows,
            title: None,
        }
    }

    /// Create a table with title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Get column count
    pub fn column_count(&self) -> usize {
        self.headers.len()
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl CommandArgs {
    /// Create new command args
    pub fn new(action: String, positional: Vec<String>, flags: HashMap<String, Option<String>>) -> Self {
        let raw_args = vec![action.clone()]
            .into_iter()
            .chain(positional.clone())
            .chain(flags.iter().flat_map(|(k, v)| {
                if let Some(val) = v {
                    vec![format!("--{}", k), val.clone()]
                } else {
                    vec![format!("--{}", k)]
                }
            }))
            .collect();

        Self {
            action,
            positional,
            flags,
            raw_args,
        }
    }

    /// Get flag value as string
    pub fn get_flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|v| v.as_deref())
    }

    /// Check if flag is present (boolean flag)
    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    /// Get positional argument by index
    pub fn get_positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|s| s.as_str())
    }

    /// Get required positional argument with error
    pub fn require_positional(&self, index: usize, name: &str) -> Result<&str> {
        self.get_positional(index)
            .ok_or_else(|| anyhow::anyhow!("Missing required argument: {}", name))
    }

    /// Parse JSON from flag or positional argument
    pub fn parse_json(&self, source: &str) -> Result<Value> {
        serde_json::from_str(source)
            .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))
    }

    /// Get flag as boolean with default
    pub fn get_bool_flag(&self, name: &str, default: bool) -> bool {
        self.get_flag(name)
            .map(|v| v.parse().unwrap_or(default))
            .unwrap_or(default)
    }

    /// Get flag as number with default
    pub fn get_number_flag<T>(&self, name: &str, default: T) -> T
    where
        T: std::str::FromStr + Copy,
    {
        self.get_flag(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}