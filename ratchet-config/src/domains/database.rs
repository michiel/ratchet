//! Database configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::validation::{Validatable, validate_required_string, validate_positive};
use crate::error::ConfigResult;

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://ratchet.db", "postgres://user:pass@host/db")
    #[serde(default = "default_database_url")]
    pub url: String,
    
    /// Maximum number of database connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    
    /// Minimum number of idle connections in the pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    
    /// Connection timeout
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_connection_timeout")]
    pub connection_timeout: Duration,
    
    /// Idle timeout for connections
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_idle_timeout")]
    pub idle_timeout: Duration,
    
    /// Maximum lifetime for connections
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_max_lifetime")]
    pub max_lifetime: Duration,
    
    /// Database-specific configuration
    #[serde(default)]
    pub database_specific: DatabaseSpecificConfig,
    
    /// Migration configuration
    #[serde(default)]
    pub migrations: MigrationConfig,
}

/// Database-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct DatabaseSpecificConfig {
    /// SQLite-specific configuration
    pub sqlite: SqliteConfig,
    
    /// PostgreSQL-specific configuration
    pub postgres: PostgresConfig,
    
    /// MySQL-specific configuration
    pub mysql: MysqlConfig,
}

/// SQLite-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SqliteConfig {
    /// Journal mode
    #[serde(default = "default_sqlite_journal_mode")]
    pub journal_mode: String,
    
    /// Synchronous mode
    #[serde(default = "default_sqlite_synchronous")]
    pub synchronous: String,
    
    /// Cache size in KB
    #[serde(default = "default_sqlite_cache_size")]
    pub cache_size_kb: i32,
    
    /// Busy timeout in milliseconds
    #[serde(default = "default_sqlite_busy_timeout")]
    pub busy_timeout_ms: u32,
}

/// PostgreSQL-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PostgresConfig {
    /// Application name for connection
    #[serde(default = "default_postgres_application_name")]
    pub application_name: String,
    
    /// Statement timeout
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_postgres_statement_timeout")]
    pub statement_timeout: Duration,
    
    /// Lock timeout
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_postgres_lock_timeout")]
    pub lock_timeout: Duration,
    
    /// Whether to use SSL
    #[serde(default = "default_postgres_ssl")]
    pub ssl_mode: String,
}

/// MySQL-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MysqlConfig {
    /// Character set
    #[serde(default = "default_mysql_charset")]
    pub charset: String,
    
    /// Collation
    #[serde(default = "default_mysql_collation")]
    pub collation: String,
    
    /// SQL mode
    #[serde(default = "default_mysql_sql_mode")]
    pub sql_mode: String,
    
    /// Connection timeout
    #[serde(with = "crate::domains::utils::serde_duration", default = "default_mysql_connect_timeout")]
    pub connect_timeout: Duration,
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MigrationConfig {
    /// Whether to run migrations automatically on startup
    #[serde(default = "crate::domains::utils::default_true")]
    pub auto_migrate: bool,
    
    /// Directory containing migration files
    #[serde(default = "default_migration_dir")]
    pub migration_dir: String,
    
    /// Migration table name
    #[serde(default = "default_migration_table")]
    pub table_name: String,
    
    /// Whether to validate migration checksums
    #[serde(default = "crate::domains::utils::default_true")]
    pub validate_checksums: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connection_timeout: default_connection_timeout(),
            idle_timeout: default_idle_timeout(),
            max_lifetime: default_max_lifetime(),
            database_specific: DatabaseSpecificConfig::default(),
            migrations: MigrationConfig::default(),
        }
    }
}


impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            journal_mode: default_sqlite_journal_mode(),
            synchronous: default_sqlite_synchronous(),
            cache_size_kb: default_sqlite_cache_size(),
            busy_timeout_ms: default_sqlite_busy_timeout(),
        }
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            application_name: default_postgres_application_name(),
            statement_timeout: default_postgres_statement_timeout(),
            lock_timeout: default_postgres_lock_timeout(),
            ssl_mode: default_postgres_ssl(),
        }
    }
}

impl Default for MysqlConfig {
    fn default() -> Self {
        Self {
            charset: default_mysql_charset(),
            collation: default_mysql_collation(),
            sql_mode: default_mysql_sql_mode(),
            connect_timeout: default_mysql_connect_timeout(),
        }
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            migration_dir: default_migration_dir(),
            table_name: default_migration_table(),
            validate_checksums: true,
        }
    }
}

