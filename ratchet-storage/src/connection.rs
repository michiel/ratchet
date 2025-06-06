//! Connection management abstractions

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::{StorageConfig, StorageError, StorageResult};

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Total number of connections created
    pub total_connections: u64,

    /// Currently active connections
    pub active_connections: u32,

    /// Currently idle connections
    pub idle_connections: u32,

    /// Total number of queries executed
    pub total_queries: u64,

    /// Number of failed queries
    pub failed_queries: u64,

    /// Average query duration
    pub avg_query_duration: Duration,

    /// Pool utilization percentage
    pub pool_utilization: f64,
}

/// Generic connection trait (object-safe version)
#[async_trait]
pub trait Connection: Send + Sync {
    /// Execute a query that returns no results
    async fn execute(&self, query: &str, params: &[serde_json::Value]) -> StorageResult<u64>;

    /// Execute a raw query that returns JSON results
    async fn fetch_json(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<Vec<serde_json::Value>>;

    /// Execute a query that returns a single JSON row
    async fn fetch_one_json(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<serde_json::Value>;

    /// Execute a query that may return zero or one JSON row
    async fn fetch_optional_json(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<Option<serde_json::Value>>;

    /// Begin a transaction
    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>>;

    /// Check if the connection is healthy
    async fn ping(&self) -> StorageResult<bool>;

    /// Get connection statistics
    async fn stats(&self) -> StorageResult<ConnectionStats>;
}

/// Transaction trait (object-safe version)
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Execute a query within the transaction
    async fn execute(&mut self, query: &str, params: &[serde_json::Value]) -> StorageResult<u64>;

    /// Fetch JSON rows within the transaction
    async fn fetch_json(
        &mut self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<Vec<serde_json::Value>>;

    /// Fetch one JSON row within the transaction
    async fn fetch_one_json(
        &mut self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<serde_json::Value>;

    /// Fetch optional JSON row within the transaction
    async fn fetch_optional_json(
        &mut self,
        query: &str,
        params: &[serde_json::Value],
    ) -> StorageResult<Option<serde_json::Value>>;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> StorageResult<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> StorageResult<()>;
}

/// Connection manager trait
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// Get a connection from the pool
    async fn get_connection(&self) -> StorageResult<Arc<dyn Connection>>;

    /// Get pool statistics
    async fn pool_stats(&self) -> StorageResult<ConnectionStats>;

    /// Health check for the connection pool
    async fn health_check(&self) -> StorageResult<bool>;

    /// Close all connections
    async fn close(&self) -> StorageResult<()>;
}

/// Simple in-memory connection manager for testing
pub struct InMemoryConnectionManager {
    stats: Arc<RwLock<ConnectionStats>>,
}

impl Default for InMemoryConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryConnectionManager {
    /// Create a new in-memory connection manager
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ConnectionStats {
                total_connections: 1,
                active_connections: 1,
                idle_connections: 0,
                total_queries: 0,
                failed_queries: 0,
                avg_query_duration: Duration::from_millis(1),
                pool_utilization: 0.1,
            })),
        }
    }
}

#[async_trait]
impl ConnectionManager for InMemoryConnectionManager {
    async fn get_connection(&self) -> StorageResult<Arc<dyn Connection>> {
        Ok(Arc::new(InMemoryConnection::new(self.stats.clone())))
    }

    async fn pool_stats(&self) -> StorageResult<ConnectionStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn health_check(&self) -> StorageResult<bool> {
        Ok(true)
    }

    async fn close(&self) -> StorageResult<()> {
        Ok(())
    }
}

/// Simple in-memory connection for testing
pub struct InMemoryConnection {
    stats: Arc<RwLock<ConnectionStats>>,
}

impl InMemoryConnection {
    fn new(stats: Arc<RwLock<ConnectionStats>>) -> Self {
        Self { stats }
    }

    async fn increment_query_count(&self) {
        let mut stats = self.stats.write().await;
        stats.total_queries += 1;
    }
}

