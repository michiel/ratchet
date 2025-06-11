//! Command-line interface for configuration migration
//!
//! This module provides a CLI tool for migrating configuration files
//! from legacy to modern format, with options for backup and validation.

use std::path::PathBuf;
use clap::{Parser, Subcommand};

use crate::{
    migration::{ConfigMigrator, ConfigCompatibilityService, MigrationReport},
    ConfigResult,
};

/// Configuration migration CLI tool
#[derive(Parser)]
#[command(name = "ratchet-config")]
#[command(about = "Configuration migration utilities for Ratchet")]
#[command(version = "1.0.0")]
pub struct ConfigCli {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Available configuration commands
#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Check if a configuration file needs migration
    Check {
        /// Path to the configuration file
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Migrate a configuration file to modern format
    Migrate {
        /// Path to the configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Skip creating a backup file
        #[arg(long)]
        no_backup: bool,

        /// Preserve the original file (don't overwrite)
        #[arg(long)]
        preserve_original: bool,

        /// Output directory for migrated file (if preserving original)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Force migration even if file appears to be modern format
        #[arg(long)]
        force: bool,
    },

    /// Validate a configuration file
    Validate {
        /// Path to the configuration file
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Convert modern config to legacy format (for testing)
    Convert {
        /// Path to the modern configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Output path for legacy format file
        #[arg(short, long)]
        output: PathBuf,
    },
}

/// CLI application runner
pub struct ConfigCliRunner;

impl ConfigCliRunner {
    /// Run the CLI application
    pub async fn run(cli: ConfigCli) -> ConfigResult<()> {
        match cli.command {
            ConfigCommand::Check { config } => {
                Self::check_config(config).await
            }
            ConfigCommand::Migrate {
                config,
                no_backup,
                preserve_original,
                output,
                force: _force,
            } => {
                Self::migrate_config(config, !no_backup, preserve_original, output).await
            }
            ConfigCommand::Validate { config } => {
                Self::validate_config(config).await
            }
            ConfigCommand::Convert { config, output } => {
                Self::convert_config(config, output).await
            }
        }
    }

    async fn check_config(config_path: PathBuf) -> ConfigResult<()> {
        println!("🔍 Checking configuration file: {}", config_path.display());

        match ConfigCompatibilityService::needs_migration(&config_path) {
            Ok(true) => {
                println!("📋 Status: Migration needed");
                println!("   This configuration file uses legacy format and should be migrated.");
                println!("   Run: ratchet-config migrate --config {}", config_path.display());
            }
            Ok(false) => {
                println!("✅ Status: Modern format");
                println!("   This configuration file is already in modern format.");
            }
            Err(e) => {
                println!("❌ Error: Failed to check configuration");
                println!("   {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn migrate_config(
        config_path: PathBuf,
        create_backup: bool,
        preserve_original: bool,
        output: Option<PathBuf>,
    ) -> ConfigResult<()> {
        println!("🔧 Migrating configuration file: {}", config_path.display());

        let migrator = ConfigMigrator::with_options(create_backup, preserve_original);
        let (modern_config, report) = migrator.migrate_config_file(&config_path).await?;

        // If output path is specified and we're preserving original, save to output location
        if let Some(output_path) = output {
            if preserve_original {
                migrator.save_modern_config(&output_path, &modern_config).await?;
                println!("💾 Migrated configuration saved to: {}", output_path.display());
            } else {
                println!("⚠️  Output path specified but not preserving original - file will be overwritten in place");
            }
        }

        // Print migration report
        report.print_summary();

        if !report.is_successful() {
            std::process::exit(1);
        }

        Ok(())
    }

    async fn validate_config(config_path: PathBuf) -> ConfigResult<()> {
        println!("🔍 Validating configuration file: {}", config_path.display());

        // Load config with migration if needed
        let (config, report) = ConfigCompatibilityService::load_with_migration(&config_path).await?;

        if report.migration_performed {
            println!("ℹ️  Configuration was auto-migrated during loading");
        }

        // Perform validation using the validation module
        crate::validation::validate_config(&config)?;

        println!("✅ Configuration is valid!");

        // Print configuration summary
        Self::print_config_summary(&config);

        Ok(())
    }

    async fn convert_config(config_path: PathBuf, output_path: PathBuf) -> ConfigResult<()> {
        println!("🔄 Converting modern config to legacy format");
        println!("   Input: {}", config_path.display());
        println!("   Output: {}", output_path.display());

        // Load modern config
        let (modern_config, _) = ConfigCompatibilityService::load_with_migration(&config_path).await?;

        // Convert to legacy format
        let legacy_config = ConfigCompatibilityService::to_legacy_format(&modern_config);

        // Save legacy format
        let content = serde_yaml::to_string(&legacy_config).map_err(|e| crate::ConfigError::ParseError {
            message: format!("Failed to serialize legacy config: {}", e),
        })?;

        std::fs::write(&output_path, content).map_err(|e| crate::ConfigError::FileIo {
            path: output_path.clone(),
            error: e,
        })?;

        println!("✅ Conversion completed successfully!");

        Ok(())
    }

    fn print_config_summary(config: &crate::domains::RatchetConfig) {
        println!("\n📋 Configuration Summary:");
        
        if let Some(server) = &config.server {
            println!("   🖥️  Server: {}:{}", server.bind_address, server.port);
        }

        println!("   💾 Database: {}", config.database.url);
        println!("   ⚡ Execution: max duration {}s, {} concurrent tasks",
            config.execution.max_execution_duration.as_secs(),
            config.execution.max_concurrent
        );
        println!("   🌐 HTTP: timeout {}s, verify SSL: {}",
            config.http.timeout.as_secs(),
            config.http.verify_ssl
        );
        println!("   📝 Logging: level {}, format {}",
            config.logging.level,
            config.logging.format
        );

        if let Some(mcp) = &config.mcp {
            println!("   🔗 MCP: enabled {}, transport {}",
                mcp.enabled,
                mcp.transport
            );
        }

        if config.cache.enabled {
            println!("   💽 Cache: enabled, {} entries max",
                config.cache.max_entries
            );
        }

        if config.registry.enabled {
            println!("   📚 Registry: {} sources configured",
                config.registry.sources.len()
            );
        }

        if !config.output.destinations.is_empty() {
            println!("   📤 Output: {} destinations configured",
                config.output.destinations.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_cli_check_legacy() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a legacy config file
        let legacy_config = r#"
server:
  bind_address: "127.0.0.1"
  port: 3000
  database:
    url: "sqlite://test.db"
max_execution_duration: 600
validate_schemas: true
"#;

        let mut file = File::create(&config_path).unwrap();
        file.write_all(legacy_config.as_bytes()).unwrap();

        // Test CLI check command
        let cli = ConfigCli {
            command: ConfigCommand::Check { config: config_path },
        };

        let result = ConfigCliRunner::run(cli).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cli_migrate() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a legacy config file
        let legacy_config = r#"
server:
  bind_address: "127.0.0.1"
  port: 3000
  database:
    url: "sqlite://test.db"
    max_connections: 5
max_execution_duration: 600
validate_schemas: true
max_concurrent_tasks: 8
timeout_grace_period: 60
"#;

        let mut file = File::create(&config_path).unwrap();
        file.write_all(legacy_config.as_bytes()).unwrap();

        // Test CLI migrate command
        let cli = ConfigCli {
            command: ConfigCommand::Migrate {
                config: config_path.clone(),
                no_backup: true,
                preserve_original: false,
                output: None,
                force: false,
            },
        };

        let result = ConfigCliRunner::run(cli).await;
        assert!(result.is_ok());

        // Verify file was migrated
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("cache:")); // Should have modern domains
        assert!(content.contains("registry:"));
        assert!(content.contains("output:"));
    }

    #[test]
    fn test_cli_parsing() {
        // Test check command
        let cli = ConfigCli::try_parse_from(&[
            "ratchet-config",
            "check",
            "--config",
            "test.yaml"
        ]).unwrap();

        match cli.command {
            ConfigCommand::Check { config } => {
                assert_eq!(config, PathBuf::from("test.yaml"));
            }
            _ => panic!("Expected Check command"),
        }

        // Test migrate command with flags
        let cli = ConfigCli::try_parse_from(&[
            "ratchet-config",
            "migrate",
            "--config", "test.yaml",
            "--no-backup",
            "--preserve-original",
            "--output", "output.yaml"
        ]).unwrap();

        match cli.command {
            ConfigCommand::Migrate { config, no_backup, preserve_original, output, force } => {
                assert_eq!(config, PathBuf::from("test.yaml"));
                assert!(no_backup);
                assert!(preserve_original);
                assert_eq!(output, Some(PathBuf::from("output.yaml")));
                assert!(!force);
            }
            _ => panic!("Expected Migrate command"),
        }
    }
}