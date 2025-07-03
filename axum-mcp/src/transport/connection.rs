//! Connection pooling and health monitoring for MCP transports

use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use uuid::Uuid;

use super::{McpTransport, TransportFactory, TransportHealth, TransportType};
use crate::protocol::messages::McpNotification;
use crate::{McpError, McpResult};

/// Connection identifier
pub type ConnectionId = String;

/// Server identifier  
pub type ServerId = String;

/// Transport connection trait for different connection types (stdio, SSE, etc)
#[async_trait]
pub trait TransportConnection: Send + Sync {
    /// Send a notification to the client
    async fn send_notification(
        &self,
        notification: McpNotification,
    ) -> McpResult<()>;

    /// Close the connection
    async fn close(&self) -> McpResult<()>;
}

/// Connection wrapper with metadata
pub struct ConnectionWrapper {
    /// Unique connection ID
    pub id: ConnectionId,

    /// Server this connection belongs to
    pub server_id: ServerId,

    /// The actual transport
    pub transport: Box<dyn McpTransport>,

    /// When this connection was created
    pub created_at: Instant,

    /// When this connection was last used
    pub last_used: Instant,

    /// Number of times this connection has been used
    pub use_count: u64,

    /// Whether this connection is currently in use
    pub in_use: bool,
}

impl ConnectionWrapper {
    /// Create a new connection wrapper
    pub fn new(server_id: ServerId, transport: Box<dyn McpTransport>) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            server_id,
            transport,
            created_at: now,
            last_used: now,
            use_count: 0,
            in_use: false,
        }
    }

    /// Mark connection as used
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
        self.in_use = true;
    }

    /// Mark connection as available
    pub fn mark_available(&mut self) {
        self.in_use = false;
    }

    /// Check if connection is idle for too long
    pub fn is_idle(&self, max_idle: Duration) -> bool {
        !self.in_use && self.last_used.elapsed() > max_idle
    }

    /// Check if connection is too old
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum connections per server
    pub max_connections_per_server: usize,

    /// Maximum idle time before closing connection
    pub max_idle_time: Duration,

    /// Maximum connection age
    pub max_connection_age: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Minimum pool size (keep alive)
    pub min_pool_size: usize,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_server: 10,
            max_idle_time: Duration::from_secs(300),        // 5 minutes
            max_connection_age: Duration::from_secs(3600),  // 1 hour
            health_check_interval: Duration::from_secs(60), // 1 minute
            connection_timeout: Duration::from_secs(30),
            min_pool_size: 1,
        }
    }
}

/// Connection pool for managing MCP transport connections
pub struct ConnectionPool {
    /// Pool configuration
    config: ConnectionPoolConfig,

    /// Server configurations
    server_configs: RwLock<HashMap<ServerId, TransportType>>,

    /// Active connections by server
    connections: RwLock<HashMap<ServerId, VecDeque<ConnectionWrapper>>>,

    /// Health monitor
    health_monitor: Arc<HealthMonitor>,

    /// Pool statistics
    stats: Mutex<PoolStats>,
}

