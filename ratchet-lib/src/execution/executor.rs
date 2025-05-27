use crate::database::{
    entities::{Execution, ExecutionStatus, Job, JobStatus},
    repositories::RepositoryFactory,
    DatabaseError,
};
use crate::services::{ServiceError, ServiceResult, RatchetEngine};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Task execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    
    #[error("Service error: {0}")]
    ServiceError(#[from] ServiceError),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Job not found: {0}")]
    JobNotFound(i32),
    
    #[error("Invalid execution state: {0}")]
    InvalidState(String),
}

/// Execution context containing metadata about the execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: i32,
    pub job_id: Option<i32>,
    pub task_id: i32,
    pub input_data: JsonValue,
    pub worker_id: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub execution_id: i32,
    pub success: bool,
    pub output: Option<JsonValue>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub http_requests: Option<JsonValue>,
}

/// Task executor trait for different execution strategies
#[async_trait(?Send)]
pub trait TaskExecutor {
    /// Execute a task with given input
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError>;
    
    /// Execute a job from the job queue
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError>;
    
    /// Check if executor is healthy
    async fn health_check(&self) -> Result<(), ExecutionError>;
}

/// Database-backed task executor
pub struct DatabaseTaskExecutor {
    engine: RatchetEngine,
    repositories: RepositoryFactory,
}

impl DatabaseTaskExecutor {
    /// Create a new database task executor
    pub fn new(engine: RatchetEngine, repositories: RepositoryFactory) -> Self {
        Self { engine, repositories }
    }
    
    /// Create execution record in database
    async fn create_execution_record(
        &self,
        task_id: i32,
        input_data: &JsonValue,
    ) -> Result<Execution, ExecutionError> {
        let execution = Execution::new(task_id, input_data.clone());
        let created = self.repositories.execution_repo.create(execution).await?;
        debug!("Created execution record: {}", created.id);
        Ok(created)
    }
    
    /// Update execution with result
    async fn update_execution_result(
        &self,
        execution_id: i32,
        result: &ExecutionResult,
    ) -> Result<(), ExecutionError> {
        if result.success {
            if let Some(output) = &result.output {
                self.repositories
                    .execution_repo
                    .mark_completed(execution_id, output.clone())
                    .await?;
            } else {
                // This shouldn't happen, but handle gracefully
                self.repositories
                    .execution_repo
                    .mark_failed(execution_id, "No output produced".to_string(), None)
                    .await?;
            }
        } else {
            let error_msg = result.error.as_ref().unwrap_or(&"Unknown error".to_string()).clone();
            self.repositories
                .execution_repo
                .mark_failed(execution_id, error_msg, None)
                .await?;
        }
        Ok(())
    }
    
    /// Get task path from database
    async fn get_task_path(&self, task_id: i32) -> Result<String, ExecutionError> {
        let task = self.repositories.task_repo.find_by_id(task_id).await?;
        match task {
            Some(task) => Ok(task.path),
            None => Err(ExecutionError::TaskNotFound(format!("Task ID: {}", task_id))),
        }
    }
}

