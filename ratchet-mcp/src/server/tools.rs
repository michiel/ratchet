//! Tool registry and definitions for MCP server

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::{Tool, ToolContent, ToolsCallResult};
use crate::security::SecurityContext;
use crate::{McpError, McpResult};

// Import Ratchet's execution types
use ratchet_interfaces::logging::StructuredLogger;
use ratchet_interfaces::{RepositoryFactory, ExecutionRepository, JobRepository, ScheduleRepository, TaskRepository, ExecutionFilters, JobFilters, ScheduleFilters};
use ratchet_api_types::{ApiId, PaginationInput, ExecutionStatus as ApiExecutionStatus, JobStatus, JobPriority};
use ratchet_api_types::pagination::ListInput;

/// MCP tool definition with execution capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool metadata
    pub tool: Tool,

    /// Tool category for organization
    pub category: String,

    /// Whether this tool requires authentication
    pub requires_auth: bool,

    /// Whether this tool can be called by any client
    pub public: bool,
}

impl McpTool {
    /// Create a new MCP tool
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        category: impl Into<String>,
    ) -> Self {
        Self {
            tool: Tool {
                name: name.into(),
                description: description.into(),
                input_schema,
                metadata: HashMap::new(),
            },
            category: category.into(),
            requires_auth: true,
            public: false,
        }
    }

    /// Make this tool public (accessible without authentication)
    pub fn public(mut self) -> Self {
        self.public = true;
        self.requires_auth = false;
        self
    }

    /// Add metadata to the tool
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.tool.metadata.insert(key.into(), value);
        self
    }
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    /// Security context for the request
    pub security: SecurityContext,

    /// Tool arguments
    pub arguments: Option<Value>,

    /// Request correlation ID
    pub request_id: Option<String>,
}

/// Tool registry trait for managing available tools
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    /// List all available tools
    async fn list_tools(&self, context: &SecurityContext) -> McpResult<Vec<Tool>>;

    /// Get a specific tool by name
    async fn get_tool(&self, name: &str, context: &SecurityContext) -> McpResult<Option<McpTool>>;

    /// Execute a tool
    async fn execute_tool(
        &self,
        name: &str,
        execution_context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult>;

    /// Check if a tool exists and is accessible
    async fn can_access_tool(&self, name: &str, context: &SecurityContext) -> bool;
}

/// Simplified task info for MCP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTaskInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
}

/// Execution status for MCP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExecutionStatus {
    pub execution_id: String,
    pub status: String,
    pub task_id: i32,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub error_message: Option<String>,
    pub error_details: Option<Value>,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i32>,
    pub progress: Option<Value>,
}

/// Task executor trait for MCP server
#[async_trait]
pub trait McpTaskExecutor: Send + Sync {
    /// Execute a task
    async fn execute_task(&self, task_path: &str, input: Value) -> Result<Value, String>;

    /// Execute a task with progress streaming support
    async fn execute_task_with_progress(
        &self,
        task_path: &str,
        input: Value,
        progress_manager: Option<Arc<super::progress::ProgressNotificationManager>>,
        connection: Option<Arc<dyn crate::transport::connection::TransportConnection>>,
        filter: Option<super::progress::ProgressFilter>,
    ) -> Result<(String, Value), String>;

    /// List available tasks
    async fn list_tasks(&self, filter: Option<&str>) -> Result<Vec<McpTaskInfo>, String>;

    /// Get execution logs
    async fn get_execution_logs(
        &self,
        execution_id: &str,
        level: &str,
        limit: usize,
    ) -> Result<String, String>;

    /// Get execution status
    async fn get_execution_status(&self, execution_id: &str) -> Result<McpExecutionStatus, String>;
}

/// Ratchet-specific tool registry implementation
pub struct RatchetToolRegistry {
    /// Available tools
    tools: HashMap<String, McpTool>,

    /// Task executor for MCP operations
    task_executor: Option<Arc<dyn McpTaskExecutor>>,

    /// Logger for structured logging
    logger: Option<Arc<dyn StructuredLogger>>,

    /// Progress notification manager
    progress_manager: Arc<super::progress::ProgressNotificationManager>,
    
    /// Task development service for dev tools
    task_dev_service: Option<Arc<super::task_dev_tools::TaskDevelopmentService>>,
    
    /// Repository factory for data access
    repositories: Option<Arc<dyn RepositoryFactory>>,
}

