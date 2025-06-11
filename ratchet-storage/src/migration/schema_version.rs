//! Database schema version detection and management
//!
//! This module provides utilities for detecting database schema versions,
//! managing migration state, and ensuring compatibility between different
//! versions of the database schema.

use sea_orm::{DatabaseConnection, Statement, QueryResult, Value, ConnectionTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::migration::{MigrationError, MigrationReport};

/// Database schema version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Version identifier (e.g., "1.0.0", "20241201_000005")
    pub version: String,
    
    /// Human-readable description
    pub description: String,
    
    /// When this version was applied
    pub applied_at: chrono::DateTime<chrono::Utc>,
    
    /// Whether this is a ratchet-lib or ratchet-storage schema
    pub system: DatabaseSystem,
    
    /// List of migrations that have been applied
    pub applied_migrations: Vec<String>,
}

/// Database system identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseSystem {
    RatchetLib,
    RatchetStorage,
    Unknown,
}

impl std::fmt::Display for DatabaseSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseSystem::RatchetLib => write!(f, "ratchet-lib"),
            DatabaseSystem::RatchetStorage => write!(f, "ratchet-storage"),
            DatabaseSystem::Unknown => write!(f, "unknown"),
        }
    }
}

/// Schema version detector
pub struct SchemaVersionDetector {
    connection: DatabaseConnection,
}

impl SchemaVersionDetector {
    pub fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Detect the current schema version of the database
    pub async fn detect_version(&self) -> Result<SchemaVersion, MigrationError> {
        // Check if migration metadata table exists
        if self.has_migration_metadata_table().await? {
            self.get_version_from_metadata().await
        } else {
            self.detect_version_from_schema().await
        }
    }

    /// Check if this is a ratchet-lib database
    pub async fn is_legacy_database(&self) -> Result<bool, MigrationError> {
        let version = self.detect_version().await?;
        Ok(version.system == DatabaseSystem::RatchetLib)
    }

    /// Check if this is a ratchet-storage database
    pub async fn is_modern_database(&self) -> Result<bool, MigrationError> {
        let version = self.detect_version().await?;
        Ok(version.system == DatabaseSystem::RatchetStorage)
    }

    /// Check if the database is empty
    pub async fn is_empty_database(&self) -> Result<bool, MigrationError> {
        let tables = self.get_table_list().await?;
        Ok(tables.is_empty() || tables.iter().all(|t| self.is_system_table(t)))
    }

    /// Get list of applied migrations
    pub async fn get_applied_migrations(&self) -> Result<Vec<String>, MigrationError> {
        if self.has_seaorm_migration_table().await? {
            self.get_seaorm_migrations().await
        } else {
            // Legacy detection based on table structure
            self.detect_migrations_from_schema().await
        }
    }

    /// Create or update migration metadata
    pub async fn record_migration_metadata(&self, version: &SchemaVersion) -> Result<(), MigrationError> {
        // Create migration_metadata table if it doesn't exist
        self.create_migration_metadata_table().await?;
        
        // Insert or update version information
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!(
                r#"INSERT OR REPLACE INTO migration_metadata 
                   (version, description, applied_at, system, applied_migrations) 
                   VALUES ('{}', '{}', '{}', '{}', '{}')"#,
                version.version,
                version.description,
                version.applied_at.to_rfc3339(),
                version.system,
                serde_json::to_string(&version.applied_migrations)?
            )
        );
        
