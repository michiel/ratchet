//! Tool registry trait and implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    error::{McpError, McpResult},
    protocol::{Tool, ToolContent, ToolsCallResult},
    security::{SecurityContext},
};

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

    /// Make this tool require authentication
    pub fn require_auth(mut self) -> Self {
        self.requires_auth = true;
        self.public = false;
        self
    }

    /// Add metadata to the tool
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.tool.metadata.insert(key.into(), value);
        self
    }
    
    /// Set the tool category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
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
    
    /// Additional execution metadata
    pub metadata: HashMap<String, Value>,
}

impl ToolExecutionContext {
    /// Create a new execution context
    pub fn new(security: SecurityContext) -> Self {
        Self {
            security,
            arguments: None,
            request_id: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Set the tool arguments
    pub fn with_arguments(mut self, arguments: Value) -> Self {
        self.arguments = Some(arguments);
        self
    }
    
    /// Set the request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Tool registry trait for managing available tools
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    /// List all available tools
    async fn list_tools(&self, context: &SecurityContext) -> McpResult<Vec<Tool>>;

    /// Get a specific tool by name
    async fn get_tool(&self, name: &str, context: &SecurityContext) -> McpResult<Option<McpTool>>;

    /// Execute a tool
    async fn execute_tool(&self, name: &str, execution_context: ToolExecutionContext) -> McpResult<ToolsCallResult>;

    /// Check if a tool exists and is accessible
    async fn can_access_tool(&self, name: &str, context: &SecurityContext) -> bool;
    
    /// Get tool categories
    async fn get_categories(&self, context: &SecurityContext) -> McpResult<Vec<String>> {
        let tools = self.list_tools(context).await?;
        let mut categories = std::collections::HashSet::new();
        
        // For the base trait, we can't access the category directly from Tool
        // so we'll return a default implementation
        categories.insert("general".to_string());
        
        Ok(categories.into_iter().collect())
    }
    
    /// Search tools by name or description
    async fn search_tools(&self, query: &str, context: &SecurityContext) -> McpResult<Vec<Tool>> {
        let tools = self.list_tools(context).await?;
        let query_lower = query.to_lowercase();
        
        Ok(tools
            .into_iter()
            .filter(|tool| {
                tool.name.to_lowercase().contains(&query_lower)
                    || tool.description.to_lowercase().contains(&query_lower)
            })
            .collect())
    }
}

/// Simple in-memory tool registry implementation
#[derive(Clone)]
pub struct InMemoryToolRegistry {
    tools: HashMap<String, McpTool>,
}

impl InMemoryToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Add a tool to the registry
    pub fn register_tool(&mut self, tool: McpTool) {
        self.tools.insert(tool.tool.name.clone(), tool);
    }
    
    /// Remove a tool from the registry
    pub fn unregister_tool(&mut self, name: &str) -> Option<McpTool> {
        self.tools.remove(name)
    }
    
    /// Get all registered tools
    pub fn get_all_tools(&self) -> Vec<&McpTool> {
        self.tools.values().collect()
    }
}

impl Default for InMemoryToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolRegistry for InMemoryToolRegistry {
    async fn list_tools(&self, context: &SecurityContext) -> McpResult<Vec<Tool>> {
        let tools = self.tools
            .values()
            .filter(|tool| {
                // Filter tools based on authentication requirements
                if tool.requires_auth && context.is_anonymous() {
                    false
                } else {
                    true
                }
            })
            .map(|mcp_tool| mcp_tool.tool.clone())
            .collect();
        
        Ok(tools)
    }

    async fn get_tool(&self, name: &str, context: &SecurityContext) -> McpResult<Option<McpTool>> {
        if let Some(tool) = self.tools.get(name) {
            // Check access permissions
            if tool.requires_auth && context.is_anonymous() {
                return Err(McpError::Authorization {
                    message: "Tool requires authentication".to_string(),
                });
            }
            Ok(Some(tool.clone()))
        } else {
            Ok(None)
        }
    }

    async fn execute_tool(&self, name: &str, _execution_context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        // Default implementation returns an error - users should override this
        Err(McpError::ToolExecution {
            tool: name.to_string(),
            message: "Tool execution not implemented".to_string(),
        })
    }

    async fn can_access_tool(&self, name: &str, context: &SecurityContext) -> bool {
        if let Some(tool) = self.tools.get(name) {
            if tool.requires_auth && context.is_anonymous() {
                false
            } else {
                true
            }
        } else {
            false
        }
    }
    
    async fn get_categories(&self, context: &SecurityContext) -> McpResult<Vec<String>> {
        let mut categories = std::collections::HashSet::new();
        
        for tool in self.tools.values() {
            if !tool.requires_auth || !context.is_anonymous() {
                categories.insert(tool.category.clone());
            }
        }
        
        Ok(categories.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityContext;

    #[tokio::test]
    async fn test_in_memory_registry() {
        let mut registry = InMemoryToolRegistry::new();
        let context = SecurityContext::system();

        // Test empty registry
        let tools = registry.list_tools(&context).await.unwrap();
        assert!(tools.is_empty());

        // Add a tool
        let tool = McpTool::new(
            "test_tool",
            "A test tool",
            serde_json::json!({"type": "object"}),
            "test",
        );
        registry.register_tool(tool);

        // Test tool listing
        let tools = registry.list_tools(&context).await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");

        // Test tool retrieval
        let retrieved = registry.get_tool("test_tool", &context).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tool.name, "test_tool");

        // Test tool access
        assert!(registry.can_access_tool("test_tool", &context).await);
        assert!(!registry.can_access_tool("nonexistent", &context).await);
    }

    #[tokio::test]
    async fn test_tool_authentication() {
        let mut registry = InMemoryToolRegistry::new();
        let system_context = SecurityContext::system();
        let anon_context = SecurityContext::anonymous();

        // Add a tool that requires authentication
        let auth_tool = McpTool::new(
            "auth_tool",
            "Requires auth",
            serde_json::json!({"type": "object"}),
            "secure",
        ).require_auth();
        
        let public_tool = McpTool::new(
            "public_tool",
            "Public access",
            serde_json::json!({"type": "object"}),
            "public",
        ).public();

        registry.register_tool(auth_tool);
        registry.register_tool(public_tool);

        // System context should see both tools
        let system_tools = registry.list_tools(&system_context).await.unwrap();
        assert_eq!(system_tools.len(), 2);

        // Anonymous context should only see public tool
        let anon_tools = registry.list_tools(&anon_context).await.unwrap();
        assert_eq!(anon_tools.len(), 1);
        assert_eq!(anon_tools[0].name, "public_tool");

        // Test tool access permissions
        assert!(registry.can_access_tool("auth_tool", &system_context).await);
        assert!(!registry.can_access_tool("auth_tool", &anon_context).await);
        assert!(registry.can_access_tool("public_tool", &anon_context).await);
    }
}