//! Ratchet CLI main entry point

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

mod cli;
mod commands;
mod utils;
mod worker;

use cli::{Cli, Commands, GenerateCommands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle worker mode early
    if cli.worker {
        return worker::run_worker(cli.worker_id).await;
    }

    // Initialize tracing
    utils::init_tracing(cli.log_level.as_ref(), cli.command.as_ref().and_then(|cmd| {
        match cmd {
            Commands::RunOnce { record, .. } => record.as_ref(),
            _ => None,
        }
    }))?;

    info!("Ratchet CLI starting");

    // Dispatch to command handlers
    match &cli.command {
        Some(Commands::RunOnce { from_fs, input_json, record }) => {
            commands::run_once_command(from_fs, input_json.as_ref(), record.as_ref()).await
        }
        Some(Commands::Serve { config }) => {
            commands::serve_command(config.as_ref()).await
        }
        Some(Commands::McpServe { config, transport, host, port }) => {
            commands::mcp_serve_command(config.as_ref(), transport, host, *port).await
        }
        Some(Commands::Validate { from_fs }) => {
            commands::validate_command(from_fs)
        }
        Some(Commands::Test { from_fs }) => {
            commands::test_command(from_fs).await
        }
        Some(Commands::Replay { from_fs, recording }) => {
            commands::replay_command(from_fs, recording.as_ref()).await
        }
        Some(Commands::Generate { generate_cmd }) => {
            match generate_cmd {
                GenerateCommands::Task { path, label, description, version } => {
                    commands::generate_task_command(path, label.as_ref(), description.as_ref(), version.as_ref())
                }
            }
        }
        None => {
            warn!("No command specified");
            println!("No command specified. Use --help to see available commands.");
            Ok(())
        }
    }
}