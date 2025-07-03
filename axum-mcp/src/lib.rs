//! # Axum-MCP: Generic MCP Implementation with Axum Integration
//!
//! This crate provides a comprehensive implementation of the Model Context Protocol (MCP)
//! with Axum web framework integration. It offers both client and server capabilities,
//! multiple transport options, and a flexible architecture for building MCP-enabled applications.
//!
//! ## Features
//!
//! - **Complete MCP Protocol Support** - JSON-RPC 2.0 with all MCP message types
//! - **Multiple Transports** - stdio, Server-Sent Events (SSE), and StreamableHTTP
//! - **Claude Desktop Compatibility** - StreamableHTTP transport for Claude integration
//! - **Trait-Based Architecture** - Flexible, extensible design with clean abstractions
//! - **Production Ready** - Session management, authentication, progress reporting
//! - **Axum Integration** - First-class Axum support with clean HTTP handlers
//!
//! ## Quick Start
//!
//! ### Basic MCP Server
//!
//! ```rust,no_run
//! use axum_mcp::{
//!     server::{McpServer, McpServerConfig, McpServerState, InMemoryToolRegistry},
//!     security::{McpAuth, SecurityContext, ClientContext},
//!     axum_integration::mcp_routes,
//!     error::McpResult,
//! };
//! use async_trait::async_trait;
//!
//! // Implement your server state
//! #[derive(Clone)]
//! struct MyServerState {
//!     tools: InMemoryToolRegistry,
//!     auth: MyAuth,
//! }
//!
//! #[derive(Clone)]
//! struct MyAuth;
//!
//! #[async_trait]
//! impl McpAuth for MyAuth {
//!     async fn authenticate(&self, _client: &ClientContext) -> McpResult<SecurityContext> {
//!         Ok(SecurityContext::system())
//!     }
//!
//!     async fn authorize(&self, _context: &SecurityContext, _resource: &str, _action: &str) -> bool {
//!         true
//!     }
//! }
//!
//! impl McpServerState for MyServerState {
//!     type ToolRegistry = InMemoryToolRegistry;
//!     type AuthManager = MyAuth;
//!
//!     fn tool_registry(&self) -> &Self::ToolRegistry { &self.tools }
//!     fn auth_manager(&self) -> &Self::AuthManager { &self.auth }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = McpServerConfig::default();
//!     let state = MyServerState {
//!         tools: InMemoryToolRegistry::new(),
//!         auth: MyAuth,
//!     };
//!     
//!     let server = McpServer::new(config, state);
//!     
//!     let app = axum::Router::new()
//!         .merge(mcp_routes())
//!         .with_state(server);
//!
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ### Custom Tool Implementation
//!
//! ```rust,no_run
//! use axum_mcp::{
//!     server::registry::{ToolRegistry, ToolExecutionContext, McpTool},
//!     protocol::{Tool, ToolsCallResult, ToolContent},
//!     security::SecurityContext,
//!     error::{McpResult, McpError},
//! };
//! use async_trait::async_trait;
//! use serde_json::json;
//!
//! struct MyToolRegistry;
//!
//! #[async_trait]
//! impl ToolRegistry for MyToolRegistry {
//!     async fn list_tools(&self, _context: &SecurityContext) -> McpResult<Vec<Tool>> {
//!         Ok(vec![Tool {
//!             name: "echo".to_string(),
//!             description: "Echo the input message".to_string(),
//!             input_schema: json!({
//!                 "type": "object",
//!                 "properties": {
//!                     "message": {"type": "string"}
//!                 },
//!                 "required": ["message"]
//!             }),
//!             metadata: std::collections::HashMap::new(),
//!         }])
//!     }
//!
//!     async fn get_tool(&self, name: &str, _context: &SecurityContext) -> McpResult<Option<McpTool>> {
//!         if name == "echo" {
//!             Ok(Some(McpTool::new(
//!                 "echo",
//!                 "Echo the input message",
//!                 json!({"type": "object", "properties": {"message": {"type": "string"}}}),
//!                 "utility"
//!             )))
//!         } else {
//!             Ok(None)
//!         }
//!     }
//!
//!     async fn execute_tool(&self, name: &str, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
//!         if name == "echo" {
//!             if let Some(args) = context.arguments {
//!                 if let Some(message) = args.get("message") {
//!                     return Ok(ToolsCallResult {
//!                         content: vec![ToolContent::Text {
//!                             text: format!("Echo: {}", message.as_str().unwrap_or(""))
//!                         }],
//!                         is_error: false,
//!                     });
//!                 }
//!             }
//!         }
//!         
//!         Err(McpError::ToolExecution {
//!             tool: name.to_string(),
//!             message: "Invalid tool or arguments".to_string(),
//!         })
//!     }
//!
//!     async fn can_access_tool(&self, name: &str, _context: &SecurityContext) -> bool {
//!         name == "echo"
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`protocol`] - Complete MCP protocol implementation with JSON-RPC 2.0
//! - [`transport`] - Multiple transport implementations (stdio, SSE, StreamableHTTP)
//! - [`server`] - Server framework with trait-based architecture
//! - [`security`] - Authentication and authorization framework
//! - [`error`] - Comprehensive error types and handling
//! - [`axum_integration`] - Axum-specific HTTP handlers and utilities
//!
//! ## Transports
//!
//! ### Server-Sent Events (SSE)
//! Traditional HTTP with Server-Sent Events for streaming responses.
//!
//! ### StreamableHTTP
//! Combines HTTP POST for requests with SSE for responses, specifically designed
//! for Claude Desktop compatibility with session management and resumability.
//!
//! ### stdio
//! Standard input/output transport for local process communication.
//!
//! ## Claude Desktop Integration
//!
//! The crate includes specific support for Claude Desktop through the StreamableHTTP
//! transport, which provides:
//!
//! - Session-based communication with automatic cleanup
//! - Event storage for session resumability  
//! - Health monitoring and connection tracking
//! - Proper JSON-RPC 2.0 protocol implementation
//!
//! ## Security
//!
//! The security framework provides:
//!
//! - Authentication through the [`McpAuth`](security::McpAuth) trait
//! - Authorization with resource and action-based permissions
//! - Security contexts for request tracking
//! - Audit logging capabilities
//! - Rate limiting support

