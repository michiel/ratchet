//! Database migration management

use async_trait::async_trait;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::{StorageResult, StorageError, connection::Connection};

/// Migration trait
#[async_trait]
pub trait Migration: Send + Sync {
    /// Get migration ID (unique identifier)
    fn id(&self) -> &str;
    
    /// Get migration description
    fn description(&self) -> &str;
    
    /// Execute the migration (up direction)
    async fn up(&self, connection: &dyn Connection) -> StorageResult<()>;
    
    /// Rollback the migration (down direction)
    async fn down(&self, connection: &dyn Connection) -> StorageResult<()>;
    
    /// Check if migration can be rolled back
    fn can_rollback(&self) -> bool {
        true
    }
    
    /// Get migration dependencies (other migration IDs that must run first)
    fn dependencies(&self) -> Vec<&str> {
        Vec::new()
    }
}

/// Migration record for tracking applied migrations
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub id: String,
    pub description: String,
    pub applied_at: DateTime<Utc>,
    pub checksum: String,
}

/// Migration manager
pub struct MigrationManager {
    migrations: HashMap<String, Box<dyn Migration>>,
    table_name: String,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new() -> Self {
        Self {
            migrations: HashMap::new(),
            table_name: "schema_migrations".to_string(),
        }
    }
    
