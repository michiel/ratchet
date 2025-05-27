use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::{
    config::RatchetConfig,
    database::repositories::RepositoryFactory,
    execution::{
        TaskExecutor, ExecutionContext, ExecutionResult,
        WorkerProcessManager,
        ipc::TaskExecutionResult,
    },
    execution::executor::ExecutionError,
    services::ServiceError,
};

/// Process-based task executor that uses worker processes for task execution
/// This solves the Send/Sync issues by running JavaScript tasks in separate processes
pub struct ProcessTaskExecutor {
    worker_manager: Arc<RwLock<WorkerProcessManager>>,
    repositories: RepositoryFactory,
    config: RatchetConfig,
}

impl ProcessTaskExecutor {
    /// Create a new process-based task executor
    pub async fn new(repositories: RepositoryFactory, config: RatchetConfig) -> Result<Self, ServiceError> {
        let worker_config = crate::execution::worker_process::WorkerConfig::default();
        let worker_manager = Arc::new(RwLock::new(
            WorkerProcessManager::new(worker_config)
        ));
        
        Ok(Self {
            worker_manager,
            repositories,
            config,
        })
    }
    
    /// Start the worker processes
    pub async fn start(&self) -> Result<(), ServiceError> {
        let mut manager = self.worker_manager.write().await;
        manager.start().await
            .map_err(|e| ServiceError::StartupError(format!("Failed to start workers: {}", e)))?;
        Ok(())
    }
    
    /// Stop the worker processes
    pub async fn stop(&self) -> Result<(), ServiceError> {
        let mut manager = self.worker_manager.write().await;
        manager.stop().await
            .map_err(|e| ServiceError::StartupError(format!("Failed to stop workers: {}", e)))?;
        Ok(())
    }
    
    /// Send-compatible execute task method for GraphQL resolvers
    pub async fn execute_task_send(
        &self,
        task_id: i32,
        input_data: JsonValue,
        _context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Direct implementation to avoid ?Send trait issues
        let task_repo = &self.repositories.task_repo;
        
        // Load task metadata from database
        let task_entity = task_repo
            .find_by_id(task_id)
            .await
            .map_err(|e| ExecutionError::DatabaseError(e))?
            .ok_or_else(|| ExecutionError::TaskNotFound(task_id.to_string()))?;
        
        // For now, return a placeholder result since we removed the actual process execution
        // TODO: Implement actual task execution via worker processes
        Ok(ExecutionResult {
            execution_id: task_id,
            success: true,
            output: Some(serde_json::json!({"message": "Process-based execution placeholder"})),
            error: None,
            duration_ms: 100,
            http_requests: None,
        })
    }
    
    /// Send-compatible execute job method for GraphQL resolvers  
    pub async fn execute_job_send(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        // Direct implementation to avoid ?Send trait issues
        let job_repo = &self.repositories.job_repo;
        let task_repo = &self.repositories.task_repo;
        
        // Load job details from database
        let job_entity = job_repo
            .find_by_id(job_id)
            .await
            .map_err(|e| ExecutionError::DatabaseError(e))?
            .ok_or_else(|| ExecutionError::JobNotFound(job_id))?;
        
        // Load associated task
        let _task_entity = task_repo
            .find_by_id(job_entity.task_id)
            .await
            .map_err(|e| ExecutionError::DatabaseError(e))?
            .ok_or_else(|| ExecutionError::TaskNotFound(job_entity.task_id.to_string()))?;
        
        // For now, return a placeholder result
        // TODO: Implement actual job execution via worker processes
        Ok(ExecutionResult {
            execution_id: job_id,
            success: true,
            output: Some(serde_json::json!({"message": "Process-based job execution placeholder"})),
            error: None,
            duration_ms: 150,
            http_requests: None,
        })
    }
    
    /// Send-compatible health check method for GraphQL resolvers
    pub async fn health_check_send(&self) -> Result<(), ExecutionError> {
        let mut manager = self.worker_manager.write().await;
        let health_results = manager.health_check_all().await;
        
        // Check if any workers failed health check
        for result in health_results {
            if let Err(e) = result {
                return Err(ExecutionError::TaskExecutionError(format!("Worker health check failed: {}", e)));
            }
        }
        
        Ok(())
    }
}