pub mod error;
pub mod protocol;
pub mod security;
pub mod server;
pub mod transport;

// Re-export commonly used types
pub use error::{McpError, McpResult};

// Re-export protocol types
pub use protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    McpMessage, McpMethod, Tool, ToolsCallResult, ToolContent,
    InitializeParams, InitializeResult, BatchRequest, BatchResult,
    MCP_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSIONS,
};

// Re-export server framework
pub use server::{
    McpServerConfig, McpServerState,
    ToolRegistry, ToolExecutionContext, McpTool,
    ProgressReporter, ProgressUpdate, ProgressLevel,
    InMemoryToolRegistry,
};

// Re-export transport types
pub use transport::{
    McpTransport, TransportType, TransportFactory,
    StreamableHttpTransport, SseTransport, StdioTransport,
    SessionManager, EventStore, InMemoryEventStore, McpEvent,
    TransportHealth,
};

// Re-export security framework
pub use security::{
    McpAuth, SecurityContext, ClientContext, ClientPermissions,
};

/// Axum integration module
#[cfg(feature = "handlers")]
pub mod axum_integration {
    //! Axum-specific HTTP handlers and utilities
    
    pub use crate::server::handler::{
        McpHandlerState, mcp_get_handler, mcp_post_handler, 
        mcp_sse_handler, mcp_delete_handler, mcp_routes,
        McpQueryParams, McpEndpointInfo,
    };
}

// Convenience re-exports for common use cases
pub mod prelude {
    //! Commonly used types and traits
    
    pub use crate::{
        error::{McpError, McpResult},
        protocol::{JsonRpcRequest, JsonRpcResponse, Tool, ToolsCallResult},
        security::{McpAuth, SecurityContext, ClientContext},
        server::{
            McpServerConfig, McpServerState,
            ToolRegistry, ToolExecutionContext, McpTool,
            InMemoryToolRegistry,
        },
        transport::{McpTransport, TransportType, SessionManager},
    };
    
    pub use async_trait::async_trait;
    pub use serde_json::{json, Value};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_crate_exports() {
        // Test that all major types are accessible
        let _error: McpError = McpError::Internal { message: "test".to_string() };
        let _protocol_version = MCP_PROTOCOL_VERSION;
        let _supported_versions = SUPPORTED_PROTOCOL_VERSIONS;
    }

    #[tokio::test]
    async fn test_basic_server_creation() {
        #[derive(Clone)]
        struct TestState {
            tools: InMemoryToolRegistry,
            auth: TestAuth,
        }

        #[derive(Clone)]
        struct TestAuth;

        #[async_trait]
        impl McpAuth for TestAuth {
            async fn authenticate(&self, _client: &ClientContext) -> McpResult<SecurityContext> {
                Ok(SecurityContext::system())
            }

            async fn authorize(&self, _context: &SecurityContext, _resource: &str, _action: &str) -> bool {
                true
            }
        }

        impl McpServerState for TestState {
            type ToolRegistry = InMemoryToolRegistry;
            type AuthManager = TestAuth;

            fn tool_registry(&self) -> &Self::ToolRegistry {
                &self.tools
            }

            fn auth_manager(&self) -> &Self::AuthManager {
                &self.auth
            }
        }

        let config = McpServerConfig::default();
        let state = TestState {
            tools: InMemoryToolRegistry::new(),
            auth: TestAuth,
        };

        let _config = config;
        let _state = state;
        // Basic trait implementation should succeed
    }
}