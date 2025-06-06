//! MCP service implementation for integration with Ratchet's service architecture

use async_trait::async_trait;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;

use ratchet_lib::services::base::{Service, ServiceHealth, ServiceMetrics, HealthStatus};
use ratchet_lib::execution::ProcessTaskExecutor;
use ratchet_storage::seaorm::repositories::{
    task_repository::TaskRepository,
    execution_repository::ExecutionRepository,
};

use crate::{McpResult, McpError};
use crate::server::{McpServer, McpServerBuilder};
use crate::server::adapter::RatchetMcpAdapter;
use crate::server::tools::RatchetToolRegistry;
use crate::transport::{StdioTransport};
use crate::security::SecurityConfig;

/// Simple transport configuration for MCP service
#[derive(Debug, Clone)]
pub enum SimpleTransportConfig {
    Stdio,
    Sse { bind_address: String },
}

impl Default for SimpleTransportConfig {
    fn default() -> Self {
        Self::Stdio
    }
}

/// MCP service configuration
#[derive(Debug, Clone)]
pub struct McpServiceConfig {
    /// Transport configuration
    pub transport: SimpleTransportConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Optional log file path for enhanced logging
    pub log_file_path: Option<std::path::PathBuf>,
}

impl Default for McpServiceConfig {
    fn default() -> Self {
        Self {
            transport: SimpleTransportConfig::Stdio,
            security: SecurityConfig::default(),
            log_file_path: None,
        }
    }
}

/// MCP service that can be integrated into Ratchet's service architecture
pub struct McpService {
    /// MCP server instance
    server: Arc<McpServer>,
    /// Server task handle (for SSE transport)
    server_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Service configuration
    config: McpServiceConfig,
    /// Service metrics
    metrics: Arc<Mutex<ServiceMetrics>>,
    /// Service start time
    start_time: std::time::Instant,
}

impl McpService {
    /// Create a new MCP service with repositories and executor
    pub async fn new(
        config: McpServiceConfig,
        task_executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> McpResult<Self> {
        // Create the MCP adapter
        let adapter = if let Some(log_path) = &config.log_file_path {
            RatchetMcpAdapter::with_log_file(
                task_executor,
                task_repository,
                execution_repository,
                log_path.clone(),
            )
        } else {
            RatchetMcpAdapter::new(
                task_executor,
                task_repository,
                execution_repository,
            )
        };

        // Create tool registry with the adapter
        let tool_registry = RatchetToolRegistry::new()
            .with_task_executor(Arc::new(adapter));

        // Build MCP server
        let server = McpServerBuilder::new()
            .with_tool_registry(Arc::new(tool_registry))
            .with_security(config.security.clone())
            .build()?;

        Ok(Self {
            server: Arc::new(server),
            server_handle: Arc::new(Mutex::new(None)),
            config,
            metrics: Arc::new(Mutex::new(ServiceMetrics::default())),
            start_time: std::time::Instant::now(),
        })
    }

    /// Start the MCP server
    pub async fn start(&self) -> McpResult<()> {
        match &self.config.transport {
            SimpleTransportConfig::Stdio => {
                // For stdio transport, we run in the current task
                // The server will block until shutdown
                info!("Starting MCP server with STDIO transport");
                let transport = StdioTransport::new();
                self.server.run(Box::new(transport)).await?;
            }
            SimpleTransportConfig::Sse { bind_address } => {
                // For SSE transport, spawn a background task
                info!("Starting MCP server with SSE transport on {}", bind_address);
                
                let server = self.server.clone();
                let bind_address = bind_address.clone();
                
                let handle = tokio::spawn(async move {
                    // TODO: Implement SSE transport and start server
                    warn!("SSE transport not yet implemented");
                    // For now, just sleep to simulate running server
                    tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
                });

                let mut server_handle = self.server_handle.lock().await;
                *server_handle = Some(handle);
            }
        }

        Ok(())
    }

    /// Stop the MCP server
    pub async fn stop(&self) -> McpResult<()> {
        info!("Stopping MCP server");
        
        // Cancel the server task if running
        let mut handle_guard = self.server_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            // Wait for task to finish (with timeout)
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                handle
            ).await;
        }

