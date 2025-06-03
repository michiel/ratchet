//! # Ratchet MCP (Model Context Protocol) Implementation
//!
//! This crate provides both MCP client and server implementations for Ratchet,
//! enabling bidirectional communication between Ratchet and Large Language Models (LLMs).
//!
//! ## Features
//!
//! - **MCP Server**: Expose Ratchet capabilities as MCP tools for LLM consumption
//! - **MCP Client**: Enable Ratchet tasks to call LLM services via MCP
//! - **Transport Layer**: Support for stdio and Server-Sent Events (SSE) transports
//! - **Security**: Authentication, authorization, and rate limiting
//! - **Performance**: Connection pooling, message batching, and streaming support
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────┐
//! │   LLM/AI Agent      │
//! │  (Claude, GPT-4)    │ 
//! │  ┌───────────────┐  │
//! │  │ MCP Client    │  │
//! │  └───────┬───────┘  │
//! └──────────┼──────────┘
//!            │
//!     ┌──────┴──────┐
//!     │   Transport │
//!     │ (stdio/SSE) │
//!     └──────┬──────┘
//!            │
//! ┌──────────▼──────────┐
//! │  Ratchet MCP Server │
//! │  ┌───────────────┐  │
//! │  │  Tool Registry│  │
//! │  └───────┬───────┘  │
//! └──────────┼──────────┘
//!            │
//!     ┌──────┴──────────┐
//!     │ Ratchet Core    │
//!     │ - Task Execution│
//!     │ - Logging       │
//!     │ - Tracing       │
//!     └─────────────────┘
//! ```

pub mod protocol;
pub mod transport;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub mod client;

pub mod security;
pub mod error;

// Re-export commonly used types
pub use error::{McpError, McpResult};
pub use protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    McpMessage, McpMethod, McpCapabilities,
};

#[cfg(feature = "server")]
pub use server::{McpServer, McpServerConfig, McpTool, ToolRegistry};

#[cfg(feature = "client")]
pub use client::{McpClient, McpClientConfig, ServerConnection};

pub use transport::{McpTransport, TransportType};
pub use security::{McpAuth, McpAuthManager, ClientPermissions};

/// MCP protocol version supported by this implementation
pub const MCP_VERSION: &str = "1.0.0";

/// Default timeout for MCP operations
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum message size for MCP operations (in bytes)
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB