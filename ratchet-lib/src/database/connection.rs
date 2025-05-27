use crate::config::DatabaseConfig;
use sea_orm::{Database, DatabaseConnection as SeaConnection, DbErr, ConnectOptions};
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
}

impl DatabaseConnection {
    /// Create a new database connection with configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        info!("Connecting to database: {}", config.url);
        
        let mut opts = ConnectOptions::new(&config.url);
        opts
            .max_connections(config.max_connections)
            .min_connections(1)
            .connect_timeout(config.connection_timeout)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(Duration::from_secs(300)) // 5 minutes idle timeout
            .max_lifetime(Duration::from_secs(3600)) // 1 hour max lifetime
            .sqlx_logging(false) // Disable SQLx query logging to reduce verbose output
            .sqlx_logging_level(log::LevelFilter::Debug); // If enabled, use DEBUG level
        
        let connection = Database::connect(opts).await?;
        
        debug!("Database connection established with {} max connections", config.max_connections);
        
        Ok(Self { connection, config })
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
}