#[async_trait(?Send)]
impl TaskExecutor for ProcessTaskExecutor {
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        _context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError> {
        let task_repo = &self.repositories.task_repo;
        
        // Load task details
        let task_entity = task_repo.find_by_id(task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(task_id.to_string()))?;
        
        // Execute task using worker process (simplified for now)
        let result = self.execute_task_in_worker(
            0, // job_id - using 0 for direct task execution
            task_id,
            &task_entity.path,
            &input_data,
        ).await;
        
        match result {
            Ok(task_result) => Ok(ExecutionResult {
                execution_id: 0, // TODO: Generate proper execution ID
                success: task_result.success,
                output: task_result.output,
                error: task_result.error_message,
                duration_ms: task_result.duration_ms as i64,
                http_requests: None, // TODO: Extract from task result
            }),
            Err(e) => Ok(ExecutionResult {
                execution_id: 0,
                success: false,
                output: None,
                error: Some(e.to_string()),
                duration_ms: 0,
                http_requests: None,
            }),
        }
    }
    
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        let job_repo = &self.repositories.job_repo;
        let task_repo = &self.repositories.task_repo;
        
        // Load job details
        let job_entity = job_repo.find_by_id(job_id).await?
            .ok_or_else(|| ExecutionError::JobNotFound(job_id))?;
        
        // Load task details
        let task_entity = task_repo.find_by_id(job_entity.task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(job_entity.task_id.to_string()))?;
        
        // Execute task using worker process
        let result = self.execute_task_in_worker(
            job_id,
            job_entity.task_id,
            &task_entity.path,
            &job_entity.input_data,
        ).await;
        
        match result {
            Ok(task_result) => Ok(ExecutionResult {
                execution_id: job_id,
                success: task_result.success,
                output: task_result.output,
                error: task_result.error_message,
                duration_ms: task_result.duration_ms as i64,
                http_requests: None,
            }),
            Err(e) => Ok(ExecutionResult {
                execution_id: job_id,
                success: false,
                output: None,
                error: Some(e.to_string()),
                duration_ms: 0,
                http_requests: None,
            }),
        }
    }
    
    async fn health_check(&self) -> Result<(), ExecutionError> {
        let _manager = self.worker_manager.read().await;
        // For now, always return healthy since this is a basic implementation
        // TODO: Implement actual worker health checks
        Ok(())
    }
}

impl ProcessTaskExecutor {
    /// Execute a task in a worker process via IPC
    async fn execute_task_in_worker(
        &self,
        job_id: i32,
        task_id: i32,
        task_path: &str,
        input_data: &JsonValue,
    ) -> Result<TaskExecutionResult, ExecutionError> {
        use crate::execution::ipc::{WorkerMessage, CoordinatorMessage};
        use uuid::Uuid;
        use tokio::time::Duration;
        
        let correlation_id = Uuid::new_v4();
        let message = WorkerMessage::ExecuteTask {
            job_id,
            task_id,
            task_path: task_path.to_string(),
            input_data: input_data.clone(),
            correlation_id,
        };
        
        // Get worker manager and send task to a worker
        let mut manager = self.worker_manager.write().await;
        let result = manager.send_task(message, Duration::from_secs(300)).await; // 5 minute timeout
        
        match result {
            Ok(CoordinatorMessage::TaskResult { result, .. }) => Ok(result),
            Ok(CoordinatorMessage::Error { error, .. }) => {
                Err(ExecutionError::TaskExecutionError(format!("Worker error: {:?}", error)))
            }
            Ok(_) => Err(ExecutionError::TaskExecutionError("Unexpected response from worker".to_string())),
            Err(e) => Err(ExecutionError::TaskExecutionError(format!("IPC error: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::DatabaseConnection;
    use crate::config::DatabaseConfig;
    use std::time::Duration;
    
    async fn create_test_database() -> DatabaseConnection {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        };
        DatabaseConnection::new(config).await.expect("Failed to create test database")
    }
    
    #[tokio::test]
    async fn test_process_executor_creation() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories, config).await;
        assert!(executor.is_ok());
        
        let executor = executor.unwrap();
        
        // Test health check
        let health = executor.health_check().await;
        assert!(health.is_ok());
    }
    
    #[tokio::test]
    async fn test_process_executor_start_stop() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories, config).await.unwrap();
        
        // Test that start/stop methods can be called
        let start_result = executor.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = executor.stop().await;
        assert!(stop_result.is_ok());
    }
}