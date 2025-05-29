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
    _config: RatchetConfig,
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
            _config: config,
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
        let execution_repo = &self.repositories.execution_repo;
        
        // Load task details
        let task_entity = task_repo.find_by_id(task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(task_id.to_string()))?;
        
        // Create execution record
        let execution = crate::database::entities::Execution::new(task_id, input_data.clone());
        let created_execution = execution_repo.create(execution).await?;
        let execution_id = created_execution.id;
        
        // Execute task using worker process
        let result = self.execute_task_in_worker(
            0, // job_id - using 0 for direct task execution
            task_id,
            &task_entity.path,
            &input_data,
        ).await;
        
        // Update execution record with results
        match result {
            Ok(task_result) => {
                let output_for_db = task_result.output.clone().unwrap_or_else(|| serde_json::json!({}));
                execution_repo.mark_completed(execution_id, output_for_db).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: task_result.success,
                    output: task_result.output,
                    error: task_result.error_message,
                    duration_ms: task_result.duration_ms as i64,
                    http_requests: None,
                })
            },
            Err(e) => {
                execution_repo.mark_failed(
                    execution_id,
                    e.to_string(),
                    None,
                ).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                    http_requests: None,
                })
            },
        }
    }
    
    /// Send-compatible execute job method for GraphQL resolvers  
    pub async fn execute_job_send(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        // Direct implementation to avoid ?Send trait issues
        let job_repo = &self.repositories.job_repo;
        let task_repo = &self.repositories.task_repo;
        let execution_repo = &self.repositories.execution_repo;
        
        // Load job details
        let job_entity = job_repo.find_by_id(job_id).await?
            .ok_or_else(|| ExecutionError::JobNotFound(job_id))?;
        
        // Load task details
        let task_entity = task_repo.find_by_id(job_entity.task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(job_entity.task_id.to_string()))?;
        
        // Create execution record for this job
        let execution = crate::database::entities::Execution::new(job_entity.task_id, job_entity.input_data.clone());
        let created_execution = execution_repo.create(execution).await?;
        let execution_id = created_execution.id;
        
        // Update job status to processing
        job_repo.mark_processing(job_id, execution_id).await?;
        
        // Execute task using worker process
        let result = self.execute_task_in_worker(
            job_id,
            job_entity.task_id,
            &task_entity.path,
            &job_entity.input_data,
        ).await;
        
        // Update both execution and job records with results
        match result {
            Ok(task_result) => {
                let output_for_db = task_result.output.clone().unwrap_or_else(|| serde_json::json!({}));
                execution_repo.mark_completed(execution_id, output_for_db).await?;
                
                job_repo.mark_completed(job_id).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: task_result.success,
                    output: task_result.output,
                    error: task_result.error_message,
                    duration_ms: task_result.duration_ms as i64,
                    http_requests: None,
                })
            },
            Err(e) => {
                execution_repo.mark_failed(
                    execution_id,
                    e.to_string(),
                    None,
                ).await?;
                
                job_repo.mark_failed(job_id, e.to_string(), None).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                    http_requests: None,
                })
            },
        }
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
    
    /// Create a test instance with minimal configuration
    #[cfg(test)]
    pub fn new_test() -> Self {
        use crate::database::connection::DatabaseConnection;
        
        // Create minimal repositories with in-memory database
        let db_config = crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
            connection_timeout: std::time::Duration::from_secs(5),
        };
        
        let db_connection = futures::executor::block_on(async {
            DatabaseConnection::new(db_config).await.unwrap()
        });
        let repositories = RepositoryFactory::new(db_connection);
        
        // Create minimal config from example file
        let config = RatchetConfig::from_file("example-config.yaml")
            .unwrap_or_else(|_| RatchetConfig::default());
        
        let worker_config = crate::execution::worker_process::WorkerConfig::default();
        let worker_manager = Arc::new(RwLock::new(
            WorkerProcessManager::new(worker_config)
        ));
        
        Self {
            worker_manager,
            repositories,
            _config: config,
        }
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
        let execution_repo = &self.repositories.execution_repo;
        
        // Load task details
        let task_entity = task_repo.find_by_id(task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(task_id.to_string()))?;
        
        // Create execution record
        let execution = crate::database::entities::Execution::new(task_id, input_data.clone());
        let created_execution = execution_repo.create(execution).await?;
        let execution_id = created_execution.id;
        
        // Execute task using worker process
        let result = self.execute_task_in_worker(
            0, // job_id - using 0 for direct task execution
            task_id,
            &task_entity.path,
            &input_data,
        ).await;
        
        // Update execution record with results
        match result {
            Ok(task_result) => {
                let output_for_db = task_result.output.clone().unwrap_or_else(|| serde_json::json!({}));
                execution_repo.mark_completed(execution_id, output_for_db).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: task_result.success,
                    output: task_result.output,
                    error: task_result.error_message,
                    duration_ms: task_result.duration_ms as i64,
                    http_requests: None, // TODO: Extract from task result
                })
            },
            Err(e) => {
                execution_repo.mark_failed(
                    execution_id,
                    e.to_string(),
                    None,
                ).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                    http_requests: None,
                })
            },
        }
    }
    
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        let job_repo = &self.repositories.job_repo;
        let task_repo = &self.repositories.task_repo;
        let execution_repo = &self.repositories.execution_repo;
        
        // Load job details
        let job_entity = job_repo.find_by_id(job_id).await?
            .ok_or_else(|| ExecutionError::JobNotFound(job_id))?;
        
        // Load task details
        let task_entity = task_repo.find_by_id(job_entity.task_id).await?
            .ok_or_else(|| ExecutionError::TaskNotFound(job_entity.task_id.to_string()))?;
        
        // Create execution record for this job
        let execution = crate::database::entities::Execution::new(job_entity.task_id, job_entity.input_data.clone());
        let created_execution = execution_repo.create(execution).await?;
        let execution_id = created_execution.id;
        
        // Update job status to processing
        job_repo.mark_processing(job_id, execution_id).await?;
        
        // Execute task using worker process
        let result = self.execute_task_in_worker(
            job_id,
            job_entity.task_id,
            &task_entity.path,
            &job_entity.input_data,
        ).await;
        
        // Update both execution and job records with results
        match result {
            Ok(task_result) => {
                let output_for_db = task_result.output.clone().unwrap_or_else(|| serde_json::json!({}));
                execution_repo.mark_completed(execution_id, output_for_db).await?;
                
                job_repo.mark_completed(job_id).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: task_result.success,
                    output: task_result.output,
                    error: task_result.error_message,
                    duration_ms: task_result.duration_ms as i64,
                    http_requests: None,
                })
            },
            Err(e) => {
                execution_repo.mark_failed(
                    execution_id,
                    e.to_string(),
                    None,
                ).await?;
                
                job_repo.mark_failed(job_id, e.to_string(), None).await?;
                
                Ok(ExecutionResult {
                    execution_id,
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                    http_requests: None,
                })
            },
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
    use crate::database::entities::{Job as JobEntity};
    use std::time::Duration;
    use tempfile::tempdir;
    use std::fs;
    use serde_json::json;
    
    async fn create_test_database() -> DatabaseConnection {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            connection_timeout: Duration::from_secs(10),
        };
        let db = DatabaseConnection::new(config).await.expect("Failed to create test database");
        
        // Run migrations to create tables
        db.migrate().await.expect("Failed to run migrations");
        
        db
    }
    
    async fn create_test_task_in_db(repositories: &RepositoryFactory) -> (i32, String) {
        // Create a temporary task directory with files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let task_path = temp_dir.path();
        
        // Create metadata.json
        let metadata = json!({
            "uuid": "test-task-uuid",
            "version": "1.0.0",
            "label": "Test Task",
            "description": "A test task for process executor testing"
        });
        fs::write(task_path.join("metadata.json"), serde_json::to_string_pretty(&metadata).unwrap())
            .expect("Failed to write metadata");
        
        // Create input schema
        let input_schema = json!({
            "type": "object",
            "properties": {
                "num1": { "type": "number" },
                "num2": { "type": "number" }
            },
            "required": ["num1", "num2"]
        });
        fs::write(task_path.join("input.schema.json"), serde_json::to_string_pretty(&input_schema).unwrap())
            .expect("Failed to write input schema");
        
        // Create output schema
        let output_schema = json!({
            "type": "object",
            "properties": {
                "sum": { "type": "number" }
            },
            "required": ["sum"]
        });
        fs::write(task_path.join("output.schema.json"), serde_json::to_string_pretty(&output_schema).unwrap())
            .expect("Failed to write output schema");
        
        // Create main.js
        let main_js = r#"(function(input) {
            const {num1, num2} = input;
            if (typeof num1 !== 'number' || typeof num2 !== 'number') {
                throw new Error('num1 and num2 must be numbers');
            }
            return { sum: num1 + num2 };
        })"#;
        fs::write(task_path.join("main.js"), main_js)
            .expect("Failed to write main.js");
        
        let task_path_str = task_path.to_string_lossy().to_string();
        
        // Create task entity in database
        let task_entity = crate::database::entities::tasks::Model {
            id: 0, // Will be set by database
            uuid: uuid::Uuid::new_v4(), // Generate unique UUID for each test
            name: "Test Task".to_string(),
            description: Some("A test task for process executor testing".to_string()),
            version: "1.0.0".to_string(),
            path: task_path_str.clone(),
            metadata: sea_orm::entity::prelude::Json::from(metadata),
            input_schema: sea_orm::entity::prelude::Json::from(input_schema),
            output_schema: sea_orm::entity::prelude::Json::from(output_schema),
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None,
        };
        
        let created_task = repositories.task_repo.create(task_entity).await
            .expect("Failed to create task in database");
        
        // Keep temp dir alive by leaking it (for test purposes only)
        std::mem::forget(temp_dir);
        
        (created_task.id, task_path_str)
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
    
    #[tokio::test]
    async fn test_process_executor_health_check_send() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories, config).await.unwrap();
        
        // Test Send-compatible health check method
        let health = executor.health_check_send().await;
        // Note: This will succeed since the basic health_check_all returns empty Vec
        assert!(health.is_ok());
    }
    
    #[tokio::test]
    async fn test_execute_task_send_with_database_integration() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
        
        // Create a test task in the database
        let (task_id, _task_path) = create_test_task_in_db(&repositories).await;
        
        // Test input data
        let input_data = json!({ "num1": 5, "num2": 3 });
        
        // Execute task (this will create execution records even if worker fails)
        let result = executor.execute_task_send(task_id, input_data.clone(), None).await;
        
        // Should always return a result (success or failure)
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        
        // Verify execution record was created
        assert!(execution_result.execution_id > 0);
        
        // Verify execution was recorded in database
        let execution = repositories.execution_repo
            .find_by_id(execution_result.execution_id).await
            .expect("Database query should succeed")
            .expect("Execution record should exist");
        
        assert_eq!(execution.task_id, task_id);
        // Compare JSON values (note: Sea-ORM Json wraps serde_json::Value)
        let execution_input: serde_json::Value = execution.input.clone().into();
        assert_eq!(execution_input, input_data);
        
        // Status should be either completed or failed (depending on worker availability)
        use crate::database::entities::executions::ExecutionStatus;
        assert!(matches!(execution.status, ExecutionStatus::Completed | ExecutionStatus::Failed));
    }
    
    #[tokio::test]
    async fn test_execute_job_send_with_database_integration() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
        
        // Create a test task in the database
        let (task_id, _task_path) = create_test_task_in_db(&repositories).await;
        
        // Create a test job in the database
        let input_data = json!({ "num1": 10, "num2": 20 });
        let job_entity = JobEntity::new(
            task_id,
            input_data.clone(),
            crate::database::entities::jobs::JobPriority::Normal,
        );
        
        let created_job = repositories.job_repo.create(job_entity).await
            .expect("Failed to create job in database");
        
        // Execute job
        let result = executor.execute_job_send(created_job.id).await;
        
        // Should always return a result (success or failure)
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        
        // Verify execution record was created
        assert!(execution_result.execution_id > 0);
        
        // Verify both execution and job were updated in database
        let execution = repositories.execution_repo
            .find_by_id(execution_result.execution_id).await
            .expect("Database query should succeed")
            .expect("Execution record should exist");
        
        let job = repositories.job_repo
            .find_by_id(created_job.id).await
            .expect("Database query should succeed")
            .expect("Job record should exist");
        
        assert_eq!(execution.task_id, task_id);
        // Compare JSON values (note: Sea-ORM Json wraps serde_json::Value)
        let execution_input: serde_json::Value = execution.input.clone().into();
        assert_eq!(execution_input, input_data);
        
        // Both execution and job status should be updated (may be completed, failed, or retrying since workers might not be available)
        use crate::database::entities::executions::ExecutionStatus;
        use crate::database::entities::jobs::JobStatus;
        assert!(matches!(execution.status, ExecutionStatus::Completed | ExecutionStatus::Failed));
        
        // Job status should be updated from the initial "Queued" status
        // In test environment without real workers, it may be Failed, Retrying, or Completed
        assert!(!matches!(job.status, JobStatus::Queued));
        assert!(matches!(job.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Retrying));
        
        // Job should reference the execution
        assert_eq!(job.execution_id, Some(execution_result.execution_id));
    }
    
    #[tokio::test]
    async fn test_execute_task_send_nonexistent_task() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories, config).await.unwrap();
        
        // Try to execute a non-existent task
        let input_data = json!({ "test": "data" });
        let result = executor.execute_task_send(99999, input_data, None).await;
        
        // Should return an error for non-existent task
        assert!(result.is_err());
        
        if let Err(ExecutionError::TaskNotFound(task_id)) = result {
            assert_eq!(task_id, "99999");
        } else {
            panic!("Expected TaskNotFound error");
        }
    }
    
    #[tokio::test]
    async fn test_execute_job_send_nonexistent_job() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories, config).await.unwrap();
        
        // Try to execute a non-existent job
        let result = executor.execute_job_send(99999).await;
        
        // Should return an error for non-existent job
        assert!(result.is_err());
        
        if let Err(ExecutionError::JobNotFound(job_id)) = result {
            assert_eq!(job_id, 99999);
        } else {
            panic!("Expected JobNotFound error, got: {:?}", result);
        }
    }
    
    #[tokio::test]
    async fn test_worker_manager_integration() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let _executor = ProcessTaskExecutor::new(repositories, config).await.unwrap();
        
        // Test that worker manager is properly initialized
        let worker_manager = &_executor.worker_manager;
        let manager_guard = worker_manager.read().await;
        
        // Should have worker stats (even if empty)
        let stats = manager_guard.get_worker_stats().await;
        assert!(stats.len() == 0); // No workers started yet
        
        drop(manager_guard);
        
        // Test starting and stopping workers
        let start_result = _executor.start().await;
        assert!(start_result.is_ok(), "Worker start should succeed or fail gracefully");
        
        let stop_result = _executor.stop().await;
        assert!(stop_result.is_ok(), "Worker stop should succeed");
    }
    
    #[tokio::test]
    async fn test_ipc_message_handling() {
        use crate::execution::ipc::{WorkerMessage};
        use uuid::Uuid;
        
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let _executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
        
        // Create a test task
        let (task_id, task_path) = create_test_task_in_db(&repositories).await;
        
        // Test IPC message creation for task execution
        let input_data = json!({ "num1": 7, "num2": 8 });
        
        // This tests the message preparation logic in execute_task_in_worker
        // Even though workers aren't actually running, we can verify the message structure
        let correlation_id = Uuid::new_v4();
        let message = WorkerMessage::ExecuteTask {
            job_id: 0,
            task_id,
            task_path: task_path.clone(),
            input_data: input_data.clone(),
            correlation_id,
        };
        
        // Verify message serialization
        let serialized = serde_json::to_string(&message);
        assert!(serialized.is_ok());
        
        // Verify message can be deserialized
        let deserialized: Result<WorkerMessage, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());
        
        if let Ok(WorkerMessage::ExecuteTask { task_id: tid, input_data: input, .. }) = deserialized {
            assert_eq!(tid, task_id);
            assert_eq!(input, input_data);
        } else {
            panic!("Failed to deserialize WorkerMessage correctly");
        }
    }
    
    #[tokio::test] 
    async fn test_concurrent_task_execution() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = std::sync::Arc::new(
            ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap()
        );
        
        // Create multiple test tasks
        let (task_id1, _) = create_test_task_in_db(&repositories).await;
        let (task_id2, _) = create_test_task_in_db(&repositories).await;
        
        // Execute tasks concurrently
        let executor1 = executor.clone();
        let executor2 = executor.clone();
        
        let handle1 = tokio::spawn(async move {
            executor1.execute_task_send(
                task_id1,
                json!({ "num1": 1, "num2": 2 }),
                None,
            ).await
        });
        
        let handle2 = tokio::spawn(async move {
            executor2.execute_task_send(
                task_id2,
                json!({ "num1": 3, "num2": 4 }),
                None,
            ).await
        });
        
        // Wait for both tasks to complete
        let (result1, result2) = tokio::join!(handle1, handle2);
        
        // Both should complete (successfully or with worker errors)
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let exec_result1 = result1.unwrap();
        let exec_result2 = result2.unwrap();
        
        assert!(exec_result1.is_ok());
        assert!(exec_result2.is_ok());
        
        // Should have different execution IDs
        assert_ne!(exec_result1.unwrap().execution_id, exec_result2.unwrap().execution_id);
    }
    
    #[tokio::test]
    async fn test_task_executor_trait_implementation() {
        let db = create_test_database().await;
        let repositories = RepositoryFactory::new(db);
        let config = RatchetConfig::default();
        
        let executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
        
        // Create a test task
        let (task_id, _) = create_test_task_in_db(&repositories).await;
        
        // Test TaskExecutor trait methods
        let input_data = json!({ "num1": 15, "num2": 25 });
        
        // Test execute_task (non-Send trait method)
        let result = executor.execute_task(task_id, input_data.clone(), None).await;
        assert!(result.is_ok());
        
        // Test health_check (non-Send trait method)
        let health = executor.health_check().await;
        assert!(health.is_ok());
    }
}