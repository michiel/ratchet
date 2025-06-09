//! Ratchet Server Binary
//!
//! A unified server combining REST and GraphQL APIs with all necessary services.

use anyhow::Result;
use clap::Parser;
use serde_json;
use std::path::PathBuf;

use ratchet_server::{ServerConfig, Server};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Server bind address
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind: String,

    /// Enable REST API
    #[arg(long, default_value = "true")]
    rest: bool,

    /// Enable GraphQL API  
    #[arg(long, default_value = "true")]
    graphql: bool,

    /// Enable GraphQL playground
    #[arg(long, default_value = "true")]
    playground: bool,

    /// Database URL
    #[arg(long, default_value = "sqlite://ratchet.db")]
    database_url: String,

    /// Task registry paths (can be specified multiple times)
    #[arg(long)]
    registry_path: Vec<String>,

    /// Print default configuration and exit
    #[arg(long)]
    print_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Print default configuration if requested
    if cli.print_config {
        let default_config = ServerConfig::default();
        println!("{}", serde_json::to_string_pretty(&default_config)?);
        return Ok(());
    }

    // Load configuration
    let mut config = if let Some(config_path) = &cli.config {
        load_config_from_file(config_path).await?
    } else {
        ServerConfig::default()
    };

    // Override with CLI arguments
    apply_cli_overrides(&mut config, &cli)?;

    // Create and start server
    let server = Server::new(config).await?;
    server.start().await
}

/// Load configuration from file
async fn load_config_from_file(path: &PathBuf) -> Result<ServerConfig> {
    let content = tokio::fs::read_to_string(path).await?;
    
    // Support both JSON and YAML formats
    let config = if path.extension().map_or(false, |ext| ext == "json") {
        serde_json::from_str(&content)?
    } else {
        // Try YAML
        serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML config: {}", e))?
    };

    Ok(config)
}

/// Apply CLI argument overrides to configuration
fn apply_cli_overrides(config: &mut ServerConfig, cli: &Cli) -> Result<()> {
    // Override bind address
    config.server.bind_address = cli.bind.parse()
        .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", cli.bind, e))?;

    // Override API enables
    config.rest_api.enabled = cli.rest;
    config.graphql_api.enabled = cli.graphql;
    config.graphql_api.enable_playground = cli.playground;

    // Override database URL
    config.database.url = cli.database_url.clone();

    // Override registry paths if provided
    if !cli.registry_path.is_empty() {
        config.registry.filesystem_paths = cli.registry_path.clone();
    }

    Ok(())
}