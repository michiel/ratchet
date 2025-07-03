//! Command parsing for console commands

use anyhow::{anyhow, Result};
use serde_json::Value;

/// Represents a parsed console command
#[derive(Debug, Clone)]
pub struct ConsoleCommand {
    pub category: String,
    pub action: String,
    pub arguments: Vec<String>,
    pub flags: std::collections::HashMap<String, String>,
    pub json_input: Option<Value>,
}

/// Command parser for console input
pub struct CommandParser {}

impl CommandParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a command line into a structured command
    pub fn parse(&self, input: &str) -> Result<ConsoleCommand> {
        let parts = self.tokenize(input)?;
        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let mut args_iter = parts.iter();
        let category = args_iter.next()
            .expect("First argument should exist after empty check")
            .clone();

        let action = if let Some(action) = args_iter.next() {
            action.clone()
        } else {
            // No action specified - determine default based on category
            match category.as_str() {
                "repo" | "task" | "execution" | "job" | "server" | "db" => "list".to_string(),
                "health" | "stats" | "monitor" => "".to_string(), // No action needed
                _ => "help".to_string(),
            }
        };

        let mut arguments = Vec::new();
        let mut flags = std::collections::HashMap::new();
        let mut json_input = None;

        let remaining: Vec<String> = args_iter.cloned().collect();
        let mut i = 0;

        while i < remaining.len() {
            let arg = &remaining[i];

            if arg.starts_with("--") {
                // Long flag
                let flag_name = arg.trim_start_matches("--");
                if i + 1 < remaining.len() && !remaining[i + 1].starts_with("-") {
                    flags.insert(flag_name.to_string(), remaining[i + 1].clone());
                    i += 2;
                } else {
                    flags.insert(flag_name.to_string(), "true".to_string());
                    i += 1;
                }
            } else if arg.starts_with("-") {
                // Short flag
                let flag_name = arg.trim_start_matches("-");
                if i + 1 < remaining.len() && !remaining[i + 1].starts_with("-") {
                    flags.insert(flag_name.to_string(), remaining[i + 1].clone());
                    i += 2;
                } else {
                    flags.insert(flag_name.to_string(), "true".to_string());
                    i += 1;
                }
            } else {
                // Regular argument or JSON
                if arg.starts_with('{') || arg.starts_with('[') {
                    // Try to parse as JSON
                    match serde_json::from_str(arg) {
                        Ok(json) => json_input = Some(json),
                        Err(_) => arguments.push(arg.clone()),
                    }
                } else {
                    arguments.push(arg.clone());
                }
                i += 1;
            }
        }

        Ok(ConsoleCommand {
            category,
            action,
            arguments,
            flags,
            json_input,
        })
    }

    /// Tokenize input while preserving quoted strings and JSON objects
    fn tokenize(&self, input: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let chars = input.chars().peekable();

        for ch in chars {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current_token.push(ch);
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    current_token.push(ch);
                }
                '{' if !in_quotes => {
                    brace_depth += 1;
                    current_token.push(ch);
                }
                '}' if !in_quotes => {
                    brace_depth -= 1;
                    current_token.push(ch);
                }
                '[' if !in_quotes => {
                    bracket_depth += 1;
                    current_token.push(ch);
                }
                ']' if !in_quotes => {
                    bracket_depth -= 1;
                    current_token.push(ch);
                }
                ' ' | '\t' if !in_quotes && brace_depth == 0 && bracket_depth == 0 => {
                    if !current_token.is_empty() {
                        tokens.push(self.clean_token(&current_token));
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(self.clean_token(&current_token));
        }

        if in_quotes {
            return Err(anyhow!("Unclosed quote in command"));
        }

        if brace_depth != 0 {
            return Err(anyhow!("Unmatched braces in command"));
        }

        if bracket_depth != 0 {
            return Err(anyhow!("Unmatched brackets in command"));
        }

        Ok(tokens)
    }

    /// Clean a token by removing outer quotes if present
    fn clean_token(&self, token: &str) -> String {
        if (token.starts_with('"') && token.ends_with('"')) || (token.starts_with('\'') && token.ends_with('\'')) {
            token[1..token.len() - 1].to_string()
        } else {
            token.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let parser = CommandParser::new();
        let cmd = parser.parse("repo list").unwrap();
        assert_eq!(cmd.category, "repo");
        assert_eq!(cmd.action, "list");
        assert!(cmd.arguments.is_empty());
    }

    #[test]
    fn test_command_with_flags() {
        let parser = CommandParser::new();
        let cmd = parser
            .parse("task execute my-task --input '{\"key\": \"value\"}' --force")
            .unwrap();
        assert_eq!(cmd.category, "task");
        assert_eq!(cmd.action, "execute");
        assert_eq!(cmd.arguments, vec!["my-task"]);
        assert_eq!(cmd.flags.get("force"), Some(&"true".to_string()));
    }

    #[test]
    fn test_json_parsing() {
        let parser = CommandParser::new();
        let cmd = parser
            .parse("task execute my-task '{\"num1\": 42, \"num2\": 58}'")
            .unwrap();
        assert!(cmd.json_input.is_some());
        if let Some(json) = cmd.json_input {
            assert_eq!(json["num1"], 42);
            assert_eq!(json["num2"], 58);
        }
    }
}