        self.connection.execute(stmt).await?;
        Ok(())
    }

    /// Validate schema compatibility for migration
    pub async fn validate_migration_compatibility(
        &self,
        source_version: &SchemaVersion,
        target_version: &SchemaVersion,
    ) -> Result<bool, MigrationError> {
        // Check if source is legacy and target is modern
        if source_version.system == DatabaseSystem::RatchetLib 
            && target_version.system == DatabaseSystem::RatchetStorage {
            return Ok(true);
        }

        // Check if both systems are the same and target is newer
        if source_version.system == target_version.system {
            return Ok(self.is_version_newer(&target_version.version, &source_version.version));
        }

        // Other combinations are not supported
        Ok(false)
    }

    // Private helper methods

    async fn has_migration_metadata_table(&self) -> Result<bool, MigrationError> {
        let tables = self.get_table_list().await?;
        Ok(tables.contains(&"migration_metadata".to_string()))
    }

    async fn has_seaorm_migration_table(&self) -> Result<bool, MigrationError> {
        let tables = self.get_table_list().await?;
        Ok(tables.contains(&"seaql_migrations".to_string()))
    }

    async fn get_table_list(&self) -> Result<Vec<String>, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table'".to_string()
        );
        
        let results = self.connection.query_all(stmt).await?;
        let tables: Vec<String> = results
            .into_iter()
            .filter_map(|row| row.try_get("", "name").ok())
            .collect();
        
        Ok(tables)
    }

    async fn get_version_from_metadata(&self) -> Result<SchemaVersion, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT * FROM migration_metadata ORDER BY applied_at DESC LIMIT 1".to_string()
        );
        
        let result = self.connection.query_one(stmt).await?;
        
        match result {
            Some(row) => {
                let version: String = row.try_get("", "version")?;
                let description: String = row.try_get("", "description")?;
                let applied_at_str: String = row.try_get("", "applied_at")?;
                let system_str: String = row.try_get("", "system")?;
                let migrations_json: String = row.try_get("", "applied_migrations")?;
                
                let applied_at = chrono::DateTime::parse_from_rfc3339(&applied_at_str)
                    .map_err(|e| MigrationError::DataTransformation(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                
                let system = match system_str.as_str() {
                    "ratchet-lib" => DatabaseSystem::RatchetLib,
                    "ratchet-storage" => DatabaseSystem::RatchetStorage,
                    _ => DatabaseSystem::Unknown,
                };
                
                let applied_migrations: Vec<String> = serde_json::from_str(&migrations_json)?;
                
                Ok(SchemaVersion {
                    version,
                    description,
                    applied_at,
                    system,
                    applied_migrations,
                })
            }
            None => Err(MigrationError::ValidationFailed("No migration metadata found".to_string())),
        }
    }

    async fn detect_version_from_schema(&self) -> Result<SchemaVersion, MigrationError> {
        let tables = self.get_table_list().await?;
        let applied_migrations = self.get_applied_migrations().await?;
        
        // Determine system based on table structure and migration history
        let system = if self.has_seaorm_migration_table().await? {
            // Check if this looks like a ratchet-storage database
            if tables.contains(&"delivery_results".to_string()) {
                DatabaseSystem::RatchetStorage
            } else {
                DatabaseSystem::RatchetLib
            }
        } else if !tables.is_empty() {
            // Assume legacy system if has tables but no migration tracking
            DatabaseSystem::RatchetLib
        } else {
            DatabaseSystem::Unknown
        };

        // Determine version based on latest migration
        let version = applied_migrations
            .last()
            .unwrap_or(&"unknown".to_string())
            .clone();

        Ok(SchemaVersion {
            version: version.clone(),
            description: format!("Auto-detected {} schema", system),
            applied_at: chrono::Utc::now(),
            system,
            applied_migrations,
        })
    }

    async fn get_seaorm_migrations(&self) -> Result<Vec<String>, MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations ORDER BY version".to_string()
        );
        
        let results = self.connection.query_all(stmt).await?;
        let migrations: Vec<String> = results
            .into_iter()
            .filter_map(|row| row.try_get("", "version").ok())
            .collect();
        
        Ok(migrations)
    }

    async fn detect_migrations_from_schema(&self) -> Result<Vec<String>, MigrationError> {
        let tables = self.get_table_list().await?;
        let mut migrations = Vec::new();

        // Infer migrations based on table existence
        if tables.contains(&"tasks".to_string()) {
            migrations.push("m20241201_000001_create_tasks_table".to_string());
        }
        if tables.contains(&"executions".to_string()) {
            migrations.push("m20241201_000002_create_executions_table".to_string());
        }
        if tables.contains(&"schedules".to_string()) {
            migrations.push("m20241201_000003_create_schedules_table".to_string());
        }
        if tables.contains(&"jobs".to_string()) {
            migrations.push("m20241201_000004_create_jobs_table".to_string());
        }
        if tables.contains(&"delivery_results".to_string()) {
            migrations.push("m20250106_000001_add_output_destinations".to_string());
        }

        Ok(migrations)
    }

    async fn create_migration_metadata_table(&self) -> Result<(), MigrationError> {
        let stmt = Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            r#"CREATE TABLE IF NOT EXISTS migration_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                system TEXT NOT NULL,
                applied_migrations TEXT NOT NULL
            )"#.to_string()
        );
        
        self.connection.execute(stmt).await?;
        Ok(())
    }

    fn is_system_table(&self, table_name: &str) -> bool {
        matches!(table_name, "sqlite_master" | "sqlite_temp_master" | "sqlite_sequence")
    }

    fn is_version_newer(&self, version1: &str, version2: &str) -> bool {
        // Simple lexicographic comparison for migration versions
        // In practice, you might want more sophisticated version comparison
        version1 > version2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestDatabase;

    #[tokio::test]
    async fn test_empty_database_detection() {
        let test_db = TestDatabase::new().await.unwrap();
        let detector = SchemaVersionDetector::new(test_db.connection.clone());

        // Initially empty (only has migration tables)
        let is_empty = detector.is_empty_database().await.unwrap();
        assert!(!is_empty); // Has seaql_migrations table from TestDatabase setup
    }

    #[tokio::test]
    async fn test_migration_metadata_creation() {
        let test_db = TestDatabase::new().await.unwrap();
        let detector = SchemaVersionDetector::new(test_db.connection.clone());

        let version = SchemaVersion {
            version: "test_1.0.0".to_string(),
            description: "Test version".to_string(),
            applied_at: chrono::Utc::now(),
            system: DatabaseSystem::RatchetStorage,
            applied_migrations: vec!["m20241201_000001_create_tasks_table".to_string()],
        };

        detector.record_migration_metadata(&version).await.unwrap();

        // Verify we can read it back
        let detected = detector.get_version_from_metadata().await.unwrap();
        assert_eq!(detected.version, version.version);
        assert_eq!(detected.system, version.system);
    }

    #[tokio::test]
    async fn test_migration_compatibility() {
        let test_db = TestDatabase::new().await.unwrap();
        let detector = SchemaVersionDetector::new(test_db.connection.clone());

        let legacy_version = SchemaVersion {
            version: "1.0.0".to_string(),
            description: "Legacy".to_string(),
            applied_at: chrono::Utc::now(),
            system: DatabaseSystem::RatchetLib,
            applied_migrations: vec![],
        };

        let modern_version = SchemaVersion {
            version: "2.0.0".to_string(),
            description: "Modern".to_string(),
            applied_at: chrono::Utc::now(),
            system: DatabaseSystem::RatchetStorage,
            applied_migrations: vec![],
        };

        let compatible = detector
            .validate_migration_compatibility(&legacy_version, &modern_version)
            .await
            .unwrap();
        
        assert!(compatible);
    }
}