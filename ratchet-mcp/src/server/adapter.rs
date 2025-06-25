//! Adapter to bridge Ratchet's execution engine with MCP server

use async_trait::async_trait;
use serde_json::{Value, Value as JsonValue};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

use ratchet_execution::{ExecutionBridge, ExecutionError, ProcessTaskExecutor, TaskExecutionResult};
use ratchet_interfaces::execution::TaskExecutor as InterfaceTaskExecutor;
use ratchet_interfaces::logging::{LogEvent, LogLevel};
use ratchet_interfaces::{TaskService, TaskServiceFilters};
use ratchet_runtime::executor::TaskExecutor;
use ratchet_storage::seaorm::entities::ExecutionStatus;
use ratchet_storage::seaorm::repositories::execution_repository::ExecutionRepository;

use super::tools::{McpExecutionStatus, McpTaskExecutor, McpTaskInfo};

/// Executor type that can handle both execution engines
pub enum ExecutorType {
    /// Process executor from ratchet-execution (legacy)
    Process(Arc<ProcessTaskExecutor>),
    /// Bridge executor from ratchet-execution (new modular approach)
    Bridge(Arc<ExecutionBridge>),
    /// New modular task executor from ratchet-runtime
    Runtime(Arc<dyn TaskExecutor>),
}