/// Connection pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total connections created
    pub total_created: u64,

    /// Total connections closed
    pub total_closed: u64,

    /// Current active connections
    pub active_connections: u64,

    /// Total requests served
    pub total_requests: u64,

    /// Average connection reuse
    pub avg_connection_reuse: f64,

    /// Health check failures
    pub health_check_failures: u64,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let health_monitor = Arc::new(HealthMonitor::new());

        Self {
            config,
            server_configs: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            health_monitor,
            stats: Mutex::new(PoolStats::default()),
        }
    }

    /// Add a server configuration
    pub async fn add_server(&self, server_id: ServerId, config: TransportType) -> McpResult<()> {
        config.validate()?;

        let mut configs = self.server_configs.write().await;
        configs.insert(server_id.clone(), config);

        // Initialize connection pool for this server
        let mut connections = self.connections.write().await;
        connections.insert(server_id, VecDeque::new());

        Ok(())
    }

    /// Remove a server configuration
    pub async fn remove_server(&self, server_id: &str) -> McpResult<()> {
        // Close all connections for this server
        let mut connections = self.connections.write().await;
        if let Some(mut server_connections) = connections.remove(server_id) {
            for mut conn in server_connections.drain(..) {
                let _ = conn.transport.close().await;

                let mut stats = self.stats.lock().await;
                stats.total_closed += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }
        }

        // Remove server config
        let mut configs = self.server_configs.write().await;
        configs.remove(server_id);

        Ok(())
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self, server_id: &str) -> McpResult<ConnectionWrapper> {
        // Try to get existing connection
        if let Some(conn) = self.try_get_existing_connection(server_id).await? {
            return Ok(conn);
        }

        // Create new connection
        self.create_new_connection(server_id).await
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, mut connection: ConnectionWrapper) -> McpResult<()> {
        connection.mark_available();

        let mut connections = self.connections.write().await;
        if let Some(server_connections) = connections.get_mut(&connection.server_id) {
            // Check if we're at capacity
            if server_connections.len() < self.config.max_connections_per_server {
                server_connections.push_back(connection);
            } else {
                // Pool is full, close the connection
                drop(connections); // Release lock before async operation
                let _ = connection.transport.close().await;

                let mut stats = self.stats.lock().await;
                stats.total_closed += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }
        } else {
            // Server was removed, close connection
            let _ = connection.transport.close().await;

            let mut stats = self.stats.lock().await;
            stats.total_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        Ok(())
    }

    /// Try to get an existing connection
    async fn try_get_existing_connection(&self, server_id: &str) -> McpResult<Option<ConnectionWrapper>> {
        let mut connections = self.connections.write().await;
        if let Some(server_connections) = connections.get_mut(server_id) {
            // Find a healthy, available connection
            while let Some(mut conn) = server_connections.pop_front() {
                // Check if connection is still healthy
                if conn.transport.is_connected().await {
                    conn.mark_used();

                    let mut stats = self.stats.lock().await;
                    stats.total_requests += 1;

                    return Ok(Some(conn));
                } else {
                    // Connection is dead, close it
                    let _ = conn.transport.close().await;

                    let mut stats = self.stats.lock().await;
                    stats.total_closed += 1;
                    stats.active_connections = stats.active_connections.saturating_sub(1);
                }
            }
        }

        Ok(None)
    }

    /// Create a new connection
    async fn create_new_connection(&self, server_id: &str) -> McpResult<ConnectionWrapper> {
        // Get server config
        let configs = self.server_configs.read().await;
        let config = configs
            .get(server_id)
            .ok_or_else(|| McpError::Configuration {
                message: format!("No configuration found for server: {}", server_id),
            })?
            .clone();
        drop(configs);

        // Create transport
        let mut transport = TransportFactory::create(config).await?;

        // Connect with timeout
        tokio::time::timeout(self.config.connection_timeout, transport.connect())
            .await
            .map_err(|_| McpError::ConnectionTimeout {
                message: format!("Connection timeout after {:?}", self.config.connection_timeout),
            })??;

        // Create connection wrapper
        let mut connection = ConnectionWrapper::new(server_id.to_string(), transport);
        connection.mark_used();

        // Update stats
        let mut stats = self.stats.lock().await;
        stats.total_created += 1;
        stats.active_connections += 1;
        stats.total_requests += 1;

        Ok(connection)
    }

    /// Start background tasks
    pub async fn start_background_tasks(self: Arc<Self>) -> McpResult<()> {
        let pool = self.clone();

        // Start cleanup task
        let cleanup_pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = interval(cleanup_pool.config.health_check_interval);
            loop {
                interval.tick().await;
                let _ = cleanup_pool.cleanup_connections().await;
            }
        });

        // Start health check task
        let health_pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = interval(health_pool.config.health_check_interval);
            loop {
                interval.tick().await;
                let _ = health_pool.health_check().await;
            }
        });

        Ok(())
    }

    /// Clean up old/idle connections
    async fn cleanup_connections(&self) -> McpResult<()> {
        let mut connections = self.connections.write().await;
        let mut stats = self.stats.lock().await;

        for server_connections in connections.values_mut() {
            let mut to_remove = Vec::new();

            for (index, conn) in server_connections.iter().enumerate() {
                if conn.is_idle(self.config.max_idle_time) || conn.is_expired(self.config.max_connection_age) {
                    to_remove.push(index);
                }
            }

            // Remove connections in reverse order to maintain indices
            for &index in to_remove.iter().rev() {
                if let Some(mut conn) = server_connections.remove(index) {
                    let _ = conn.transport.close().await;
                    stats.total_closed += 1;
                    stats.active_connections = stats.active_connections.saturating_sub(1);
                }
            }
        }

        Ok(())
    }

    /// Perform health checks
    async fn health_check(&self) -> McpResult<()> {
        let connections = self.connections.read().await;

        for (server_id, server_connections) in connections.iter() {
            for conn in server_connections.iter() {
                if !conn.in_use {
                    let health = conn.transport.health().await;
                    self.health_monitor.update_health(server_id, &conn.id, health).await;
                }
            }
        }

        Ok(())
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.stats.lock().await.clone()
    }

    /// Get server health information
    pub async fn server_health(&self, server_id: &str) -> Option<Vec<ConnectionHealth>> {
        self.health_monitor.get_server_health(server_id).await
    }
}

