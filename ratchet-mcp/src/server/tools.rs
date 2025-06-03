//! Tool registry and definitions for MCP server

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{McpError, McpResult};
use crate::protocol::{Tool, ToolsCallResult, ToolContent};
use crate::security::SecurityContext;

// Import Ratchet's execution types
use ratchet_lib::logging::logger::StructuredLogger;

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

/// Task executor trait for MCP server
#[async_trait]
pub trait McpTaskExecutor: Send + Sync {
    /// Execute a task
    async fn execute_task(&self, task_path: &str, input: Value) -> Result<Value, String>;
    
    /// List available tasks
    async fn list_tasks(&self, filter: Option<&str>) -> Result<Vec<McpTaskInfo>, String>;
    
    /// Get execution logs
    async fn get_execution_logs(&self, execution_id: &str, level: &str, limit: usize) -> Result<String, String>;
}

/// Ratchet-specific tool registry implementation
pub struct RatchetToolRegistry {
    /// Available tools
    tools: HashMap<String, McpTool>,
    
    /// Task executor for MCP operations
    task_executor: Option<Arc<dyn McpTaskExecutor>>,
    
    /// Logger for structured logging
    logger: Option<Arc<dyn StructuredLogger>>,
}

impl RatchetToolRegistry {
    /// Create a new Ratchet tool registry
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            task_executor: None,
            logger: None,
        };
        
        // Register built-in Ratchet tools
        registry.register_builtin_tools();
        
        registry
    }
    
    /// Register all built-in Ratchet tools
    fn register_builtin_tools(&mut self) {
        // Task execution tool
        let execute_task_tool = McpTool::new(
            "ratchet.execute_task",
            "Execute a Ratchet task with given input",
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
                    }
                },
                "required": ["task_id", "input"]
            }),
            "execution",
        );
        self.tools.insert("ratchet.execute_task".to_string(), execute_task_tool);
        
        // Execution status tool
        let status_tool = McpTool::new(
            "ratchet.get_execution_status",
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
        self.tools.insert("ratchet.get_execution_status".to_string(), status_tool);
        
        // Logs retrieval tool
        let logs_tool = McpTool::new(
            "ratchet.get_execution_logs",
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
        self.tools.insert("ratchet.get_execution_logs".to_string(), logs_tool);
        
        // Trace retrieval tool
        let trace_tool = McpTool::new(
            "ratchet.get_execution_trace",
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
        self.tools.insert("ratchet.get_execution_trace".to_string(), trace_tool);
        
        // Task discovery tool
        let list_tasks_tool = McpTool::new(
            "ratchet.list_available_tasks",
            "List all available tasks with their schemas",
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
                    }
                }
            }),
            "discovery",
        );
        self.tools.insert("ratchet.list_available_tasks".to_string(), list_tasks_tool);
        
        // Error analysis tool
        let analyze_error_tool = McpTool::new(
            "ratchet.analyze_execution_error",
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
        self.tools.insert("ratchet.analyze_execution_error".to_string(), analyze_error_tool);
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
        let _tool = self.get_tool(name, &execution_context.security).await?
            .ok_or_else(|| McpError::ToolNotFound {
                tool_name: name.to_string(),
            })?;
        
        // Execute the tool based on its name
        match name {
            "ratchet.execute_task" => {
                self.execute_task_tool(execution_context).await
            }
            "ratchet.get_execution_status" => {
                self.get_execution_status_tool(execution_context).await
            }
            "ratchet.get_execution_logs" => {
                self.get_execution_logs_tool(execution_context).await
            }
            "ratchet.get_execution_trace" => {
                self.get_execution_trace_tool(execution_context).await
            }
            "ratchet.list_available_tasks" => {
                self.list_available_tasks_tool(execution_context).await
            }
            "ratchet.analyze_execution_error" => {
                self.analyze_execution_error_tool(execution_context).await
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
            method: "ratchet.execute_task".to_string(),
            details: "Missing arguments".to_string(),
        })?;
        
        // Parse task ID and input
        let task_id = args.get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet.execute_task".to_string(),
                details: "Missing or invalid task_id".to_string(),
            })?;
            
        let input = args.get("input")
            .cloned()
            .unwrap_or(serde_json::json!({}));
            
        let trace_enabled = args.get("trace")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
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
                        meta.insert("task_id".to_string(), serde_json::Value::String(task_id.to_string()));
                        meta.insert("error_type".to_string(), serde_json::Value::String("configuration_error".to_string()));
                        meta
                    },
                });
            }
        };
        
        // Execute the task
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
                        meta.insert("task_id".to_string(), serde_json::Value::String(task_id.to_string()));
                        if trace_enabled {
                            meta.insert("trace_enabled".to_string(), serde_json::Value::Bool(true));
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
                        meta.insert("task_id".to_string(), serde_json::Value::String(task_id.to_string()));
                        meta.insert("error_type".to_string(), serde_json::Value::String("execution_error".to_string()));
                        meta
                    },
                })
            }
        }
    }
    
    /// Execute the execution status tool
    async fn get_execution_status_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Execution status monitoring is not yet implemented in this MCP server foundation.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
        })
    }
    
    /// Execute the logs retrieval tool
    async fn get_execution_logs_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
            method: "ratchet.get_execution_logs".to_string(),
            details: "Missing arguments".to_string(),
        })?;
        
        // Parse execution ID
        let execution_id = args.get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams {
                method: "ratchet.get_execution_logs".to_string(),
                details: "Missing or invalid execution_id".to_string(),
            })?;
            
        let level = args.get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");
            
        let limit = args.get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as usize;
            
        let format = args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");
        
        // For now, return a placeholder since we need to implement log retrieval
        // In a full implementation, this would query the logging system
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: format!(
                    "Log retrieval for execution {} (level: {}, limit: {}, format: {}) - Integration pending",
                    execution_id, level, limit, format
                ),
            }],
            is_error: false,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("execution_id".to_string(), serde_json::Value::String(execution_id.to_string()));
                meta.insert("level".to_string(), serde_json::Value::String(level.to_string()));
                meta.insert("limit".to_string(), serde_json::Value::Number(serde_json::Number::from(limit)));
                meta
            },
        })
    }
    
    /// Execute the trace retrieval tool
    async fn get_execution_trace_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Execution tracing is not yet implemented in this MCP server foundation.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
        })
    }
    
    /// Execute the task listing tool
    async fn list_available_tasks_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        let args = context.arguments.unwrap_or(serde_json::json!({}));
        
        let filter = args.get("filter")
            .and_then(|v| v.as_str());
            
        let include_schemas = args.get("include_schemas")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let category = args.get("category")
            .and_then(|v| v.as_str());
        
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
                    metadata: HashMap::new(),
                });
            }
        };
        
        // Query tasks
        match executor.list_tasks(filter).await {
            Ok(tasks) => {
                let mut task_list = Vec::new();
                
                for task in tasks {
                    // Apply category filter if provided
                    if let Some(cat) = category {
                        if !task.tags.contains(&cat.to_string()) {
                            continue;
                        }
                    }
                    
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
                
                Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&task_list)
                            .unwrap_or_else(|_| "[]".to_string()),
                    }],
                    is_error: false,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("task_count".to_string(), serde_json::Value::Number(serde_json::Number::from(task_list.len())));
                        if let Some(f) = filter {
                            meta.insert("filter".to_string(), serde_json::Value::String(f.to_string()));
                        }
                        meta
                    },
                })
            }
            Err(e) => {
                Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to list tasks: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                })
            }
        }
    }
    
    /// Execute the error analysis tool
    async fn analyze_execution_error_tool(&self, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Error analysis is not yet implemented in this MCP server foundation.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
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
        assert!(registry.tools.contains_key("ratchet.execute_task"));
        assert!(registry.tools.contains_key("ratchet.get_execution_logs"));
        assert!(registry.tools.contains_key("ratchet.list_available_tasks"));
    }

    #[tokio::test]
    async fn test_list_tools() {
        let registry = RatchetToolRegistry::new();
        let context = create_test_context();
        
        let tools = registry.list_tools(&context).await.unwrap();
        assert!(!tools.is_empty());
        
        // Find the execute task tool
        let execute_tool = tools.iter().find(|t| t.name == "ratchet.execute_task");
        assert!(execute_tool.is_some());
        assert_eq!(execute_tool.unwrap().description, "Execute a Ratchet task with given input");
    }

    #[tokio::test]
    async fn test_get_tool() {
        let registry = RatchetToolRegistry::new();
        let context = create_test_context();
        
        let tool = registry.get_tool("ratchet.execute_task", &context).await.unwrap();
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().tool.name, "ratchet.execute_task");
        
        let nonexistent = registry.get_tool("nonexistent.tool", &context).await.unwrap();
        assert!(nonexistent.is_none());
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
        let result = registry.execute_tool("ratchet.execute_task", execution_context).await;
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
}