impl RatchetToolRegistry {
    /// Create a new Ratchet tool registry
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            task_executor: None,
            logger: None,
            progress_manager: Arc::new(super::progress::ProgressNotificationManager::new()),
            task_dev_service: None,
            repositories: None,
        };

        // Register built-in Ratchet tools
        registry.register_builtin_tools();

        registry
    }

    /// Register all built-in Ratchet tools
    fn register_builtin_tools(&mut self) {
        // Register task development tools
        super::task_dev_tools::register_task_dev_tools(&mut self.tools);
        
        // Task execution tool
        let execute_task_tool = McpTool::new(
            "ratchet_execute_task",
            "Execute a Ratchet task with given input and optional progress streaming",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID or name of the task to execute"
                    },
                    "input": {
                        "type": "object",
                        "description": "Input data for the task"
                    },
                    "trace": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to enable detailed tracing"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Execution timeout in seconds"
                    },
                    "stream_progress": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to stream real-time progress updates via notifications"
                    },
                    "progress_filter": {
                        "type": "object",
                        "description": "Filter criteria for progress notifications",
                        "properties": {
                            "min_progress_delta": {
                                "type": "number",
                                "description": "Minimum progress change (0.0-1.0) to trigger notification"
                            },
                            "max_frequency_ms": {
                                "type": "integer",
                                "description": "Maximum notification frequency in milliseconds"
                            },
                            "step_filter": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Include only specific step names"
                            },
                            "include_data": {
                                "type": "boolean",
                                "default": true,
                                "description": "Include progress data in notifications"
                            }
                        }
                    }
                },
                "required": ["task_id", "input"]
            }),
            "execution",
        );
        self.tools
            .insert("ratchet_execute_task".to_string(), execute_task_tool);

        // Execution status tool
        let status_tool = McpTool::new(
            "ratchet_get_execution_status",
            "Get status and progress of a running execution",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "ID of the execution to check"
                    }
                },
                "required": ["execution_id"]
            }),
            "monitoring",
        );
        self.tools
            .insert("ratchet_get_execution_status".to_string(), status_tool);

        // Logs retrieval tool
        let logs_tool = McpTool::new(
            "ratchet_get_execution_logs",
            "Retrieve logs for a specific execution",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "ID of the execution"
                    },
                    "level": {
                        "type": "string",
                        "enum": ["trace", "debug", "info", "warn", "error"],
                        "description": "Minimum log level to retrieve"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 100,
                        "description": "Maximum number of log entries"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "text"],
                        "default": "json",
                        "description": "Output format"
                    }
                },
                "required": ["execution_id"]
            }),
            "monitoring",
        );
        self.tools
            .insert("ratchet_get_execution_logs".to_string(), logs_tool);

        // Trace retrieval tool
        let trace_tool = McpTool::new(
            "ratchet_get_execution_trace",
            "Get detailed execution trace with timing and context",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "ID of the execution"
                    },
                    "include_http_calls": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to include HTTP call traces"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "flamegraph"],
                        "default": "json",
                        "description": "Output format"
                    }
                },
                "required": ["execution_id"]
            }),
            "debugging",
        );
        self.tools
            .insert("ratchet_get_execution_trace".to_string(), trace_tool);

        // Task discovery tool with pagination support
        let list_tasks_tool = McpTool::new(
            "ratchet_list_available_tasks",
            "List all available tasks with their schemas and pagination support",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Filter tasks by name pattern"
                    },
                    "include_schemas": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to include input/output schemas"
                    },
                    "category": {
                        "type": "string",
                        "description": "Filter by task category"
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0,
                        "description": "Page number (0-based) for pagination"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "default": 50,
                        "description": "Maximum number of tasks to return per page"
                    },
                    "sort_by": {
                        "type": "string",
                        "enum": ["name", "created_at", "updated_at", "version"],
                        "default": "name",
                        "description": "Field to sort tasks by"
                    },
                    "sort_order": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "default": "asc",
                        "description": "Sort order (ascending or descending)"
                    }
                }
            }),
            "discovery",
        );
        self.tools
            .insert("ratchet_list_available_tasks".to_string(), list_tasks_tool);

        // Error analysis tool
        let analyze_error_tool = McpTool::new(
            "ratchet_analyze_execution_error",
            "Get detailed error analysis for failed execution",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "ID of the failed execution"
                    },
                    "include_suggestions": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to include fix suggestions"
                    },
                    "include_context": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to include execution context"
                    }
                },
                "required": ["execution_id"]
            }),
            "debugging",
        );
        self.tools.insert(
            "ratchet_analyze_execution_error".to_string(),
            analyze_error_tool,
        );

        // Batch execution tool
        let batch_execute_tool = McpTool::new(
            "ratchet_batch_execute",
            "Execute multiple tasks in parallel or sequence with dependency management",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "requests": {
                        "type": "array",
                        "description": "Array of task execution requests",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Unique identifier for this request within the batch"
                                },
                                "task_id": {
                                    "type": "string",
                                    "description": "ID of the task to execute"
                                },
                                "input": {
                                    "type": "object",
                                    "description": "Input data for the task"
                                },
                                "dependencies": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "default": [],
                                    "description": "IDs of other requests this depends on"
                                },
                                "timeout_ms": {
                                    "type": "integer",
                                    "description": "Execution timeout in milliseconds"
                                },
                                "priority": {
                                    "type": "integer",
                                    "default": 0,
                                    "description": "Execution priority (higher values executed first)"
                                }
                            },
                            "required": ["id", "task_id", "input"]
                        }
                    },
                    "execution_mode": {
                        "type": "string",
                        "enum": ["parallel", "sequential", "dependency", "priority_dependency"],
                        "default": "parallel",
                        "description": "How to execute the batch requests"
                    },
                    "max_parallel": {
                        "type": "integer",
                        "description": "Maximum number of requests to execute in parallel"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Total batch timeout in milliseconds"
                    },
                    "stop_on_error": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to stop execution on first error"
                    },
                    "correlation_token": {
                        "type": "string",
                        "description": "Token for tracking batch progress"
                    }
                },
                "required": ["requests"]
            }),
            "execution",
        );
        self.tools
            .insert("ratchet_batch_execute".to_string(), batch_execute_tool);

        // Executions listing tool with pagination
        let list_executions_tool = McpTool::new(
            "ratchet_list_executions",
            "List task executions with filtering, sorting, and pagination support",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Filter executions by task ID"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["queued", "running", "completed", "failed", "cancelled"],
                        "description": "Filter executions by status"
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0,
                        "description": "Page number (0-based) for pagination"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "default": 50,
                        "description": "Maximum number of executions to return per page"
                    },
                    "sort_by": {
                        "type": "string",
                        "enum": ["queued_at", "started_at", "completed_at", "duration_ms", "status"],
                        "default": "queued_at",
                        "description": "Field to sort executions by"
                    },
                    "sort_order": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "default": "desc",
                        "description": "Sort order (ascending or descending)"
                    },
                    "include_output": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to include execution output in results"
                    }
                }
            }),
            "monitoring",
        );
        self.tools
            .insert("ratchet_list_executions".to_string(), list_executions_tool);

        // Jobs listing tool with pagination  
        let list_jobs_tool = McpTool::new(
            "ratchet_list_jobs",
            "List jobs with filtering, sorting, and pagination support",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Filter jobs by task ID"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["queued", "processing", "completed", "failed", "cancelled"],
                        "description": "Filter jobs by status"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["low", "normal", "high", "urgent"],
                        "description": "Filter jobs by priority"
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0,
                        "description": "Page number (0-based) for pagination"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "default": 50,
                        "description": "Maximum number of jobs to return per page"
                    },
                    "sort_by": {
                        "type": "string",
                        "enum": ["queued_at", "scheduled_for", "priority", "status", "retry_count"],
                        "default": "queued_at",
                        "description": "Field to sort jobs by"
                    },
                    "sort_order": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "default": "desc",
                        "description": "Sort order (ascending or descending)"
                    }
                }
            }),
            "monitoring",
        );
        self.tools
            .insert("ratchet_list_jobs".to_string(), list_jobs_tool);

        // Schedules listing tool with pagination
        let list_schedules_tool = McpTool::new(
            "ratchet_list_schedules",
            "List schedules with filtering, sorting, and pagination support",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Filter schedules by task ID"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Filter schedules by enabled status"
                    },
                    "ready_to_run": {
                        "type": "boolean",
                        "description": "Filter schedules that are ready to run now"
                    },
                    "page": {
                        "type": "integer",
                        "minimum": 0,
                        "default": 0,
                        "description": "Page number (0-based) for pagination"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "default": 50,
                        "description": "Maximum number of schedules to return per page"
                    },
                    "sort_by": {
                        "type": "string",
                        "enum": ["name", "created_at", "updated_at", "next_run", "last_run"],
                        "default": "name",
                        "description": "Field to sort schedules by"
                    },
                    "sort_order": {
                        "type": "string",
                        "enum": ["asc", "desc"],
                        "default": "asc",
                        "description": "Sort order (ascending or descending)"
                    }
                }
            }),
            "monitoring",
        );
        self.tools
            .insert("ratchet_list_schedules".to_string(), list_schedules_tool);
    }

    /// Configure the registry with task executor
    pub fn with_task_executor(mut self, executor: Arc<dyn McpTaskExecutor>) -> Self {
        self.task_executor = Some(executor);
        self
    }

    /// Configure with logger
    pub fn with_logger(mut self, logger: Arc<dyn StructuredLogger>) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Set the task executor (for mutable access)
    pub fn set_executor(&mut self, executor: Arc<dyn McpTaskExecutor>) {
        self.task_executor = Some(executor);
    }
    
    /// Set the task development service
    pub fn set_task_dev_service(&mut self, service: Arc<super::task_dev_tools::TaskDevelopmentService>) {
        self.task_dev_service = Some(service);
    }
    
    /// Configure with task development service
    pub fn with_task_dev_service(mut self, service: Arc<super::task_dev_tools::TaskDevelopmentService>) -> Self {
        self.task_dev_service = Some(service);
        self
    }

    /// Set the repository factory for data access
    pub fn with_repositories(mut self, repositories: Arc<dyn RepositoryFactory>) -> Self {
        self.repositories = Some(repositories);
        self
    }

    /// Get the progress manager
    pub fn get_progress_manager(&self) -> Arc<super::progress::ProgressNotificationManager> {
        self.progress_manager.clone()
    }
}

#[async_trait]
impl ToolRegistry for RatchetToolRegistry {
    async fn list_tools(&self, context: &SecurityContext) -> McpResult<Vec<Tool>> {
        let mut accessible_tools = Vec::new();

        for tool in self.tools.values() {
            if self.can_access_tool_internal(tool, context) {
                accessible_tools.push(tool.tool.clone());
            }
        }

        Ok(accessible_tools)
    }

    async fn get_tool(&self, name: &str, context: &SecurityContext) -> McpResult<Option<McpTool>> {
        if let Some(tool) = self.tools.get(name) {
            if self.can_access_tool_internal(tool, context) {
                Ok(Some(tool.clone()))
            } else {
                Err(McpError::AuthorizationDenied {
                    reason: format!("Access denied to tool: {}", name),
                })
            }
        } else {
            Ok(None)
        }
    }

    async fn execute_tool(
        &self,
        name: &str,
        execution_context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        // Get the tool to verify it exists
        let _tool = self
            .get_tool(name, &execution_context.security)
            .await?
            .ok_or_else(|| McpError::ToolNotFound {
                tool_name: name.to_string(),
            })?;

        // Execute the tool based on its name
        match name {
            "ratchet_execute_task" => self.execute_task_tool(execution_context).await,
            "ratchet_get_execution_status" => {
                self.get_execution_status_tool(execution_context).await
            }
            "ratchet_get_execution_logs" => self.get_execution_logs_tool(execution_context).await,
            "ratchet_get_execution_trace" => self.get_execution_trace_tool(execution_context).await,
            "ratchet_list_available_tasks" => {
                self.list_available_tasks_tool(execution_context).await
            }
            "ratchet_analyze_execution_error" => {
                self.analyze_execution_error_tool(execution_context).await
            }
            "ratchet_batch_execute" => self.batch_execute_tool(execution_context).await,
            "ratchet_list_executions" => self.list_executions_tool(execution_context).await,
            "ratchet_list_jobs" => self.list_jobs_tool(execution_context).await,
            "ratchet_list_schedules" => self.list_schedules_tool(execution_context).await,
            // Task development tools
            "ratchet_create_task" | "ratchet_validate_task" | "ratchet_debug_task_execution" | 
            "ratchet_run_task_tests" | "ratchet_create_task_version" | "ratchet_edit_task" |
            "ratchet_delete_task" | "ratchet_import_tasks" | "ratchet_export_tasks" | "ratchet_generate_from_template" |
            "ratchet_list_templates" | "ratchet_store_result" | "ratchet_get_results" => {
                if let Some(service) = &self.task_dev_service {
                    super::task_dev_tools::execute_task_dev_tool(name, execution_context, service.clone()).await
                } else {
                    Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: "Task development service not configured".to_string(),
                        }],
                        is_error: true,
                        metadata: HashMap::new(),
                    })
                }
            }
            _ => Err(McpError::ToolNotFound {
                tool_name: name.to_string(),
            }),
        }
    }

    async fn can_access_tool(&self, name: &str, context: &SecurityContext) -> bool {
        if let Some(tool) = self.tools.get(name) {
            self.can_access_tool_internal(tool, context)
        } else {
            false
        }
    }
}

impl RatchetToolRegistry {
    /// Check if a client can access a specific tool
    fn can_access_tool_internal(&self, tool: &McpTool, context: &SecurityContext) -> bool {
        // Public tools can be accessed by anyone
        if tool.public {
            return true;
        }

        // Check if tool requires authentication
        if tool.requires_auth {
            // For now, just check if the client is authenticated
            // In a real implementation, this would check specific permissions
            !context.client.id.is_empty()
        } else {
            true
        }
    }

