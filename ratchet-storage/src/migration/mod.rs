//! Migration utilities for ratchet-storage
//!
//! This module provides tools for migrating data from legacy ratchet-lib database
//! structures to the modern ratchet-storage format.

pub mod legacy_migrator;
pub mod schema_version;
pub mod validation;

#[cfg(feature = "testing")]
pub mod cli;

// Re-export commonly used types
pub use legacy_migrator::*;
pub use schema_version::*;
pub use validation::*;

use thiserror::Error;

/// Migration result containing counts and status
#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub entity_type: String,
    pub migrated_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

impl MigrationReport {
    pub fn new(entity_type: String) -> Self {
        Self {
            entity_type,
            migrated_count: 0,
            skipped_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn total_processed(&self) -> u64 {
        self.migrated_count + self.skipped_count + self.failed_count
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_processed();
        if total == 0 {
            1.0
        } else {
            self.migrated_count as f64 / total as f64
        }
    }
}

/// Migration error types
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),

    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: String, found: String },

    #[error("Migration validation failed: {0}")]
    ValidationFailed(String),

    #[error("Data transformation error: {0}")]
    DataTransformation(String),

    #[error("Legacy database not found or inaccessible")]
    LegacyDatabaseNotFound,

    #[error("Target database is not empty, use force flag to overwrite")]
    TargetDatabaseNotEmpty,

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Source database URL (ratchet-lib)
    pub source_db_url: String,
    
    /// Target database URL (ratchet-storage)
    pub target_db_url: String,
    
    /// Batch size for processing records
    pub batch_size: u64,
    
    /// Whether to force migration even if target is not empty
    pub force: bool,
    
    /// Whether to validate data after migration
    pub validate: bool,
    
    /// Whether to create backup before migration
    pub create_backup: bool,
    
    /// Maximum number of retry attempts for failed records
    pub max_retries: u32,
    
    /// Whether to continue on errors or fail fast
    pub continue_on_error: bool,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            source_db_url: "sqlite://legacy.db".to_string(),
            target_db_url: "sqlite://modern.db".to_string(),
            batch_size: 1000,
            force: false,
            validate: true,
            create_backup: true,
            max_retries: 3,
            continue_on_error: false,
        }
    }
}

/// Overall migration summary
#[derive(Debug, Clone)]
pub struct MigrationSummary {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_duration_ms: u64,
    pub reports: Vec<MigrationReport>,
    pub success: bool,
    pub backup_path: Option<String>,
}

impl MigrationSummary {
    pub fn new() -> Self {
        Self {
            started_at: chrono::Utc::now(),
            completed_at: None,
            total_duration_ms: 0,
            reports: Vec::new(),
            success: false,
            backup_path: None,
        }
    }

    pub fn complete(&mut self, success: bool) {
        self.completed_at = Some(chrono::Utc::now());
        self.success = success;
        self.total_duration_ms = self.completed_at.unwrap()
            .signed_duration_since(self.started_at)
            .num_milliseconds() as u64;
    }

    pub fn total_migrated(&self) -> u64 {
        self.reports.iter().map(|r| r.migrated_count).sum()
    }

    pub fn total_failed(&self) -> u64 {
        self.reports.iter().map(|r| r.failed_count).sum()
    }

    pub fn overall_success_rate(&self) -> f64 {
        let total_processed: u64 = self.reports.iter().map(|r| r.total_processed()).sum();
        if total_processed == 0 {
            1.0
        } else {
            self.total_migrated() as f64 / total_processed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_report() {
        let mut report = MigrationReport::new("tasks".to_string());
        report.migrated_count = 100;
        report.failed_count = 5;
        report.skipped_count = 2;

        assert_eq!(report.total_processed(), 107);
        assert!((report.success_rate() - 0.934).abs() < 0.001);
    }

    #[test]
    fn test_migration_summary() {
        let mut summary = MigrationSummary::new();
        
        let mut report1 = MigrationReport::new("tasks".to_string());
        report1.migrated_count = 100;
        report1.failed_count = 5;
        
        let mut report2 = MigrationReport::new("executions".to_string());
        report2.migrated_count = 200;
        report2.failed_count = 0;
        
        summary.reports.push(report1);
        summary.reports.push(report2);
        summary.complete(true);

        assert_eq!(summary.total_migrated(), 300);
        assert_eq!(summary.total_failed(), 5);
        assert!((summary.overall_success_rate() - 0.984).abs() < 0.001);
        assert!(summary.success);
        assert!(summary.completed_at.is_some());
    }

    #[test]
    fn test_migration_config_default() {
        let config = MigrationConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert!(!config.force);
        assert!(config.validate);
        assert!(config.create_backup);
    }
}