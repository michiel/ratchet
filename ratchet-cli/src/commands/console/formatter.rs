//! Output formatting for console results

use colored::*;
use serde_json::Value;
use std::io::{self, Write};

use super::executor::CommandResult;

/// Border types for table formatting
#[derive(Clone, Copy)]
enum BorderType {
    Top,
    Middle,
    Bottom,
}

/// Output formatter for console results
pub struct OutputFormatter {}

impl OutputFormatter {
    pub fn new() -> Self {
        Self {}
    }

    /// Display a command result
    pub fn display_result(&self, result: CommandResult) {
        match result {
            CommandResult::Success { message, data } => {
                self.print_success(&message);
                if let Some(data) = data {
                    self.print_json(&data);
                }
            }
            CommandResult::Error { message } => {
                self.print_error(&message);
            }
            CommandResult::Table { headers, rows } => {
                self.print_table(&headers, &rows);
            }
            CommandResult::Json { data } => {
                self.print_json(&data);
            }
            CommandResult::Text { content } => {
                println!("{}", content);
            }
        }
    }

    /// Print a success message
    pub fn print_success(&self, message: &str) {
        println!("{} {}", "✓".bright_green().bold(), message);
    }

    /// Print an error message
    pub fn print_error(&self, message: &str) {
        eprintln!("{} {}", "✗".bright_red().bold(), message.bright_red());
    }

    /// Print a warning message
    pub fn print_warning(&self, message: &str) {
        println!("{} {}", "⚠".bright_yellow().bold(), message.bright_yellow());
    }

    /// Print an info message
    pub fn print_info(&self, message: &str) {
        println!("{} {}", "ℹ".bright_blue().bold(), message);
    }

    /// Print JSON data with formatting
    pub fn print_json(&self, data: &Value) {
        match serde_json::to_string_pretty(data) {
            Ok(formatted) => {
                for line in formatted.lines() {
                    println!("{}", self.colorize_json_line(line));
                }
            }
            Err(_) => {
                println!("{}", data);
            }
        }
    }

    /// Print a table with headers and rows
    pub fn print_table(&self, headers: &[String], rows: &[Vec<String>]) {
        if headers.is_empty() || rows.is_empty() {
            self.print_info("No data to display");
            return;
        }

        // Calculate column widths
        let mut col_widths = headers.iter().map(|h| h.len()).collect::<Vec<_>>();

        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }

        // Print top border
        self.print_table_border(&col_widths, BorderType::Top);
        
        // Print headers
        print!("│");
        for (i, header) in headers.iter().enumerate() {
            print!(" {:width$} │", header.bright_cyan().bold(), width = col_widths[i]);
        }
        println!();
        
        // Print middle border
        self.print_table_border(&col_widths, BorderType::Middle);

        // Print rows
        for row in rows {
            print!("│");
            for (i, cell) in row.iter().enumerate() {
                let width = if i < col_widths.len() { col_widths[i] } else { 0 };
                print!(" {:width$} │", cell, width = width);
            }
            println!();
        }
        
        // Print bottom border
        self.print_table_border(&col_widths, BorderType::Bottom);
    }

    /// Print table border with correct corner and junction characters
    fn print_table_border(&self, col_widths: &[usize], border_type: BorderType) {
        let (left, junction, right) = match border_type {
            BorderType::Top => ("┌", "┬", "┐"),
            BorderType::Middle => ("├", "┼", "┤"),
            BorderType::Bottom => ("└", "┴", "┘"),
        };

        print!("{}", left);
        for (i, &width) in col_widths.iter().enumerate() {
            if i > 0 {
                print!("{}", junction);
            }
            print!("{}", "─".repeat(width + 2));
        }
        println!("{}", right);
    }

    /// Colorize a JSON line for better readability
    fn colorize_json_line(&self, line: &str) -> String {
        let trimmed = line.trim_start();
        let indent = " ".repeat(line.len() - trimmed.len());

        if trimmed.starts_with('"') && trimmed.contains(':') {
            // Key-value pair
            if let Some(colon_pos) = trimmed.find(':') {
                let key_part = &trimmed[..colon_pos + 1];
                let value_part = &trimmed[colon_pos + 1..];
                return format!("{}{}{}", indent, key_part.bright_blue(), value_part.bright_white());
            }
        } else if trimmed.starts_with('"') {
            // String value
            return format!("{}{}", indent, trimmed.bright_green());
        } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
            || trimmed.starts_with("true")
            || trimmed.starts_with("false")
            || trimmed.starts_with("null")
        {
            // Number, boolean, or null
            return format!("{}{}", indent, trimmed.bright_yellow());
        }

        // Default formatting
        line.to_string()
    }

    /// Print a progress indicator (for long-running operations)
    pub fn print_progress(&self, message: &str) {
        print!("\r{} {}...", "⏳".bright_yellow(), message);
        io::stdout().flush().unwrap();
    }

    /// Clear progress indicator
    pub fn clear_progress(&self) {
        print!("\r{}", " ".repeat(80));
        print!("\r");
        io::stdout().flush().unwrap();
    }
}