        Ok(())
    }

    /// Check if the server is running
    pub async fn is_running(&self) -> bool {
        let handle_guard = self.server_handle.lock().await;
        if let Some(handle) = &*handle_guard {
            !handle.is_finished()
        } else {
            // For stdio transport, we can't easily check
            // Assume it's running if we haven't explicitly stopped it
            matches!(self.config.transport, SimpleTransportConfig::Stdio)
        }
    }

    /// Get server address (for SSE transport)
    pub fn server_address(&self) -> Option<SocketAddr> {
        match &self.config.transport {
            SimpleTransportConfig::Sse { bind_address } => {
                bind_address.parse().ok()
            }
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpServiceError {
    #[error("MCP error: {0}")]
    McpError(#[from] McpError),
    
    #[error("Service configuration error: {0}")]
    ConfigError(String),
    
    #[error("Service initialization failed: {0}")]
    InitializationFailed(String),
}

#[async_trait]
impl Service for McpService {
    type Error = McpServiceError;
    type Config = McpServiceConfig;

    async fn initialize(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        // This would need the repositories and executor to be passed somehow
        // For now, return an error as we can't create a complete service without dependencies
        Err(McpServiceError::InitializationFailed(
            "McpService requires repositories and executor - use McpService::new instead".to_string()
        ))
    }

    fn name(&self) -> &'static str {
        "mcp-server"
    }

    async fn health_check(&self) -> Result<ServiceHealth, Self::Error> {
        let is_running = self.is_running().await;
        
        let mut health = if is_running {
            ServiceHealth::healthy()
                .with_message("MCP server is running")
        } else {
            ServiceHealth::unhealthy("MCP server is not running")
        };

        // Add transport info
        health = health.with_metadata("transport", match &self.config.transport {
            SimpleTransportConfig::Stdio => "stdio",
            SimpleTransportConfig::Sse { .. } => "sse",
        });

        // Add server address for SSE
        if let Some(addr) = self.server_address() {
            health = health.with_metadata("address", addr.to_string());
        }

        // Add metrics
        let metrics = self.metrics.lock().await;
        health = health
            .with_metadata("requests_total", metrics.requests_total)
            .with_metadata("requests_failed", metrics.requests_failed)
            .with_metadata("uptime_seconds", self.start_time.elapsed().as_secs());

        Ok(health)
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        self.stop().await?;
        Ok(())
    }

    fn metrics(&self) -> ServiceMetrics {
        // Return a clone of the current metrics
        // In a real implementation, this would be updated by the MCP server
        let metrics = self.metrics.blocking_lock();
        metrics.clone()
    }

    fn config(&self) -> Option<&Self::Config> {
        Some(&self.config)
    }
}

/// Builder for creating MCP service with all dependencies
pub struct McpServiceBuilder {
    config: McpServiceConfig,
    task_executor: Option<Arc<ProcessTaskExecutor>>,
    task_repository: Option<Arc<TaskRepository>>,
    execution_repository: Option<Arc<ExecutionRepository>>,
}

impl McpServiceBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: McpServiceConfig::default(),
            task_executor: None,
            task_repository: None,
            execution_repository: None,
        }
    }

    /// Set the service configuration
    pub fn with_config(mut self, config: McpServiceConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the task executor
    pub fn with_task_executor(mut self, executor: Arc<ProcessTaskExecutor>) -> Self {
        self.task_executor = Some(executor);
        self
    }

    /// Set the task repository
    pub fn with_task_repository(mut self, repo: Arc<TaskRepository>) -> Self {
        self.task_repository = Some(repo);
        self
    }

    /// Set the execution repository
    pub fn with_execution_repository(mut self, repo: Arc<ExecutionRepository>) -> Self {
        self.execution_repository = Some(repo);
        self
    }

    /// Build the MCP service
    pub async fn build(self) -> Result<McpService, McpServiceError> {
        let task_executor = self.task_executor
            .ok_or_else(|| McpServiceError::InitializationFailed("Task executor is required".to_string()))?;
        
        let task_repository = self.task_repository
            .ok_or_else(|| McpServiceError::InitializationFailed("Task repository is required".to_string()))?;
            
        let execution_repository = self.execution_repository
            .ok_or_else(|| McpServiceError::InitializationFailed("Execution repository is required".to_string()))?;

        McpService::new(
            self.config,
            task_executor,
            task_repository,
            execution_repository,
        ).await.map_err(Into::into)
    }
}

