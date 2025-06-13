//! REPL (Read-Eval-Print Loop) implementation for Ratchet console

use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use colored::*;
use rustyline::{DefaultEditor, Result as RustylineResult};

use super::{ConsoleConfig, parser::CommandParser, executor::CommandExecutor, formatter::OutputFormatter};

/// Main console REPL implementation
pub struct RatchetConsole {
    config: ConsoleConfig,
    editor: DefaultEditor,
    parser: CommandParser,
    executor: CommandExecutor,
    formatter: OutputFormatter,
    variables: HashMap<String, String>,
    running: bool,
}

impl RatchetConsole {
    /// Create a new console instance
    pub async fn new(config: ConsoleConfig) -> Result<Self> {
        let mut editor = DefaultEditor::new()?;
        
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

        Ok(Self {
            config,
            editor,
            parser,
            executor,
            formatter,
            variables: HashMap::new(),
            running: false,
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
        
        // Attempt to connect and show connection status
        match self.executor.connect().await {
            Ok(info) => {
                println!("Connected to: {}", info.bright_green());
                let health = self.executor.check_health().await?;
                self.formatter.print_success(&format!("Server status: {}", health));
            }
            Err(e) => {
                self.formatter.print_warning(&format!("Connection warning: {}", e));
                self.formatter.print_info("Some commands may not be available");
            }
        }
        
        println!("Type '{}' for available commands, '{}' to quit", "help".bright_yellow(), "exit".bright_yellow());
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
                    self.formatter.print_success(&format!("Set {} = {}", var_name, var_value));
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
                        self.formatter.print_warning(&format!("Variable {} not found", var_name));
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
            _ => Ok(None)
        }
    }

    /// Show help information
    fn show_help(&self) {
        println!("{}", "Console Commands:".bright_cyan().bold());
        println!("  {}             - Show this help", "help".bright_yellow());
        println!("  {}             - Exit the console", "exit, quit".bright_yellow());
        println!("  {}            - Clear the screen", "clear".bright_yellow());
        println!("  {}          - Show command history", "history".bright_yellow());
        println!("  {}   - Set a variable", "set <var> = <value>".bright_yellow());
        println!("  {}      - Unset a variable", "unset <var>".bright_yellow());
        println!("  {}             - Show all variables", "vars".bright_yellow());
        println!("  {}    - Execute a script file", "source <file>".bright_yellow());
        println!("  {}          - Connect to server", "connect".bright_yellow());
        println!("  {}       - Disconnect from server", "disconnect".bright_yellow());
        println!();
        println!("{}", "Ratchet Commands:".bright_cyan().bold());
        println!("  {}          - List repositories", "repo list".bright_yellow());
        println!("  {}          - List tasks", "task list".bright_yellow());
        println!("  {}       - Show server status", "server status".bright_yellow());
        println!("  {}            - Check server health", "health".bright_yellow());
        println!("  {}          - Show system stats", "stats".bright_yellow());
        println!();
        println!("{}", "Use tab completion for command suggestions".bright_green());
    }

    /// Show command history
    fn show_history(&self) {
        let history = self.editor.history();
        for (i, entry) in history.iter().enumerate() {
            println!("{:3}: {}", i + 1, entry);
        }
    }

    /// Substitute variables in input
    fn substitute_variables(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (name, value) in &self.variables {
            let pattern = format!("${}", name);
            result = result.replace(&pattern, value);
        }
        result
    }

    /// Execute a script file
    fn execute_script<'a>(&'a mut self, script_path: &'a PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
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