//! Adapter to bridge Ratchet's execution engine with MCP server

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};

use ratchet_lib::execution::ProcessTaskExecutor;
use ratchet_runtime::executor::TaskExecutor;
use ratchet_lib::logging::event::{LogEvent, LogLevel};
use ratchet_storage::seaorm::repositories::{
    task_repository::{TaskRepository, TaskFilters, Pagination},
    execution_repository::ExecutionRepository,
};
use ratchet_storage::seaorm::entities::{ExecutionStatus, Task, Execution};

use super::tools::{McpTaskExecutor, McpTaskInfo, McpExecutionStatus};

/// Executor type that can handle both legacy and new execution engines
pub enum ExecutorType {
    /// Legacy process executor from ratchet_lib
    Legacy(Arc<ProcessTaskExecutor>),
    /// New modular task executor from ratchet-runtime
    Runtime(Arc<dyn TaskExecutor>),
}

/// Adapter that wraps Ratchet's task execution to provide MCP-compatible task execution
pub struct RatchetMcpAdapter {
    /// The task executor (either legacy or new runtime)
    executor: ExecutorType,
    
    /// Task repository for task discovery
    task_repository: Arc<TaskRepository>,
    
    /// Execution repository for monitoring
    execution_repository: Arc<ExecutionRepository>,
    
    /// Optional path to log file for log retrieval
    log_file_path: Option<PathBuf>,
}

