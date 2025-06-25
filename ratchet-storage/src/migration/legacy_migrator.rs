//! Legacy data migration from ratchet-lib to ratchet-storage
//!
//! This module provides the main migration logic for transforming data from
//! the legacy ratchet-lib database format to the modern ratchet-storage format.

use sea_orm::DatabaseConnection;
use std::time::Instant;
use uuid::Uuid;

use crate::migration::{
    DatabaseSystem, MigrationConfig, MigrationError, MigrationReport, MigrationSummary, MigrationValidator,
    SchemaVersion, SchemaVersionDetector,
};

/// Main legacy data migrator
pub struct LegacyMigrator {
    config: MigrationConfig,
    source_db: DatabaseConnection,
    target_db: DatabaseConnection,
    schema_detector: SchemaVersionDetector,
    validator: MigrationValidator,
}

impl LegacyMigrator {
    /// Create a new legacy migrator
    pub async fn new(config: MigrationConfig) -> Result<Self, MigrationError> {
        // Connect to source database
        let source_db = sea_orm::Database::connect(&config.source_db_url).await?;

        // Connect to target database
        let target_db = sea_orm::Database::connect(&config.target_db_url).await?;

        // Create schema detector for source database
        let schema_detector = SchemaVersionDetector::new(source_db.clone());

        // Create validator
        let validator = MigrationValidator::new(source_db.clone(), target_db.clone());

        Ok(Self {
            config,
            source_db,
            target_db,
            schema_detector,
            validator,
        })
    }

    /// Perform complete migration with validation
    pub async fn migrate(&self) -> Result<MigrationSummary, MigrationError> {
        let mut summary = MigrationSummary::new();

        // Pre-migration validation
        self.validate_pre_migration().await?;

        // Create backup if requested
        if self.config.create_backup {
            summary.backup_path = Some(self.create_backup().await?);
        }

        // Migrate each entity type in dependency order
        let migration_order = ["tasks", "executions", "jobs", "schedules", "delivery_results"];

        for entity_type in &migration_order {
            let report = match *entity_type {
                "tasks" => self.migrate_tasks().await?,
                "executions" => self.migrate_executions().await?,
                "jobs" => self.migrate_jobs().await?,
                "schedules" => self.migrate_schedules().await?,
                "delivery_results" => self.migrate_delivery_results().await?,
                _ => unreachable!(),
            };

            // Stop on first failure if not configured to continue
            let should_stop = !self.config.continue_on_error && report.failed_count > 0;

            summary.reports.push(report);

            if should_stop {
                summary.complete(false);
                return Ok(summary);
            }
        }

        // Post-migration validation
        if self.config.validate {
            let validation_report = self.validator.validate_migration().await?;
            if !validation_report.success {
                summary.complete(false);
                return Err(MigrationError::ValidationFailed(format!(
                    "Post-migration validation failed: {:?}",
                    validation_report.errors
                )));
            }
        }

        // Record migration metadata
        self.record_migration_completion().await?;

        summary.complete(true);
        Ok(summary)
    }

