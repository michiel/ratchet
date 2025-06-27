//! REPL (Read-Eval-Print Loop) implementation for Ratchet console

use anyhow::Result;
use colored::*;
use regex;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::{CmdKind, Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator};
use rustyline::Result as RustylineResult;
use rustyline::{Context, Editor, Helper};
use std::collections::HashMap;
use std::path::PathBuf;

use super::{
    executor::CommandExecutor, 
    formatter::OutputFormatter, 
    parser::CommandParser, 
    ConsoleConfig,
    enhanced_mcp_client::EnhancedMcpClient,
    command_registry::CommandRegistry,
};

/// Ratchet command completer for tab completion
struct RatchetHelper {
    completer: RatchetCompleter,
    hinter: HistoryHinter,
    validator: MatchingBracketValidator,
    highlighter: MatchingBracketHighlighter,
}

impl Helper for RatchetHelper {}

impl Completer for RatchetHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for RatchetHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Validator for RatchetHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        self.validator.validate(ctx)
    }
}

impl Highlighter for RatchetHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, default: bool) -> std::borrow::Cow<'b, str> {
        self.highlighter.highlight_prompt(prompt, default)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        self.highlighter.highlight_hint(hint)
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }
}

/// Custom completer for Ratchet console commands
struct RatchetCompleter {
    filename_completer: FilenameCompleter,
}

impl RatchetCompleter {
    fn new() -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
        }
    }

    fn get_command_categories() -> Vec<&'static str> {
        vec![
            "repo",
            "task",
            "execution",
            "job",
            "server",
            "db",
            "health",
            "stats",
            "monitor",
            "mcp",
            "help",
            "help-extended",
            "exit",
            "quit",
            "clear",
            "history",
            "set",
            "unset",
            "vars",
            "env",
            "source",
            "connect",
            "disconnect",
        ]
    }

    fn get_repo_commands() -> Vec<&'static str> {
        vec!["list", "add", "remove", "refresh", "status", "verify"]
    }

    fn get_task_commands() -> Vec<&'static str> {
        vec!["list", "show", "enable", "disable", "execute"]
    }

    fn get_execution_commands() -> Vec<&'static str> {
        vec!["list", "show"]
    }

    fn get_job_commands() -> Vec<&'static str> {
        vec!["list", "clear", "pause", "resume"]
    }

    fn get_server_commands() -> Vec<&'static str> {
        vec!["status", "workers", "metrics"]
    }

    fn get_db_commands() -> Vec<&'static str> {
        vec!["status", "migrate", "stats"]
    }
}

impl Completer for RatchetCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_cursor = &line[..pos];
        let tokens: Vec<&str> = line_to_cursor.split_whitespace().collect();

        let mut candidates = Vec::new();
        let start: usize;

        if tokens.is_empty() || (tokens.len() == 1 && !line_to_cursor.ends_with(' ')) {
            // Complete command categories
            start = line_to_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
            let prefix = &line_to_cursor[start..];

            for category in Self::get_command_categories() {
                if category.starts_with(prefix) {
                    candidates.push(Pair {
                        display: category.to_string(),
                        replacement: category.to_string(),
                    });
                }
            }
        } else if tokens.len() == 2 || (tokens.len() == 1 && line_to_cursor.ends_with(' ')) {
            // Complete command actions
            let category = tokens[0];
            start = line_to_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
            let prefix = if tokens.len() == 2 { tokens[1] } else { "" };

            let actions = match category {
                "repo" => Self::get_repo_commands(),
                "task" => Self::get_task_commands(),
                "execution" => Self::get_execution_commands(),
                "job" => Self::get_job_commands(),
                "server" => Self::get_server_commands(),
                "db" => Self::get_db_commands(),
                _ => Vec::new(),
            };

            for action in actions {
                if action.starts_with(prefix) {
                    candidates.push(Pair {
                        display: action.to_string(),
                        replacement: action.to_string(),
                    });
                }
            }
        } else if line_to_cursor.contains("source ") {
            // Complete filenames for source command
            return self.filename_completer.complete(line, pos, _ctx);
        } else {
            // Default start position
            start = line_to_cursor.rfind(' ').map(|i| i + 1).unwrap_or(0);
        }

        Ok((start, candidates))
    }
}

