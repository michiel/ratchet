//! Generic MCP server framework with trait-based architecture

pub mod config;
pub mod handler;
pub mod progress;
pub mod registry;
pub mod service;

pub use config::McpServerConfig;
pub use handler::McpHandlerState;
pub use progress::{ProgressReporter, ProgressUpdate, ProgressLevel};
pub use registry::{ToolRegistry, ToolExecutionContext, McpTool, InMemoryToolRegistry};
pub use service::McpServer;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::{
    error::{McpError, McpResult},
    protocol::{
        ClientCapabilities, InitializeParams, InitializeResult, McpCapabilities, 
        ServerCapabilities, ServerInfo, Tool, ToolsCallResult, ToolsListResult,
        BatchRequest, BatchResult, JsonRpcRequest, JsonRpcResponse
    },
    security::{SecurityContext, McpAuth},
};

/// Core trait for MCP server state management
/// 
/// This trait provides the foundation for building MCP servers with custom
/// tool registries and authentication mechanisms.
#[async_trait]
pub trait McpServerState: Send + Sync + Clone + 'static {
    /// The tool registry implementation for this server
    type ToolRegistry: ToolRegistry;
    
    /// The authentication implementation for this server
    type AuthManager: McpAuth;

    /// Get the tool registry instance
    fn tool_registry(&self) -> &Self::ToolRegistry;
    
    /// Get the authentication manager instance
    fn auth_manager(&self) -> &Self::AuthManager;
    
    /// Get server information for the initialize response
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "MCP Server".to_string(),
            version: "0.1.0".to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// Get server capabilities
    fn server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            experimental: HashMap::new(),
            logging: None,
            prompts: None,
            resources: None,
            tools: Some(crate::protocol::ToolsCapability {
                list_changed: false,
            }),
            batch: None,
        }
    }
    
    /// Handle server initialization
    async fn initialize(&self, params: InitializeParams) -> McpResult<InitializeResult> {
        // Default implementation validates protocol version and returns capabilities
        let protocol_version = crate::protocol::get_protocol_version_for_client(
            &params.protocol_version
        );
        
        Ok(InitializeResult {
            protocol_version,
            capabilities: self.server_capabilities(),
            server_info: self.server_info(),
        })
    }
    
    /// Handle custom methods not covered by the standard MCP protocol
    async fn handle_custom_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        context: &SecurityContext,
    ) -> McpResult<Option<serde_json::Value>> {
        // Default implementation returns method not found
        Err(McpError::ToolNotFound {
            name: method.to_string(),
        })
    }
}

/// Batch execution mode for handling multiple operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchExecutionMode {
    /// Execute all operations in parallel
    Parallel,
    /// Execute operations sequentially
    Sequential,
    /// Execute operations sequentially but stop on first error
    FailFast,
}

/// Batch execution context
#[derive(Debug, Clone)]
pub struct BatchContext {
    /// Execution mode for the batch
    pub mode: BatchExecutionMode,
    /// Maximum number of operations to execute in parallel
    pub max_parallel: Option<usize>,
    /// Timeout for the entire batch operation
    pub timeout: Option<std::time::Duration>,
    /// Security context for the batch
    pub security: SecurityContext,
}

/// Health check information for the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealth {
    /// Whether the server is healthy
    pub healthy: bool,
    /// Server status message
    pub status: String,
    /// Number of active connections
    pub active_connections: usize,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Additional server metrics
    pub metrics: HashMap<String, serde_json::Value>,
}

impl Default for ServerHealth {
    fn default() -> Self {
        Self {
            healthy: true,
            status: "Server is running".to_string(),
            active_connections: 0,
            uptime_seconds: 0,
            metrics: HashMap::new(),
        }
    }
}