#[async_trait]
impl Connection for InMemoryConnection {
    async fn execute(&self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<u64> {
        self.increment_query_count().await;
        // Simulate execution
        tokio::time::sleep(Duration::from_millis(1)).await;
        Ok(1)
    }

    async fn fetch_json(
        &self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<Vec<serde_json::Value>> {
        self.increment_query_count().await;
        Ok(Vec::new())
    }

    async fn fetch_one_json(
        &self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<serde_json::Value> {
        self.increment_query_count().await;
        Err(StorageError::NotFound)
    }

    async fn fetch_optional_json(
        &self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<Option<serde_json::Value>> {
        self.increment_query_count().await;
        Ok(None)
    }

    async fn begin_transaction(&self) -> StorageResult<Box<dyn Transaction>> {
        Ok(Box::new(InMemoryTransaction::new(self.stats.clone())))
    }

    async fn ping(&self) -> StorageResult<bool> {
        Ok(true)
    }

    async fn stats(&self) -> StorageResult<ConnectionStats> {
        Ok(self.stats.read().await.clone())
    }
}

/// Simple in-memory transaction for testing
pub struct InMemoryTransaction {
    stats: Arc<RwLock<ConnectionStats>>,
    committed: bool,
}

impl InMemoryTransaction {
    fn new(stats: Arc<RwLock<ConnectionStats>>) -> Self {
        Self {
            stats,
            committed: false,
        }
    }
}

#[async_trait]
impl Transaction for InMemoryTransaction {
    async fn execute(&mut self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<u64> {
        let mut stats = self.stats.write().await;
        stats.total_queries += 1;
        Ok(1)
    }

    async fn fetch_json(
        &mut self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<Vec<serde_json::Value>> {
        Ok(Vec::new())
    }

    async fn fetch_one_json(
        &mut self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<serde_json::Value> {
        Err(StorageError::NotFound)
    }

    async fn fetch_optional_json(
        &mut self,
        _query: &str,
        _params: &[serde_json::Value],
    ) -> StorageResult<Option<serde_json::Value>> {
        Ok(None)
    }

    async fn commit(mut self: Box<Self>) -> StorageResult<()> {
        self.committed = true;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> StorageResult<()> {
        Ok(())
    }
}

/// Factory function to create connection manager based on configuration
pub async fn create_connection_manager(
    config: &StorageConfig,
) -> StorageResult<Arc<dyn ConnectionManager>> {
    config.validate()?;

    match &config.backend {
        crate::config::StorageBackend::InMemory => Ok(Arc::new(InMemoryConnectionManager::new())),

        #[cfg(feature = "database")]
        _ => {
            // TODO: Create Sea-ORM based connection manager when database module is implemented
            Err(StorageError::ConfigError(
                "Sea-ORM connection manager not yet implemented".to_string(),
            ))
        }

        #[cfg(not(feature = "database"))]
        _ => Err(StorageError::ConfigError(
            "Database backend requires 'database' feature to be enabled".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_connection_manager() {
        let manager = InMemoryConnectionManager::new();

        // Test health check
        assert!(manager.health_check().await.unwrap());

        // Test getting connection
        let conn = manager.get_connection().await.unwrap();
        assert!(conn.ping().await.unwrap());

        // Test connection stats
        let stats = conn.stats().await.unwrap();
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.active_connections, 1);
    }

    #[tokio::test]
    async fn test_in_memory_connection() {
        let manager = InMemoryConnectionManager::new();
        let conn = manager.get_connection().await.unwrap();

        // Test execute
        let result = conn.execute("INSERT INTO test", &[]).await.unwrap();
        assert_eq!(result, 1);

        // Test transaction
        let tx = conn.begin_transaction().await.unwrap();
        tx.commit().await.unwrap();

        // Test stats after queries
        let stats = conn.stats().await.unwrap();
        assert!(stats.total_queries > 0);
    }

    #[tokio::test]
    async fn test_create_connection_manager() {
        let config = StorageConfig::in_memory();
        let manager = create_connection_manager(&config).await.unwrap();

        assert!(manager.health_check().await.unwrap());
    }
}