impl Default for RatchetHelper {
    fn default() -> Self {
        Self {
            completer: RatchetCompleter::new(),
            hinter: HistoryHinter {},
            validator: MatchingBracketValidator::new(),
            highlighter: MatchingBracketHighlighter::new(),
        }
    }
}

/// Main console REPL implementation
pub struct RatchetConsole {
    config: ConsoleConfig,
    editor: Editor<RatchetHelper, rustyline::history::FileHistory>,
    parser: CommandParser,
    executor: CommandExecutor,
    formatter: OutputFormatter,
    variables: HashMap<String, String>,
    running: bool,
    // Enhanced components for Phase 1
    enhanced_mcp_client: EnhancedMcpClient,
    command_registry: CommandRegistry,
}

impl RatchetConsole {
    /// Create a new console instance
    pub async fn new(config: ConsoleConfig) -> Result<Self> {
        let mut editor = Editor::new()?;
        editor.set_helper(Some(RatchetHelper::default()));

        // Load history file if specified
        if let Some(history_file) = &config.history_file {
            let _ = editor.load_history(history_file);
        } else {
            // Use default history file location
            if let Some(home) = dirs::home_dir() {
                let default_history = home.join(".ratchet_history");
                let _ = editor.load_history(&default_history);
            }
        }

        let parser = CommandParser::new();
        let executor = CommandExecutor::new(&config).await?;
        let formatter = OutputFormatter::new();

        // Initialize enhanced components
        let enhanced_mcp_client = EnhancedMcpClient::new(config.clone());
        let command_registry = CommandRegistry::new();

        Ok(Self {
            config,
            editor,
            parser,
            executor,
            formatter,
            variables: HashMap::new(),
            running: false,
            enhanced_mcp_client,
            command_registry,
        })
    }

    /// Start the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        self.show_banner().await?;
        self.running = true;

        // Execute startup script if provided
        if let Some(script_file) = self.config.script_file.clone() {
            self.execute_script(&script_file).await?;
        }

        while self.running {
            match self.read_command().await {
                Ok(input) => {
                    if let Err(e) = self.process_command(&input).await {
                        self.formatter.print_error(&format!("Error: {}", e));
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("Use 'exit' or Ctrl+D to quit");
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    break;
                }
                Err(e) => {
                    self.formatter.print_error(&format!("Input error: {}", e));
                    break;
                }
            }
        }

        self.save_history().await?;
        self.formatter.print_info("Goodbye!");
        Ok(())
    }

