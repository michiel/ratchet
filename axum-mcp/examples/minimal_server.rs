//! Minimal MCP server example
//!
//! This example demonstrates how to create a basic MCP server with Axum integration.
//! 
//! Run with: cargo run --example minimal_server

use axum_mcp::{
    prelude::*,
    axum_integration::mcp_routes,
    server::{config::McpServerConfig, service::McpServer},
};
use std::collections::HashMap;
use tokio::net::TcpListener;

// Define a simple server state
#[derive(Clone)]
struct SimpleServerState {
    tools: InMemoryToolRegistry,
    auth: SimpleAuth,
}

// Simple authentication that allows everything
#[derive(Clone)]
struct SimpleAuth;

#[async_trait]
impl McpAuth for SimpleAuth {
    async fn authenticate(&self, _client_info: &ClientContext) -> McpResult<SecurityContext> {
        // For this example, all clients get full access
        Ok(SecurityContext::system())
    }

    async fn authorize(&self, _context: &SecurityContext, _resource: &str, _action: &str) -> bool {
        // Allow all operations
        true
    }
}

impl McpServerState for SimpleServerState {
    type ToolRegistry = InMemoryToolRegistry;
    type AuthManager = SimpleAuth;

    fn tool_registry(&self) -> &Self::ToolRegistry {
        &self.tools
    }

    fn auth_manager(&self) -> &Self::AuthManager {
        &self.auth
    }

    fn server_info(&self) -> axum_mcp::protocol::ServerInfo {
        axum_mcp::protocol::ServerInfo {
            name: "Minimal MCP Server".to_string(),
            version: "1.0.0".to_string(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

// Custom tool registry with a simple echo tool
struct EchoToolRegistry;

#[async_trait]
impl ToolRegistry for EchoToolRegistry {
    async fn list_tools(&self, _context: &SecurityContext) -> McpResult<Vec<Tool>> {
        Ok(vec![Tool {
            name: "echo".to_string(),
            description: "Echo back the input message".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"]
            }),
            metadata: HashMap::new(),
        }])
    }

    async fn get_tool(&self, name: &str, _context: &SecurityContext) -> McpResult<Option<McpTool>> {
        if name == "echo" {
            Ok(Some(McpTool::new(
                "echo",
                "Echo back the input message",
                json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    },
                    "required": ["message"]
                }),
                "utility"
            ).public())) // Make it publicly accessible
        } else {
            Ok(None)
        }
    }

    async fn execute_tool(&self, name: &str, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
        if name == "echo" {
            if let Some(args) = context.arguments {
                if let Some(message) = args.get("message").and_then(|v| v.as_str()) {
                    return Ok(ToolsCallResult {
                        content: vec![axum_mcp::protocol::ToolContent::Text {
                            text: format!("Echo: {}", message)
                        }],
                        is_error: false,
                        metadata: std::collections::HashMap::new(),
                    });
                }
            }
            
            Err(McpError::ToolExecution {
                tool: name.to_string(),
                message: "Missing or invalid 'message' parameter".to_string(),
            })
        } else {
            Err(McpError::ToolNotFound {
                name: name.to_string(),
            })
        }
    }

    async fn can_access_tool(&self, name: &str, _context: &SecurityContext) -> bool {
        name == "echo"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create server configuration
    let config = McpServerConfig::sse_with_host(3000, "0.0.0.0")
        .with_batch(50)
        .with_metadata("example", json!({"type": "minimal"}));

    // Create tools registry
    let mut tools = InMemoryToolRegistry::new();
    
    // Register the echo tool manually
    let echo_tool = McpTool::new(
        "echo",
        "Echo back the input message",
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        }),
        "utility"
    ).public();
    
    tools.register_tool(echo_tool);

    // Create server state
    let state = SimpleServerState {
        tools,
        auth: SimpleAuth,
    };

    // Create MCP server
    let mcp_server = McpServer::new(config, state);

    // Create Axum app with MCP routes
    let app = axum::Router::new()
        .merge(mcp_routes())
        .with_state(mcp_server);

    // Start the server
    println!("Starting MCP server on http://0.0.0.0:3000");
    println!("Endpoints:");
    println!("  GET  /mcp     - Server information and health");
    println!("  POST /mcp     - JSON-RPC requests");
    println!("  GET  /mcp/sse - Server-Sent Events stream");
    println!();
    println!("Try testing with:");
    println!("  curl http://localhost:3000/mcp");
    println!("  curl -X POST http://localhost:3000/mcp \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\",\"id\":1}}'");

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}