    /// Execute the task execution tool
    async fn execute_task_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        // Extract arguments
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_execute_task".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        // Parse task ID and input
        let task_id = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet_execute_task".to_string(),
                details: "Missing or invalid task_id".to_string(),
            })?;

        let input = args.get("input").cloned().unwrap_or(serde_json::json!({}));

        let trace_enabled = args.get("trace").and_then(|v| v.as_bool()).unwrap_or(true);

        let stream_progress = args
            .get("stream_progress")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse progress filter if provided
        let progress_filter =
            args.get("progress_filter")
                .map(|filter_json| super::progress::ProgressFilter {
                    min_progress_delta: filter_json
                        .get("min_progress_delta")
                        .and_then(|v| v.as_f64().map(|f| f as f32)),
                    max_frequency_ms: filter_json.get("max_frequency_ms").and_then(|v| v.as_u64()),
                    step_filter: filter_json
                        .get("step_filter")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect()
                        }),
                    include_data: filter_json
                        .get("include_data")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                });

        // Check if executor is configured
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                // Return error as a tool result rather than failing the request
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert(
                            "task_id".to_string(),
                            serde_json::Value::String(task_id.to_string()),
                        );
                        meta.insert(
                            "error_type".to_string(),
                            serde_json::Value::String("configuration_error".to_string()),
                        );
                        meta
                    },
                });
            }
        };

        // Execute the task with or without progress streaming

        if stream_progress {
            // Use progress streaming
            match executor
                .execute_task_with_progress(
                    task_id,
                    input,
                    Some(self.progress_manager.clone()),
                    None, // TODO: Get connection from context
                    progress_filter,
                )
                .await
            {
                Ok((execution_id, output)) => {
                    // Success response with execution ID for progress tracking
                    Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: serde_json::to_string_pretty(&serde_json::json!({
                                "execution_id": execution_id,
                                "output": output,
                                "streaming": true
                            }))
                            .unwrap_or_else(|_| output.to_string()),
                        }],
                        is_error: false,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "task_id".to_string(),
                                serde_json::Value::String(task_id.to_string()),
                            );
                            meta.insert(
                                "execution_id".to_string(),
                                serde_json::Value::String(execution_id),
                            );
                            meta.insert("streaming".to_string(), serde_json::Value::Bool(true));
                            if trace_enabled {
                                meta.insert(
                                    "trace_enabled".to_string(),
                                    serde_json::Value::Bool(true),
                                );
                            }
                            meta
                        },
                    })
                }
                Err(e) => {
                    // Error response
                    Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: format!("Task execution failed: {}", e),
                        }],
                        is_error: true,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "task_id".to_string(),
                                serde_json::Value::String(task_id.to_string()),
                            );
                            meta.insert(
                                "error_type".to_string(),
                                serde_json::Value::String("execution_error".to_string()),
                            );
                            meta.insert("streaming".to_string(), serde_json::Value::Bool(true));
                            meta
                        },
                    })
                }
            }
        } else {
            // Use regular execution
            match executor.execute_task(task_id, input).await {
                Ok(output) => {
                    // Success response
                    Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: serde_json::to_string_pretty(&output)
                                .unwrap_or_else(|_| output.to_string()),
                        }],
                        is_error: false,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "task_id".to_string(),
                                serde_json::Value::String(task_id.to_string()),
                            );
                            meta.insert("streaming".to_string(), serde_json::Value::Bool(false));
                            if trace_enabled {
                                meta.insert(
                                    "trace_enabled".to_string(),
                                    serde_json::Value::Bool(true),
                                );
                            }
                            meta
                        },
                    })
                }
                Err(e) => {
                    // Error response
                    Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: format!("Task execution failed: {}", e),
                        }],
                        is_error: true,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "task_id".to_string(),
                                serde_json::Value::String(task_id.to_string()),
                            );
                            meta.insert(
                                "error_type".to_string(),
                                serde_json::Value::String("execution_error".to_string()),
                            );
                            meta.insert("streaming".to_string(), serde_json::Value::Bool(false));
                            meta
                        },
                    })
                }
            }
        }
    }

    /// Execute the execution status tool
    async fn get_execution_status_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_get_execution_status".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        // Parse execution ID
        let execution_id = args
            .get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet_get_execution_status".to_string(),
                details: "Missing or invalid execution_id".to_string(),
            })?;

        // Check if executor is configured (which provides access to repositories)
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Get real execution status from the repository
        match executor.get_execution_status(execution_id).await {
            Ok(status) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&status)
                        .unwrap_or_else(|_| "Failed to serialize execution status".to_string()),
                }],
                is_error: false,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "status".to_string(),
                        serde_json::Value::String(status.status.clone()),
                    );
                    meta.insert(
                        "task_id".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(status.task_id)),
                    );
                    meta
                },
            }),
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to get execution status: {}", e),
                }],
                is_error: true,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("status_retrieval_error".to_string()),
                    );
                    meta
                },
            }),
        }
    }

    /// Execute the logs retrieval tool
    async fn get_execution_logs_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_get_execution_logs".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        // Parse execution ID
        let execution_id = args
            .get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet_get_execution_logs".to_string(),
                details: "Missing or invalid execution_id".to_string(),
            })?;

        let level = args.get("level").and_then(|v| v.as_str()).unwrap_or("info");

        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(100) as usize;

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        // Check if executor is configured
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Use the improved logs retrieval from adapter
        match executor
            .get_execution_logs(execution_id, level, limit)
            .await
        {
            Ok(logs_output) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text { text: logs_output }],
                is_error: false,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "level".to_string(),
                        serde_json::Value::String(level.to_string()),
                    );
                    meta.insert(
                        "limit".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(limit)),
                    );
                    meta.insert(
                        "format".to_string(),
                        serde_json::Value::String(format.to_string()),
                    );
                    meta
                },
            }),
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to retrieve logs: {}", e),
                }],
                is_error: true,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("logs_retrieval_error".to_string()),
                    );
                    meta
                },
            }),
        }
    }

    /// Execute the trace retrieval tool
    async fn get_execution_trace_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_get_execution_trace".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        // Parse execution ID
        let execution_id = args
            .get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet_get_execution_trace".to_string(),
                details: "Missing or invalid execution_id".to_string(),
            })?;

        let include_http_calls = args
            .get("include_http_calls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        // Check if executor is configured
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Try to get the execution from the adapter
        match self
            .get_execution_trace_data(executor, execution_id, include_http_calls)
            .await
        {
            Ok(trace_data) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: if format == "flamegraph" {
                        "Flamegraph format not yet supported - returning JSON trace data"
                            .to_string()
                    } else {
                        serde_json::to_string_pretty(&trace_data)
                            .unwrap_or_else(|_| trace_data.to_string())
                    },
                }],
                is_error: false,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "format".to_string(),
                        serde_json::Value::String(format.to_string()),
                    );
                    meta.insert(
                        "include_http_calls".to_string(),
                        serde_json::Value::Bool(include_http_calls),
                    );
                    meta.insert(
                        "trace_type".to_string(),
                        serde_json::Value::String("detailed".to_string()),
                    );
                    meta
                },
            }),
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to get execution trace: {}", e),
                }],
                is_error: true,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("trace_retrieval_error".to_string()),
                    );
                    meta
                },
            }),
        }
    }

    /// Execute the task listing tool with pagination support
    async fn list_available_tasks_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.unwrap_or(serde_json::json!({}));

        let filter = args.get("filter").and_then(|v| v.as_str());
        let include_schemas = args
            .get("include_schemas")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let category = args.get("category").and_then(|v| v.as_str());
        
        // Pagination parameters
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let sort_by = args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("name");
        let sort_order = args.get("sort_order").and_then(|v| v.as_str()).unwrap_or("asc");

        // Validate pagination parameters
        if limit > 1000 {
            return Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: "Limit cannot exceed 1000 tasks per page".to_string(),
                }],
                is_error: true,
                metadata: HashMap::new(),
            });
        }

        // Check if executor is configured
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Query tasks
        match executor.list_tasks(filter).await {
            Ok(mut tasks) => {
                // Apply category filter if provided
                if let Some(cat) = category {
                    tasks.retain(|task| task.tags.contains(&cat.to_string()));
                }

                // Sort tasks
                tasks.sort_by(|a, b| {
                    let ordering = match sort_by {
                        "name" => a.name.cmp(&b.name),
                        "version" => a.version.cmp(&b.version),
                        // Note: created_at and updated_at would need to be added to McpTaskInfo
                        "created_at" | "updated_at" => a.name.cmp(&b.name), // fallback to name
                        _ => a.name.cmp(&b.name),
                    };
                    
                    if sort_order == "desc" {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                });

                // Calculate pagination
                let total_count = tasks.len();
                let total_pages = (total_count + limit - 1) / limit; // ceiling division
                let start_index = page * limit;
                let end_index = std::cmp::min(start_index + limit, total_count);
                
                // Check if page is valid
                if start_index >= total_count && total_count > 0 {
                    return Ok(ToolsCallResult {
                        content: vec![ToolContent::Text {
                            text: format!("Page {} is out of range. Total pages: {}", page, total_pages),
                        }],
                        is_error: true,
                        metadata: HashMap::new(),
                    });
                }

                // Extract paginated tasks
                let paginated_tasks = if total_count == 0 {
                    Vec::new()
                } else {
                    tasks[start_index..end_index].to_vec()
                };

                // Build task list with optional schemas
                let mut task_list = Vec::new();
                for task in paginated_tasks {
                    let mut task_info = serde_json::json!({
                        "id": task.id,
                        "name": task.name,
                        "version": task.version,
                        "description": task.description,
                        "tags": task.tags,
                        "enabled": task.enabled,
                    });

                    if include_schemas {
                        if let Some(input_schema) = &task.input_schema {
                            task_info["input_schema"] = input_schema.clone();
                        }
                        if let Some(output_schema) = &task.output_schema {
                            task_info["output_schema"] = output_schema.clone();
                        }
                    }

                    task_list.push(task_info);
                }

                // Build paginated response
                let response = serde_json::json!({
                    "tasks": task_list,
                    "pagination": {
                        "page": page,
                        "limit": limit,
                        "total_count": total_count,
                        "total_pages": total_pages,
                        "has_next": page + 1 < total_pages,
                        "has_previous": page > 0,
                        "next_page": if page + 1 < total_pages { Some(page + 1) } else { None::<usize> },
                        "previous_page": if page > 0 { Some(page - 1) } else { None::<usize> }
                    },
                    "sorting": {
                        "sort_by": sort_by,
                        "sort_order": sort_order
                    },
                    "filters": {
                        "name_filter": filter,
                        "category_filter": category
                    }
                });

                Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|_| "{}".to_string()),
                    }],
                    is_error: false,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert(
                            "total_count".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(total_count)),
                        );
                        meta.insert(
                            "page".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(page)),
                        );
                        meta.insert(
                            "limit".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(limit)),
                        );
                        meta.insert(
                            "total_pages".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(total_pages)),
                        );
                        if let Some(f) = filter {
                            meta.insert(
                                "filter".to_string(),
                                serde_json::Value::String(f.to_string()),
                            );
                        }
                        if let Some(cat) = category {
                            meta.insert(
                                "category".to_string(),
                                serde_json::Value::String(cat.to_string()),
                            );
                        }
                        meta.insert(
                            "sort_by".to_string(),
                            serde_json::Value::String(sort_by.to_string()),
                        );
                        meta.insert(
                            "sort_order".to_string(),
                            serde_json::Value::String(sort_order.to_string()),
                        );
                        meta
                    },
                })
            }
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to list tasks: {}", e),
                }],
                is_error: true,
                metadata: HashMap::new(),
            }),
        }
    }

    /// Execute the error analysis tool
    async fn analyze_execution_error_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_analyze_execution_error".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        // Parse execution ID
        let execution_id = args
            .get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet_analyze_execution_error".to_string(),
                details: "Missing or invalid execution_id".to_string(),
            })?;

        let include_suggestions = args
            .get("include_suggestions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let include_context = args
            .get("include_context")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Check if executor is configured
        let executor = match self.task_executor.as_ref() {
            Some(exec) => exec,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Task executor not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Perform detailed error analysis
        match self
            .perform_error_analysis(executor, execution_id, include_suggestions, include_context)
            .await
        {
            Ok(error_analysis) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&error_analysis)
                        .unwrap_or_else(|_| error_analysis.to_string()),
                }],
                is_error: false,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "include_suggestions".to_string(),
                        serde_json::Value::Bool(include_suggestions),
                    );
                    meta.insert(
                        "include_context".to_string(),
                        serde_json::Value::Bool(include_context),
                    );
                    meta.insert(
                        "analysis_type".to_string(),
                        serde_json::Value::String("detailed".to_string()),
                    );
                    meta
                },
            }),
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to analyze execution error: {}", e),
                }],
                is_error: true,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "execution_id".to_string(),
                        serde_json::Value::String(execution_id.to_string()),
                    );
                    meta.insert(
                        "error_type".to_string(),
                        serde_json::Value::String("analysis_error".to_string()),
                    );
                    meta
                },
            }),
        }
    }

    /// Helper method to perform detailed error analysis
    async fn perform_error_analysis(
        &self,
        executor: &Arc<dyn McpTaskExecutor>,
        execution_id: &str,
        include_suggestions: bool,
        include_context: bool,
    ) -> Result<serde_json::Value, String> {
        // Get execution status to understand the failure
        let execution_status = executor.get_execution_status(execution_id).await?;

        // Only analyze failed executions
        if execution_status.status != "failed" {
            return Ok(serde_json::json!({
                "execution_id": execution_id,
                "status": execution_status.status,
                "message": format!("Cannot analyze error for execution with status: {}", execution_status.status),
                "analysis": {
                    "error_type": "not_applicable",
                    "root_cause": "Execution did not fail",
                    "impact": "none",
                    "severity": "none"
                }
            }));
        }

        // Extract error information
        let error_message = execution_status
            .error_message
            .unwrap_or_else(|| "Unknown error".to_string());
        let error_details = execution_status.error_details;

        // Analyze error patterns
        let (error_type, severity, category) = Self::classify_error(&error_message, &error_details);
        let root_cause = Self::determine_root_cause(&error_message, &error_details);
        let impact = Self::assess_error_impact(&error_type, &severity);

        // Build context if requested
        let context = if include_context {
            Some(serde_json::json!({
                "task_id": execution_status.task_id,
                "input_data": execution_status.input,
                "queued_at": execution_status.queued_at,
                "started_at": execution_status.started_at,
                "completed_at": execution_status.completed_at,
                "duration_ms": execution_status.duration_ms,
                "error_details": error_details
            }))
        } else {
            None
        };

        // Generate suggestions if requested
        let suggestions = if include_suggestions {
            Self::generate_error_suggestions(&error_type, &error_message, &error_details)
        } else {
            Vec::new()
        };

        // Try to get related logs for additional context
        let log_context = match executor.get_execution_logs(execution_id, "error", 10).await {
            Ok(logs) => Some(logs),
            Err(_) => None,
        };

        Ok(serde_json::json!({
            "execution_id": execution_id,
            "analysis": {
                "error_type": error_type,
                "category": category,
                "root_cause": root_cause,
                "impact": impact,
                "severity": severity,
                "error_message": error_message
            },
            "context": context,
            "suggestions": suggestions,
            "next_steps": Self::generate_next_steps(&error_type, &severity),
            "log_context": log_context,
            "analysis_timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Helper method to get execution trace data
    async fn get_execution_trace_data(
        &self,
        executor: &Arc<dyn McpTaskExecutor>,
        execution_id: &str,
        include_http_calls: bool,
    ) -> Result<serde_json::Value, String> {
        // Get basic execution status
        let execution_status = executor.get_execution_status(execution_id).await?;

        // Calculate timing information
        let timing = Self::calculate_execution_timing(&execution_status);

        // Get logs to extract trace events
        let events = match executor
            .get_execution_logs(execution_id, "trace", 1000)
            .await
        {
            Ok(logs_str) => Self::extract_trace_events_from_logs(&logs_str),
            Err(_) => Vec::new(),
        };

        // Extract HTTP calls if requested and available
        let http_calls = if include_http_calls {
            Self::extract_http_calls(&execution_status)
        } else {
            serde_json::Value::Null
        };

        // Generate spans from events
        let spans = Self::generate_spans_from_events(&events);

        Ok(serde_json::json!({
            "execution_id": execution_id,
            "status": execution_status.status,
            "task_id": execution_status.task_id,
            "trace": {
                "spans": spans,
                "events": events,
                "timing": timing,
                "http_calls": http_calls,
                "total_events": events.len(),
                "trace_complete": execution_status.status == "completed" || execution_status.status == "failed"
            },
            "metadata": {
                "input_size_bytes": execution_status.input.as_ref().map(|i| serde_json::to_string(i).unwrap_or_default().len()),
                "output_size_bytes": execution_status.output.as_ref().map(|o| serde_json::to_string(o).unwrap_or_default().len()),
                "has_error": execution_status.error_message.is_some()
            },
            "trace_timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Classify error type and severity based on error message and details
    fn classify_error(
        error_message: &str,
        error_details: &Option<serde_json::Value>,
    ) -> (String, String, String) {
        let message_lower = error_message.to_lowercase();

        // Check for common error patterns
        if message_lower.contains("timeout") || message_lower.contains("timed out") {
            (
                "timeout_error".to_string(),
                "medium".to_string(),
                "transient".to_string(),
            )
        } else if message_lower.contains("network") || message_lower.contains("connection") {
            (
                "network_error".to_string(),
                "medium".to_string(),
                "transient".to_string(),
            )
        } else if message_lower.contains("permission") || message_lower.contains("unauthorized") {
            (
                "permission_error".to_string(),
                "high".to_string(),
                "security".to_string(),
            )
        } else if message_lower.contains("not found") || message_lower.contains("404") {
            (
                "not_found_error".to_string(),
                "low".to_string(),
                "client".to_string(),
            )
        } else if message_lower.contains("validation") || message_lower.contains("invalid") {
            (
                "validation_error".to_string(),
                "medium".to_string(),
                "input".to_string(),
            )
        } else if message_lower.contains("out of memory") || message_lower.contains("oom") {
            (
                "resource_error".to_string(),
                "critical".to_string(),
                "system".to_string(),
            )
        } else if message_lower.contains("database") || message_lower.contains("sql") {
            (
                "database_error".to_string(),
                "high".to_string(),
                "infrastructure".to_string(),
            )
        } else if message_lower.contains("javascript") || message_lower.contains("js") {
            (
                "javascript_error".to_string(),
                "medium".to_string(),
                "runtime".to_string(),
            )
        } else if message_lower.contains("syntax") {
            (
                "syntax_error".to_string(),
                "high".to_string(),
                "code".to_string(),
            )
        } else {
            // Try to extract more specific information from error details
            if let Some(details) = error_details {
                if let Some(error_code) = details.get("error_code").and_then(|v| v.as_str()) {
                    return (
                        format!("{}_{}", error_code.to_lowercase(), "error"),
                        "medium".to_string(),
                        "application".to_string(),
                    );
                }
            }
            (
                "unknown_error".to_string(),
                "medium".to_string(),
                "unknown".to_string(),
            )
        }
    }

    /// Determine root cause based on error analysis
    fn determine_root_cause(
        error_message: &str,
        _error_details: &Option<serde_json::Value>,
    ) -> String {
        let message_lower = error_message.to_lowercase();

        if message_lower.contains("timeout") {
            "Operation exceeded maximum allowed execution time".to_string()
        } else if message_lower.contains("network") {
            "Network connectivity issue or service unavailability".to_string()
        } else if message_lower.contains("permission") {
            "Insufficient permissions or authentication failure".to_string()
        } else if message_lower.contains("not found") {
            "Requested resource does not exist or is inaccessible".to_string()
        } else if message_lower.contains("validation") {
            "Input data does not meet required schema or business rules".to_string()
        } else if message_lower.contains("out of memory") {
            "Insufficient system memory to complete operation".to_string()
        } else if message_lower.contains("database") {
            "Database connection or query execution problem".to_string()
        } else {
            format!("Application error: {}", error_message)
        }
    }

    /// Assess the impact of the error
    fn assess_error_impact(error_type: &str, severity: &str) -> String {
        match (error_type, severity) {
            (_, "critical") => {
                "System stability affected, immediate intervention required".to_string()
            }
            ("permission_error", "high") => {
                "Security breach potential, access controls may be compromised".to_string()
            }
            ("database_error", "high") => {
                "Data integrity concerns, service degradation likely".to_string()
            }
            ("resource_error", _) => {
                "System performance degraded, may affect other operations".to_string()
            }
            ("timeout_error", _) => "Operation incomplete, data consistency uncertain".to_string(),
            ("validation_error", _) => {
                "Invalid data processed, output reliability compromised".to_string()
            }
            _ => "Isolated failure, limited impact on overall system".to_string(),
        }
    }

    /// Generate actionable suggestions based on error type
    fn generate_error_suggestions(
        error_type: &str,
        error_message: &str,
        _error_details: &Option<serde_json::Value>,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        match error_type {
            "timeout_error" => {
                suggestions.push("Increase timeout configuration for the task".to_string());
                suggestions.push("Check if external services are responding slowly".to_string());
                suggestions.push(
                    "Consider breaking down large operations into smaller chunks".to_string(),
                );
                suggestions.push("Review task input size and complexity".to_string());
            }
            "network_error" => {
                suggestions.push("Verify network connectivity to external services".to_string());
                suggestions.push("Check firewall rules and proxy configurations".to_string());
                suggestions.push("Implement retry logic with exponential backoff".to_string());
                suggestions.push("Monitor service health of external dependencies".to_string());
            }
            "permission_error" => {
                suggestions.push("Verify API keys and authentication credentials".to_string());
                suggestions.push("Check user permissions and access rights".to_string());
                suggestions.push("Review security policies and access controls".to_string());
                suggestions.push("Ensure proper authorization headers are included".to_string());
            }
            "validation_error" => {
                suggestions.push("Validate input data against the task schema".to_string());
                suggestions.push("Check for required fields and data types".to_string());
                suggestions.push("Review input transformation logic".to_string());
                suggestions.push("Test with known valid input samples".to_string());
            }
            "resource_error" => {
                suggestions.push("Increase system memory allocation".to_string());
                suggestions.push("Optimize task code for better memory usage".to_string());
                suggestions.push("Consider processing data in smaller batches".to_string());
                suggestions.push("Monitor system resource usage patterns".to_string());
            }
            "database_error" => {
                suggestions.push("Check database connection configuration".to_string());
                suggestions.push("Verify database credentials and permissions".to_string());
                suggestions.push("Review query performance and optimization".to_string());
                suggestions.push("Monitor database health and connection pool".to_string());
            }
            "javascript_error" => {
                suggestions.push("Review JavaScript code for syntax errors".to_string());
                suggestions.push("Check for undefined variables or functions".to_string());
                suggestions.push("Validate external library dependencies".to_string());
                suggestions.push("Test task code in isolation".to_string());
            }
            _ => {
                suggestions.push("Enable debug logging for detailed error information".to_string());
                suggestions.push("Check system logs for additional context".to_string());
                suggestions.push("Verify task configuration and dependencies".to_string());
                suggestions.push("Test with minimal input to isolate the issue".to_string());
            }
        }

        // Add general suggestions
        if error_message.contains("Error:") || error_message.contains("Exception:") {
            suggestions.push("Review the complete stack trace for error origin".to_string());
        }

        suggestions
    }

    /// Generate next steps based on error analysis
    fn generate_next_steps(error_type: &str, severity: &str) -> Vec<String> {
        let mut steps = Vec::new();

        // Immediate actions based on severity
        match severity {
            "critical" => {
                steps.push(
                    "Immediately stop similar executions to prevent further issues".to_string(),
                );
                steps.push("Alert system administrators and on-call team".to_string());
                steps.push(
                    "Implement emergency rollback if recent changes are suspected".to_string(),
                );
            }
            "high" => {
                steps.push("Investigate and resolve within the next hour".to_string());
                steps.push("Monitor for similar errors in other executions".to_string());
            }
            _ => {
                steps.push("Schedule investigation within normal business hours".to_string());
            }
        }

        // Specific actions based on error type
        match error_type {
            "timeout_error" => {
                steps.push(
                    "Review task performance metrics and optimization opportunities".to_string(),
                );
            }
            "permission_error" => {
                steps.push("Coordinate with security team to review access policies".to_string());
            }
            "validation_error" => {
                steps.push("Update input validation rules and documentation".to_string());
            }
            _ => {}
        }

        // General next steps
        steps.push("Document the error and resolution for future reference".to_string());
        steps.push("Consider adding monitoring alerts for this error pattern".to_string());

        steps
    }

    /// Calculate execution timing information
    fn calculate_execution_timing(execution_status: &McpExecutionStatus) -> serde_json::Value {
        let queued_time = chrono::DateTime::parse_from_rfc3339(&execution_status.queued_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok();

        let start_time = execution_status
            .started_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let _complete_time = execution_status
            .completed_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let queue_duration = if let (Some(queued), Some(started)) = (queued_time, start_time) {
            Some(started.signed_duration_since(queued).num_milliseconds())
        } else {
            None
        };

        serde_json::json!({
            "total_duration_ms": execution_status.duration_ms,
            "queue_duration_ms": queue_duration,
            "execution_start": execution_status.started_at,
            "execution_end": execution_status.completed_at,
            "was_queued": queue_duration.is_some(),
            "is_complete": execution_status.completed_at.is_some()
        })
    }

    /// Extract trace events from log output
    fn extract_trace_events_from_logs(logs_str: &str) -> Vec<serde_json::Value> {
        let mut events = Vec::new();

        // Try to parse logs as JSON and extract events
        if let Ok(logs_json) = serde_json::from_str::<serde_json::Value>(logs_str) {
            if let Some(logs_array) = logs_json.get("logs").and_then(|l| l.as_array()) {
                for log_entry in logs_array {
                    if let Some(level) = log_entry.get("level").and_then(|l| l.as_str()) {
                        let event = serde_json::json!({
                            "timestamp": log_entry.get("timestamp"),
                            "level": level,
                            "message": log_entry.get("message"),
                            "logger": log_entry.get("logger"),
                            "fields": log_entry.get("fields").unwrap_or(&serde_json::Value::Object(serde_json::Map::new())),
                            "trace_id": log_entry.get("trace_id"),
                            "span_id": log_entry.get("span_id")
                        });
                        events.push(event);
                    }
                }
            }
        }

        // If no structured events found, create a summary event
        if events.is_empty() {
            events.push(serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "level": "info",
                "message": "No structured trace events found in logs",
                "event_type": "trace_summary",
                "raw_logs_available": !logs_str.is_empty()
            }));
        }

        events
    }

    /// Extract HTTP call information from execution status
    fn extract_http_calls(_execution_status: &McpExecutionStatus) -> serde_json::Value {
        // Note: This would integrate with the http_requests field from the execution entity
        // For now, return a placeholder structure
        serde_json::json!({
            "total_calls": 0,
            "successful_calls": 0,
            "failed_calls": 0,
            "calls": [],
            "message": "HTTP call tracking not yet fully implemented"
        })
    }

    /// Generate execution spans from trace events
    fn generate_spans_from_events(events: &[serde_json::Value]) -> Vec<serde_json::Value> {
        let mut spans = Vec::new();

        // Group events by span_id to create spans
        let mut span_groups: std::collections::HashMap<String, Vec<&serde_json::Value>> =
            std::collections::HashMap::new();

        for event in events {
            if let Some(span_id) = event.get("span_id").and_then(|s| s.as_str()) {
                span_groups
                    .entry(span_id.to_string())
                    .or_default()
                    .push(event);
            }
        }

        // Create spans from grouped events
        for (span_id, span_events) in span_groups {
            let start_time = span_events
                .iter()
                .filter_map(|e| e.get("timestamp").and_then(|t| t.as_str()))
                .min();

            let end_time = span_events
                .iter()
                .filter_map(|e| e.get("timestamp").and_then(|t| t.as_str()))
                .max();

            let operation_name = span_events
                .first()
                .and_then(|e| e.get("fields").and_then(|f| f.get("operation")))
                .and_then(|o| o.as_str())
                .unwrap_or("unknown_operation");

            spans.push(serde_json::json!({
                "span_id": span_id,
                "operation_name": operation_name,
                "start_time": start_time,
                "end_time": end_time,
                "event_count": span_events.len(),
                "tags": {
                    "component": "ratchet",
                    "span.kind": "internal"
                }
            }));
        }

        // If no spans found, create a default execution span
        if spans.is_empty() {
            spans.push(serde_json::json!({
                "span_id": "execution",
                "operation_name": "task_execution",
                "start_time": null,
                "end_time": null,
                "event_count": events.len(),
                "tags": {
                    "component": "ratchet",
                    "span.kind": "internal",
                    "synthetic": true
                }
            }));
        }

        spans
    }

    /// Execute multiple tasks in a batch with support for dependencies and parallel execution
    async fn batch_execute_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        #[derive(Deserialize)]
        struct BatchExecuteRequest {
            requests: Vec<BatchTaskRequest>,
            #[serde(default)]
            execution_mode: String,
            max_parallel: Option<u32>,
            timeout_ms: Option<u64>,
            #[serde(default)]
            stop_on_error: bool,
            correlation_token: Option<String>,
        }

        #[derive(Deserialize)]
        struct BatchTaskRequest {
            id: String,
            task_id: String,
            input: Value,
            #[serde(default)]
            dependencies: Vec<String>,
            timeout_ms: Option<u64>,
            #[serde(default)]
            priority: i32,
        }

        let arguments = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet_batch_execute".to_string(),
            details: "Missing arguments".to_string(),
        })?;

        let request: BatchExecuteRequest =
            serde_json::from_value(arguments).map_err(|e| McpError::InvalidParams {
                method: "ratchet_batch_execute".to_string(),
                details: format!("Invalid batch execute request: {}", e),
            })?;

        // Get task executor
        let _executor = self
            .task_executor
            .as_ref()
            .ok_or_else(|| McpError::Internal {
                message: "Task executor not configured".to_string(),
            })?;

        // Convert to MCP batch format
        let mcp_batch_requests: Vec<crate::protocol::BatchRequest> = request
            .requests
            .into_iter()
            .map(|req| crate::protocol::BatchRequest {
                id: req.id,
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "name": "ratchet_execute_task",
                    "arguments": {
                        "task_id": req.task_id,
                        "input": req.input
                    }
                })),
                dependencies: req.dependencies,
                timeout_ms: req.timeout_ms,
                priority: req.priority,
                metadata: std::collections::HashMap::new(),
            })
            .collect();

        let execution_mode = match request.execution_mode.as_str() {
            "sequential" => crate::protocol::BatchExecutionMode::Sequential,
            "dependency" => crate::protocol::BatchExecutionMode::Dependency,
            "priority_dependency" => crate::protocol::BatchExecutionMode::PriorityDependency,
            _ => crate::protocol::BatchExecutionMode::Parallel,
        };

        let batch_params = crate::protocol::BatchParams {
            requests: mcp_batch_requests,
            execution_mode,
            max_parallel: request.max_parallel,
            timeout_ms: request.timeout_ms,
            stop_on_error: request.stop_on_error,
            correlation_token: request.correlation_token,
            metadata: std::collections::HashMap::new(),
        };

        // Create a batch processor
        use super::batch::BatchProcessor;
        use std::time::Duration;

        let batch_processor = BatchProcessor::new(
            100,                                // max_batch_size
            request.max_parallel.unwrap_or(10), // max_parallel
            Duration::from_secs(300),           // default_timeout
            Arc::new(move |_request| {
                // This is a placeholder - in practice, we'd need access to the handler
                Box::pin(async move {
                    crate::protocol::JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(
                            serde_json::json!({"error": "Not implemented in tool context"}),
                        ),
                        error: None,
                        id: None,
                    }
                })
            }),
            None, // progress_callback
        );

        match batch_processor.process_batch(batch_params).await {
            Ok(result) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "Failed to serialize batch result".to_string()),
                }],
                is_error: false,
                metadata: std::collections::HashMap::new(),
            }),
            Err(e) => Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Batch execution failed: {}", e),
                }],
                is_error: true,
                metadata: std::collections::HashMap::from([
                    (
                        "error_type".to_string(),
                        Value::String("batch_execution_error".to_string()),
                    ),
                    ("error_details".to_string(), Value::String(e.to_string())),
                ]),
            }),
        }
    }

    /// Execute the executions listing tool with pagination support
    async fn list_executions_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.unwrap_or(serde_json::json!({}));

        // Extract filters and pagination parameters
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        let status = args.get("status").and_then(|v| v.as_str());
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let sort_by = args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("queued_at");
        let sort_order = args.get("sort_order").and_then(|v| v.as_str()).unwrap_or("desc");
        let include_output = args.get("include_output").and_then(|v| v.as_bool()).unwrap_or(false);

        // Validate pagination parameters
        if limit > 1000 {
            return Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: "Limit cannot exceed 1000 executions per page".to_string(),
                }],
                is_error: true,
                metadata: HashMap::new(),
            });
        }

        // Check if repositories are available
        let repositories = match &self.repositories {
            Some(repos) => repos,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Repository factory not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Convert status string to ExecutionStatus if provided
        let status_filter = if let Some(status_str) = status {
            match status_str.to_lowercase().as_str() {
                "pending" => Some(ApiExecutionStatus::Pending),
                "running" => Some(ApiExecutionStatus::Running),
                "completed" => Some(ApiExecutionStatus::Completed),
                "failed" => Some(ApiExecutionStatus::Failed),
                "cancelled" => Some(ApiExecutionStatus::Cancelled),
                _ => None,
            }
        } else {
            None
        };

        // Convert task_id string to ApiId if provided
        let task_id_filter = task_id.map(|id| ApiId::from_string(id.to_string()));

        // Create execution filters
        let filters = ExecutionFilters {
            task_id: task_id_filter,
            status: status_filter,
            queued_after: None,
            completed_after: None,
            task_id_in: None,
            id_in: None,
            status_in: None,
            status_not: None,
            queued_before: None,
            started_after: None,
            started_before: None,
            completed_before: None,
            duration_min_ms: None,
            duration_max_ms: None,
            progress_min: None,
            progress_max: None,
            has_progress: None,
            has_error: None,
            error_message_contains: None,
            can_retry: None,
            can_cancel: None,
        };

        // Create pagination input
        let pagination = PaginationInput {
            page: Some(page as u32),
            limit: Some(limit as u32),
            offset: Some((page * limit) as u32),
        };

        // Query the execution repository
        let execution_repo = repositories.execution_repository();
        let response = match execution_repo.find_with_filters(filters, pagination).await {
            Ok(list_response) => {
                // Convert executions to MCP format
                let executions: Vec<serde_json::Value> = list_response.items.into_iter().map(|execution| {
                    let mut exec_json = serde_json::json!({
                        "id": execution.id.to_string(),
                        "task_id": execution.task_id.to_string(),
                        "status": execution.status,
                        "queued_at": execution.queued_at,
                        "started_at": execution.started_at,
                        "completed_at": execution.completed_at,
                        "duration_ms": execution.duration_ms,
                        "progress": execution.progress,
                        "error_message": execution.error_message,
                    });

                    // Include output if requested
                    if include_output {
                        exec_json["output"] = execution.output.unwrap_or(serde_json::Value::Null);
                        exec_json["error_details"] = execution.error_details.unwrap_or(serde_json::Value::Null);
                    }

                    exec_json
                }).collect();

                serde_json::json!({
                    "executions": executions,
                    "pagination": {
                        "page": list_response.meta.page,
                        "limit": list_response.meta.limit,
                        "total_count": list_response.meta.total,
                        "total_pages": list_response.meta.total_pages,
                        "has_next": list_response.meta.has_next,
                        "has_previous": list_response.meta.has_previous,
                        "next_page": if list_response.meta.has_next { Some(list_response.meta.page + 1) } else { None },
                        "previous_page": if list_response.meta.has_previous { Some(list_response.meta.page - 1) } else { None }
                    },
                    "sorting": {
                        "sort_by": sort_by,
                        "sort_order": sort_order
                    },
                    "filters": {
                        "task_id": task_id,
                        "status": status,
                        "include_output": include_output
                    }
                })
            }
            Err(e) => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to fetch executions: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "{}".to_string()),
            }],
            is_error: false,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("page".to_string(), serde_json::Value::Number(serde_json::Number::from(page)));
                meta.insert("limit".to_string(), serde_json::Value::Number(serde_json::Number::from(limit)));
                meta.insert("sort_by".to_string(), serde_json::Value::String(sort_by.to_string()));
                meta.insert("sort_order".to_string(), serde_json::Value::String(sort_order.to_string()));
                meta
            },
        })
    }

    /// Execute the jobs listing tool with pagination support
    async fn list_jobs_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.unwrap_or(serde_json::json!({}));

        // Extract filters and pagination parameters
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        let status = args.get("status").and_then(|v| v.as_str());
        let priority = args.get("priority").and_then(|v| v.as_str());
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let sort_by = args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("queued_at");
        let sort_order = args.get("sort_order").and_then(|v| v.as_str()).unwrap_or("desc");

        // Validate pagination parameters
        if limit > 1000 {
            return Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: "Limit cannot exceed 1000 jobs per page".to_string(),
                }],
                is_error: true,
                metadata: HashMap::new(),
            });
        }

        // Check if repositories are available
        let repositories = match &self.repositories {
            Some(repos) => repos,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Repository factory not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Convert status string to JobStatus if provided
        let status_filter = if let Some(status_str) = status {
            match status_str.to_lowercase().as_str() {
                "queued" => Some(ratchet_api_types::JobStatus::Queued),
                "processing" => Some(ratchet_api_types::JobStatus::Processing),
                "completed" => Some(ratchet_api_types::JobStatus::Completed),
                "failed" => Some(ratchet_api_types::JobStatus::Failed),
                "cancelled" => Some(ratchet_api_types::JobStatus::Cancelled),
                "retrying" => Some(ratchet_api_types::JobStatus::Retrying),
                _ => None,
            }
        } else {
            None
        };

        // Convert priority string to JobPriority if provided
        let priority_filter = if let Some(priority_str) = priority {
            match priority_str.to_lowercase().as_str() {
                "low" => Some(ratchet_api_types::JobPriority::Low),
                "normal" => Some(ratchet_api_types::JobPriority::Normal),
                "high" => Some(ratchet_api_types::JobPriority::High),
                "critical" => Some(ratchet_api_types::JobPriority::Critical),
                _ => None,
            }
        } else {
            None
        };

        // Convert task_id string to ApiId if provided
        let task_id_filter = task_id.map(|id| ApiId::from_string(id.to_string()));

        // Create job filters
        let filters = JobFilters {
            task_id: task_id_filter,
            status: status_filter,
            priority: priority_filter,
            queued_after: None,
            scheduled_before: None,
            task_id_in: None,
            id_in: None,
            status_in: None,
            status_not: None,
            priority_in: None,
            priority_min: None,
            queued_before: None,
            scheduled_after: None,
            retry_count_min: None,
            retry_count_max: None,
            max_retries_min: None,
            max_retries_max: None,
            has_retries_remaining: None,
            has_error: None,
            error_message_contains: None,
            is_scheduled: None,
            due_now: None,
        };

        // Create pagination input
        let pagination = PaginationInput {
            page: Some(page as u32),
            limit: Some(limit as u32),
            offset: Some((page * limit) as u32),
        };

        // Query the job repository
        let job_repo = repositories.job_repository();
        let response = match job_repo.find_with_filters(filters, pagination).await {
            Ok(list_response) => {
                // Convert jobs to MCP format
                let jobs: Vec<serde_json::Value> = list_response.items.into_iter().map(|job| {
                    serde_json::json!({
                        "id": job.id.to_string(),
                        "task_id": job.task_id.to_string(),
                        "priority": job.priority,
                        "status": job.status,
                        "retry_count": job.retry_count,
                        "max_retries": job.max_retries,
                        "queued_at": job.queued_at,
                        "scheduled_for": job.scheduled_for,
                        "error_message": job.error_message,
                        "output_destinations": job.output_destinations,
                    })
                }).collect();

                serde_json::json!({
                    "jobs": jobs,
                    "pagination": {
                        "page": list_response.meta.page,
                        "limit": list_response.meta.limit,
                        "total_count": list_response.meta.total,
                        "total_pages": list_response.meta.total_pages,
                        "has_next": list_response.meta.has_next,
                        "has_previous": list_response.meta.has_previous,
                        "next_page": if list_response.meta.has_next { Some(list_response.meta.page + 1) } else { None },
                        "previous_page": if list_response.meta.has_previous { Some(list_response.meta.page - 1) } else { None }
                    },
                    "sorting": {
                        "sort_by": sort_by,
                        "sort_order": sort_order
                    },
                    "filters": {
                        "task_id": task_id,
                        "status": status,
                        "priority": priority
                    }
                })
            }
            Err(e) => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to fetch jobs: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "{}".to_string()),
            }],
            is_error: false,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("page".to_string(), serde_json::Value::Number(serde_json::Number::from(page)));
                meta.insert("limit".to_string(), serde_json::Value::Number(serde_json::Number::from(limit)));
                meta.insert("sort_by".to_string(), serde_json::Value::String(sort_by.to_string()));
                meta.insert("sort_order".to_string(), serde_json::Value::String(sort_order.to_string()));
                meta
            },
        })
    }

    /// Execute the schedules listing tool with pagination support
    async fn list_schedules_tool(
        &self,
        context: ToolExecutionContext,
    ) -> McpResult<ToolsCallResult> {
        let args = context.arguments.unwrap_or(serde_json::json!({}));

        // Extract filters and pagination parameters
        let task_id = args.get("task_id").and_then(|v| v.as_str());
        let enabled = args.get("enabled").and_then(|v| v.as_bool());
        let ready_to_run = args.get("ready_to_run").and_then(|v| v.as_bool());
        let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let sort_by = args.get("sort_by").and_then(|v| v.as_str()).unwrap_or("name");
        let sort_order = args.get("sort_order").and_then(|v| v.as_str()).unwrap_or("asc");

        // Validate pagination parameters
        if limit > 1000 {
            return Ok(ToolsCallResult {
                content: vec![ToolContent::Text {
                    text: "Limit cannot exceed 1000 schedules per page".to_string(),
                }],
                is_error: true,
                metadata: HashMap::new(),
            });
        }

        // Check if repositories are available
        let repositories = match &self.repositories {
            Some(repos) => repos,
            None => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: "Repository factory not configured for MCP server".to_string(),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        // Convert task_id string to ApiId if provided
        let task_id_filter = task_id.map(|id| ApiId::from_string(id.to_string()));

        // Create schedule filters
        let filters = ScheduleFilters {
            task_id: task_id_filter,
            enabled,
            next_run_before: None,
            task_id_in: None,
            id_in: None,
            name_contains: None,
            name_exact: None,
            name_starts_with: None,
            name_ends_with: None,
            cron_expression_contains: None,
            cron_expression_exact: None,
            next_run_after: None,
            last_run_after: None,
            last_run_before: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            has_next_run: None,
            has_last_run: None,
            is_due: ready_to_run, // Map ready_to_run to is_due filter
            overdue: None,
        };

        // Create pagination input
        let pagination = PaginationInput {
            page: Some(page as u32),
            limit: Some(limit as u32),
            offset: Some((page * limit) as u32),
        };

        // Query the schedule repository
        let schedule_repo = repositories.schedule_repository();
        let response = match schedule_repo.find_with_filters(filters, pagination).await {
            Ok(list_response) => {
                // Convert schedules to MCP format
                let schedules: Vec<serde_json::Value> = list_response.items.into_iter().map(|schedule| {
                    serde_json::json!({
                        "id": schedule.id.to_string(),
                        "task_id": schedule.task_id.to_string(),
                        "name": schedule.name,
                        "description": schedule.description,
                        "cron_expression": schedule.cron_expression,
                        "enabled": schedule.enabled,
                        "next_run": schedule.next_run,
                        "last_run": schedule.last_run,
                        "created_at": schedule.created_at,
                        "updated_at": schedule.updated_at,
                    })
                }).collect();

                serde_json::json!({
                    "schedules": schedules,
                    "pagination": {
                        "page": list_response.meta.page,
                        "limit": list_response.meta.limit,
                        "total_count": list_response.meta.total,
                        "total_pages": list_response.meta.total_pages,
                        "has_next": list_response.meta.has_next,
                        "has_previous": list_response.meta.has_previous,
                        "next_page": if list_response.meta.has_next { Some(list_response.meta.page + 1) } else { None },
                        "previous_page": if list_response.meta.has_previous { Some(list_response.meta.page - 1) } else { None }
                    },
                    "sorting": {
                        "sort_by": sort_by,
                        "sort_order": sort_order
                    },
                    "filters": {
                        "task_id": task_id,
                        "enabled": enabled,
                        "ready_to_run": ready_to_run
                    }
                })
            }
            Err(e) => {
                return Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to fetch schedules: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                });
            }
        };

        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "{}".to_string()),
            }],
            is_error: false,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("page".to_string(), serde_json::Value::Number(serde_json::Number::from(page)));
                meta.insert("limit".to_string(), serde_json::Value::Number(serde_json::Number::from(limit)));
                meta.insert("sort_by".to_string(), serde_json::Value::String(sort_by.to_string()));
                meta.insert("sort_order".to_string(), serde_json::Value::String(sort_order.to_string()));
                meta
            },
        })
    }
}