impl ExecutorType {
    /// Execute task directly using the new API
    pub async fn execute_task_direct(
        &self,
        task_id: i32,
        task_path: String,
        input_data: JsonValue,
        context: Option<ratchet_execution::ipc::ExecutionContext>,
    ) -> Result<TaskExecutionResult, ExecutionError> {
        match self {
            ExecutorType::Process(executor) => {
                executor
                    .execute_task_direct(task_id, task_path, input_data, context)
                    .await
            }
            ExecutorType::Bridge(executor) => {
                // Use the bridge's execute_task method with interface-style parameters
                use ratchet_interfaces::execution::ExecutionContext;
                use std::time::Duration;

                // Convert task_id to string and create execution context if provided
                let task_id_str = task_id.to_string();
                let exec_context = context.map(|_| ExecutionContext::new().with_timeout(Duration::from_secs(300)));

                // Execute using the bridge interface
                match executor.execute_task(&task_id_str, input_data, exec_context).await {
                    Ok(result) => {
                        // Convert interface ExecutionResult back to TaskExecutionResult
                        use chrono::Utc;
                        let now = Utc::now();
                        let started_at = now - chrono::Duration::milliseconds(result.execution_time_ms as i64);

                        Ok(TaskExecutionResult {
                            success: result.status.is_success(),
                            output: Some(result.output),
                            error_message: match result.status {
                                ratchet_interfaces::execution::ExecutionStatus::Failed { error_message } => {
                                    Some(error_message)
                                }
                                _ => None,
                            },
                            error_details: result.trace,
                            started_at,
                            completed_at: now,
                            duration_ms: result.execution_time_ms as i32,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
            ExecutorType::Runtime(_executor) => {
                // For runtime executor, we need to implement this
                // This is a simplified implementation - in production you'd want proper conversion
                Err(ExecutionError::TaskExecutionError(
                    "Runtime executor not yet fully implemented".to_string(),
                ))
            }
        }
    }
}

/// Adapter that wraps Ratchet's task execution to provide MCP-compatible task execution
pub struct RatchetMcpAdapter {
    /// The task executor (either legacy or new runtime)
    executor: ExecutorType,

    /// Unified task service for task discovery (replaces direct repository access)
    task_service: Arc<dyn TaskService>,

    /// Execution repository for monitoring
    execution_repository: Arc<ExecutionRepository>,

    /// Optional path to log file for log retrieval
    log_file_path: Option<PathBuf>,
}

impl RatchetMcpAdapter {
    /// Create a new adapter with ProcessTaskExecutor from ratchet-execution (legacy)
    pub fn new(
        executor: Arc<ProcessTaskExecutor>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor: ExecutorType::Process(executor),
            task_service,
            execution_repository,
            log_file_path: None,
        }
    }

    /// Create a new adapter with ExecutionBridge (recommended for new implementations)
    pub fn with_bridge_executor(
        executor: Arc<ExecutionBridge>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor: ExecutorType::Bridge(executor),
            task_service,
            execution_repository,
            log_file_path: None,
        }
    }

    /// Create a new adapter with new runtime TaskExecutor
    pub fn with_runtime_executor(
        executor: Arc<dyn TaskExecutor>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
    ) -> Self {
        Self {
            executor: ExecutorType::Runtime(executor),
            task_service,
            execution_repository,
            log_file_path: None,
        }
    }

    /// Create a new adapter with log file path for log retrieval (legacy)
    pub fn with_log_file(
        executor: Arc<ProcessTaskExecutor>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: PathBuf,
    ) -> Self {
        Self {
            executor: ExecutorType::Process(executor),
            task_service,
            execution_repository,
            log_file_path: Some(log_file_path),
        }
    }

    /// Create a new adapter with ExecutionBridge and log file path for log retrieval
    pub fn with_bridge_executor_and_log_file(
        executor: Arc<ExecutionBridge>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: PathBuf,
    ) -> Self {
        Self {
            executor: ExecutorType::Bridge(executor),
            task_service,
            execution_repository,
            log_file_path: Some(log_file_path),
        }
    }

    /// Create a new adapter with log file path for log retrieval (runtime)
    pub fn with_runtime_executor_and_log_file(
        executor: Arc<dyn TaskExecutor>,
        task_service: Arc<dyn TaskService>,
        execution_repository: Arc<ExecutionRepository>,
        log_file_path: PathBuf,
    ) -> Self {
        Self {
            executor: ExecutorType::Runtime(executor),
            task_service,
            execution_repository,
            log_file_path: Some(log_file_path),
        }
    }
}

#[async_trait]
impl McpTaskExecutor for RatchetMcpAdapter {
    async fn execute_task(&self, task_path: &str, input: Value) -> Result<Value, String> {
        // Use unified task service to find the task (abstracts storage location)
        let task = match self.task_service.find_by_name(task_path).await {
            Ok(Some(task)) => task,
            Ok(None) => {
                // Try to parse as UUID
                if let Ok(uuid) = uuid::Uuid::parse_str(task_path) {
                    match self.task_service.find_by_id(uuid).await {
                        Ok(Some(task)) => task,
                        Ok(None) => return Err(format!("Task not found: {}", task_path)),
                        Err(e) => return Err(format!("Task service error: {}", e)),
                    }
                } else {
                    return Err(format!("Task not found: {}", task_path));
                }
            }
            Err(e) => return Err(format!("Task service error: {}", e)),
        };

        // Create an execution context
        use ratchet_execution::ipc::ExecutionContext;
        let context = ExecutionContext::new(uuid::Uuid::new_v4(), None, task.uuid, task.version.clone());

        // Convert string ID to i32 for legacy execution interface
        // For registry tasks, we'll use a synthetic ID since they're not stored in DB
        let task_id = if task.registry_source {
            // Use 0 as a placeholder for registry tasks (legacy executor won't use this for registry tasks)
            0
        } else {
            // Parse database task ID
            task.id.to_string().parse::<i32>().map_err(|e| format!("Invalid task ID format: {}", e))?
        };

        // Execute the task using the process executor
        match self
            .executor
            .execute_task_direct(
                task_id,                         // Database task ID or 0 for registry
                format!("/tasks/{}", task.uuid), // Use UUID as task path
                input,
                Some(context),
            )
            .await
        {
            Ok(task_result) => task_result
                .output
                .ok_or_else(|| "No output from task execution".to_string()),
            Err(e) => Err(format!("Task execution failed: {}", e)),
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
        // Use unified task service for listing (abstracts storage location)
        use ratchet_api_types::PaginationInput;
        let pagination = Some(PaginationInput {
            page: Some(0),
            limit: Some(100),
            offset: None,
        });

        let filters = Some(TaskServiceFilters {
            enabled_only: Some(true),
            source_type: None, // Get tasks from all sources
            name_contains: filter.map(|s| s.to_string()),
        });

        let response = self
            .task_service
            .list_tasks(pagination, filters)
            .await
            .map_err(|e| format!("Failed to list tasks: {}", e))?;

        // Convert unified tasks to MCP task info
        Ok(response.items
            .into_iter()
            .map(|task| McpTaskInfo {
                id: task.uuid.to_string(),
                name: task.name.clone(),
                version: task.version.clone(),
                description: task.description.clone(),
                tags: vec![], // UnifiedTask doesn't have tags field
                enabled: task.enabled,
                input_schema: task.input_schema.clone(),
                output_schema: task.output_schema.clone(),
            })
            .collect())
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
                        if let Ok(logs) = self
                            .search_log_file_for_execution(log_path, execution_id, &min_level, limit)
                            .await
                        {
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
                    Ok(serde_json::to_string_pretty(&log_info).unwrap_or_else(|_| log_info.to_string()))
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
                    }
                    .to_string();

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
                        }
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
        limit: usize,
    ) -> Result<String, String> {
        // For now, return basic recording info
        // In a full implementation, this would parse the HAR file from the recording
        let recording_info = serde_json::json!({
            "recording_path": recording_path,
            "logs": [],
            "limit": limit,
            "message": "Recording-based log retrieval not yet implemented - HAR parsing needed"
        });

        Ok(serde_json::to_string_pretty(&recording_info).unwrap_or_else(|_| recording_info.to_string()))
    }

    /// Search log file for execution-specific logs
    async fn search_log_file_for_execution(
        &self,
        log_path: &PathBuf,
        execution_id: &str,
        min_level: &LogLevel,
        limit: usize,
    ) -> Result<String, String> {
        let file = File::open(log_path).map_err(|e| format!("Cannot open log file {}: {}", log_path.display(), e))?;

        let reader = BufReader::new(file);
        let mut matching_logs = Vec::new();
        let mut total_lines = 0;
        let mut parse_errors = 0;

        for (line_num, line) in reader.lines().enumerate() {
            total_lines += 1;

            let line = line.map_err(|e| format!("Error reading log file at line {}: {}", line_num, e))?;

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
                        || log_event.fields.get("execution_id").and_then(|v| v.as_str()) == Some(execution_id)
                        || log_event.fields.get("exec_id").and_then(|v| v.as_str()) == Some(execution_id)
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

        Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
    }
}

/// Builder for creating the MCP adapter with all required components
pub struct RatchetMcpAdapterBuilder {
    executor: Option<ExecutorType>,
    task_service: Option<Arc<dyn TaskService>>,
    execution_repository: Option<Arc<ExecutionRepository>>,
}

impl RatchetMcpAdapterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            executor: None,
            task_service: None,
            execution_repository: None,
        }
    }

    /// Set the process task executor (legacy)
    pub fn with_executor(mut self, executor: Arc<ProcessTaskExecutor>) -> Self {
        self.executor = Some(ExecutorType::Process(executor));
        self
    }

    /// Set the execution bridge executor (recommended)
    pub fn with_bridge_executor(mut self, executor: Arc<ExecutionBridge>) -> Self {
        self.executor = Some(ExecutorType::Bridge(executor));
        self
    }

    /// Set the task service
    pub fn with_task_service(mut self, service: Arc<dyn TaskService>) -> Self {
        self.task_service = Some(service);
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
        
        // If no task service is provided, create a minimal stub for CLI compatibility
        let task_service = if let Some(service) = self.task_service {
            service
        } else {
            Arc::new(StubTaskService)
        };
        
        let exec_repo = self.execution_repository.ok_or("Execution repository is required")?;

        Ok(RatchetMcpAdapter {
            executor,
            task_service,
            execution_repository: exec_repo,
            log_file_path: None,
        })
    }
}

impl Default for RatchetMcpAdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub task service for CLI compatibility when no real task service is available
struct StubTaskService;

#[async_trait]
impl ratchet_interfaces::TaskService for StubTaskService {
    async fn find_by_id(&self, _id: uuid::Uuid) -> Result<Option<ratchet_api_types::UnifiedTask>, ratchet_interfaces::TaskServiceError> {
        Err(ratchet_interfaces::TaskServiceError::Internal { 
            message: "Task service not configured - use server mode for full functionality".to_string() 
        })
    }
    
