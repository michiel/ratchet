//! MCP service implementation for integration with Ratchet's service architecture

use async_trait::async_trait;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;
use tracing::{info, error};

use ratchet_lib::services::base::{Service, ServiceHealth, ServiceMetrics};
use ratchet_lib::execution::ProcessTaskExecutor;
use ratchet_lib::database::repositories::{TaskRepository, ExecutionRepository};

use crate::{McpResult, McpError, McpAuth};
use crate::server::{McpServer, McpServerConfig, McpServerTransport, RatchetMcpAdapter, RatchetToolRegistry};
use crate::security::{McpAuthManager, AuditLogger, SecurityConfig};

/// MCP service configuration
#[derive(Debug, Clone)]
pub struct McpServiceConfig {
    /// Server configuration
    pub server_config: McpServerConfig,
    /// Optional log file path for enhanced logging
    pub log_file_path: Option<std::path::PathBuf>,
}

impl Default for McpServiceConfig {
    fn default() -> Self {
        Self {
            server_config: McpServerConfig {
                transport: McpServerTransport::Stdio,
                security: SecurityConfig::default(),
                bind_address: None,
            },
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
        let mut tool_registry = RatchetToolRegistry::new();
        tool_registry.set_executor(Arc::new(adapter));

        // Create security components
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::None)); // TODO: Configure from config
        let audit_logger = Arc::new(AuditLogger::new(false)); // TODO: Make configurable

        // Create MCP server
        let server = McpServer::new(
            config.server_config.clone(),
            Arc::new(tool_registry),
            auth_manager,
            audit_logger,
        );

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
        match &self.config.server_config.transport {
            McpServerTransport::Stdio => {
                // For stdio transport, we run in the current task
                // The server will block until shutdown
                info!("Starting MCP server with STDIO transport");
                self.server.start().await?;
            }
            McpServerTransport::Sse { host, port, .. } => {
                // For SSE transport, spawn a background task
                info!("Starting MCP server with SSE transport on {}:{}", host, port);
                
                let server = self.server.clone();
                
                let handle = tokio::spawn(async move {
                    if let Err(e) = server.start().await {
                        error!("MCP server error: {}", e);
                    }
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
            matches!(self.config.server_config.transport, McpServerTransport::Stdio)
        }
    }

    /// Get server address (for SSE transport)
    pub fn server_address(&self) -> Option<SocketAddr> {
        match &self.config.server_config.transport {
            McpServerTransport::Sse { host, port, .. } => {
                format!("{}:{}", host, port).parse().ok()
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

    async fn initialize(_config: Self::Config) -> Result<Self, Self::Error>
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
        health = health.with_metadata("transport", match &self.config.server_config.transport {
            McpServerTransport::Stdio => "stdio",
            McpServerTransport::Sse { .. } => "sse",
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
    /// Create from Ratchet's MCP configuration
    pub async fn from_ratchet_config(
        mcp_config: &ratchet_lib::config::McpServerConfig,
        task_executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: Option<std::path::PathBuf>,
    ) -> McpResult<Self> {
        // Convert Ratchet config to MCP service config
        let transport = match mcp_config.transport.as_str() {
            "stdio" => McpServerTransport::Stdio,
            "sse" => McpServerTransport::Sse {
                host: mcp_config.host.clone(),
                port: mcp_config.port,
                tls: false, // TODO: Make configurable
                cors: crate::server::config::CorsConfig {
                    allowed_origins: vec!["*".to_string()], // TODO: Make configurable
                    allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
                    allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
                    allow_credentials: false,
                },
                timeout: std::time::Duration::from_secs(mcp_config.request_timeout),
            },
            _ => return Err(McpError::Configuration {
                message: format!("Unknown transport: {}", mcp_config.transport),
            }),
        };

        let security = SecurityConfig {
            max_execution_time: std::time::Duration::from_secs(mcp_config.request_timeout),
            max_log_entries: 1000,
            allow_dangerous_tasks: false,
            audit_log_enabled: true,
            input_sanitization: true,
            max_request_size: 1024 * 1024, // 1MB
            max_response_size: 10 * 1024 * 1024, // 10MB
            session_timeout: std::time::Duration::from_secs(3600), // 1 hour
            require_encryption: false,
        };

        let server_config = McpServerConfig {
            transport,
            security,
            bind_address: Some(format!("{}:{}", mcp_config.host, mcp_config.port)),
        };

        let config = McpServiceConfig {
            server_config,
            log_file_path,
        };

        Self::new(config, task_executor, task_repository, execution_repository).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratchet_lib::config::{RatchetConfig, DatabaseConfig};
    use ratchet_lib::database::{DatabaseConnection, repositories::RepositoryFactory};
    use ratchet_lib::services::base::HealthStatus;

    async fn create_test_dependencies() -> (Arc<ProcessTaskExecutor>, Arc<TaskRepository>, Arc<ExecutionRepository>) {
        let config = RatchetConfig::default();
        let db_config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: std::time::Duration::from_secs(10),
        };
        
        let db = DatabaseConnection::new(db_config).await.unwrap();
        db.migrate().await.unwrap();
        
        let repos = RepositoryFactory::new(db);
        let executor = Arc::new(ProcessTaskExecutor::new(repos.clone(), config).await.unwrap());
        
        (executor, Arc::new(repos.task_repository()), Arc::new(repos.execution_repository()))
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
        
        // Use SSE transport config so it properly shows as not running
        let config = McpServiceConfig {
            server_config: McpServerConfig {
                transport: McpServerTransport::Sse {
                    host: "localhost".to_string(),
                    port: 3001,
                    tls: false,
                    cors: crate::server::config::CorsConfig {
                        allowed_origins: vec!["*".to_string()],
                        allowed_methods: vec!["GET".to_string(), "POST".to_string()],
                        allowed_headers: vec!["Content-Type".to_string()],
                        allow_credentials: false,
                    },
                    timeout: std::time::Duration::from_secs(30),
                },
                security: SecurityConfig::default(),
                bind_address: Some("localhost:3001".to_string()),
            },
            log_file_path: None,
        };
        
        let service = McpService::new(
            config,
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