/// Connection health information
#[derive(Debug, Clone)]
pub struct ConnectionHealth {
    pub connection_id: ConnectionId,
    pub server_id: ServerId,
    pub transport_health: TransportHealth,
    pub last_checked: Instant,
}

/// Health monitor for tracking connection health
pub struct HealthMonitor {
    health_data: RwLock<HashMap<ServerId, HashMap<ConnectionId, ConnectionHealth>>>,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            health_data: RwLock::new(HashMap::new()),
        }
    }

    /// Update health information for a connection
    pub async fn update_health(&self, server_id: &str, connection_id: &str, transport_health: TransportHealth) {
        let mut health_data = self.health_data.write().await;

        let server_health = health_data.entry(server_id.to_string()).or_insert_with(HashMap::new);

        server_health.insert(
            connection_id.to_string(),
            ConnectionHealth {
                connection_id: connection_id.to_string(),
                server_id: server_id.to_string(),
                transport_health,
                last_checked: Instant::now(),
            },
        );
    }

    /// Get health information for a server
    pub async fn get_server_health(&self, server_id: &str) -> Option<Vec<ConnectionHealth>> {
        let health_data = self.health_data.read().await;
        health_data
            .get(server_id)
            .map(|server_health| server_health.values().cloned().collect())
    }

    /// Remove health data for a connection
    pub async fn remove_connection(&self, server_id: &str, connection_id: &str) {
        let mut health_data = self.health_data.write().await;
        if let Some(server_health) = health_data.get_mut(server_id) {
            server_health.remove(connection_id);
            if server_health.is_empty() {
                health_data.remove(server_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportType;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);

        // Add a server
        let server_config = TransportType::Stdio {
            command: "echo".to_string(),
            args: vec![],
            env: HashMap::new(),
            cwd: None,
        };

        assert!(pool.add_server("test-server".to_string(), server_config).await.is_ok());

        // Remove the server
        assert!(pool.remove_server("test-server").await.is_ok());
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);

        let stats = pool.stats().await;
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_connection_wrapper() {
        use crate::transport::stdio::StdioTransport;

        let transport = StdioTransport::new("echo".to_string(), vec![], HashMap::new(), None).unwrap();

        let mut conn = ConnectionWrapper::new("test-server".to_string(), Box::new(transport));

        assert!(!conn.in_use);
        assert_eq!(conn.use_count, 0);

        conn.mark_used();
        assert!(conn.in_use);
        assert_eq!(conn.use_count, 1);

        conn.mark_available();
        assert!(!conn.in_use);
        assert_eq!(conn.use_count, 1);
    }
}