    /// Show the console banner
    async fn show_banner(&mut self) -> Result<()> {
        println!("{}", "Ratchet Console v0.6.0".bright_cyan().bold());

        // Show connection status without failing on connection errors
        match self.executor.connect().await {
            Ok(info) => {
                println!("Connected to: {}", info.bright_green());
                match self.executor.check_health().await {
                    Ok(health) => {
                        self.formatter.print_success(&format!("Server status: {}", health));
                    }
                    Err(e) => {
                        self.formatter.print_warning(&format!("Health check failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.formatter.print_warning(&format!("Connection failed: {}", e));
                self.formatter
                    .print_info("Console running in offline mode. Use 'connect' to retry connection.");
            }
        }

        // Try to connect enhanced MCP client as well
        match self.enhanced_mcp_client.connect().await {
            Ok(info) => {
                self.formatter.print_success(&format!("Enhanced MCP client connected: {}", info));
                
                // Show enhanced capabilities
                if let Some(capabilities) = self.enhanced_mcp_client.get_capabilities() {
                    if self.enhanced_mcp_client.supports_streaming() {
                        self.formatter.print_info("✓ Streaming support enabled");
                    }
                    if self.enhanced_mcp_client.supports_batch() {
                        self.formatter.print_info("✓ Batch operations enabled");
                    }
                }
            }
            Err(e) => {
                self.formatter.print_warning(&format!("Enhanced MCP client connection failed: {}", e));
                self.formatter.print_info("Enhanced commands may not be available.");
            }
        }

        println!(
            "Type '{}' for available commands, '{}' to quit",
            "help".bright_yellow(),
            "exit".bright_yellow()
        );
        println!();
        Ok(())
    }

    /// Read a command from the user
    async fn read_command(&mut self) -> RustylineResult<String> {
        let prompt = self.get_prompt();
        self.editor.readline(&prompt)
    }

    /// Get the current prompt string
    fn get_prompt(&self) -> String {
        if self.executor.is_connected() {
            "ratchet> ".bright_green().to_string()
        } else {
            "ratchet> ".bright_red().to_string()
        }
    }

    /// Process a single command
    async fn process_command(&mut self, input: &str) -> Result<()> {
        let input = input.trim();

        // Skip empty lines
        if input.is_empty() {
            return Ok(());
        }

        // Add to history
        self.editor.add_history_entry(input)?;

        // Handle built-in commands
        if let Some(result) = self.handle_builtin_command(input).await? {
            return result;
        }

        // Try enhanced commands first
        if let Some(result) = self.try_enhanced_command(input).await? {
            return result;
        }

        // Substitute variables
        let substituted = self.substitute_variables(input);

        // Parse the command
        let command = self.parser.parse(&substituted)?;

        // Execute the command
        let result = self.executor.execute(command).await?;

        // Format and display the result
        self.formatter.display_result(result);

        Ok(())
    }

    /// Handle built-in console commands
    async fn handle_builtin_command(&mut self, input: &str) -> Result<Option<Result<()>>> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        match parts[0] {
            "exit" | "quit" => {
                self.running = false;
                Ok(Some(Ok(())))
            }
            "help" => {
                self.show_help();
                Ok(Some(Ok(())))
            }
            "help-extended" => {
                self.show_help_extended();
                Ok(Some(Ok(())))
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
                Ok(Some(Ok(())))
            }
            "history" => {
                self.show_history();
                Ok(Some(Ok(())))
            }
            "set" => {
                if parts.len() >= 3 && parts[2] == "=" {
                    let var_name = parts[1].to_string();
                    let var_value = parts[3..].join(" ");
                    self.variables.insert(var_name.clone(), var_value.clone());
                    self.formatter
                        .print_success(&format!("Set {} = {}", var_name, var_value));
                } else {
                    self.formatter.print_error("Usage: set <variable> = <value>");
                }
                Ok(Some(Ok(())))
            }
            "unset" => {
                if parts.len() >= 2 {
                    let var_name = parts[1];
                    if self.variables.remove(var_name).is_some() {
                        self.formatter.print_success(&format!("Unset {}", var_name));
                    } else {
                        self.formatter
                            .print_warning(&format!("Variable {} not found", var_name));
                    }
                } else {
                    self.formatter.print_error("Usage: unset <variable>");
                }
                Ok(Some(Ok(())))
            }
            "vars" => {
                if self.variables.is_empty() {
                    self.formatter.print_info("No variables set");
                } else {
                    for (name, value) in &self.variables {
                        println!("{} = {}", name.bright_yellow(), value);
                    }
                }
                Ok(Some(Ok(())))
            }
            "source" => {
                if parts.len() >= 2 {
                    let script_path = PathBuf::from(parts[1]);
                    if let Err(e) = self.execute_script(&script_path).await {
                        self.formatter.print_error(&format!("Script error: {}", e));
                    }
                } else {
                    self.formatter.print_error("Usage: source <script-file>");
                }
                Ok(Some(Ok(())))
            }
            "connect" => {
                match self.executor.connect().await {
                    Ok(info) => self.formatter.print_success(&format!("Connected: {}", info)),
                    Err(e) => self.formatter.print_error(&format!("Connection failed: {}", e)),
                }
                Ok(Some(Ok(())))
            }
            "disconnect" => {
                self.executor.disconnect().await;
                self.formatter.print_info("Disconnected");
                Ok(Some(Ok(())))
            }
            "env" => {
                if parts.len() >= 2 {
                    // Show specific environment variable
                    let env_var = parts[1];
                    match std::env::var(env_var) {
                        Ok(value) => println!("{}={}", env_var, value),
                        Err(_) => self
                            .formatter
                            .print_warning(&format!("Environment variable '{}' not found", env_var)),
                    }
                } else {
                    // Show all environment variables
                    let mut env_vars: Vec<_> = std::env::vars().collect();
                    env_vars.sort_by(|a, b| a.0.cmp(&b.0));
                    for (key, value) in env_vars {
                        println!("{}={}", key.bright_yellow(), value);
                    }
                }
                Ok(Some(Ok(())))
            }
            _ => Ok(None),
        }
    }

    /// Try to execute enhanced commands using the command registry
    async fn try_enhanced_command(&mut self, input: &str) -> Result<Option<Result<()>>> {
        use super::command_trait::CommandArgs;
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        let command_name = parts[0];
        
        // Check if this is an enhanced command
        if !self.command_registry.has_command(command_name) {
            return Ok(None);
        }

        // Parse arguments
        let action = if parts.len() > 1 { parts[1].to_string() } else { "help".to_string() };
        let positional: Vec<String> = parts.iter().skip(2).map(|s| s.to_string()).collect();
        let flags = std::collections::HashMap::new(); // TODO: Parse flags properly
        
        let args = CommandArgs::new(action, positional, flags);

        // Execute enhanced command
        match self.command_registry.execute_command(command_name, args, &self.enhanced_mcp_client).await {
            Ok(output) => {
                self.display_enhanced_output(output);
                Ok(Some(Ok(())))
            }
            Err(e) => {
                self.formatter.print_error(&format!("Enhanced command error: {}", e));
                Ok(Some(Ok(())))
            }
        }
    }

    /// Display enhanced command output
    fn display_enhanced_output(&self, output: super::command_trait::CommandOutput) {
        use super::command_trait::CommandOutput;
        
        match output {
            CommandOutput::Text(text) => {
                println!("{}", text);
            }
            CommandOutput::Json(value) => {
                println!("{}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()));
            }
            CommandOutput::Table(table) => {
                if let Some(title) = &table.title {
                    println!("{}", title.bright_cyan().bold());
                }
                
                // Print headers
                if !table.headers.is_empty() {
                    println!("{}", table.headers.join("  ").bright_yellow());
                }
                
                // Print rows
                for row in &table.rows {
                    println!("{}", row.join("  "));
                }
            }
            CommandOutput::Success { message, data } => {
                self.formatter.print_success(&message);
                if let Some(data) = data {
                    println!("{}", serde_json::to_string_pretty(&data).unwrap_or_else(|_| data.to_string()));
                }
            }
            CommandOutput::Error { message, context } => {
                self.formatter.print_error(&message);
                if let Some(context) = context {
                    println!("Context: {}", serde_json::to_string_pretty(&context).unwrap_or_else(|_| context.to_string()));
                }
            }
            CommandOutput::Progress { message, percentage } => {
                if let Some(pct) = percentage {
                    println!("{} ({}%)", message, pct);
                } else {
                    println!("{}", message);
                }
            }
            CommandOutput::Stream(_) => {
                println!("Streaming output not yet implemented in console");
            }
            CommandOutput::Dashboard(_) => {
                println!("Dashboard output not yet implemented in console");
            }
        }
    }

    /// Show help information
    fn show_help(&self) {
        println!("{}", "Console Commands:".bright_cyan().bold());
        println!("  {}             - Show this help", "help".bright_yellow());
        println!("  {}     - Show detailed help with examples", "help-extended".bright_yellow());
        println!("  {}             - Exit the console", "exit, quit".bright_yellow());
        println!("  {}            - Clear the screen", "clear".bright_yellow());
        println!("  {}          - Show command history", "history".bright_yellow());
        println!("  {}    - Execute a script file", "source <file>".bright_yellow());
        println!("  {}          - Connect to server", "connect".bright_yellow());
        println!("  {}       - Disconnect from server", "disconnect".bright_yellow());
        println!();
        
        println!("{}", "Core Ratchet Commands:".bright_cyan().bold());
        println!("  {}          - List repositories", "repo list".bright_yellow());
        println!("  {}          - List basic tasks", "task list".bright_yellow());
        println!("  {}       - Show server status", "server status".bright_yellow());
        println!("  {}            - Check server health", "health".bright_yellow());
        println!("  {}          - Show system stats", "stats".bright_yellow());
        println!();
        
        // Show enhanced commands integrated
        println!("{}", "Enhanced Development Commands:".bright_cyan().bold());
        let categories = self.command_registry.list_commands_by_category();
        for (category, commands) in &categories {
            if !commands.is_empty() {
                println!("  {} Commands:", category.bright_green());
                for (name, description) in commands {
                    let short_desc = description.lines().next().unwrap_or("").chars().take(50).collect::<String>();
                    println!("    {} - {}", name.bright_yellow(), short_desc);
                }
                println!();
            }
        }
        
        println!("{}", "Use 'help-extended' for detailed examples and variable expansion".bright_green());
    }

    /// Show extended help with examples and variable expansion
    fn show_help_extended(&self) {
        println!("{}", "=== RATCHET CONSOLE - EXTENDED HELP ===".bright_cyan().bold());
        println!();
        
        // Console Commands with examples
        println!("{}", "Console Commands:".bright_cyan().bold());
        println!("  {}             - Show basic help", "help".bright_yellow());
        println!("  {}     - Show this extended help", "help-extended".bright_yellow());
        println!("  {}             - Exit the console", "exit, quit".bright_yellow());
        println!("  {}            - Clear the screen", "clear".bright_yellow());
        println!("  {}          - Show command history", "history".bright_yellow());
        println!("  {}   - Set a variable", "set <var> = <value>".bright_yellow());
        println!("    Example: {}", "set PROJECT_NAME = my-project".bright_white());
        println!("  {}      - Unset a variable", "unset <var>".bright_yellow());
        println!("  {}             - Show all variables", "vars".bright_yellow());
        println!("  {}              - Show environment variables", "env [var]".bright_yellow());
        println!("    Example: {}", "env PATH".bright_white());
        println!("  {}    - Execute a script file", "source <file>".bright_yellow());
        println!("    Example: {}", "source my-script.ratchet".bright_white());
        println!("  {}          - Connect to server", "connect".bright_yellow());
        println!("  {}       - Disconnect from server", "disconnect".bright_yellow());
        println!();

        // Core Ratchet Commands with examples
        println!("{}", "Core Ratchet Commands:".bright_cyan().bold());
        println!("  {}          - List repositories", "repo list".bright_yellow());
        println!("  {}           - Add repository", "repo add <url>".bright_yellow());
        println!("    Example: {}", "repo add https://github.com/user/tasks".bright_white());
        println!("  {}        - Repository status", "repo status".bright_yellow());
        println!("  {}          - List basic tasks", "task list".bright_yellow());
        println!("  {}          - Show task details", "task show <id>".bright_yellow());
        println!("  {}       - Execute basic task", "task execute <id>".bright_yellow());
        println!("  {}       - Show server status", "server status".bright_yellow());
        println!("  {}            - Check server health", "health".bright_yellow());
        println!("  {}          - Show system stats", "stats".bright_yellow());
        println!("  {}      - List executions", "execution list".bright_yellow());
        println!("  {}      - Show execution", "execution show <id>".bright_yellow());
        println!("  {}            - List jobs", "job list".bright_yellow());
        println!("  {}           - Show job", "job show <id>".bright_yellow());
        println!();

        // Enhanced Development Commands with detailed examples
        println!("{}", "Enhanced Development Commands:".bright_cyan().bold());
        let categories = self.command_registry.list_commands_by_category();
        
        for (category, commands) in &categories {
            if !commands.is_empty() {
                println!();
                println!("  {} Commands:", category.bright_green().bold());
                
                for (name, _description) in commands {
                    if let Some(examples) = self.get_command_examples(name) {
                        println!("    {} - Enhanced {} operations", name.bright_yellow(), category.to_lowercase());
                        for example in examples {
                            println!("      {}", example.bright_white());
                        }
                    }
                }
            }
        }
        
        println!();
        println!("{}", "Variable Expansion:".bright_cyan().bold());
        println!("  {}              - Simple variable substitution", "$VAR".bright_yellow());
        println!("    Example: {}", "task execute $TASK_ID".bright_white());
        println!("  {}            - Variable with braces", "${VAR}".bright_yellow());
        println!("    Example: {}", "execution show ${EXEC_ID}".bright_white());
        println!("  {}        - Environment variable", "${ENV:VAR}".bright_yellow());
        println!("    Example: {}", "set API_KEY = ${ENV:RATCHET_API_KEY}".bright_white());
        println!("  {}   - Variable with default value", "${VAR:-default}".bright_yellow());
        println!("    Example: {}", "task execute ${TASK_ID:-default-task}".bright_white());
        println!("  {}    - Value if variable is set", "${VAR:+value}".bright_yellow());
        println!("    Example: {}", "task execute ${DEBUG:+--verbose}".bright_white());
        println!();
        
        println!("{}", "Scripting Features:".bright_cyan().bold());
        println!("  - Comments start with '#'");
        println!("  - Variables persist across commands in the session");
        println!("  - Environment variables are accessible via ${{ENV:VAR}}");
        println!("  - Tab completion works for commands, files, and task IDs");
        println!("  - Command history is saved between sessions");
        println!();
        
        println!("{}", "Advanced Usage Examples:".bright_cyan().bold());
        println!("  {}", "# Set up common variables".bright_green());
        println!("  {}", "set PROJECT = weather-api".bright_white());
        println!("  {}", "set VERSION = 1.2.0".bright_white());
        println!();
        println!("  {}", "# Create and execute a task".bright_green());
        println!("  {}", "task create $PROJECT --template http-client".bright_white());
        println!("  {}", "task execute $PROJECT --input '{\"city\": \"London\"}'".bright_white());
        println!();
        println!("  {}", "# Monitor execution progress".bright_green());
        println!("  {}", "execution list --status running".bright_white());
        println!("  {}", "monitor dashboard".bright_white());
        println!();
        println!("  {}", "# Job scheduling".bright_green());
        println!("  {}", "job create daily-backup --task backup --schedule \"0 2 * * *\"".bright_white());
        println!("  {}", "job list --status active".bright_white());
        println!();
        
        println!("{}", "Use tab completion for command suggestions and available options".bright_green());
    }

    /// Get command usage examples for help-extended
    fn get_command_examples(&self, command_name: &str) -> Option<Vec<String>> {
        match command_name {
            "task" => Some(vec![
                "task create my-api --template http-client".to_string(),
                "task edit my-api --description \"Updated API\"".to_string(),
                "task execute my-api --input '{\"key\": \"value\"}'".to_string(),
                "task validate my-api --fix".to_string(),
            ]),
            "template" => Some(vec![
                "template list --category web".to_string(),
                "template generate http-client my-service".to_string(),
            ]),
            "execution" => Some(vec![
                "execution list --limit 10".to_string(),
                "execution show abc123 --logs".to_string(),
                "execution cancel xyz789 --reason \"timeout\"".to_string(),
                "execution retry failed-exec --input '{\"retry\": true}'".to_string(),
                "execution analyze error-exec".to_string(),
            ]),
            "monitor" => Some(vec![
                "monitor dashboard".to_string(),
                "monitor health --detailed".to_string(),
                "monitor stats --range 1h".to_string(),
                "monitor live --filter executions".to_string(),
            ]),
            "job" => Some(vec![
                "job list --status active".to_string(),
                "job create backup-job --task backup --schedule \"0 2 * * *\"".to_string(),
                "job show job123".to_string(),
                "job trigger job123".to_string(),
                "job update job123 --enabled false".to_string(),
            ]),
            _ => None,
        }
    }

    /// Show command history
    fn show_history(&self) {
        for (i, entry) in self.editor.history().iter().enumerate() {
            println!("{:3}: {}", i + 1, entry);
        }
    }

    /// Substitute variables in input
    fn substitute_variables(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Enhanced variable substitution with multiple formats
        // Support: $VAR, ${VAR}, $ENV{VAR}, ${ENV:VAR}, ${VAR:-default}

        let re = regex::Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();

        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                if let Some(var_expr) = caps.get(1) {
                    // Handle ${...} format with advanced features
                    let expr = var_expr.as_str();

                    // Handle ${ENV:VAR} - environment variable
                    if let Some(env_var) = expr.strip_prefix("ENV:") {
                        return std::env::var(env_var).unwrap_or_default();
                    }

                    // Handle ${VAR:-default} - variable with default value
                    if let Some((var_name, default_value)) = expr.split_once(":-") {
                        if let Some(value) = self.variables.get(var_name) {
                            return value.clone();
                        } else {
                            return default_value.to_string();
                        }
                    }

                    // Handle ${VAR:+value} - value if variable is set
                    if let Some((var_name, value_if_set)) = expr.split_once(":+") {
                        if self.variables.contains_key(var_name) {
                            return value_if_set.to_string();
                        } else {
                            return String::new();
                        }
                    }

                    // Handle ${VAR} - simple variable
                    if let Some(value) = self.variables.get(expr) {
                        return value.clone();
                    }

                    // Check environment variables as fallback
                    std::env::var(expr).unwrap_or_else(|_| format!("${{{}}}", expr))
                } else if let Some(var_name) = caps.get(2) {
                    // Handle $VAR format
                    let var_name = var_name.as_str();

                    // Check local variables first
                    if let Some(value) = self.variables.get(var_name) {
                        return value.clone();
                    }

                    // Check environment variables as fallback
                    std::env::var(var_name).unwrap_or_else(|_| format!("${}", var_name))
                } else {
                    caps.get(0).unwrap().as_str().to_string()
                }
            })
            .to_string();

        result
    }

    /// Execute a script file
    fn execute_script<'a>(
        &'a mut self,
        script_path: &'a PathBuf,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let content = std::fs::read_to_string(script_path)?;

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue; // Skip empty lines and comments
                }

                self.formatter.print_info(&format!("> {}", line));
                if let Err(e) = self.process_command(line).await {
                    self.formatter.print_error(&format!("Script error: {}", e));
                    return Err(e);
                }
            }

            Ok(())
        })
    }

    /// Save command history
    async fn save_history(&mut self) -> Result<()> {
        if let Some(history_file) = &self.config.history_file {
            self.editor.save_history(history_file)?;
        } else if let Some(home) = dirs::home_dir() {
            let default_history = home.join(".ratchet_history");
            let _ = self.editor.save_history(&default_history);
        }
        Ok(())
    }
}