impl Default for RatchetToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{ClientContext, ClientPermissions};

    fn create_test_context() -> SecurityContext {
        let client = ClientContext {
            id: "test-client".to_string(),
            name: "Test Client".to_string(),
            permissions: ClientPermissions::default(),
            authenticated_at: chrono::Utc::now(),
            session_id: "session-123".to_string(),
        };

        SecurityContext::new(client, crate::security::SecurityConfig::default())
    }

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = RatchetToolRegistry::new();
        assert!(!registry.tools.is_empty());

        // Check that built-in tools are registered
        assert!(registry.tools.contains_key("ratchet_execute_task"));
        assert!(registry.tools.contains_key("ratchet_get_execution_logs"));
        assert!(registry.tools.contains_key("ratchet_list_available_tasks"));
        assert!(registry
            .tools
            .contains_key("ratchet_analyze_execution_error"));
        assert!(registry.tools.contains_key("ratchet_get_execution_trace"));
        assert!(registry.tools.contains_key("ratchet_batch_execute"));
        
        // Check that pagination listing tools are registered
        assert!(registry.tools.contains_key("ratchet_list_executions"));
        assert!(registry.tools.contains_key("ratchet_list_jobs"));
        assert!(registry.tools.contains_key("ratchet_list_schedules"));
        
        // Check that task development tools are registered
        assert!(registry.tools.contains_key("ratchet_create_task"));
        assert!(registry.tools.contains_key("ratchet_validate_task"));
        assert!(registry.tools.contains_key("ratchet_debug_task_execution"));
        assert!(registry.tools.contains_key("ratchet_run_task_tests"));
        assert!(registry.tools.contains_key("ratchet_create_task_version"));
    }

    #[tokio::test]
    async fn test_list_tools() {
        let registry = RatchetToolRegistry::new();
        let context = create_test_context();

        let tools = registry.list_tools(&context).await.unwrap();
        assert!(!tools.is_empty());

        // Find the execute task tool
        let execute_tool = tools.iter().find(|t| t.name == "ratchet_execute_task");
        assert!(execute_tool.is_some());
        assert_eq!(
            execute_tool.unwrap().description,
            "Execute a Ratchet task with given input and optional progress streaming"
        );

        // Find the debugging tools
        let error_analysis_tool = tools
            .iter()
            .find(|t| t.name == "ratchet_analyze_execution_error");
        assert!(error_analysis_tool.is_some());
        assert_eq!(
            error_analysis_tool.unwrap().description,
            "Get detailed error analysis for failed execution"
        );

        let trace_tool = tools
            .iter()
            .find(|t| t.name == "ratchet_get_execution_trace");
        assert!(trace_tool.is_some());
        assert_eq!(
            trace_tool.unwrap().description,
            "Get detailed execution trace with timing and context"
        );

        // Find the batch execution tool
        let batch_tool = tools.iter().find(|t| t.name == "ratchet_batch_execute");
        assert!(batch_tool.is_some());
        assert_eq!(
            batch_tool.unwrap().description,
            "Execute multiple tasks in parallel or sequence with dependency management"
        );
        
        // Find task development tools
        let create_tool = tools.iter().find(|t| t.name == "ratchet_create_task");
        assert!(create_tool.is_some());
        assert_eq!(
            create_tool.unwrap().description,
            "Create a new task with code, schemas, and optional test cases"
        );
        
        let validate_tool = tools.iter().find(|t| t.name == "ratchet_validate_task");
        assert!(validate_tool.is_some());
        
        let debug_tool = tools.iter().find(|t| t.name == "ratchet_debug_task_execution");
        assert!(debug_tool.is_some());
    }

    #[tokio::test]
    async fn test_get_tool() {
        let registry = RatchetToolRegistry::new();
        let context = create_test_context();

        let tool = registry
            .get_tool("ratchet_execute_task", &context)
            .await
            .unwrap();
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().tool.name, "ratchet_execute_task");

        let nonexistent = registry
            .get_tool("nonexistent.tool", &context)
            .await
            .unwrap();
        assert!(nonexistent.is_none());

        // Test debugging tools
        let error_tool = registry
            .get_tool("ratchet_analyze_execution_error", &context)
            .await
            .unwrap();
        assert!(error_tool.is_some());
        assert_eq!(error_tool.unwrap().category, "debugging");

        let trace_tool = registry
            .get_tool("ratchet_get_execution_trace", &context)
            .await
            .unwrap();
        assert!(trace_tool.is_some());
        assert_eq!(trace_tool.unwrap().category, "debugging");
    }

    #[tokio::test]
    async fn test_tool_execution_without_executor() {
        let registry = RatchetToolRegistry::new();
        let context = create_test_context();

        let execution_context = ToolExecutionContext {
            security: context,
            arguments: Some(serde_json::json!({
                "task_id": "test-task",
                "input": {"key": "value"}
            })),
            request_id: Some("req-123".to_string()),
        };

        // Without a configured executor, the tool should return an error result
        let result = registry
            .execute_tool("ratchet_execute_task", execution_context)
            .await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.is_error);
        assert!(!tool_result.content.is_empty());

        // Check that the error message mentions the missing executor
        if let ToolContent::Text { text } = &tool_result.content[0] {
            assert!(text.contains("not configured"));
        }
    }

    #[test]
    fn test_mcp_tool_creation() {
        let tool = McpTool::new(
            "test.tool",
            "A test tool",
            serde_json::json!({"type": "object"}),
            "testing",
        );

        assert_eq!(tool.tool.name, "test.tool");
        assert_eq!(tool.category, "testing");
        assert!(tool.requires_auth);
        assert!(!tool.public);

        let public_tool = tool.public();
        assert!(!public_tool.requires_auth);
        assert!(public_tool.public);
    }

    #[test]
    fn test_error_classification_patterns() {
        // Test timeout error classification
        let (error_type, severity, category) =
            RatchetToolRegistry::classify_error("Operation timed out after 60 seconds", &None);
        assert_eq!(error_type, "timeout_error");
        assert_eq!(severity, "medium");
        assert_eq!(category, "transient");

        // Test permission error classification
        let (error_type, severity, category) =
            RatchetToolRegistry::classify_error("Permission denied: unauthorized access", &None);
        assert_eq!(error_type, "permission_error");
        assert_eq!(severity, "high");
        assert_eq!(category, "security");

        // Test validation error classification
        let (error_type, severity, category) =
            RatchetToolRegistry::classify_error("Validation failed: invalid input format", &None);
        assert_eq!(error_type, "validation_error");
        assert_eq!(severity, "medium");
        assert_eq!(category, "input");

        // Test resource error classification
        let (error_type, severity, category) =
            RatchetToolRegistry::classify_error("Out of memory: insufficient heap space", &None);
        assert_eq!(error_type, "resource_error");
        assert_eq!(severity, "critical");
        assert_eq!(category, "system");
    }

    #[test]
    fn test_error_suggestions_generation() {
        // Test timeout error suggestions
        let suggestions = RatchetToolRegistry::generate_error_suggestions(
            "timeout_error",
            "Request timed out",
            &None,
        );
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.contains("timeout configuration")));
        assert!(suggestions.iter().any(|s| s.contains("external services")));

        // Test network error suggestions
        let suggestions = RatchetToolRegistry::generate_error_suggestions(
            "network_error",
            "Connection refused",
            &None,
        );
        assert!(suggestions
            .iter()
            .any(|s| s.contains("network connectivity")));
        assert!(suggestions.iter().any(|s| s.contains("retry logic")));

        // Test validation error suggestions
        let suggestions = RatchetToolRegistry::generate_error_suggestions(
            "validation_error",
            "Invalid input schema",
            &None,
        );
        assert!(suggestions.iter().any(|s| s.contains("input data")));
        assert!(suggestions.iter().any(|s| s.contains("schema")));
    }

    #[test]
    fn test_next_steps_generation() {
        // Test critical severity next steps
        let steps = RatchetToolRegistry::generate_next_steps("resource_error", "critical");
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("Immediately stop")));
        assert!(steps
            .iter()
            .any(|s| s.contains("Alert system administrators")));

        // Test medium severity next steps
        let steps = RatchetToolRegistry::generate_next_steps("timeout_error", "medium");
        assert!(steps.iter().any(|s| s.contains("normal business hours")));
        assert!(steps.iter().any(|s| s.contains("Document the error")));
    }

    #[test]
    fn test_timing_calculation() {
        let mock_status = McpExecutionStatus {
            execution_id: "test-123".to_string(),
            status: "completed".to_string(),
            task_id: 1,
            input: None,
            output: None,
            error_message: None,
            error_details: None,
            queued_at: "2023-12-01T10:00:00Z".to_string(),
            started_at: Some("2023-12-01T10:00:05Z".to_string()),
            completed_at: Some("2023-12-01T10:00:15Z".to_string()),
            duration_ms: Some(10000),
            progress: None,
        };

        let timing = RatchetToolRegistry::calculate_execution_timing(&mock_status);

        assert_eq!(timing["total_duration_ms"], 10000);
        assert_eq!(timing["queue_duration_ms"], 5000); // 5 second queue time
        assert!(timing["was_queued"].as_bool().unwrap());
        assert!(timing["is_complete"].as_bool().unwrap());
    }
}