#[async_trait(?Send)]
impl TaskExecutor for DatabaseTaskExecutor {
    async fn execute_task(
        &self,
        task_id: i32,
        input_data: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, ExecutionError> {
        let start_time = Instant::now();
        
        // Create execution record or use existing one from context
        let execution = if let Some(ctx) = &context {
            // Mark existing execution as started
            self.repositories
                .execution_repo
                .mark_started(ctx.execution_id)
                .await?;
            
            // We need to fetch the execution to get the full record
            self.repositories
                .execution_repo
                .find_by_id(ctx.execution_id)
                .await?
                .ok_or_else(|| ExecutionError::InvalidState("Execution not found".to_string()))?
        } else {
            // Create new execution
            self.create_execution_record(task_id, &input_data).await?
        };
        
        info!("Starting task execution: {} for task ID: {}", execution.id, task_id);
        
        // Get task path for execution
        let task_path = self.get_task_path(task_id).await?;
        
        // Execute the task using the engine
        let execution_result = match self.engine.execute_task_from_path(&task_path, input_data.clone()).await {
            Ok(output) => {
                let duration = start_time.elapsed();
                info!("Task execution completed successfully: {} in {}ms", execution.id, duration.as_millis());
                
                ExecutionResult {
                    execution_id: execution.id,
                    success: true,
                    output: Some(output),
                    error: None,
                    duration_ms: duration.as_millis() as i64,
                    http_requests: None, // TODO: Extract from recording if available
                }
            }
            Err(err) => {
                let duration = start_time.elapsed();
                error!("Task execution failed: {} - {}", execution.id, err);
                
                ExecutionResult {
                    execution_id: execution.id,
                    success: false,
                    output: None,
                    error: Some(err.to_string()),
                    duration_ms: duration.as_millis() as i64,
                    http_requests: None,
                }
            }
        };
        
        // Update execution record with result
        self.update_execution_result(execution.id, &execution_result).await?;
        
        Ok(execution_result)
    }
    
    async fn execute_job(&self, job_id: i32) -> Result<ExecutionResult, ExecutionError> {
        info!("Starting job execution: {}", job_id);
        
        // Get job from database
        let job = self.repositories.job_repo.find_by_id(job_id).await?;
        let job = job.ok_or_else(|| ExecutionError::JobNotFound(job_id))?;
        
        // Verify job is ready for processing
        if !job.is_ready_for_processing() {
            return Err(ExecutionError::InvalidState(
                format!("Job {} is not ready for processing (status: {:?})", job_id, job.status)
            ));
        }
        
        // Create execution record
        let execution = self.create_execution_record(job.task_id, &job.input_data.0).await?;
        
        // Mark job as processing with execution ID
        self.repositories
            .job_repo
            .mark_processing(job_id, execution.id)
            .await?;
        
        // Create execution context
        let context = ExecutionContext {
            execution_id: execution.id,
            job_id: Some(job_id),
            task_id: job.task_id,
            input_data: job.input_data.0.clone(),
            worker_id: None, // Will be set by worker pool
            started_at: chrono::Utc::now(),
        };
        
        // Execute the task
        let result = self.execute_task(job.task_id, job.input_data.0.clone(), Some(context)).await?;
        
        // Update job status based on result
        if result.success {
            self.repositories.job_repo.mark_completed(job_id).await?;
            info!("Job execution completed successfully: {}", job_id);
        } else {
            let error_msg = result.error.as_ref().unwrap_or(&"Unknown error".to_string()).clone();
            let will_retry = self.repositories
                .job_repo
                .mark_failed(job_id, error_msg, None)
                .await?;
            
            if will_retry {
                warn!("Job execution failed, will retry: {}", job_id);
            } else {
                error!("Job execution failed, no more retries: {}", job_id);
            }
        }
        
        Ok(result)
    }
    
    async fn health_check(&self) -> Result<(), ExecutionError> {
        // Check database connectivity through repositories
        self.repositories.task_repo.health_check().await?;
        self.repositories.execution_repo.health_check().await?;
        self.repositories.job_repo.health_check().await?;
        
        debug!("Task executor health check passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RatchetConfig, DatabaseConfig};
    use crate::database::DatabaseConnection;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    async fn create_test_setup() -> (DatabaseTaskExecutor, RepositoryFactory) {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();
        
        let mut config = RatchetConfig::default();
        config.server = Some(crate::config::ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            database: DatabaseConfig {
                url: format!("sqlite://{}?mode=rwc", db_path),
                max_connections: 5,
                connection_timeout: Duration::from_secs(10),
            },
            auth: None,
        });

        let db = DatabaseConnection::new(config.server.as_ref().unwrap().database.clone()).await.unwrap();
        db.migrate().await.unwrap();
        
        let repositories = RepositoryFactory::new(db);
        let engine = RatchetEngine::new(config).unwrap();
        let executor = DatabaseTaskExecutor::new(engine, repositories.clone());
        
        (executor, repositories)
    }

    #[tokio::test]
    async fn test_executor_health_check() {
        let (executor, _) = create_test_setup().await;
        assert!(executor.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_create_execution_record() {
        let (executor, _) = create_test_setup().await;
        
        // This would require a task in the database first
        // For now, just test the error case
        let result = executor.create_execution_record(999, &json!({"test": "data"})).await;
        assert!(result.is_ok()); // Creating execution record should work even if task doesn't exist yet
    }
}