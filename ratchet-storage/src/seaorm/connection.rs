use super::config::DatabaseConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection as SeaConnection, DbErr};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info};

/// Database connection wrapper with configuration
#[derive(Clone)]
pub struct DatabaseConnection {
    connection: SeaConnection,
    config: DatabaseConfig,
}

/// Database-related errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    DbError(#[from] DbErr),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Validation error: {0}")]
    ValidationError(#[from] crate::database::filters::validation::ValidationError),
}

impl DatabaseConnection {
    /// Create a new database connection with configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        info!("Connecting to database: {}", config.url);

        // Handle SQLite file creation if needed
        Self::ensure_sqlite_file_exists(&config.url)?;

        let mut opts = ConnectOptions::new(&config.url);
        opts.max_connections(config.max_connections)
            .min_connections(1)
            .connect_timeout(config.connection_timeout)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(Duration::from_secs(300)) // 5 minutes idle timeout
            .max_lifetime(Duration::from_secs(3600)) // 1 hour max lifetime
            .sqlx_logging(true) // Enable SQLx query logging
            .sqlx_logging_level(log::LevelFilter::Debug); // Set SQLx logging to DEBUG level instead of INFO

        let connection = Database::connect(opts).await?;

        debug!(
            "Database connection established with {} max connections",
            config.max_connections
        );

        Ok(Self { connection, config })
    }

    /// Ensure SQLite database file and directory exist for file-based databases
    fn ensure_sqlite_file_exists(database_url: &str) -> Result<(), DatabaseError> {
        // Check if this is a file-based SQLite database
        if database_url.starts_with("sqlite:") && !database_url.contains(":memory:") {
            // Extract the file path from the URL, handling various SQLite URL formats
            let file_path = if database_url.starts_with("sqlite:///") {
                // Absolute path: sqlite:///path/to/file.db -> /path/to/file.db
                database_url.strip_prefix("sqlite://").ok_or_else(|| {
                    DatabaseError::ConfigError(format!(
                        "Invalid SQLite URL format: {}",
                        database_url
                    ))
                })?
            } else if database_url.starts_with("sqlite://") {
                // Relative path: sqlite://file.db -> file.db
                database_url.strip_prefix("sqlite://").ok_or_else(|| {
                    DatabaseError::ConfigError(format!(
                        "Invalid SQLite URL format: {}",
                        database_url
                    ))
                })?
            } else if database_url.starts_with("sqlite:") {
                // Direct path: sqlite:file.db -> file.db
                database_url.strip_prefix("sqlite:").ok_or_else(|| {
                    DatabaseError::ConfigError(format!(
                        "Invalid SQLite URL format: {}",
                        database_url
                    ))
                })?
            } else {
                return Err(DatabaseError::ConfigError(format!(
                    "Invalid SQLite URL format: {}",
                    database_url
                )));
            };

            let path = std::path::Path::new(file_path);

            // Create parent directory if it doesn't exist
            if let Some(parent_dir) = path.parent() {
                if !parent_dir.exists() {
                    info!("Creating database directory: {:?}", parent_dir);
                    std::fs::create_dir_all(parent_dir).map_err(|e| {
                        DatabaseError::ConfigError(format!(
                            "Failed to create database directory {:?}: {}",
                            parent_dir, e
                        ))
                    })?;
                }
            }

            // SQLite will create the file if it doesn't exist, we just need to ensure the directory exists
            if !path.exists() {
                info!("Database file will be created by SQLite: {:?}", path);
            } else {
                debug!("Using existing database file: {:?}", path);
            }
        } else if database_url.contains(":memory:") {
            debug!("Using in-memory SQLite database");
        } else {
            debug!("Non-SQLite database detected, skipping file creation logic");
        }

        Ok(())
    }

    /// Get the underlying Sea-ORM connection
    pub fn get_connection(&self) -> &SeaConnection {
        &self.connection
    }

    /// Get database configuration
    pub fn get_config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), DatabaseError> {
        use sea_orm_migration::MigratorTrait;

        info!("Running database migrations");

        crate::database::migrations::Migrator::up(&self.connection, None)
            .await
            .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Check database connectivity
    pub async fn ping(&self) -> Result<(), DatabaseError> {
        debug!("Pinging database");

        match self.connection.ping().await {
            Ok(_) => {
                debug!("Database ping successful");
                Ok(())
            }
            Err(e) => {
                debug!("Database ping failed: {}", e);
                Err(DatabaseError::DbError(e))
            }
        }
    }

    /// Close the database connection
    pub async fn close(self) -> Result<(), DatabaseError> {
        info!("Closing database connection");
        self.connection.close().await?;
        debug!("Database connection closed");
        Ok(())
    }

    /// Get database connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        // Note: Sea-ORM doesn't expose pool stats directly
        // This would need to be implemented by tracking connections
        ConnectionStats {
            max_connections: self.config.max_connections,
            // active_connections: 0, // Would need pool access
            // idle_connections: 0,   // Would need pool access
        }
    }
}

/// Database connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub max_connections: u32,
    // pub active_connections: u32,
    // pub idle_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_config() -> DatabaseConfig {
        DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        }
    }

    #[tokio::test]
    async fn test_database_connection() {
        let config = create_test_config().await;
        let db = DatabaseConnection::new(config).await;
        assert!(db.is_ok());

        let db = db.unwrap();
        assert!(db.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_database_migration() {
        let config = create_test_config().await;
        let db = DatabaseConnection::new(config).await.unwrap();

        // Migration should succeed (even if no tables exist yet)
        let result = db.migrate().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let config = create_test_config().await;
        let db = DatabaseConnection::new(config).await.unwrap();

        let stats = db.get_stats();
        assert_eq!(stats.max_connections, 5);
    }

    #[tokio::test]
    async fn test_in_memory_database() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        };

        let db = DatabaseConnection::new(config).await;
        assert!(db.is_ok());

        let db = db.unwrap();
        assert!(db.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_file_database_directory_creation() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("subdir").join("test.db");
        let db_url = format!("sqlite://{}", db_path.display());

        // Verify directory doesn't exist initially
        assert!(!db_path.parent().unwrap().exists());

        // Test the directory creation logic directly
        let result = DatabaseConnection::ensure_sqlite_file_exists(&db_url);
        assert!(result.is_ok());

        // Verify directory was created
        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn test_ensure_sqlite_file_exists_in_memory() {
        let result = DatabaseConnection::ensure_sqlite_file_exists("sqlite::memory:");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_sqlite_file_exists_file_path() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("nested").join("test.db");
        let db_url = format!("sqlite://{}", db_path.display());

        // Verify directory doesn't exist initially
        assert!(!db_path.parent().unwrap().exists());

        let result = DatabaseConnection::ensure_sqlite_file_exists(&db_url);
        assert!(result.is_ok());

        // Verify directory was created
        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn test_ensure_sqlite_file_exists_non_sqlite() {
        let result = DatabaseConnection::ensure_sqlite_file_exists("postgresql://localhost/test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_sqlite_file_exists_invalid_url() {
        let result = DatabaseConnection::ensure_sqlite_file_exists("invalid-url");
        assert!(result.is_ok()); // Should handle gracefully
    }
}