    /// Set custom migration table name
    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = table_name.into();
        self
    }
    
    /// Register a migration
    pub fn add_migration(mut self, migration: Box<dyn Migration>) -> Self {
        let id = migration.id().to_string();
        self.migrations.insert(id, migration);
        self
    }
    
    /// Initialize migration system (create migration table if needed)
    pub async fn initialize(&self, connection: &dyn Connection) -> StorageResult<()> {
        let query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id VARCHAR(255) PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TIMESTAMP NOT NULL,
                checksum VARCHAR(64) NOT NULL
            )
            "#,
            self.table_name
        );
        
        connection.execute(&query, &[]).await?;
        log::info!("Migration system initialized with table: {}", self.table_name);
        Ok(())
    }
    
    /// Get all applied migrations
    pub async fn get_applied_migrations(&self, connection: &dyn Connection) -> StorageResult<Vec<MigrationRecord>> {
        let query = format!(
            "SELECT id, description, applied_at, checksum FROM {} ORDER BY applied_at",
            self.table_name
        );
        
        // In a real implementation, this would parse the database results
        // For now, return empty list
        log::debug!("Getting applied migrations from table: {}", self.table_name);
        Ok(Vec::new())
    }
    
    /// Get pending migrations (not yet applied)
    pub async fn get_pending_migrations(&self, connection: &dyn Connection) -> StorageResult<Vec<&str>> {
        let applied = self.get_applied_migrations(connection).await?;
        let applied_ids: std::collections::HashSet<String> = 
            applied.into_iter().map(|m| m.id).collect();
        
        let mut pending: Vec<&str> = self.migrations
            .keys()
            .filter(|id| !applied_ids.contains(*id))
            .map(|id| id.as_str())
            .collect();
        
        // Sort by dependencies
        pending.sort_by(|a, b| {
            let deps_a = self.migrations[*a].dependencies();
            let deps_b = self.migrations[*b].dependencies();
            
            if deps_a.contains(b) {
                std::cmp::Ordering::Greater
            } else if deps_b.contains(a) {
                std::cmp::Ordering::Less
            } else {
                a.cmp(b) // Fallback to alphabetical
            }
        });
        
        Ok(pending)
    }
    
    /// Run all pending migrations
    pub async fn migrate(&self, connection: &dyn Connection) -> StorageResult<usize> {
        self.initialize(connection).await?;
        
        let pending = self.get_pending_migrations(connection).await?;
        let count = pending.len();
        
        if count == 0 {
            log::info!("No pending migrations to run");
            return Ok(0);
        }
        
        log::info!("Running {} pending migrations", count);
        
        for migration_id in pending {
            self.run_migration(connection, migration_id).await?;
        }
        
        log::info!("Successfully applied {} migrations", count);
        Ok(count)
    }
    
    /// Run a specific migration
    async fn run_migration(&self, connection: &dyn Connection, migration_id: &str) -> StorageResult<()> {
        let migration = self.migrations.get(migration_id)
            .ok_or_else(|| StorageError::MigrationFailed(
                format!("Migration not found: {}", migration_id)
            ))?;
        
        // Check dependencies
        for dep_id in migration.dependencies() {
            if !self.is_migration_applied(connection, dep_id).await? {
                return Err(StorageError::MigrationFailed(
                    format!("Migration {} depends on {} which is not applied", migration_id, dep_id)
                ));
            }
        }
        
        log::info!("Running migration: {} - {}", migration_id, migration.description());
        
        // Execute migration in transaction
        let tx = connection.begin_transaction().await?;
        
        match migration.up(connection).await {
            Ok(()) => {
                // Record successful migration
                self.record_migration(connection, migration_id, migration.description()).await?;
                tx.commit().await?;
                log::info!("Migration completed: {}", migration_id);
                Ok(())
            }
            Err(error) => {
                tx.rollback().await?;
                log::error!("Migration failed: {} - {}", migration_id, error);
                Err(StorageError::MigrationFailed(
                    format!("Migration {} failed: {}", migration_id, error)
                ))
            }
        }
    }
    
    /// Check if a migration has been applied
    async fn is_migration_applied(&self, connection: &dyn Connection, migration_id: &str) -> StorageResult<bool> {
        let query = format!(
            "SELECT COUNT(*) FROM {} WHERE id = ?",
            self.table_name
        );
        
        // In a real implementation, this would execute the query
        // For now, return false (not applied)
        log::debug!("Checking if migration is applied: {}", migration_id);
        Ok(false)
    }
    
    /// Record a successful migration
    async fn record_migration(&self, connection: &dyn Connection, migration_id: &str, description: &str) -> StorageResult<()> {
        let query = format!(
            "INSERT INTO {} (id, description, applied_at, checksum) VALUES (?, ?, ?, ?)",
            self.table_name
        );
        
        let now = Utc::now();
        let checksum = self.calculate_checksum(migration_id, description);
        
        // In a real implementation, this would execute the query
        log::debug!("Recording migration: {} at {}", migration_id, now);
        Ok(())
    }
    
    /// Calculate migration checksum
    fn calculate_checksum(&self, migration_id: &str, description: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        migration_id.hash(&mut hasher);
        description.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
    
    /// Rollback last migration
    pub async fn rollback_last(&self, connection: &dyn Connection) -> StorageResult<bool> {
        let applied = self.get_applied_migrations(connection).await?;
        
        if let Some(last_migration) = applied.last() {
            self.rollback_migration(connection, &last_migration.id).await?;
            Ok(true)
        } else {
            log::info!("No migrations to rollback");
            Ok(false)
        }
    }
    
    /// Rollback a specific migration
    async fn rollback_migration(&self, connection: &dyn Connection, migration_id: &str) -> StorageResult<()> {
        let migration = self.migrations.get(migration_id)
            .ok_or_else(|| StorageError::MigrationFailed(
                format!("Migration not found: {}", migration_id)
            ))?;
        
        if !migration.can_rollback() {
            return Err(StorageError::MigrationFailed(
                format!("Migration {} cannot be rolled back", migration_id)
            ));
        }
        
        log::info!("Rolling back migration: {} - {}", migration_id, migration.description());
        
        // Execute rollback in transaction
        let tx = connection.begin_transaction().await?;
        
        match migration.down(connection).await {
            Ok(()) => {
                // Remove migration record
                self.remove_migration_record(connection, migration_id).await?;
                tx.commit().await?;
                log::info!("Migration rolled back: {}", migration_id);
                Ok(())
            }
            Err(error) => {
                tx.rollback().await?;
                log::error!("Migration rollback failed: {} - {}", migration_id, error);
                Err(StorageError::MigrationFailed(
                    format!("Migration rollback {} failed: {}", migration_id, error)
                ))
            }
        }
    }
    
    /// Remove migration record
    async fn remove_migration_record(&self, connection: &dyn Connection, migration_id: &str) -> StorageResult<()> {
        let query = format!("DELETE FROM {} WHERE id = ?", self.table_name);
        
        // In a real implementation, this would execute the query
        log::debug!("Removing migration record: {}", migration_id);
        Ok(())
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple migration implementation
pub struct SimpleMigration {
    id: String,
    description: String,
    up_sql: String,
    down_sql: Option<String>,
    dependencies: Vec<String>,
}

impl SimpleMigration {
    /// Create a new simple migration
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        up_sql: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            up_sql: up_sql.into(),
            down_sql: None,
            dependencies: Vec::new(),
        }
    }
    
    /// Set down SQL for rollback
    pub fn with_down_sql(mut self, down_sql: impl Into<String>) -> Self {
        self.down_sql = Some(down_sql.into());
        self
    }
    
    /// Set dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[async_trait]
impl Migration for SimpleMigration {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn up(&self, connection: &dyn Connection) -> StorageResult<()> {
        connection.execute(&self.up_sql, &[]).await?;
        Ok(())
    }
    
    async fn down(&self, connection: &dyn Connection) -> StorageResult<()> {
        if let Some(down_sql) = &self.down_sql {
            connection.execute(down_sql, &[]).await?;
            Ok(())
        } else {
            Err(StorageError::MigrationFailed(
                "No down migration defined".to_string()
            ))
        }
    }
    
    fn can_rollback(&self) -> bool {
        self.down_sql.is_some()
    }
    
    fn dependencies(&self) -> Vec<&str> {
        self.dependencies.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{InMemoryConnectionManager, ConnectionManager};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_migration_manager() {
        let manager = MigrationManager::new()
            .add_migration(Box::new(SimpleMigration::new(
                "001_create_users",
                "Create users table",
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"
            )))
            .add_migration(Box::new(SimpleMigration::new(
                "002_add_email",
                "Add email column",
                "ALTER TABLE users ADD COLUMN email TEXT"
            )));
        
        let conn_manager = Arc::new(InMemoryConnectionManager::new());
        let connection = conn_manager.get_connection().await.unwrap();
        
        // Test initialization
        assert!(manager.initialize(connection.as_ref()).await.is_ok());
        
        // Test getting pending migrations
        let pending = manager.get_pending_migrations(connection.as_ref()).await.unwrap();
        assert_eq!(pending.len(), 2);
        
        // Test migration
        let applied = manager.migrate(connection.as_ref()).await.unwrap();
        assert_eq!(applied, 2);
    }
    
    #[test]
    fn test_simple_migration() {
        let migration = SimpleMigration::new(
            "test",
            "Test migration",
            "CREATE TABLE test (id INTEGER)"
        ).with_down_sql("DROP TABLE test");
        
        assert_eq!(migration.id(), "test");
        assert_eq!(migration.description(), "Test migration");
        assert!(migration.can_rollback());
    }
}