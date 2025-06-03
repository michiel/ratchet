//! Tool registry and definitions for MCP server

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{McpError, McpResult};
use crate::protocol::{Tool, ToolsCallResult, ToolContent};
use crate::security::SecurityContext;

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

/// Ratchet-specific tool registry implementation
pub struct RatchetToolRegistry {
    /// Available tools
    tools: HashMap<String, McpTool>,
}

impl RatchetToolRegistry {
    /// Create a new Ratchet tool registry
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
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
        // Get the tool
        let tool = self.get_tool(name, &execution_context.security).await?
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
        // This is a placeholder - actual implementation would integrate with Ratchet's task execution
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Task execution is not yet implemented in this MCP server foundation. This would integrate with Ratchet's task execution engine.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
        })
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
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Log retrieval is not yet implemented in this MCP server foundation.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
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
        let _args = context.arguments.unwrap_or_default();
        
        Ok(ToolsCallResult {
            content: vec![ToolContent::Text {
                text: "Task discovery is not yet implemented in this MCP server foundation.".to_string(),
            }],
            is_error: false,
            metadata: HashMap::new(),
        })
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
    async fn test_tool_execution() {
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
        
        let result = registry.execute_tool("ratchet.execute_task", execution_context).await;
        assert!(result.is_ok());
        
        let tool_result = result.unwrap();
        assert!(!tool_result.is_error);
        assert!(!tool_result.content.is_empty());
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