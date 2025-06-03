//! Adapter to bridge Ratchet's execution engine with MCP server

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use ratchet_lib::execution::ProcessTaskExecutor;
use ratchet_lib::database::repositories::{ExecutionRepository, TaskRepository};
use ratchet_lib::database::repositories::task_repository::{TaskFilters, Pagination};

use super::tools::{McpTaskExecutor, McpTaskInfo};

/// Adapter that wraps Ratchet's process executor to provide MCP-compatible task execution
pub struct RatchetMcpAdapter {
    /// The process-based task executor (Send + Sync)
    executor: Arc<ProcessTaskExecutor>,
    
    /// Task repository for task discovery
    task_repository: Arc<TaskRepository>,
    
    /// Execution repository for monitoring
    execution_repository: Arc<ExecutionRepository>,
}

impl RatchetMcpAdapter {
    /// Create a new adapter
    pub fn new(
        executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor,
            task_repository,
            execution_repository,
        }
    }
}

#[async_trait]
impl McpTaskExecutor for RatchetMcpAdapter {
    async fn execute_task(&self, task_path: &str, input: Value) -> Result<Value, String> {
        // First, try to find the task by name in the database
        let task = match self.task_repository.find_by_name(task_path).await {
            Ok(Some(task)) => task,
            Ok(None) => {
                // Try to parse as UUID
                if let Ok(uuid) = uuid::Uuid::parse_str(task_path) {
                    match self.task_repository.find_by_uuid(uuid).await {
                        Ok(Some(task)) => task,
                        Ok(None) => return Err(format!("Task not found: {}", task_path)),
                        Err(e) => return Err(format!("Database error: {}", e)),
                    }
                } else {
                    return Err(format!("Task not found: {}", task_path));
                }
            }
            Err(e) => return Err(format!("Database error: {}", e)),
        };
        
        // Create an execution context
        use ratchet_lib::execution::ipc::ExecutionContext;
        let context = ExecutionContext {
            execution_id: uuid::Uuid::new_v4().to_string(),
            job_id: None,
            task_id: task.uuid.to_string(),
            task_version: task.version.clone(),
        };
        
        // Execute the task using the process executor
        match self.executor.execute_task_send(
            task.id, // Database task ID
            input,
            Some(context),
        ).await {
            Ok(result) => {
                result.output.ok_or_else(|| "No output from task execution".to_string())
            }
            Err(e) => {
                Err(format!("Task execution failed: {}", e))
            }
        }
    }
    
    async fn list_tasks(&self, filter: Option<&str>) -> Result<Vec<McpTaskInfo>, String> {
        let filters = TaskFilters {
            name: filter.map(|s| s.to_string()),
            enabled: Some(true),
            has_validation: None,
            version: None,
        };
        
        let pagination = Pagination {
            limit: Some(100),
            offset: None,
            order_by: Some("name".to_string()),
            order_desc: Some(false),
        };
        
        let tasks = self.task_repository
            .find_with_filters(filters, pagination)
            .await
            .map_err(|e| format!("Failed to list tasks: {}", e))?;
            
        // Convert database tasks to MCP task info
        Ok(tasks.into_iter().map(|task| McpTaskInfo {
            id: task.uuid.to_string(),
            name: task.name.clone(),
            version: task.version.clone(),
            description: task.description.clone(),
            tags: vec![], // Database entity doesn't have tags directly
            enabled: task.enabled,
            input_schema: Some(task.input_schema),
            output_schema: Some(task.output_schema),
        }).collect())
    }
    
    async fn get_execution_logs(&self, execution_id: &str, level: &str, limit: usize) -> Result<String, String> {
        // For now, return a placeholder
        // In a full implementation, this would query the logging system
        Ok(format!(
            "Logs for execution {} (level: {}, limit: {})",
            execution_id, level, limit
        ))
    }
}

/// Builder for creating the MCP adapter with all required components
pub struct RatchetMcpAdapterBuilder {
    executor: Option<Arc<ProcessTaskExecutor>>,
    task_repository: Option<Arc<TaskRepository>>,
    execution_repository: Option<Arc<ExecutionRepository>>,
}

impl RatchetMcpAdapterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            executor: None,
            task_repository: None,
            execution_repository: None,
        }
    }
    
    /// Set the process task executor
    pub fn with_executor(mut self, executor: Arc<ProcessTaskExecutor>) -> Self {
        self.executor = Some(executor);
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
    
    /// Build the adapter
    pub fn build(self) -> Result<RatchetMcpAdapter, String> {
        let executor = self.executor.ok_or("Executor is required")?;
        let task_repo = self.task_repository.ok_or("Task repository is required")?;
        let exec_repo = self.execution_repository.ok_or("Execution repository is required")?;
        
        Ok(RatchetMcpAdapter::new(executor, task_repo, exec_repo))
    }
}

impl Default for RatchetMcpAdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}