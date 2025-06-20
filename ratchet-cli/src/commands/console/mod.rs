//! Console command implementation
//!
//! Provides an interactive REPL console for Ratchet administration

use anyhow::Result;
use std::path::PathBuf;

pub mod commands;
pub mod executor;
pub mod formatter;
pub mod mcp_client;
pub mod parser;
pub mod repl;

use repl::RatchetConsole;

/// Console command configuration
#[derive(Debug, Clone)]
pub struct ConsoleConfig {
    pub config_file: Option<PathBuf>,
    pub connect_url: Option<String>,
    pub transport: String,
    pub host: String,
    pub port: u16,
    pub auth_token: Option<String>,
    pub history_file: Option<PathBuf>,
    pub script_file: Option<PathBuf>,
}

/// Main entry point for the console command
pub async fn run_console(config: ConsoleConfig) -> Result<()> {
    let mut console = RatchetConsole::new(config).await?;
    console.run().await
}