impl RatchetMcpAdapter {
    /// Create a new adapter with legacy ProcessTaskExecutor
    pub fn new(
        executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor: ExecutorType::Legacy(executor),
            task_repository,
            execution_repository,
            log_file_path: None,
        }
    }
    
    /// Create a new adapter with new runtime TaskExecutor
    pub fn with_runtime_executor(
        executor: Arc<dyn TaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor: ExecutorType::Runtime(executor),
            task_repository,
            execution_repository,
            log_file_path: None,
        }
    }
    
    /// Create a new adapter with log file path for log retrieval (legacy)
    pub fn with_log_file(
        executor: Arc<ProcessTaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: PathBuf,
    ) -> Self {
        Self {
            executor: ExecutorType::Legacy(executor),
            task_repository,
            execution_repository,
            log_file_path: Some(log_file_path),
        }
    }
    
    /// Create a new adapter with log file path for log retrieval (runtime)
    pub fn with_runtime_executor_and_log_file(
        executor: Arc<dyn TaskExecutor>,
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: PathBuf,
    ) -> Self {
        Self {
            executor: ExecutorType::Runtime(executor),
            task_repository,
            execution_repository,
            log_file_path: Some(log_file_path),
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
    
    async fn execute_task_with_progress(
        &self, 
        task_path: &str, 
        input: Value,
        progress_manager: Option<Arc<super::progress::ProgressNotificationManager>>,
        _connection: Option<Arc<dyn crate::transport::connection::TransportConnection>>,
        _filter: Option<super::progress::ProgressFilter>,
    ) -> Result<(String, Value), String> {
        // For now, just execute the task normally and return with a fake execution ID
        // In the future, this would integrate with the worker process IPC to receive progress updates
        let result = self.execute_task(task_path, input).await?;
        
        let execution_id = uuid::Uuid::new_v4().to_string();
        
        // If progress manager is provided, send a completion update
        if let Some(manager) = progress_manager {
            let progress_update = super::progress::ProgressUpdate {
                execution_id: execution_id.clone(),
                task_id: task_path.to_string(),
                progress: 1.0,
                step: Some("completed".to_string()),
                step_number: Some(1),
                total_steps: Some(1),
                message: Some("Task completed successfully".to_string()),
                data: Some(result.clone()),
                timestamp: chrono::Utc::now(),
            };
            
            let _ = manager.send_progress_update(progress_update).await;
        }
        
        Ok((execution_id, result))
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
            input_schema: Some(task.input_schema.clone()),
            output_schema: Some(task.output_schema.clone()),
        }).collect())
    }
    
    async fn get_execution_logs(&self, execution_id: &str, level: &str, limit: usize) -> Result<String, String> {
        // Parse the log level
        let min_level = match level.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        };
        
        // Try to parse execution_id as UUID to query the execution repository
        if let Ok(exec_uuid) = uuid::Uuid::parse_str(execution_id) {
            match self.execution_repository.find_by_uuid(exec_uuid).await {
                Ok(Some(execution)) => {
                    // First check if we have a recording path (most detailed logs)
                    if let Some(recording_path) = &execution.recording_path {
                        if let Ok(logs) = self.get_logs_from_recording(recording_path, &min_level, limit).await {
                            return Ok(logs);
                        }
                    }
                    
                    // Fallback to searching log files if available
                    if let Some(log_path) = &self.log_file_path {
                        if let Ok(logs) = self.search_log_file_for_execution(log_path, execution_id, &min_level, limit).await {
                            return Ok(logs);
                        }
                    }
                    
                    // Last fallback: return basic execution info with available data
                    let log_info = serde_json::json!({
                        "execution_id": execution_id,
                        "task_id": execution.task_id,
                        "status": execution.status,
                        "started_at": execution.started_at,
                        "completed_at": execution.completed_at,
                        "error_message": execution.error_message,
                        "logs": [],
                        "message": "Detailed logs not available - no log file or recording path configured"
                    });
                    Ok(serde_json::to_string_pretty(&log_info)
                        .unwrap_or_else(|_| log_info.to_string()))
                }
                Ok(None) => Err(format!("Execution not found: {}", execution_id)),
                Err(e) => Err(format!("Database error: {}", e)),
            }
        } else {
            Err("Invalid execution ID format - must be a valid UUID".to_string())
        }
    }
    
    async fn get_execution_status(&self, execution_id: &str) -> Result<McpExecutionStatus, String> {
        // Try to parse execution_id as UUID to query the execution repository
        if let Ok(exec_uuid) = uuid::Uuid::parse_str(execution_id) {
            match self.execution_repository.find_by_uuid(exec_uuid).await {
                Ok(Some(execution)) => {
                    // Convert execution status to string
                    let status_str = match execution.status {
                        ExecutionStatus::Pending => "pending",
                        ExecutionStatus::Running => "running",
                        ExecutionStatus::Completed => "completed",
                        ExecutionStatus::Failed => "failed",
                        ExecutionStatus::Cancelled => "cancelled",
                    }.to_string();
                    
                    // Calculate progress information
                    let progress = match execution.status {
                        ExecutionStatus::Pending => Some(serde_json::json!({
                            "current_step": "pending",
                            "percentage": 0
                        })),
                        ExecutionStatus::Running => {
                            // For running executions, we can estimate progress based on time
                            if let Some(started_at) = execution.started_at {
                                let elapsed = chrono::Utc::now().signed_duration_since(started_at);
                                Some(serde_json::json!({
                                    "current_step": "running",
                                    "elapsed_ms": elapsed.num_milliseconds(),
                                    "percentage": null  // Cannot estimate without task-specific progress
                                }))
                            } else {
                                Some(serde_json::json!({
                                    "current_step": "running",
                                    "percentage": null
                                }))
                            }
                        },
                        ExecutionStatus::Completed => Some(serde_json::json!({
                            "current_step": "completed",
                            "percentage": 100
                        })),
                        ExecutionStatus::Failed | ExecutionStatus::Cancelled => Some(serde_json::json!({
                            "current_step": status_str,
                            "percentage": null
                        })),
                    };
                    
                    Ok(McpExecutionStatus {
                        execution_id: execution_id.to_string(),
                        status: status_str,
                        task_id: execution.task_id,
                        input: Some(execution.input),
                        output: execution.output,
                        error_message: execution.error_message,
                        error_details: execution.error_details,
                        queued_at: execution.queued_at.to_rfc3339(),
                        started_at: execution.started_at.map(|dt| dt.to_rfc3339()),
                        completed_at: execution.completed_at.map(|dt| dt.to_rfc3339()),
                        duration_ms: execution.duration_ms,
                        progress,
                    })
                }
                Ok(None) => Err(format!("Execution not found: {}", execution_id)),
                Err(e) => Err(format!("Database error: {}", e)),
            }
        } else {
            Err("Invalid execution ID format - must be a valid UUID".to_string())
        }
    }
}

