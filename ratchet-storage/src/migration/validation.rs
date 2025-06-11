//! Migration validation utilities
//!
//! This module provides tools for validating data integrity after migration,
//! comparing source and target databases, and ensuring migration completeness.

use sea_orm::{DatabaseConnection, Statement, QueryResult, ConnectionTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::migration::{MigrationError, MigrationReport};

/// Validation report for migration integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Overall validation success
    pub success: bool,
    
    /// Entity-specific validation results
    pub entity_validations: Vec<EntityValidation>,
    
    /// Record count comparison between source and target
    pub record_counts: RecordCounts,
    
    /// Data integrity check results
    pub integrity_checks: Vec<IntegrityCheck>,
    
    /// Performance metrics
    pub validation_duration_ms: u64,
    
    /// Any validation errors encountered
    pub errors: Vec<String>,
}

/// Validation result for a specific entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityValidation {
    pub entity_type: String,
    pub source_count: u64,
    pub target_count: u64,
    pub matches: bool,
    pub sample_comparison: Option<SampleComparison>,
    pub errors: Vec<String>,
}

/// Record counts comparison between databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCounts {
    pub tasks: CountComparison,
    pub executions: CountComparison,
    pub jobs: CountComparison,
    pub schedules: CountComparison,
    pub delivery_results: CountComparison,
}

/// Count comparison for a single entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountComparison {
    pub source: u64,
    pub target: u64,
    pub matches: bool,
}

/// Data integrity check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub check_type: String,
    pub description: String,
    pub passed: bool,
    pub details: Option<String>,
}

/// Sample data comparison for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleComparison {
    pub sample_size: usize,
    pub matching_records: usize,
    pub mismatched_fields: Vec<String>,
    pub sample_errors: Vec<String>,
}

/// Migration data validator
pub struct MigrationValidator {
    source_db: DatabaseConnection,
    target_db: DatabaseConnection,
}

impl MigrationValidator {
    pub fn new(source_db: DatabaseConnection, target_db: DatabaseConnection) -> Self {
        Self {
            source_db,
            target_db,
        }
    }

    /// Perform complete migration validation
    pub async fn validate_migration(&self) -> Result<ValidationReport, MigrationError> {
        let start_time = std::time::Instant::now();
        let mut report = ValidationReport {
            success: true,
            entity_validations: Vec::new(),
            record_counts: self.compare_record_counts().await?,
            integrity_checks: Vec::new(),
            validation_duration_ms: 0,
            errors: Vec::new(),
        };

        // Validate each entity type
        let entities = ["tasks", "executions", "jobs", "schedules", "delivery_results"];
        for entity in &entities {
            match self.validate_entity(entity).await {
                Ok(validation) => {
                    if !validation.matches {
                        report.success = false;
                    }
                    report.entity_validations.push(validation);
                }
                Err(e) => {
                    report.success = false;
                    report.errors.push(format!("Failed to validate {}: {}", entity, e));
                }
            }
        }

        // Perform integrity checks
        report.integrity_checks = self.perform_integrity_checks().await?;
        for check in &report.integrity_checks {
            if !check.passed {
                report.success = false;
            }
        }

        report.validation_duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Count records in both databases
    pub async fn compare_record_counts(&self) -> Result<RecordCounts, MigrationError> {
        Ok(RecordCounts {
            tasks: self.count_comparison("tasks").await?,
            executions: self.count_comparison("executions").await?,
            jobs: self.count_comparison("jobs").await?,
            schedules: self.count_comparison("schedules").await?,
            delivery_results: self.count_comparison("delivery_results").await?,
        })
    }

    /// Verify data integrity with foreign key and constraint checks
    pub async fn perform_integrity_checks(&self) -> Result<Vec<IntegrityCheck>, MigrationError> {
        let mut checks = Vec::new();

        // Check foreign key integrity
        checks.push(self.check_task_execution_references().await?);
        checks.push(self.check_task_job_references().await?);
        checks.push(self.check_task_schedule_references().await?);
        checks.push(self.check_execution_delivery_references().await?);

        // Check unique constraints
        checks.push(self.check_unique_task_uuids().await?);
        checks.push(self.check_unique_execution_uuids().await?);

        // Check data consistency
        checks.push(self.check_json_field_validity().await?);
        checks.push(self.check_timestamp_consistency().await?);

        Ok(checks)
    }

    /// Sample and compare specific records between databases
    pub async fn sample_comparison(&self, entity: &str, sample_size: usize) -> Result<SampleComparison, MigrationError> {
        let source_records = self.get_sample_records(&self.source_db, entity, sample_size).await?;
        let target_records = self.get_sample_records(&self.target_db, entity, sample_size).await?;

        let mut matching_records = 0;
        let mut mismatched_fields = Vec::new();
        let mut sample_errors = Vec::new();

        for (i, (source_record, target_record)) in source_records.iter().zip(target_records.iter()).enumerate() {
            match self.compare_records(entity, source_record, target_record) {
                Ok(comparison) => {
                    if comparison.matches {
                        matching_records += 1;
                    } else {
                        mismatched_fields.extend(comparison.mismatched_fields);
                    }
                }
                Err(e) => {
                    sample_errors.push(format!("Record {}: {}", i, e));
                }
            }
        }

        Ok(SampleComparison {
            sample_size: source_records.len().min(target_records.len()),
            matching_records,
            mismatched_fields: mismatched_fields.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect(),
            sample_errors,
        })
    }

    // Private helper methods

    async fn validate_entity(&self, entity: &str) -> Result<EntityValidation, MigrationError> {
        let source_count = self.count_records(&self.source_db, entity).await?;
        let target_count = self.count_records(&self.target_db, entity).await?;
        let matches = source_count == target_count;

        let sample_comparison = if matches && source_count > 0 {
            let sample_size = (source_count.min(10)) as usize;
            Some(self.sample_comparison(entity, sample_size).await?)
        } else {
            None
        };

        Ok(EntityValidation {
            entity_type: entity.to_string(),
            source_count,
            target_count,
            matches,
            sample_comparison,
            errors: Vec::new(),
        })
    }

    async fn count_comparison(&self, table: &str) -> Result<CountComparison, MigrationError> {
        let source = self.count_records(&self.source_db, table).await?;
        let target = self.count_records(&self.target_db, table).await?;
        
        Ok(CountComparison {
            source,
            target,
            matches: source == target,
        })
    }

    async fn count_records(&self, db: &DatabaseConnection, table: &str) -> Result<u64, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("SELECT COUNT(*) as count FROM {}", table)
        );

