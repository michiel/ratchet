//! Command registry for enhanced console commands

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use crate::commands::console::{
    command_trait::{BoxedCommand, CommandArgs, CommandOutput},
    commands::{EnhancedTaskCommand, TemplateCommand, ExecutionCommand, MonitorCommand, JobCommand},
    enhanced_mcp_client::EnhancedMcpClient,
};
use std::sync::Arc;

/// Registry for managing console commands
pub struct CommandRegistry {
    commands: HashMap<String, BoxedCommand>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    /// Create a new command registry with all enhanced commands
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        };

        // Register enhanced task command
        registry.register_command("task", BoxedCommand::EnhancedTask(Arc::new(EnhancedTaskCommand::new())));
        
        // Register template command
        registry.register_command("template", BoxedCommand::Template(Arc::new(TemplateCommand::new())));
        
        // Register execution command
        registry.register_command("execution", BoxedCommand::Execution(Arc::new(ExecutionCommand::new())));
        
        // Register monitor command
        registry.register_command("monitor", BoxedCommand::Monitor(Arc::new(MonitorCommand::new())));
        
        // Register job command
        registry.register_command("job", BoxedCommand::Job(Arc::new(JobCommand::new())));

        registry
    }

    /// Register a command with the registry
    pub fn register_command(&mut self, name: &str, command: BoxedCommand) {
        // Register aliases first
        for alias in command.aliases() {
            self.aliases.insert(alias.to_string(), name.to_string());
        }

        // Register main command name
        self.commands.insert(name.to_string(), command);
    }

    /// Get a command by name or alias
    pub fn get_command(&self, name: &str) -> Option<&BoxedCommand> {
        // Try direct lookup first
        if let Some(command) = self.commands.get(name) {
            return Some(command);
        }

        // Try alias lookup
        if let Some(real_name) = self.aliases.get(name) {
            return self.commands.get(real_name);
        }

        None
    }

    /// Execute a command with the given arguments
    pub async fn execute_command(
        &self,
        command_name: &str,
        args: CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let command = self
            .get_command(command_name)
            .ok_or_else(|| anyhow!("Unknown command: {}", command_name))?;

        // Validate arguments
        command.validate_args(&args)?;

        // Check connection requirement
        if command.requires_connection() && !mcp_client.is_connected() {
            return Err(anyhow!("Command '{}' requires an active MCP connection", command_name));
        }

        // Execute command
        command.execute(args, mcp_client).await
    }

    /// Get completion hints for a command
    pub fn get_completion_hints(&self, command_name: &str, partial: &str) -> Vec<String> {
        if let Some(command) = self.get_command(command_name) {
            command.completion_hints(partial)
        } else {
            vec![]
        }
    }

    /// Get help text for a command
    pub fn get_help_text(&self, command_name: &str) -> Option<String> {
        self.get_command(command_name)
            .map(|command| command.help_text().to_string())
    }

    /// List all available commands
    pub fn list_commands(&self) -> Vec<(String, &str, &str)> {
        self.commands
            .iter()
            .map(|(name, command)| {
                (
                    name.clone(),
                    command.category(),
                    command.help_text().lines().next().unwrap_or(""),
                )
            })
            .collect()
    }

    /// List commands by category
    pub fn list_commands_by_category(&self) -> HashMap<String, Vec<(String, &str)>> {
        let mut categories: HashMap<String, Vec<(String, &str)>> = HashMap::new();

        for (name, command) in &self.commands {
            let category = command.category().to_string();
            let description = command.help_text().lines().next().unwrap_or("");
            
            categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push((name.clone(), description));
        }

        categories
    }

    /// Get command names for completion
    pub fn get_command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.extend(self.aliases.keys().cloned());
        names.sort();
        names
    }

    /// Get command names that start with prefix
    pub fn get_command_names_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.get_command_names()
            .into_iter()
            .filter(|name| name.starts_with(prefix))
            .collect()
    }

    /// Check if command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name) || self.aliases.contains_key(name)
    }

    /// Get command usage examples
    pub fn get_usage_examples(&self, command_name: &str) -> Vec<String> {
        self.get_command(command_name)
            .map(|command| {
                command
                    .usage_examples()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}