    async fn find_by_name(&self, _name: &str) -> Result<Option<ratchet_api_types::UnifiedTask>, ratchet_interfaces::TaskServiceError> {
        Err(ratchet_interfaces::TaskServiceError::Internal { 
            message: "Task service not configured - use server mode for full functionality".to_string() 
        })
    }
    
    async fn list_tasks(
        &self, 
        _pagination: Option<ratchet_api_types::PaginationInput>, 
        _filters: Option<ratchet_interfaces::TaskServiceFilters>
    ) -> Result<ratchet_api_types::ListResponse<ratchet_api_types::UnifiedTask>, ratchet_interfaces::TaskServiceError> {
        Err(ratchet_interfaces::TaskServiceError::Internal { 
            message: "Task service not configured - use server mode for full functionality".to_string() 
        })
    }
    
    async fn get_task_metadata(&self, _id: uuid::Uuid) -> Result<Option<ratchet_interfaces::TaskServiceMetadata>, ratchet_interfaces::TaskServiceError> {
        Err(ratchet_interfaces::TaskServiceError::Internal { 
            message: "Task service not configured - use server mode for full functionality".to_string() 
        })
    }
    
    async fn execute_task(&self, _id: uuid::Uuid, _input: serde_json::Value) -> Result<serde_json::Value, ratchet_interfaces::TaskServiceError> {
        Err(ratchet_interfaces::TaskServiceError::Internal { 
            message: "Task service not configured - use server mode for full functionality".to_string() 
        })
    }
    
    async fn task_exists(&self, _id: uuid::Uuid) -> Result<bool, ratchet_interfaces::TaskServiceError> {
        Ok(false)
    }
    
    async fn get_task_source(&self, _id: uuid::Uuid) -> Result<Option<ratchet_interfaces::TaskSource>, ratchet_interfaces::TaskServiceError> {
        Ok(None)
    }
}
