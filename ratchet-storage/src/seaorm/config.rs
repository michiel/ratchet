use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Database configuration for SeaORM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    
    /// Connection timeout
    pub connection_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
        }
    }
}