    /// Migrate tasks from legacy to modern format
    pub async fn migrate_tasks(&self) -> Result<MigrationReport, MigrationError> {
        let start_time = Instant::now();
        let mut report = MigrationReport::new("tasks".to_string());

        // Get all tasks from source database
        let source_tasks = self.get_legacy_tasks().await?;

        for task in source_tasks {
            match self.migrate_single_task(task).await {
                Ok(_) => report.migrated_count += 1,
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Task {}: {}", report.failed_count, e));

                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Migrate executions from legacy to modern format
    pub async fn migrate_executions(&self) -> Result<MigrationReport, MigrationError> {
        let start_time = Instant::now();
        let mut report = MigrationReport::new("executions".to_string());

        // Get all executions from source database
        let source_executions = self.get_legacy_executions().await?;

        for execution in source_executions {
            match self.migrate_single_execution(execution).await {
                Ok(_) => report.migrated_count += 1,
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Execution {}: {}", report.failed_count, e));

                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Migrate jobs from legacy to modern format
    pub async fn migrate_jobs(&self) -> Result<MigrationReport, MigrationError> {
        let start_time = Instant::now();
        let mut report = MigrationReport::new("jobs".to_string());

        // Get all jobs from source database
        let source_jobs = self.get_legacy_jobs().await?;

        for job in source_jobs {
            match self.migrate_single_job(job).await {
                Ok(_) => report.migrated_count += 1,
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Job {}: {}", report.failed_count, e));

                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Migrate schedules from legacy to modern format
    pub async fn migrate_schedules(&self) -> Result<MigrationReport, MigrationError> {
        let start_time = Instant::now();
        let mut report = MigrationReport::new("schedules".to_string());

        // Get all schedules from source database
        let source_schedules = self.get_legacy_schedules().await?;

        for schedule in source_schedules {
            match self.migrate_single_schedule(schedule).await {
                Ok(_) => report.migrated_count += 1,
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Schedule {}: {}", report.failed_count, e));

                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Migrate delivery results from legacy to modern format
    pub async fn migrate_delivery_results(&self) -> Result<MigrationReport, MigrationError> {
        let start_time = Instant::now();
        let mut report = MigrationReport::new("delivery_results".to_string());

        // Get all delivery results from source database
        let source_results = self.get_legacy_delivery_results().await?;

        for result in source_results {
            match self.migrate_single_delivery_result(result).await {
                Ok(_) => report.migrated_count += 1,
                Err(e) => {
                    report.failed_count += 1;
                    report
                        .errors
                        .push(format!("DeliveryResult {}: {}", report.failed_count, e));

                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    // Private implementation methods

    async fn validate_pre_migration(&self) -> Result<(), MigrationError> {
        // Check source database is legacy format
        let source_version = self.schema_detector.detect_version().await?;
        if source_version.system != DatabaseSystem::RatchetLib {
            return Err(MigrationError::SchemaVersionMismatch {
                expected: "ratchet-lib".to_string(),
                found: source_version.system.to_string(),
            });
        }

        // Check target database compatibility
        let target_detector = SchemaVersionDetector::new(self.target_db.clone());
        if !target_detector.is_empty_database().await? && !self.config.force {
            return Err(MigrationError::TargetDatabaseNotEmpty);
        }

        Ok(())
    }

    async fn create_backup(&self) -> Result<String, MigrationError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("backup_{}_{}.db", "ratchet_migration", timestamp);

        // For SQLite, we can use the backup API or simple file copy
        // This is a simplified implementation
        std::fs::copy(self.config.source_db_url.trim_start_matches("sqlite://"), &backup_path)?;

        Ok(backup_path)
    }

    async fn record_migration_completion(&self) -> Result<(), MigrationError> {
        let target_detector = SchemaVersionDetector::new(self.target_db.clone());
        let version = SchemaVersion {
            version: "migrated_from_legacy".to_string(),
            description: "Migrated from ratchet-lib to ratchet-storage".to_string(),
            applied_at: chrono::Utc::now(),
            system: DatabaseSystem::RatchetStorage,
            applied_migrations: vec![], // Would be populated from target schema
        };

        target_detector.record_migration_metadata(&version).await?;
        Ok(())
    }

    // Entity retrieval methods (these would use the actual entity imports)
    async fn get_legacy_tasks(&self) -> Result<Vec<LegacyTask>, MigrationError> {
        // This would use: use ratchet_lib::database::entities::tasks::Entity as TaskEntity;
        // For now, return empty to avoid import issues
        Ok(Vec::new())
    }

    async fn get_legacy_executions(&self) -> Result<Vec<LegacyExecution>, MigrationError> {
        Ok(Vec::new())
    }

    async fn get_legacy_jobs(&self) -> Result<Vec<LegacyJob>, MigrationError> {
        Ok(Vec::new())
    }

    async fn get_legacy_schedules(&self) -> Result<Vec<LegacySchedule>, MigrationError> {
        Ok(Vec::new())
    }

    async fn get_legacy_delivery_results(&self) -> Result<Vec<LegacyDeliveryResult>, MigrationError> {
        Ok(Vec::new())
    }

    // Entity migration methods
    async fn migrate_single_task(&self, _legacy_task: LegacyTask) -> Result<(), MigrationError> {
        // Convert legacy task to modern format
        // This would involve field mapping and data transformation
        // For now, return Ok to avoid import issues
        Ok(())
    }

    async fn migrate_single_execution(&self, _legacy_execution: LegacyExecution) -> Result<(), MigrationError> {
        Ok(())
    }

    async fn migrate_single_job(&self, _legacy_job: LegacyJob) -> Result<(), MigrationError> {
        Ok(())
    }

    async fn migrate_single_schedule(&self, _legacy_schedule: LegacySchedule) -> Result<(), MigrationError> {
        Ok(())
    }

    async fn migrate_single_delivery_result(&self, _legacy_result: LegacyDeliveryResult) -> Result<(), MigrationError> {
        Ok(())
    }
}

// Temporary placeholder types to avoid import issues
// In the real implementation, these would import from ratchet-lib
#[derive(Debug, Clone)]
struct LegacyTask {
    pub id: i32,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub path: String,
    // ... other fields
}

#[derive(Debug, Clone)]
struct LegacyExecution {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    // ... other fields
}

#[derive(Debug, Clone)]
struct LegacyJob {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    // ... other fields
}

#[derive(Debug, Clone)]
struct LegacySchedule {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    // ... other fields
}

#[derive(Debug, Clone)]
struct LegacyDeliveryResult {
    pub id: i32,
    pub execution_id: i32,
    // ... other fields
}

/// Data transformation utilities for converting between legacy and modern formats
pub struct DataTransformer;

impl DataTransformer {
    /// Transform a legacy task path to a modern task source
    pub fn transform_task_source(legacy_path: &str) -> Result<String, MigrationError> {
        // Convert path-based task references to source type
        if legacy_path.starts_with("http://") || legacy_path.starts_with("https://") {
            Ok(format!("url:{}", legacy_path))
        } else if legacy_path.ends_with(".js") {
            Ok(format!("file:{}", legacy_path))
        } else if legacy_path.starts_with("plugin://") {
            Ok(legacy_path.to_string()) // Already in correct format
        } else {
            // Assume directory-based task
            Ok(format!("file:{}", legacy_path))
        }
    }

    /// Transform legacy metadata format to modern format
    pub fn transform_task_metadata(legacy_metadata: &serde_json::Value) -> Result<serde_json::Value, MigrationError> {
        // Convert field mappings from legacy to modern format
        let mut modern_metadata = serde_json::Map::new();

        if let Some(obj) = legacy_metadata.as_object() {
            // Map legacy fields to modern equivalents
            if let Some(label) = obj.get("label") {
                modern_metadata.insert("name".to_string(), label.clone());
            }
            if let Some(description) = obj.get("description") {
                modern_metadata.insert("description".to_string(), description.clone());
            }
            // Copy other fields as-is
            for (key, value) in obj {
                if !["label"].contains(&key.as_str()) {
                    modern_metadata.insert(key.clone(), value.clone());
                }
            }
        }

        Ok(serde_json::Value::Object(modern_metadata))
    }

    /// Validate JSON field integrity
    pub fn validate_json_field(json_str: &str) -> Result<serde_json::Value, MigrationError> {
        serde_json::from_str(json_str).map_err(|e| MigrationError::DataTransformation(format!("Invalid JSON: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_source_transformation() {
        let http_url = "https://example.com/task.js";
        let result = DataTransformer::transform_task_source(http_url).unwrap();
        assert_eq!(result, "url:https://example.com/task.js");

        let file_path = "/path/to/task.js";
        let result = DataTransformer::transform_task_source(file_path).unwrap();
        assert_eq!(result, "file:/path/to/task.js");

        let plugin_path = "plugin://core:example";
        let result = DataTransformer::transform_task_source(plugin_path).unwrap();
        assert_eq!(result, "plugin://core:example");
    }

    #[test]
    fn test_metadata_transformation() {
        let legacy_metadata = serde_json::json!({
            "label": "My Task",
            "description": "A test task",
            "version": "1.0.0",
            "other_field": "value"
        });

        let modern_metadata = DataTransformer::transform_task_metadata(&legacy_metadata).unwrap();

        assert_eq!(modern_metadata["name"], "My Task");
        assert_eq!(modern_metadata["description"], "A test task");
        assert_eq!(modern_metadata["version"], "1.0.0");
        assert_eq!(modern_metadata["other_field"], "value");
        assert!(modern_metadata.get("label").is_none());
    }

    #[test]
    fn test_json_validation() {
        let valid_json = r#"{"test": "value"}"#;
        let result = DataTransformer::validate_json_field(valid_json);
        assert!(result.is_ok());

        let invalid_json = r#"{"test": value}"#;
        let result = DataTransformer::validate_json_field(invalid_json);
        assert!(result.is_err());
    }
}
