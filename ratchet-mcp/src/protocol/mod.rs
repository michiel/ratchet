//! MCP protocol implementation including JSON-RPC 2.0 and MCP-specific message types

pub mod capabilities;
pub mod jsonrpc;
pub mod messages;

pub use capabilities::{ClientCapabilities, McpCapabilities, ServerCapabilities, ToolsCapability};
pub use jsonrpc::{JsonRpcError, JsonRpcErrorCode, JsonRpcRequest, JsonRpcResponse};
pub use messages::{
    ClientInfo, InitializeParams, InitializeResult, McpMessage, McpMethod, McpNotification,
    McpRequest, McpResponse, ResourcesListParams, ResourcesListResult, ResourcesReadParams,
    ResourcesReadResult, ServerInfo, Tool, ToolContent, ToolsCallParams, ToolsCallResult,
    ToolsListParams, ToolsListResult,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// MCP protocol version
pub const MCP_PROTOCOL_VERSION: &str = "0.1.0";

/// Generate a new request ID
pub fn generate_request_id() -> Value {
    Value::String(Uuid::new_v4().to_string())
}

/// Validate MCP protocol version
pub fn validate_protocol_version(version: &str) -> bool {
    version == MCP_PROTOCOL_VERSION
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
            | StandardMethod::NotificationsToolsListChanged => true,
            _ => false,
        }
    }
}

