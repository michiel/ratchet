//! Command-line interface for migration utilities
//!
//! This module provides CLI commands for running database migrations
//! from legacy ratchet-lib to modern ratchet-storage format.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::migration::{LegacyMigrator, MigrationConfig, MigrationError, MigrationSummary, SchemaVersionDetector};

/// Migration CLI application
#[derive(Parser)]
#[command(name = "ratchet-migrate")]
#[command(about = "Database migration utilities for Ratchet")]
#[command(version = "1.0.0")]
pub struct MigrationCli {
    #[command(subcommand)]
    pub command: MigrationCommand,
}

/// Available migration commands
#[derive(Subcommand)]
pub enum MigrationCommand {
    /// Detect and display database schema version
    Detect {
        /// Database URL to analyze
        #[arg(short, long)]
        database_url: String,
    },

    /// Migrate data from legacy to modern format
    Migrate {
        /// Source database URL (ratchet-lib)
        #[arg(short, long)]
        source: String,

        /// Target database URL (ratchet-storage)
        #[arg(short, long)]
        target: String,

        /// Batch size for processing records
        #[arg(long, default_value = "1000")]
        batch_size: u64,

        /// Force migration even if target is not empty
        #[arg(long)]
        force: bool,

        /// Skip validation after migration
        #[arg(long)]
        skip_validation: bool,

        /// Skip backup creation
        #[arg(long)]
        skip_backup: bool,

        /// Continue on errors instead of failing fast
        #[arg(long)]
        continue_on_error: bool,

        /// Maximum retry attempts for failed records
        #[arg(long, default_value = "3")]
        max_retries: u32,
    },

    /// Validate migration integrity
    Validate {
        /// Source database URL
        #[arg(short, long)]
        source: String,

        /// Target database URL
        #[arg(short, long)]
        target: String,

        /// Sample size for record comparison
        #[arg(long, default_value = "100")]
        sample_size: usize,
    },

    /// Create a backup of the database
    Backup {
        /// Database URL to backup
        #[arg(short, long)]
        database_url: String,

        /// Output backup file path
        #[arg(short, long)]
        output: PathBuf,
    },
}

/// CLI application runner
pub struct MigrationCliRunner;

impl MigrationCliRunner {
    /// Run the CLI application
    pub async fn run(cli: MigrationCli) -> Result<(), MigrationError> {
        match cli.command {
            MigrationCommand::Detect { database_url } => Self::detect_schema(&database_url).await,
            MigrationCommand::Migrate {
                source,
                target,
                batch_size,
                force,
                skip_validation,
                skip_backup,
                continue_on_error,
                max_retries,
            } => {
                let config = MigrationConfig {
                    source_db_url: source,
                    target_db_url: target,
                    batch_size,
                    force,
                    validate: !skip_validation,
                    create_backup: !skip_backup,
                    continue_on_error,
                    max_retries,
                };
                Self::run_migration(config).await
            }
            MigrationCommand::Validate {
                source,
                target,
                sample_size: _sample_size,
            } => Self::validate_migration(&source, &target).await,
            MigrationCommand::Backup { database_url, output } => Self::create_backup(&database_url, &output).await,
        }
    }

    async fn detect_schema(database_url: &str) -> Result<(), MigrationError> {
        println!("🔍 Detecting database schema...");

        let db = sea_orm::Database::connect(database_url).await?;
        let detector = SchemaVersionDetector::new(db);

        let version = detector.detect_version().await?;

        println!("📋 Schema Information:");
        println!("  Version: {}", version.version);
        println!("  Description: {}", version.description);
        println!("  System: {}", version.system);
        println!("  Applied at: {}", version.applied_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("  Applied migrations: {}", version.applied_migrations.len());

        if !version.applied_migrations.is_empty() {
            println!("\n📝 Applied Migrations:");
            for migration in &version.applied_migrations {
                println!("  - {}", migration);
            }
        }

        // Check if database is empty
        if detector.is_empty_database().await? {
            println!("\n⚠️  Database appears to be empty");
        }

        // Check system type
        if detector.is_legacy_database().await? {
            println!("\n🚀 This appears to be a legacy ratchet-lib database");
            println!("   Consider migrating to ratchet-storage format");
        } else if detector.is_modern_database().await? {
            println!("\n✅ This is a modern ratchet-storage database");
        }

        Ok(())
    }

    async fn run_migration(config: MigrationConfig) -> Result<(), MigrationError> {
        println!("🚀 Starting database migration...");
        println!("📊 Configuration:");
        println!("  Source: {}", config.source_db_url);
        println!("  Target: {}", config.target_db_url);
        println!("  Batch size: {}", config.batch_size);
        println!("  Force: {}", config.force);
        println!("  Validate: {}", config.validate);
        println!("  Create backup: {}", config.create_backup);
        println!("  Continue on error: {}", config.continue_on_error);

        let migrator = LegacyMigrator::new(config).await?;
        let summary = migrator.migrate().await?;

        Self::print_migration_summary(&summary);

        if summary.success {
            println!("✅ Migration completed successfully!");
        } else {
            println!("❌ Migration failed or completed with errors");
            return Err(MigrationError::ValidationFailed("Migration not successful".to_string()));
        }

        Ok(())
    }

    async fn validate_migration(source_url: &str, target_url: &str) -> Result<(), MigrationError> {
        println!("🔍 Validating migration integrity...");

        let source_db = sea_orm::Database::connect(source_url).await?;
        let target_db = sea_orm::Database::connect(target_url).await?;

        let validator = crate::migration::MigrationValidator::new(source_db, target_db);
        let report = validator.validate_migration().await?;

        println!("📋 Validation Report:");
        println!("  Overall success: {}", report.success);
        println!("  Validation duration: {}ms", report.validation_duration_ms);

        println!("\n📊 Record Counts:");
        println!(
            "  Tasks: {} → {} ({})",
            report.record_counts.tasks.source,
            report.record_counts.tasks.target,
            if report.record_counts.tasks.matches {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Executions: {} → {} ({})",
            report.record_counts.executions.source,
            report.record_counts.executions.target,
            if report.record_counts.executions.matches {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Jobs: {} → {} ({})",
            report.record_counts.jobs.source,
            report.record_counts.jobs.target,
            if report.record_counts.jobs.matches {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Schedules: {} → {} ({})",
            report.record_counts.schedules.source,
            report.record_counts.schedules.target,
            if report.record_counts.schedules.matches {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Delivery Results: {} → {} ({})",
            report.record_counts.delivery_results.source,
            report.record_counts.delivery_results.target,
            if report.record_counts.delivery_results.matches {
                "✅"
            } else {
                "❌"
            }
        );

        if !report.integrity_checks.is_empty() {
            println!("\n🔒 Integrity Checks:");
            for check in &report.integrity_checks {
                let status = if check.passed { "✅" } else { "❌" };
                println!("  {} {}: {}", status, check.check_type, check.description);
                if let Some(details) = &check.details {
                    println!("    Details: {}", details);
                }
            }
        }

        if !report.entity_validations.is_empty() {
            println!("\n📋 Entity Validations:");
            for validation in &report.entity_validations {
                let status = if validation.matches { "✅" } else { "❌" };
                println!(
                    "  {} {}: {} → {}",
                    status, validation.entity_type, validation.source_count, validation.target_count
                );

                if let Some(sample) = &validation.sample_comparison {
                    if sample.matching_records < sample.sample_size {
                        println!(
                            "    Sample: {}/{} records match",
                            sample.matching_records, sample.sample_size
                        );
                        if !sample.mismatched_fields.is_empty() {
                            println!("    Mismatched fields: {}", sample.mismatched_fields.join(", "));
                        }
                    }
                }
            }
        }

        if !report.errors.is_empty() {
            println!("\n❌ Validation Errors:");
            for error in &report.errors {
                println!("  - {}", error);
            }
        }

        if report.success {
            println!("\n✅ Validation passed!");
        } else {
            println!("\n❌ Validation failed!");
            return Err(MigrationError::ValidationFailed("Validation checks failed".to_string()));
        }

        Ok(())
    }

    async fn create_backup(database_url: &str, output_path: &PathBuf) -> Result<(), MigrationError> {
        println!("💾 Creating database backup...");

        // For SQLite databases, this is a simple file copy
        if database_url.starts_with("sqlite://") {
            let source_path = database_url.trim_start_matches("sqlite://");
            std::fs::copy(source_path, output_path)?;
            println!("✅ Backup created: {}", output_path.display());
        } else {
            return Err(MigrationError::ValidationFailed(
                "Backup currently only supported for SQLite databases".to_string(),
            ));
        }

        Ok(())
    }

    fn print_migration_summary(summary: &MigrationSummary) {
        println!("\n📊 Migration Summary:");
        println!("  Started: {}", summary.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(completed) = summary.completed_at {
            println!("  Completed: {}", completed.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        println!("  Duration: {}ms", summary.total_duration_ms);
        println!("  Total migrated: {}", summary.total_migrated());
        println!("  Total failed: {}", summary.total_failed());
        println!("  Success rate: {:.1}%", summary.overall_success_rate() * 100.0);

        if let Some(backup_path) = &summary.backup_path {
            println!("  Backup: {}", backup_path);
        }

        if !summary.reports.is_empty() {
            println!("\n📋 Entity Reports:");
            for report in &summary.reports {
                println!("  {}:", report.entity_type);
                println!("    Migrated: {}", report.migrated_count);
                println!("    Skipped: {}", report.skipped_count);
                println!("    Failed: {}", report.failed_count);
                println!("    Duration: {}ms", report.duration_ms);
                println!("    Success rate: {:.1}%", report.success_rate() * 100.0);

                if !report.errors.is_empty() && report.errors.len() <= 5 {
                    println!("    Errors:");
                    for error in &report.errors {
                        println!("      - {}", error);
                    }
                } else if report.errors.len() > 5 {
                    println!("    Errors: {} (showing first 3):", report.errors.len());
                    for error in report.errors.iter().take(3) {
                        println!("      - {}", error);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test detect command
        let cli =
            MigrationCli::try_parse_from(["ratchet-migrate", "detect", "--database-url", "sqlite://test.db"]).unwrap();

        match cli.command {
            MigrationCommand::Detect { database_url } => {
                assert_eq!(database_url, "sqlite://test.db");
            }
            _ => panic!("Expected Detect command"),
        }

        // Test migrate command with flags
        let cli = MigrationCli::try_parse_from([
            "ratchet-migrate",
            "migrate",
            "--source",
            "sqlite://legacy.db",
            "--target",
            "sqlite://modern.db",
            "--force",
            "--continue-on-error",
        ])
        .unwrap();

        match cli.command {
            MigrationCommand::Migrate {
                source,
                target,
                force,
                continue_on_error,
                ..
            } => {
                assert_eq!(source, "sqlite://legacy.db");
                assert_eq!(target, "sqlite://modern.db");
                assert!(force);
                assert!(continue_on_error);
            }
            _ => panic!("Expected Migrate command"),
        }
    }
}
