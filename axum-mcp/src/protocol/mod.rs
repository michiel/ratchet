//! MCP protocol implementation including JSON-RPC 2.0 and MCP-specific message types

pub mod capabilities;
pub mod jsonrpc;
pub mod messages;

pub use capabilities::{ClientCapabilities, McpCapabilities, ServerCapabilities, ToolsCapability};
pub use jsonrpc::{JsonRpcError, JsonRpcErrorCode, JsonRpcRequest, JsonRpcResponse};
pub use messages::{
    BatchCapability, BatchExecutionMode, BatchItemResult, BatchParams, BatchProgressNotification, BatchRequest,
    BatchResult, BatchStats, ClientInfo, InitializeParams, InitializeResult, McpMessage, McpMethod, McpNotification,
    McpRequest, McpResponse, ResourcesListParams, ResourcesListResult, ResourcesReadParams, ResourcesReadResult,
    ServerInfo, Tool, ToolContent, ToolsCallParams, ToolsCallResult, ToolsListParams, ToolsListResult,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// MCP protocol version (server default)
pub const MCP_PROTOCOL_VERSION: &str = "0.1.0";

/// Supported MCP protocol versions
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    "2024-11-05", // Claude Code primary version
    "2025-03-26", // Claude Code latest version
    "0.1.0",      // MCP standard version
    "1.0.0",      // Future version compatibility
];

/// Generate a new request ID
pub fn generate_request_id() -> Value {
    Value::String(Uuid::new_v4().to_string())
}

/// Validate MCP protocol version
pub fn validate_protocol_version(version: &str) -> bool {
    SUPPORTED_PROTOCOL_VERSIONS.contains(&version)
}

/// Get the best supported protocol version for negotiation
pub fn get_protocol_version_for_client(client_version: &str) -> String {
    if SUPPORTED_PROTOCOL_VERSIONS.contains(&client_version) {
        client_version.to_string()
    } else {
        // For Claude compatibility, prefer Claude's version format if possible
        if client_version.starts_with("2024") || client_version.starts_with("2025") {
            "2024-11-05".to_string()
        } else {
            MCP_PROTOCOL_VERSION.to_string()
        }
    }
}

/// Standard MCP methods
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardMethod {
    // Lifecycle methods
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "initialized")]
    Initialized,

    // Ping/pong for connection health
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,

    // Tool methods
    #[serde(rename = "tools/list")]
    ToolsList,
    #[serde(rename = "tools/call")]
    ToolsCall,

    // Batch methods
    #[serde(rename = "batch")]
    Batch,

    // Resource methods
    #[serde(rename = "resources/list")]
    ResourcesList,
    #[serde(rename = "resources/read")]
    ResourcesRead,
    #[serde(rename = "resources/subscribe")]
    ResourcesSubscribe,
    #[serde(rename = "resources/unsubscribe")]
    ResourcesUnsubscribe,

    // Prompt methods
    #[serde(rename = "prompts/list")]
    PromptsList,
    #[serde(rename = "prompts/get")]
    PromptsGet,

    // Completion/sampling methods
    #[serde(rename = "completion/complete")]
    CompletionComplete,
    #[serde(rename = "sampling/createMessage")]
    SamplingCreateMessage,

    // Logging methods
    #[serde(rename = "logging/setLevel")]
    LoggingSetLevel,

    // Notification methods
    #[serde(rename = "notifications/cancelled")]
    NotificationsCancelled,
    #[serde(rename = "notifications/progress")]
    NotificationsProgress,
    #[serde(rename = "notifications/message")]
    NotificationsMessage,
    #[serde(rename = "notifications/resources/updated")]
    NotificationsResourcesUpdated,
    #[serde(rename = "notifications/resources/list_changed")]
    NotificationsResourcesListChanged,
    #[serde(rename = "notifications/tools/list_changed")]
    NotificationsToolsListChanged,
    #[serde(rename = "notifications/batch_progress")]
    NotificationsBatchProgress,
}

impl StandardMethod {
    /// Check if this method requires initialization
    pub fn requires_initialization(&self) -> bool {
        match self {
            StandardMethod::Initialize | StandardMethod::Initialized => false,
            _ => true,
        }
    }

    /// Check if this method is a notification (no response expected)
    pub fn is_notification(&self) -> bool {
        match self {
            StandardMethod::Initialized
            | StandardMethod::NotificationsCancelled
            | StandardMethod::NotificationsProgress
            | StandardMethod::NotificationsMessage
            | StandardMethod::NotificationsResourcesUpdated
            | StandardMethod::NotificationsResourcesListChanged
            | StandardMethod::NotificationsToolsListChanged
            | StandardMethod::NotificationsBatchProgress => true,
            _ => false,
        }
    }
}
