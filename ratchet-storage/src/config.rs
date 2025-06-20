//! Storage configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,

    /// Connection configuration
    pub connection: ConnectionConfig,

    /// Performance and reliability settings
    pub performance: PerformanceConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Migration settings
    pub migrations: MigrationConfig,
}

/// Storage backend type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageBackend {
    /// SQLite backend (default)
    Sqlite {
        /// Database file path
        database_path: PathBuf,
        /// Auto-create directory if it doesn't exist
        auto_create_dir: bool,
    },

    /// PostgreSQL backend
    #[cfg(feature = "postgres")]
    Postgres {
        /// Connection URL
        url: String,
        /// SSL mode
        ssl_mode: PostgresSslMode,
    },

    /// MySQL backend
    #[cfg(feature = "mysql")]
    Mysql {
        /// Connection URL
        url: String,
        /// SSL configuration
        ssl_ca: Option<PathBuf>,
    },

    /// In-memory backend (for testing)
    InMemory,
}

/// PostgreSQL SSL modes
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostgresSslMode {
    Disable,
    Allow,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    pub min_connections: u32,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Query timeout
    pub query_timeout: Duration,

    /// Idle timeout before closing connections
    pub idle_timeout: Option<Duration>,

    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,

    /// Enable connection health checks
    pub health_check_enabled: bool,

    /// Health check interval
    pub health_check_interval: Duration,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable query logging
    pub query_logging: bool,

    /// Log slow queries above this threshold
    pub slow_query_threshold: Duration,

    /// Enable connection pool metrics
    pub pool_metrics: bool,

    /// Enable statement preparation caching
    pub statement_cache: bool,

    /// Maximum prepared statements to cache
    pub statement_cache_size: usize,

    /// Enable row-level locking where supported
    pub row_level_locking: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable input validation and sanitization
    pub input_validation: bool,

    /// Enable SQL injection protection
    pub sql_injection_protection: bool,

    /// Enable audit logging
    pub audit_logging: bool,

    /// Maximum query complexity (prevent DoS)
    pub max_query_complexity: Option<u32>,

    /// Enable data encryption at rest (if supported)
    pub encryption_at_rest: bool,
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Auto-run migrations on startup
    pub auto_migrate: bool,

    /// Migration timeout
    pub migration_timeout: Duration,

    /// Enable migration rollback on failure
    pub rollback_on_failure: bool,

    /// Maximum number of migration retries
    pub max_retries: u32,

    /// Migration retry delay
    pub retry_delay: Duration,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Sqlite {
                database_path: PathBuf::from("ratchet.db"),
                auto_create_dir: true,
            },
            connection: ConnectionConfig::default(),
            performance: PerformanceConfig::default(),
            security: SecurityConfig::default(),
            migrations: MigrationConfig::default(),
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            idle_timeout: Some(Duration::from_secs(600)),  // 10 minutes
            max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            query_logging: cfg!(debug_assertions),
            slow_query_threshold: Duration::from_millis(100),
            pool_metrics: true,
            statement_cache: true,
            statement_cache_size: 100,
            row_level_locking: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            input_validation: true,
            sql_injection_protection: true,
            audit_logging: false,
            max_query_complexity: Some(1000),
            encryption_at_rest: false,
        }
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            migration_timeout: Duration::from_secs(300), // 5 minutes
            rollback_on_failure: true,
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

impl StorageConfig {
    /// Create a new configuration for SQLite
    pub fn sqlite<P: Into<PathBuf>>(database_path: P) -> Self {
        Self {
            backend: StorageBackend::Sqlite {
                database_path: database_path.into(),
                auto_create_dir: true,
            },
            ..Default::default()
        }
    }

    /// Create a new in-memory configuration (for testing)
    pub fn in_memory() -> Self {
        Self {
            backend: StorageBackend::InMemory,
            ..Default::default()
        }
    }

    /// Get the connection URL for the backend
    pub fn connection_url(&self) -> crate::StorageResult<String> {
        match &self.backend {
            StorageBackend::Sqlite { database_path, .. } => {
                Ok(format!("sqlite://{}?mode=rwc", database_path.display()))
            }

            #[cfg(feature = "postgres")]
            StorageBackend::Postgres { url, .. } => Ok(url.clone()),

            #[cfg(feature = "mysql")]
            StorageBackend::Mysql { url, .. } => Ok(url.clone()),

            StorageBackend::InMemory => Ok("sqlite://:memory:".to_string()),
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::StorageResult<()> {
        // Validate connection settings
        if self.connection.max_connections == 0 {
            return Err(crate::StorageError::ConfigError(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        if self.connection.min_connections > self.connection.max_connections {
            return Err(crate::StorageError::ConfigError(
                "min_connections cannot be greater than max_connections".to_string(),
            ));
        }

        // Validate performance settings
        if self.performance.statement_cache_size == 0 && self.performance.statement_cache {
            return Err(crate::StorageError::ConfigError(
                "statement_cache_size must be greater than 0 when statement_cache is enabled".to_string(),
            ));
        }

        // Validate migration settings
        if self.migrations.max_retries == 0 {
            return Err(crate::StorageError::ConfigError(
                "migration max_retries must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert!(config.validate().is_ok());

        match config.backend {
            StorageBackend::Sqlite {
                database_path,
                auto_create_dir,
            } => {
                assert_eq!(database_path, PathBuf::from("ratchet.db"));
                assert!(auto_create_dir);
            }
            _ => panic!("Expected SQLite backend"),
        }
    }

    #[test]
    fn test_sqlite_config() {
        let config = StorageConfig::sqlite("/tmp/test.db");
        assert!(config.validate().is_ok());

        let url = config.connection_url().unwrap();
        assert!(url.starts_with("sqlite://"));
    }

    #[test]
    fn test_in_memory_config() {
        let config = StorageConfig::in_memory();
        assert!(config.validate().is_ok());

        let url = config.connection_url().unwrap();
        assert_eq!(url, "sqlite://:memory:");
    }

    #[test]
    fn test_config_validation() {
        let mut config = StorageConfig::default();

        // Test invalid max_connections
        config.connection.max_connections = 0;
        assert!(config.validate().is_err());

        // Test invalid min/max connection relationship
        config.connection.max_connections = 5;
        config.connection.min_connections = 10;
        assert!(config.validate().is_err());
    }
}