        let result = db.query_one(stmt).await?;
        match result {
            Some(row) => {
                let count: i64 = row.try_get("", "count")?;
                Ok(count as u64)
            }
            None => Ok(0),
        }
    }

    async fn get_sample_records(&self, db: &DatabaseConnection, table: &str, limit: usize) -> Result<Vec<QueryResult>, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("SELECT * FROM {} ORDER BY id LIMIT {}", table, limit)
        );

        let results = db.query_all(stmt).await?;
        Ok(results)
    }

    fn compare_records(&self, entity: &str, source: &QueryResult, target: &QueryResult) -> Result<RecordComparison, MigrationError> {
        let mut comparison = RecordComparison {
            matches: true,
            mismatched_fields: Vec::new(),
        };

        // Compare based on entity type
        match entity {
            "tasks" => {
                self.compare_task_records(source, target, &mut comparison)?;
            }
            "executions" => {
                self.compare_execution_records(source, target, &mut comparison)?;
            }
            "jobs" => {
                self.compare_job_records(source, target, &mut comparison)?;
            }
            "schedules" => {
                self.compare_schedule_records(source, target, &mut comparison)?;
            }
            "delivery_results" => {
                self.compare_delivery_result_records(source, target, &mut comparison)?;
            }
            _ => {
                return Err(MigrationError::ValidationFailed(format!("Unknown entity type: {}", entity)));
            }
        }

        Ok(comparison)
    }

    fn compare_task_records(&self, source: &QueryResult, target: &QueryResult, comparison: &mut RecordComparison) -> Result<(), MigrationError> {
        // Compare key fields
        if !self.fields_match(source, target, "uuid")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("uuid".to_string());
        }
        if !self.fields_match(source, target, "name")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("name".to_string());
        }
        if !self.fields_match(source, target, "version")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("version".to_string());
        }
        // Note: path field might need special handling for task source migration
        
        Ok(())
    }

    fn compare_execution_records(&self, source: &QueryResult, target: &QueryResult, comparison: &mut RecordComparison) -> Result<(), MigrationError> {
        if !self.fields_match(source, target, "uuid")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("uuid".to_string());
        }
        if !self.fields_match(source, target, "task_id")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("task_id".to_string());
        }
        if !self.fields_match(source, target, "status")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("status".to_string());
        }
        
        Ok(())
    }

    fn compare_job_records(&self, source: &QueryResult, target: &QueryResult, comparison: &mut RecordComparison) -> Result<(), MigrationError> {
        if !self.fields_match(source, target, "uuid")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("uuid".to_string());
        }
        if !self.fields_match(source, target, "task_id")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("task_id".to_string());
        }
        if !self.fields_match(source, target, "status")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("status".to_string());
        }
        
        Ok(())
    }

    fn compare_schedule_records(&self, source: &QueryResult, target: &QueryResult, comparison: &mut RecordComparison) -> Result<(), MigrationError> {
        if !self.fields_match(source, target, "uuid")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("uuid".to_string());
        }
        if !self.fields_match(source, target, "task_id")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("task_id".to_string());
        }
        if !self.fields_match(source, target, "cron_expression")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("cron_expression".to_string());
        }
        
        Ok(())
    }

    fn compare_delivery_result_records(&self, source: &QueryResult, target: &QueryResult, comparison: &mut RecordComparison) -> Result<(), MigrationError> {
        if !self.fields_match(source, target, "execution_id")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("execution_id".to_string());
        }
        if !self.fields_match(source, target, "destination_type")? {
            comparison.matches = false;
            comparison.mismatched_fields.push("destination_type".to_string());
        }
        
        Ok(())
    }

    fn fields_match(&self, source: &QueryResult, target: &QueryResult, field: &str) -> Result<bool, MigrationError> {
        let source_value: Option<String> = source.try_get("", field).ok();
        let target_value: Option<String> = target.try_get("", field).ok();
        Ok(source_value == target_value)
    }

    // Integrity check implementations

    async fn check_task_execution_references(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM executions e LEFT JOIN tasks t ON e.task_id = t.id WHERE t.id IS NULL".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let orphan_count: i64 = result
            .map(|row| row.try_get("", "count").unwrap_or(0))
            .unwrap_or(0);

        Ok(IntegrityCheck {
            check_type: "foreign_key".to_string(),
            description: "Task-Execution foreign key integrity".to_string(),
            passed: orphan_count == 0,
            details: if orphan_count > 0 {
                Some(format!("{} orphaned execution records found", orphan_count))
            } else {
                None
            },
        })
    }

    async fn check_task_job_references(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM jobs j LEFT JOIN tasks t ON j.task_id = t.id WHERE t.id IS NULL".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let orphan_count: i64 = result
            .map(|row| row.try_get("", "count").unwrap_or(0))
            .unwrap_or(0);

        Ok(IntegrityCheck {
            check_type: "foreign_key".to_string(),
            description: "Task-Job foreign key integrity".to_string(),
            passed: orphan_count == 0,
            details: if orphan_count > 0 {
                Some(format!("{} orphaned job records found", orphan_count))
            } else {
                None
            },
        })
    }

    async fn check_task_schedule_references(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM schedules s LEFT JOIN tasks t ON s.task_id = t.id WHERE t.id IS NULL".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let orphan_count: i64 = result
            .map(|row| row.try_get("", "count").unwrap_or(0))
            .unwrap_or(0);

        Ok(IntegrityCheck {
            check_type: "foreign_key".to_string(),
            description: "Task-Schedule foreign key integrity".to_string(),
            passed: orphan_count == 0,
            details: if orphan_count > 0 {
                Some(format!("{} orphaned schedule records found", orphan_count))
            } else {
                None
            },
        })
    }

    async fn check_execution_delivery_references(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM delivery_results dr LEFT JOIN executions e ON dr.execution_id = e.id WHERE e.id IS NULL".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let orphan_count: i64 = result
            .map(|row| row.try_get("", "count").unwrap_or(0))
            .unwrap_or(0);

        Ok(IntegrityCheck {
            check_type: "foreign_key".to_string(),
            description: "Execution-DeliveryResult foreign key integrity".to_string(),
            passed: orphan_count == 0,
            details: if orphan_count > 0 {
                Some(format!("{} orphaned delivery result records found", orphan_count))
            } else {
                None
            },
        })
    }

    async fn check_unique_task_uuids(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as total, COUNT(DISTINCT uuid) as unique_count FROM tasks".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let (total, unique): (i64, i64) = result
            .map(|row| {
                (
                    row.try_get("", "total").unwrap_or(0),
                    row.try_get("", "unique_count").unwrap_or(0),
                )
            })
            .unwrap_or((0, 0));

        Ok(IntegrityCheck {
            check_type: "unique_constraint".to_string(),
            description: "Task UUID uniqueness".to_string(),
            passed: total == unique,
            details: if total != unique {
                Some(format!("{} duplicate UUIDs found", total - unique))
            } else {
                None
            },
        })
    }

    async fn check_unique_execution_uuids(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as total, COUNT(DISTINCT uuid) as unique_count FROM executions".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let (total, unique): (i64, i64) = result
            .map(|row| {
                (
                    row.try_get("", "total").unwrap_or(0),
                    row.try_get("", "unique_count").unwrap_or(0),
                )
            })
            .unwrap_or((0, 0));

        Ok(IntegrityCheck {
            check_type: "unique_constraint".to_string(),
            description: "Execution UUID uniqueness".to_string(),
            passed: total == unique,
            details: if total != unique {
                Some(format!("{} duplicate UUIDs found", total - unique))
            } else {
                None
            },
        })
    }

    async fn check_json_field_validity(&self) -> Result<IntegrityCheck, MigrationError> {
        let tables_and_fields = [
            ("tasks", "metadata"),
            ("tasks", "input_schema"),
            ("tasks", "output_schema"),
            ("executions", "input_data"),
            ("executions", "output_data"),
            ("jobs", "input_data"),
            ("schedules", "input_data"),
        ];

        let mut invalid_count = 0;
        let mut errors = Vec::new();

        for (table, field) in &tables_and_fields {
            let stmt = Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                format!("SELECT id, {} FROM {} WHERE {} IS NOT NULL", field, table, field)
            );

            let results = self.target_db.query_all(stmt).await?;
            for result in results {
                let id: i32 = result.try_get("", "id").unwrap_or(0);
                let json_str: String = result.try_get("", field).unwrap_or_default();
                
                if !json_str.is_empty() {
                    if let Err(_) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        invalid_count += 1;
                        errors.push(format!("{}.{} record {} has invalid JSON", table, field, id));
                    }
                }
            }
        }

        Ok(IntegrityCheck {
            check_type: "data_validity".to_string(),
            description: "JSON field validity".to_string(),
            passed: invalid_count == 0,
            details: if invalid_count > 0 {
                Some(format!("{} invalid JSON fields found: {}", invalid_count, errors.join(", ")))
            } else {
                None
            },
        })
    }

    async fn check_timestamp_consistency(&self) -> Result<IntegrityCheck, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM tasks WHERE created_at > updated_at".to_string()
        );

        let result = self.target_db.query_one(stmt).await?;
        let inconsistent_count: i64 = result
            .map(|row| row.try_get("", "count").unwrap_or(0))
            .unwrap_or(0);

        Ok(IntegrityCheck {
            check_type: "data_consistency".to_string(),
            description: "Timestamp consistency (created_at <= updated_at)".to_string(),
            passed: inconsistent_count == 0,
            details: if inconsistent_count > 0 {
                Some(format!("{} records with inconsistent timestamps", inconsistent_count))
            } else {
                None
            },
        })
    }
}

/// Record comparison result
struct RecordComparison {
    matches: bool,
    mismatched_fields: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestDatabase;

    #[tokio::test]
    async fn test_record_count_comparison() {
        let source_db = TestDatabase::new().await.unwrap();
        let target_db = TestDatabase::new().await.unwrap();

        let validator = MigrationValidator::new(
            source_db.connection.clone(),
            target_db.connection.clone(),
        );

        let counts = validator.compare_record_counts().await.unwrap();
        
        // Both databases should start empty
        assert_eq!(counts.tasks.source, 0);
        assert_eq!(counts.tasks.target, 0);
        assert!(counts.tasks.matches);
    }

    #[tokio::test]
    async fn test_integrity_checks() {
        let source_db = TestDatabase::new().await.unwrap();
        let target_db = TestDatabase::new().await.unwrap();

        let validator = MigrationValidator::new(
            source_db.connection.clone(),
            target_db.connection.clone(),
        );

        let checks = validator.perform_integrity_checks().await.unwrap();
        
        // All checks should pass on empty databases
        for check in checks {
            assert!(check.passed, "Integrity check failed: {}", check.description);
        }
    }
}