// Additional helper methods for RatchetMcpAdapter
impl RatchetMcpAdapter {
    /// Get logs from recording path (HAR format)
    async fn get_logs_from_recording(
        &self, 
        recording_path: &str, 
        _min_level: &LogLevel, 
        limit: usize
    ) -> Result<String, String> {
        // For now, return basic recording info
        // In a full implementation, this would parse the HAR file from the recording
        let recording_info = serde_json::json!({
            "recording_path": recording_path,
            "logs": [],
            "limit": limit,
            "message": "Recording-based log retrieval not yet implemented - HAR parsing needed"
        });
        
        Ok(serde_json::to_string_pretty(&recording_info)
            .unwrap_or_else(|_| recording_info.to_string()))
    }
    
    /// Search log file for execution-specific logs
    async fn search_log_file_for_execution(
        &self,
        log_path: &PathBuf,
        execution_id: &str,
        min_level: &LogLevel,
        limit: usize,
    ) -> Result<String, String> {
        let file = File::open(log_path)
            .map_err(|e| format!("Cannot open log file {}: {}", log_path.display(), e))?;
        
        let reader = BufReader::new(file);
        let mut matching_logs = Vec::new();
        let mut total_lines = 0;
        let mut parse_errors = 0;
        
        for (line_num, line) in reader.lines().enumerate() {
            total_lines += 1;
            
            let line = line
                .map_err(|e| format!("Error reading log file at line {}: {}", line_num, e))?;
            
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }
            
            // Try to parse as JSON log event
            match serde_json::from_str::<LogEvent>(&line) {
                Ok(log_event) => {
                    // Check if this log is related to our execution
                    let is_related = log_event.trace_id.as_deref() == Some(execution_id)
                        || log_event.span_id.as_deref() == Some(execution_id)
                        || log_event.fields.get("execution_id")
                            .and_then(|v| v.as_str()) == Some(execution_id)
                        || log_event.fields.get("exec_id")
                            .and_then(|v| v.as_str()) == Some(execution_id)
                        || log_event.message.contains(execution_id);
                    
                    if is_related && log_event.level >= *min_level {
                        let log_entry = serde_json::json!({
                            "timestamp": log_event.timestamp.to_rfc3339(),
                            "level": log_event.level.as_str(),
                            "message": log_event.message,
                            "logger": log_event.logger,
                            "fields": log_event.fields,
                            "error": log_event.error,
                            "trace_id": log_event.trace_id,
                            "span_id": log_event.span_id,
                            "line_number": line_num + 1
                        });
                        
                        matching_logs.push(log_entry);
                        
                        if matching_logs.len() >= limit {
                            break;
                        }
                    }
                }
                Err(_) => {
                    parse_errors += 1;
                    // Also check plain text lines for the execution ID
                    if line.contains(execution_id) {
                        let log_entry = serde_json::json!({
                            "timestamp": null,
                            "level": "unknown",
                            "message": line,
                            "logger": "raw",
                            "fields": {},
                            "error": null,
                            "trace_id": null,
                            "span_id": null,
                            "line_number": line_num + 1,
                            "format": "plain_text"
                        });
                        
                        matching_logs.push(log_entry);
                        
                        if matching_logs.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        let result = serde_json::json!({
            "execution_id": execution_id,
            "log_file": log_path.display().to_string(),
            "logs": matching_logs,
            "total_found": matching_logs.len(),
            "limit_applied": limit,
            "min_level": min_level.as_str(),
            "search_stats": {
                "total_lines_processed": total_lines,
                "parse_errors": parse_errors,
                "has_more": matching_logs.len() >= limit
            },
            "search_criteria": {
                "execution_id": execution_id,
                "search_fields": ["trace_id", "span_id", "execution_id", "exec_id", "message_content"]
            }
        });
        
        Ok(serde_json::to_string_pretty(&result)
            .unwrap_or_else(|_| result.to_string()))
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