impl Default for McpServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration helper to create MCP service from Ratchet config
impl McpService {
    /// Create from Ratchet's new modular MCP configuration
    pub async fn from_new_ratchet_config(
        mcp_config: &ratchet_config::McpConfig,
        task_executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: Option<std::path::PathBuf>,
    ) -> McpResult<Self> {
        // Convert new config to MCP service config
        let transport = match mcp_config.transport.as_str() {
            "stdio" => SimpleTransportConfig::Stdio,
            "sse" => SimpleTransportConfig::Sse {
                bind_address: format!("{}:{}", mcp_config.host, mcp_config.port),
            },
            _ => return Err(McpError::Configuration {
                message: format!("Unknown transport: {}", mcp_config.transport),
            }),
        };

        let security = SecurityConfig::default();

        let config = McpServiceConfig {
            transport,
            security,
            log_file_path,
        };

        Self::new(config, task_executor, task_repository, execution_repository).await
    }

    /// Create from Ratchet's legacy MCP configuration (for backward compatibility)
    pub async fn from_ratchet_config(
        mcp_config: &ratchet_lib::config::McpServerConfig,
        task_executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: Option<std::path::PathBuf>,
    ) -> McpResult<Self> {
        // Convert Ratchet config to MCP service config
        let transport = match mcp_config.transport.as_str() {
            "stdio" => TransportConfig::Stdio,
            "sse" => TransportConfig::Sse {
                bind_address: format!("{}:{}", mcp_config.host, mcp_config.port),
                cors_origins: vec!["*".to_string()], // TODO: Make configurable
            },
            _ => return Err(McpError::InvalidConfiguration {
                field: "transport".to_string(),
                reason: format!("Unknown transport: {}", mcp_config.transport),
            }),
        };

        let security = SecurityConfig {
            enable_auth: mcp_config.auth_type != "none",
            api_keys: mcp_config.api_key.as_ref().map(|k| vec![k.clone()]).unwrap_or_default(),
            rate_limit_per_minute: mcp_config.rate_limit_per_minute as usize,
            allowed_tools: None, // Allow all tools by default
        };

        let config = McpServiceConfig {
            transport,
            security,
            log_file_path,
        };

        Self::new(config, task_executor, task_repository, execution_repository).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use ratchet_lib::config::RatchetConfig;
    use ratchet_lib::database::{DatabaseConnection, repositories::RepositoryFactory};

    async fn create_test_dependencies() -> (Arc<ProcessTaskExecutor>, Arc<TaskRepository>, Arc<ExecutionRepository>) {
        let config = RatchetConfig::default();
        let db = DatabaseConnection::new(config.database.clone()).await.unwrap();
        db.migrate().await.unwrap();
        
        let repos = RepositoryFactory::new(db);
        let executor = Arc::new(ProcessTaskExecutor::new(repos.clone(), config).await.unwrap());
        
        (executor, repos.task_repository(), repos.execution_repository())
    }

    #[tokio::test]
    async fn test_mcp_service_creation() {
        let (executor, task_repo, exec_repo) = create_test_dependencies().await;
        
        let service = McpService::new(
            McpServiceConfig::default(),
            executor,
            task_repo,
            exec_repo,
        ).await;
        
        assert!(service.is_ok());
        let service = service.unwrap();
        assert_eq!(service.name(), "mcp-server");
    }

    #[tokio::test]
    async fn test_mcp_service_health_check() {
        let (executor, task_repo, exec_repo) = create_test_dependencies().await;
        
        let service = McpService::new(
            McpServiceConfig::default(),
            executor,
            task_repo,
            exec_repo,
        ).await.unwrap();
        
        let health = service.health_check().await.unwrap();
        // Service is not started, so it should be unhealthy
        assert!(matches!(health.status, HealthStatus::Unhealthy { .. }));
    }

    #[tokio::test]
    async fn test_mcp_service_builder() {
        let (executor, task_repo, exec_repo) = create_test_dependencies().await;
        
        let service = McpServiceBuilder::new()
            .with_task_executor(executor)
            .with_task_repository(task_repo)
            .with_execution_repository(exec_repo)
            .build()
            .await;
            
        assert!(service.is_ok());
    }
}