impl Validatable for DatabaseConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.url, "url", self.domain_name())?;
        validate_positive(self.max_connections, "max_connections", self.domain_name())?;
        validate_positive(self.connection_timeout.as_secs(), "connection_timeout", self.domain_name())?;
        validate_positive(self.idle_timeout.as_secs(), "idle_timeout", self.domain_name())?;
        validate_positive(self.max_lifetime.as_secs(), "max_lifetime", self.domain_name())?;
        
        // Validate that min_connections <= max_connections
        if self.min_connections > self.max_connections {
            return Err(self.validation_error(
                "min_connections cannot be greater than max_connections"
            ));
        }
        
        self.database_specific.validate()?;
        self.migrations.validate()?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database"
    }
}

impl Validatable for DatabaseSpecificConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.sqlite.validate()?;
        self.postgres.validate()?;
        self.mysql.validate()?;
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database.specific"
    }
}

impl Validatable for SqliteConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate journal mode
        let valid_journal_modes = ["DELETE", "TRUNCATE", "PERSIST", "MEMORY", "WAL", "OFF"];
        crate::validation::validate_enum_choice(
            &self.journal_mode, 
            &valid_journal_modes, 
            "journal_mode", 
            self.domain_name()
        )?;
        
        // Validate synchronous mode
        let valid_sync_modes = ["OFF", "NORMAL", "FULL", "EXTRA"];
        crate::validation::validate_enum_choice(
            &self.synchronous, 
            &valid_sync_modes, 
            "synchronous", 
            self.domain_name()
        )?;
        
        validate_positive(self.busy_timeout_ms, "busy_timeout_ms", self.domain_name())?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database.sqlite"
    }
}

impl Validatable for PostgresConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.application_name, "application_name", self.domain_name())?;
        validate_positive(self.statement_timeout.as_secs(), "statement_timeout", self.domain_name())?;
        validate_positive(self.lock_timeout.as_secs(), "lock_timeout", self.domain_name())?;
        
        // Validate SSL mode
        let valid_ssl_modes = ["disable", "allow", "prefer", "require", "verify-ca", "verify-full"];
        crate::validation::validate_enum_choice(
            &self.ssl_mode, 
            &valid_ssl_modes, 
            "ssl_mode", 
            self.domain_name()
        )?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database.postgres"
    }
}

impl Validatable for MysqlConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.charset, "charset", self.domain_name())?;
        validate_required_string(&self.collation, "collation", self.domain_name())?;
        validate_positive(self.connect_timeout.as_secs(), "connect_timeout", self.domain_name())?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database.mysql"
    }
}

impl Validatable for MigrationConfig {
    fn validate(&self) -> ConfigResult<()> {
        validate_required_string(&self.migration_dir, "migration_dir", self.domain_name())?;
        validate_required_string(&self.table_name, "table_name", self.domain_name())?;
        
        Ok(())
    }
    
    fn domain_name(&self) -> &'static str {
        "database.migrations"
    }
}

// Default value functions
fn default_database_url() -> String {
    "sqlite::memory:".to_string()
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    1
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(600) // 10 minutes
}

fn default_max_lifetime() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_sqlite_journal_mode() -> String {
    "WAL".to_string()
}

fn default_sqlite_synchronous() -> String {
    "NORMAL".to_string()
}

fn default_sqlite_cache_size() -> i32 {
    2000 // 2MB
}

fn default_sqlite_busy_timeout() -> u32 {
    5000 // 5 seconds
}

fn default_postgres_application_name() -> String {
    "ratchet".to_string()
}

fn default_postgres_statement_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_postgres_lock_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_postgres_ssl() -> String {
    "prefer".to_string()
}

fn default_mysql_charset() -> String {
    "utf8mb4".to_string()
}

fn default_mysql_collation() -> String {
    "utf8mb4_unicode_ci".to_string()
}

fn default_mysql_sql_mode() -> String {
    "STRICT_TRANS_TABLES,NO_ZERO_DATE,NO_ZERO_IN_DATE,ERROR_FOR_DIVISION_BY_ZERO".to_string()
}

fn default_mysql_connect_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_migration_dir() -> String {
    "migrations".to_string()
}

fn default_migration_table() -> String {
    "__migrations".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_defaults() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url, "sqlite::memory:");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert!(config.migrations.auto_migrate);
    }

    #[test]
    fn test_database_config_validation() {
        let mut config = DatabaseConfig::default();
        assert!(config.validate().is_ok());
        
        // Test min > max connections
        config.min_connections = 20;
        config.max_connections = 10;
        assert!(config.validate().is_err());
        
        // Test empty URL
        config = DatabaseConfig::default();
        config.url = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sqlite_config_validation() {
        let mut config = SqliteConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid journal mode
        config.journal_mode = "INVALID".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_postgres_config_validation() {
        let mut config = PostgresConfig::default();
        assert!(config.validate().is_ok());
        
        // Test invalid SSL mode
        config.ssl_mode = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}