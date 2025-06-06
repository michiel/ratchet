use crate::config::RatchetConfig;
use crate::errors::JsExecutionError;
use crate::http::HttpManager;
use crate::task::{Task, TaskError};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use thiserror::Error;

pub mod base;
pub mod task_sync_service;

pub use base::{
    HealthStatus, RegistryError, Service, ServiceBuilder, ServiceHealth, ServiceMetrics,
    ServiceRegistry,
};
pub use task_sync_service::{TaskSyncService, UnifiedTask};

/// Service layer errors
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Task error: {0}")]
    TaskError(#[from] TaskError),

    #[error("Execution error: {0}")]
    ExecutionError(#[from] JsExecutionError),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),

    #[error("Service initialization error: {0}")]
    InitializationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Startup error: {0}")]
    StartupError(String),
}

/// Result type for service operations
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Task execution service trait for dependency injection
#[async_trait(?Send)]
pub trait TaskService {
    /// Load a task from filesystem path
    async fn load_task(&self, path: &str) -> ServiceResult<Task>;

    /// Execute a task with given input
    async fn execute_task(&self, task: &mut Task, input: JsonValue) -> ServiceResult<JsonValue>;

    async fn execute_task_with_context(
        &self,
        task: &mut Task,
        input: JsonValue,
        execution_context: Option<crate::execution::ipc::ExecutionContext>,
    ) -> ServiceResult<JsonValue>;

    /// Validate a task structure and syntax
    async fn validate_task(&self, task: &mut Task) -> ServiceResult<()>;

    /// Run tests for a task
    async fn run_task_tests(&self, task_path: &str) -> ServiceResult<crate::test::TestSummary>;
}

/// HTTP service trait for dependency injection
#[async_trait(?Send)]
pub trait HttpService {
    /// Make an HTTP request
    async fn make_request(
        &self,
        url: &str,
        params: Option<&JsonValue>,
        body: Option<&JsonValue>,
    ) -> ServiceResult<JsonValue>;
}

/// Configuration service trait for dependency injection  
pub trait ConfigService {
    /// Get the current configuration
    fn get_config(&self) -> &RatchetConfig;

    /// Update configuration (for runtime changes)
    fn update_config(&mut self, config: RatchetConfig) -> ServiceResult<()>;
}

/// Main service provider that coordinates all services
pub struct ServiceProvider {
    task_service: Box<dyn TaskService>,
    http_service: Box<dyn HttpService>,
    config_service: Box<dyn ConfigService>,
}

impl ServiceProvider {
    /// Create a new service provider with default implementations
    pub fn new(config: RatchetConfig) -> ServiceResult<Self> {
        let config_service = Box::new(DefaultConfigService::new(config));
        let http_service = Box::new(DefaultHttpService::new(config_service.get_config())?);
        let task_service = Box::new(DefaultTaskService::new(config_service.get_config())?);

        Ok(ServiceProvider {
            task_service,
            http_service,
            config_service,
        })
    }

    /// Create service provider with custom implementations (for testing/server)
    pub fn with_services(
        task_service: Box<dyn TaskService>,
        http_service: Box<dyn HttpService>,
        config_service: Box<dyn ConfigService>,
    ) -> Self {
        ServiceProvider {
            task_service,
            http_service,
            config_service,
        }
    }

    /// Get task service
    pub fn task_service(&self) -> &dyn TaskService {
        self.task_service.as_ref()
    }

    /// Get HTTP service
    pub fn http_service(&self) -> &dyn HttpService {
        self.http_service.as_ref()
    }

    /// Get configuration service
    pub fn config_service(&self) -> &dyn ConfigService {
        self.config_service.as_ref()
    }
}

/// Default implementation of ConfigService
pub struct DefaultConfigService {
    config: RatchetConfig,
}

impl DefaultConfigService {
    pub fn new(config: RatchetConfig) -> Self {
        Self { config }
    }
}

impl ConfigService for DefaultConfigService {
    fn get_config(&self) -> &RatchetConfig {
        &self.config
    }

    fn update_config(&mut self, config: RatchetConfig) -> ServiceResult<()> {
        self.config = config;
        Ok(())
    }
}

/// Default implementation of HttpService
pub struct DefaultHttpService {
    http_manager: HttpManager,
}

impl DefaultHttpService {
    pub fn new(config: &RatchetConfig) -> ServiceResult<Self> {
        let http_manager = HttpManager::with_config(config.http.clone());
        Ok(Self { http_manager })
    }
}

#[async_trait(?Send)]
impl HttpService for DefaultHttpService {
    async fn make_request(
        &self,
        url: &str,
        params: Option<&JsonValue>,
        body: Option<&JsonValue>,
    ) -> ServiceResult<JsonValue> {
        self.http_manager
            .call_http(url, params, body)
            .await
            .map_err(|e| {
                ServiceError::ExecutionError(JsExecutionError::ExecutionError(e.to_string()))
            })
    }
}

/// Default implementation of TaskService
pub struct DefaultTaskService {
    #[allow(dead_code)]
    config: Arc<RatchetConfig>,
}

impl DefaultTaskService {
    pub fn new(config: &RatchetConfig) -> ServiceResult<Self> {
        Ok(Self {
            config: Arc::new(config.clone()),
        })
    }
}

#[async_trait(?Send)]
impl TaskService for DefaultTaskService {
    async fn load_task(&self, path: &str) -> ServiceResult<Task> {
        Task::from_fs(path).map_err(ServiceError::TaskError)
    }

    async fn execute_task(&self, task: &mut Task, input: JsonValue) -> ServiceResult<JsonValue> {
        let http_manager = HttpManager::new();
        crate::js_executor::execute_task(task, input, &http_manager)
            .await
            .map_err(ServiceError::ExecutionError)
    }

    async fn execute_task_with_context(
        &self,
        task: &mut Task,
        input: JsonValue,
        execution_context: Option<crate::execution::ipc::ExecutionContext>,
    ) -> ServiceResult<JsonValue> {
        let http_manager = HttpManager::new();
        crate::js_executor::execute_task_with_context(task, input, &http_manager, execution_context)
            .await
            .map_err(ServiceError::ExecutionError)
    }

    async fn validate_task(&self, task: &mut Task) -> ServiceResult<()> {
        task.validate().map_err(ServiceError::TaskError)
    }

    async fn run_task_tests(&self, task_path: &str) -> ServiceResult<crate::test::TestSummary> {
        crate::test::run_tests(task_path)
            .await
            .map_err(|e| match e {
                crate::test::TestError::TaskError(task_err) => ServiceError::TaskError(task_err),
                crate::test::TestError::ExecutionError(js_err) => {
                    ServiceError::ExecutionError(js_err)
                }
                crate::test::TestError::InvalidTestFile(msg) => ServiceError::InvalidInput(msg),
                crate::test::TestError::NoTestsDirectory => {
                    ServiceError::InvalidInput("No tests directory found".to_string())
                }
                crate::test::TestError::IoError(e) => {
                    ServiceError::TaskError(TaskError::FileReadError(e))
                }
                crate::test::TestError::JsonParseError(e) => {
                    ServiceError::TaskError(TaskError::JsonParseError(e))
                }
            })
    }
}

/// High-level operations for CLI and server use
pub struct RatchetEngine {
    services: ServiceProvider,
}

impl RatchetEngine {
    /// Create a new Ratchet engine with configuration
    pub fn new(config: RatchetConfig) -> ServiceResult<Self> {
        let services = ServiceProvider::new(config)?;
        Ok(Self { services })
    }

    /// Create engine with custom service provider (for testing)
    pub fn with_services(services: ServiceProvider) -> Self {
        Self { services }
    }

    /// Execute a task from filesystem path with input
    pub async fn execute_task_from_path(
        &self,
        task_path: &str,
        input: JsonValue,
    ) -> ServiceResult<JsonValue> {
        let mut task = self.services.task_service().load_task(task_path).await?;
        self.services
            .task_service()
            .execute_task(&mut task, input)
            .await
    }

    /// Validate a task from filesystem path
    pub async fn validate_task_from_path(&self, task_path: &str) -> ServiceResult<Task> {
        let mut task = self.services.task_service().load_task(task_path).await?;
        self.services
            .task_service()
            .validate_task(&mut task)
            .await?;
        Ok(task)
    }

    /// Run tests for a task
    pub async fn run_tests(&self, task_path: &str) -> ServiceResult<crate::test::TestSummary> {
        self.services.task_service().run_task_tests(task_path).await
    }

    /// Get access to underlying services (for advanced usage)
    pub fn services(&self) -> &ServiceProvider {
        &self.services
    }

    /// Generate a new task template
    pub fn generate_task(
        &self,
        config: crate::generate::TaskGenerationConfig,
    ) -> ServiceResult<crate::generate::GeneratedTaskInfo> {
        crate::generate::generate_task(config)
            .map_err(|e| ServiceError::InitializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RatchetConfig;
    use serde_json::json;

    /// Mock task service for testing
    struct MockTaskService {
        should_fail: bool,
    }

    impl MockTaskService {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }

    #[async_trait(?Send)]
    impl TaskService for MockTaskService {
        async fn load_task(&self, _path: &str) -> ServiceResult<Task> {
            if self.should_fail {
                Err(ServiceError::TaskError(TaskError::TaskFileNotFound(
                    "mock error".to_string(),
                )))
            } else {
                // Return error to avoid file dependency in test
                Err(ServiceError::InvalidInput(
                    "Mock service - use real service for actual task loading".to_string(),
                ))
            }
        }

        async fn execute_task(
            &self,
            _task: &mut Task,
            input: JsonValue,
        ) -> ServiceResult<JsonValue> {
            if self.should_fail {
                Err(ServiceError::ExecutionError(
                    JsExecutionError::ExecutionError("mock error".to_string()),
                ))
            } else {
                Ok(input) // Echo input as output
            }
        }

        async fn execute_task_with_context(
            &self,
            _task: &mut Task,
            input: JsonValue,
            _execution_context: Option<crate::execution::ipc::ExecutionContext>,
        ) -> ServiceResult<JsonValue> {
            if self.should_fail {
                Err(ServiceError::ExecutionError(
                    JsExecutionError::ExecutionError("mock error".to_string()),
                ))
            } else {
                Ok(input) // Echo input as output
            }
        }

        async fn validate_task(&self, _task: &mut Task) -> ServiceResult<()> {
            if self.should_fail {
                Err(ServiceError::TaskError(TaskError::InvalidTaskStructure(
                    "mock error".to_string(),
                )))
            } else {
                Ok(())
            }
        }

        async fn run_task_tests(
            &self,
            _task_path: &str,
        ) -> ServiceResult<crate::test::TestSummary> {
            Ok(crate::test::TestSummary {
                total: 1,
                passed: if self.should_fail { 0 } else { 1 },
                failed: if self.should_fail { 1 } else { 0 },
                results: vec![],
            })
        }
    }

    #[tokio::test]
    async fn test_service_provider_creation() {
        let config = RatchetConfig::default();
        let provider = ServiceProvider::new(config);
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ratchet_engine_creation() {
        let config = RatchetConfig::default();
        let engine = RatchetEngine::new(config);
        assert!(engine.is_ok());
    }

    /// Mock HTTP service for testing
    struct MockHttpService {
        should_fail: bool,
    }

    impl MockHttpService {
        fn new(should_fail: bool) -> Self {
            Self { should_fail }
        }
    }

    #[async_trait(?Send)]
    impl HttpService for MockHttpService {
        async fn make_request(
            &self,
            _url: &str,
            _params: Option<&JsonValue>,
            _body: Option<&JsonValue>,
        ) -> ServiceResult<JsonValue> {
            if self.should_fail {
                Err(ServiceError::ExecutionError(
                    JsExecutionError::ExecutionError("mock http error".to_string()),
                ))
            } else {
                Ok(json!({"status": "mock_success", "data": "test_response"}))
            }
        }
    }

    #[tokio::test]
    async fn test_mock_task_service() {
        let config = RatchetConfig::default();
        let config_service = Box::new(DefaultConfigService::new(config.clone()));
        let http_service = Box::new(MockHttpService::new(false)); // Use mock instead of real HTTP
        let task_service = Box::new(MockTaskService::new(false));

        let services = ServiceProvider::with_services(task_service, http_service, config_service);

        // Test that mock service works as expected (returns error for file loading)
        let result = services.task_service().load_task("test").await;
        assert!(result.is_err());

        // Test HTTP service works (now using mock)
        let http_result = services
            .http_service()
            .make_request("http://example.com", None, None)
            .await;
        assert!(http_result.is_ok());

        // Verify mock response content
        let response = http_result.unwrap();
        assert_eq!(response["status"], "mock_success");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let config = RatchetConfig::default();
        let config_service = Box::new(DefaultConfigService::new(config.clone()));
        let http_service = Box::new(MockHttpService::new(false)); // Use mock HTTP service
        let task_service = Box::new(MockTaskService::new(true)); // Configure to fail

        let services = ServiceProvider::with_services(task_service, http_service, config_service);
        let engine = RatchetEngine::with_services(services);

        // Test error handling
        let result = engine
            .execute_task_from_path("test", json!({"test": "data"}))
            .await;
        assert!(result.is_err());
    }
}
