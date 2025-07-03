# axum-mcp

[![Crates.io](https://img.shields.io/crates/v/axum-mcp.svg)](https://crates.io/crates/axum-mcp)
[![Documentation](https://docs.rs/axum-mcp/badge.svg)](https://docs.rs/axum-mcp)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A comprehensive Model Context Protocol (MCP) implementation for Rust with Axum integration.

## Overview

`axum-mcp` is a production-ready MCP server and client library that enables seamless communication between Large Language Models (LLMs) and Rust applications. It provides a trait-based architecture for building custom MCP servers with support for multiple transport protocols.

### Key Features

- 🚀 **Production Ready** - Session management, authentication, monitoring, and error recovery
- 🔌 **Multiple Transports** - stdio, Server-Sent Events (SSE), and StreamableHTTP for Claude Desktop
- 🛡️ **Security First** - Built-in authentication, authorization, and rate limiting
- ⚡ **High Performance** - Connection pooling, message batching, and streaming support
- 🎯 **Claude Compatible** - Full support for Claude Desktop's StreamableHTTP transport
- 🧩 **Trait-Based** - Flexible architecture enabling custom tool registries and authentication
- 📊 **Observability** - Comprehensive logging, metrics, and health monitoring

## Architecture

```text
┌─────────────────────┐
│   LLM/AI Agent      │
│  (Claude, GPT-4)    │
│  ┌───────────────┐  │
│  │ MCP Client    │  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
    ┌──────┴──────┐
    │   Transport │
    │ (stdio/SSE) │
    └──────┬──────┘
           │
┌──────────▼──────────┐
│  axum-mcp Server    │
│  ┌───────────────┐  │
│  │  Tool Registry│  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
    ┌──────┴──────────┐
    │ Your App Logic  │
    │ - Custom Tools  │
    │ - Business Logic│
    │ - Data Access   │
    └─────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
axum-mcp = "0.1"
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
serde_json = "1.0"
```

### Basic Server Example

```rust
use axum_mcp::{
    prelude::*,
    server::{McpServerState, ToolRegistry},
    security::SecurityContext,
    axum::mcp_routes,
};
use async_trait::async_trait;
use std::collections::HashMap;
use axum::Router;

// Define your custom tool registry
#[derive(Clone)]
struct MyToolRegistry {
    tools: HashMap<String, Tool>,
}

#[async_trait]
impl ToolRegistry for MyToolRegistry {
    async fn list_tools(&self, _context: &SecurityContext) -> McpResult<Vec<Tool>> {
        Ok(self.tools.values().cloned().collect())
    }
    
    async fn execute_tool(
        &self, 
        name: &str, 
        context: ToolExecutionContext
    ) -> McpResult<ToolsCallResult> {
        match name {
            "echo" => {
                let message = context.arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Hello, World!");
                    
                Ok(ToolsCallResult::text(format!("Echo: {}", message)))
            }
            _ => Err(McpError::method_not_found(format!("Unknown tool: {}", name)))
        }
    }
}

// Define your server state
#[derive(Clone)]
struct MyServerState {
    tools: MyToolRegistry,
    auth: NoOpAuth,
}

impl McpServerState for MyServerState {
    type ToolRegistry = MyToolRegistry;
    type AuthManager = NoOpAuth;
    
    fn tool_registry(&self) -> &Self::ToolRegistry { &self.tools }
    fn auth_manager(&self) -> &Self::AuthManager { &self.auth }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your tool registry
    let mut tools = HashMap::new();
    tools.insert("echo".to_string(), Tool {
        name: "echo".to_string(),
        description: "Echo back a message".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo back"
                }
            }
        }),
    });
    
    let state = MyServerState {
        tools: MyToolRegistry { tools },
        auth: NoOpAuth,
    };
    
    // Create Axum app with MCP routes
    let app = Router::new()
        .merge(mcp_routes())
        .with_state(state);
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("MCP server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}
```

### Claude Desktop Integration

To integrate with Claude Desktop, add this configuration to Claude's settings:

```json
{
  "mcpServers": {
    "my-rust-server": {
      "command": "curl",
      "args": [
        "-X", "POST",
        "http://localhost:3000/mcp",
        "-H", "Content-Type: application/json",
        "-d", "@-"
      ],
      "transport": "streamable-http"
    }
  }
}
```

## Core Concepts

### Tool Registry

The `ToolRegistry` trait defines how your application exposes tools to MCP clients:

```rust
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    async fn list_tools(&self, context: &SecurityContext) -> McpResult<Vec<Tool>>;
    async fn execute_tool(&self, name: &str, context: ToolExecutionContext) -> McpResult<ToolsCallResult>;
}
```

### Server State

The `McpServerState` trait connects your tool registry and authentication:

```rust
pub trait McpServerState: Send + Sync + Clone + 'static {
    type ToolRegistry: ToolRegistry;
    type AuthManager: McpAuth;
    
    fn tool_registry(&self) -> &Self::ToolRegistry;
    fn auth_manager(&self) -> &Self::AuthManager;
}
```

### Authentication

Implement custom authentication with the `McpAuth` trait:

```rust
#[async_trait]
pub trait McpAuth: Send + Sync {
    async fn authenticate(&self, context: &ClientContext) -> McpResult<SecurityContext>;
    async fn authorize(&self, context: &SecurityContext, resource: &str, action: &str) -> bool;
}
```

## Transport Types

### Standard I/O Transport

For local processes and command-line tools:

```rust
use axum_mcp::transport::{StdioTransport, McpTransport};

let transport = StdioTransport::new();
// Use with stdio-based MCP clients
```

### Server-Sent Events (SSE)

For web-based real-time communication:

```rust
// SSE endpoints are automatically included in mcp_routes()
let app = Router::new()
    .merge(mcp_routes())
    .with_state(state);
```

### StreamableHTTP

For Claude Desktop compatibility:

```rust
// StreamableHTTP is the default transport in mcp_routes()
// Supports both request/response and streaming modes
```

## Advanced Features

### Custom Authentication

```rust
#[derive(Clone)]
struct ApiKeyAuth {
    valid_keys: HashSet<String>,
}

#[async_trait]
impl McpAuth for ApiKeyAuth {
    async fn authenticate(&self, context: &ClientContext) -> McpResult<SecurityContext> {
        let api_key = context.headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| McpError::unauthorized("Missing API key"))?;
            
        if self.valid_keys.contains(api_key) {
            Ok(SecurityContext::authenticated(api_key.to_string()))
        } else {
            Err(McpError::unauthorized("Invalid API key"))
        }
    }
    
    async fn authorize(&self, _context: &SecurityContext, _resource: &str, _action: &str) -> bool {
        true // Implement your authorization logic
    }
}
```

### Progress Reporting

For long-running operations:

```rust
async fn execute_tool(&self, name: &str, context: ToolExecutionContext) -> McpResult<ToolsCallResult> {
    match name {
        "long_task" => {
            let progress = context.progress_reporter();
            
            progress.report(0.0, "Starting task...").await;
            // Do work...
            progress.report(0.5, "Half way done...").await;
            // More work...
            progress.report(1.0, "Complete!").await;
            
            Ok(ToolsCallResult::text("Task completed"))
        }
        _ => Err(McpError::method_not_found(format!("Unknown tool: {}", name)))
    }
}
```

### Rate Limiting

```rust
use axum_mcp::security::RateLimiter;

let rate_limiter = RateLimiter::new(100, Duration::from_secs(60)); // 100 requests per minute
```

## Features

Enable specific features in your `Cargo.toml`:

```toml
[dependencies]
axum-mcp = { version = "0.1", features = ["server", "client", "transport-sse"] }
```

Available features:
- `server` - MCP server implementation (default)
- `client` - MCP client implementation
- `transport-stdio` - Standard I/O transport (default)
- `transport-sse` - Server-Sent Events transport (default)
- `transport-streamable-http` - StreamableHTTP transport for Claude Desktop (default)

## Examples

The `examples/` directory contains comprehensive examples:

- [`minimal_server.rs`](examples/minimal_server.rs) - Basic MCP server
- More examples coming soon!

## Testing

Run the test suite:

```bash
cargo test
```

Run examples:

```bash
cargo run --example minimal_server
```

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

This crate was extracted from the [Ratchet](https://github.com/ratchet-org/ratchet) project and represents a comprehensive, production-ready MCP implementation for the Rust ecosystem.