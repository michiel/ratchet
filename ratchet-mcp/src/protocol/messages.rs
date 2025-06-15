//! MCP-specific message types and protocol definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::{JsonRpcRequest, JsonRpcResponse};

/// Top-level MCP message type that wraps JSON-RPC messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
}

impl McpMessage {
    /// Check if this is a request message
    pub fn is_request(&self) -> bool {
        matches!(self, McpMessage::Request(_))
    }

    /// Check if this is a response message
    pub fn is_response(&self) -> bool {
        matches!(self, McpMessage::Response(_))
    }

    /// Get the message ID if present
    pub fn id(&self) -> Option<&Value> {
        match self {
            McpMessage::Request(req) => req.id.as_ref(),
            McpMessage::Response(resp) => resp.id.as_ref(),
        }
    }
}

/// MCP method with typed parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum McpMethod {
    /// Initialize the MCP connection
    #[serde(rename = "initialize")]
    Initialize(InitializeParams),

    /// Initialize complete notification
    #[serde(rename = "initialized")]
    Initialized,

    /// Ping for connection health
    #[serde(rename = "ping")]
    Ping,

    /// List available tools
    #[serde(rename = "tools/list")]
    ToolsList(Option<ToolsListParams>),

    /// Call a tool
    #[serde(rename = "tools/call")]
    ToolsCall(ToolsCallParams),

    /// Batch request for multiple operations
    #[serde(rename = "batch")]
    Batch(BatchParams),

    /// List available resources
    #[serde(rename = "resources/list")]
    ResourcesList(Option<ResourcesListParams>),

    /// Read a resource
    #[serde(rename = "resources/read")]
    ResourcesRead(ResourcesReadParams),

    /// List available prompts
    #[serde(rename = "prompts/list")]
    PromptsList(Option<PromptsListParams>),

    /// Get a prompt
    #[serde(rename = "prompts/get")]
    PromptsGet(PromptsGetParams),

    /// Create a completion
    #[serde(rename = "completion/complete")]
    CompletionComplete(CompletionParams),

    /// Create a sampling message
    #[serde(rename = "sampling/createMessage")]
    SamplingCreateMessage(SamplingParams),

    /// Set logging level
    #[serde(rename = "logging/setLevel")]
    LoggingSetLevel(LoggingSetLevelParams),

    /// Progress notification
    #[serde(rename = "notifications/progress")]
    NotificationsProgress(ProgressNotification),

    /// Task execution progress notification
    #[serde(rename = "notifications/task_progress")]
    NotificationsTaskProgress(TaskProgressNotification),

    /// Batch execution progress notification
    #[serde(rename = "notifications/batch_progress")]
    NotificationsBatchProgress(BatchProgressNotification),

    /// Custom method for extension
    #[serde(untagged)]
    Custom {
        method: String,
        params: Option<Value>,
    },
}

/// MCP request with typed method and parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    #[serde(flatten)]
    pub method: McpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

/// MCP response with typed result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<super::JsonRpcError>,
    pub id: Option<Value>,
}

/// MCP notification (no response expected)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpNotification {
    pub jsonrpc: String,
    #[serde(flatten)]
    pub method: McpMethod,
}

// === Initialize Protocol ===

/// Parameters for the initialize method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information (optional for backward compatibility)
    #[serde(rename = "clientInfo", skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,
}

/// Result of the initialize method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Client information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,

    /// Additional client metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Server information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,

    /// Additional server metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

// === Tool Protocol ===

/// Parameters for tools/list method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsListParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Result of tools/list method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsListResult {
    /// List of available tools
    pub tools: Vec<Tool>,

    /// Next cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Tool definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Input schema for the tool
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,

    /// Additional tool metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Parameters for tools/call method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsCallParams {
    /// Tool name to call
    pub name: String,

    /// Tool arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Result of tools/call method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsCallResult {
    /// Tool execution content
    pub content: Vec<ToolContent>,

    /// Whether the tool call is an error
    #[serde(default, rename = "isError")]
    pub is_error: bool,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Content returned by tool execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource {
        resource: ResourceReference,
        text: Option<String>,
        blob: Option<String>,
    },
}

// === Resource Protocol ===

/// Parameters for resources/list method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesListParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Result of resources/list method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesListResult {
    /// List of available resources
    pub resources: Vec<Resource>,

    /// Next cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Resource definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI
    pub uri: String,

    /// Resource name
    pub name: String,

    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none", rename = "mimeType")]
    pub mime_type: Option<String>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Resource reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceReference {
    /// Resource URI
    pub uri: String,
}

/// Parameters for resources/read method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesReadParams {
    /// Resource URI to read
    pub uri: String,
}

/// Result of resources/read method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesReadResult {
    /// Resource contents
    pub contents: Vec<ResourceContent>,
}

/// Resource content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceContent {
    #[serde(rename = "text")]
    Text {
        text: String,
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none", rename = "mimeType")]
        mime_type: Option<String>,
    },

    #[serde(rename = "blob")]
    Blob {
        blob: String,
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

// === Prompt Protocol ===

/// Parameters for prompts/list method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptsListParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Parameters for prompts/get method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptsGetParams {
    /// Prompt name
    pub name: String,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
}

// === Completion Protocol ===

/// Parameters for completion/complete method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionParams {
    /// Argument for the completion
    pub argument: CompletionArgument,

    /// Reference to the resource being completed
    pub ref_: CompletionReference,
}

/// Completion argument
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionArgument {
    /// Argument name
    pub name: String,

    /// Argument value
    pub value: String,
}

/// Completion reference  
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompletionReference {
    #[serde(rename = "ref/resource")]
    Resource { uri: String },

    #[serde(rename = "ref/prompt")]
    Prompt { name: String },
}

// === Sampling Protocol ===

/// Parameters for sampling/createMessage method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingParams {
    /// Messages for sampling
    pub messages: Vec<SamplingMessage>,

    /// Model preferences
    #[serde(skip_serializing_if = "Option::is_none", rename = "modelPreferences")]
    pub model_preferences: Option<ModelPreferences>,

    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemPrompt")]
    pub system_prompt: Option<String>,

    /// Include context
    #[serde(skip_serializing_if = "Option::is_none", rename = "includeContext")]
    pub include_context: Option<IncludeContext>,

    /// Temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Max tokens
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxTokens")]
    pub max_tokens: Option<u32>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none", rename = "stopSequences")]
    pub stop_sequences: Option<Vec<String>>,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Sampling message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Message role
    pub role: MessageRole,

    /// Message content
    pub content: MessageContent,
}

/// Message role
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Message content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Content(Vec<ContentPart>),
}

/// Content part for multimodal messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource {
        resource: ResourceReference,
        text: Option<String>,
    },
}

/// Model preferences
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Hints for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,

    /// Cost priority (0.0 = cheapest, 1.0 = most expensive)
    #[serde(skip_serializing_if = "Option::is_none", rename = "costPriority")]
    pub cost_priority: Option<f32>,

    /// Speed priority (0.0 = slowest, 1.0 = fastest)
    #[serde(skip_serializing_if = "Option::is_none", rename = "speedPriority")]
    pub speed_priority: Option<f32>,

    /// Intelligence priority (0.0 = least intelligent, 1.0 = most intelligent)
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "intelligencePriority"
    )]
    pub intelligence_priority: Option<f32>,
}

/// Model hint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelHint {
    /// Hint name
    pub name: String,
}

/// Include context options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeContext {
    None,
    ThisServer,
    AllServers,
}

// === Logging Protocol ===

/// Parameters for logging/setLevel method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggingSetLevelParams {
    /// Log level to set
    pub level: LogLevel,
}

/// Log level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

// === Notification Protocol ===

/// Progress notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Progress token
    #[serde(rename = "progressToken")]
    pub progress_token: Value,

    /// Progress value (0.0 to 1.0)
    pub progress: f32,

    /// Total work units
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Task execution progress notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskProgressNotification {
    /// Execution ID
    #[serde(rename = "executionId")]
    pub execution_id: String,

    /// Task ID or name
    #[serde(rename = "taskId")]
    pub task_id: String,

    /// Progress value (0.0 to 1.0)
    pub progress: f32,

    /// Current step description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,

    /// Step number (current step)
    #[serde(skip_serializing_if = "Option::is_none", rename = "stepNumber")]
    pub step_number: Option<u32>,

    /// Total steps
    #[serde(skip_serializing_if = "Option::is_none", rename = "totalSteps")]
    pub total_steps: Option<u32>,

    /// Custom status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Progress data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Timestamp of progress update
    pub timestamp: String,
}

// === Capabilities ===

/// Client capabilities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Experimental capabilities
    #[serde(default)]
    pub experimental: HashMap<String, Value>,

    /// Sampling capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
}

/// Server capabilities  
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Experimental capabilities
    #[serde(default)]
    pub experimental: HashMap<String, Value>,

    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,

    /// Prompts capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Resources capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Batch capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<BatchCapability>,
}

/// Sampling capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Logging capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggingCapability {}

/// Prompts capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptsCapability {
    /// Whether list_changed notifications are supported
    #[serde(default, rename = "listChanged")]
    pub list_changed: bool,
}

/// Resources capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcesCapability {
    /// Whether subscribe/unsubscribe is supported
    #[serde(default)]
    pub subscribe: bool,

    /// Whether list_changed notifications are supported
    #[serde(default, rename = "listChanged")]
    pub list_changed: bool,
}

/// Tools capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsCapability {
    /// Whether list_changed notifications are supported
    #[serde(default, rename = "listChanged")]
    pub list_changed: bool,
}

/// Batch capability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchCapability {
    /// Maximum number of requests in a single batch
    #[serde(rename = "maxBatchSize")]
    pub max_batch_size: u32,

    /// Maximum parallel executions supported
    #[serde(rename = "maxParallel")]
    pub max_parallel: u32,

    /// Whether dependency execution is supported
    #[serde(default, rename = "supportsDependencies")]
    pub supports_dependencies: bool,

    /// Whether progress notifications are supported for batches
    #[serde(default, rename = "supportsProgress")]
    pub supports_progress: bool,

    /// Supported execution modes
    #[serde(rename = "supportedExecutionModes")]
    pub supported_execution_modes: Vec<BatchExecutionMode>,
}

// === Batch Protocol ===

/// Parameters for batch method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchParams {
    /// List of requests to execute in batch
    pub requests: Vec<BatchRequest>,

    /// Execution mode for the batch
    #[serde(default, rename = "executionMode")]
    pub execution_mode: BatchExecutionMode,

    /// Maximum parallel execution count
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxParallel")]
    pub max_parallel: Option<u32>,

    /// Timeout for the entire batch in milliseconds
    #[serde(skip_serializing_if = "Option::is_none", rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,

    /// Whether to stop on first error
    #[serde(default, rename = "stopOnError")]
    pub stop_on_error: bool,

    /// Correlation token for tracking the batch
    #[serde(skip_serializing_if = "Option::is_none", rename = "correlationToken")]
    pub correlation_token: Option<String>,

    /// Additional batch metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Individual request within a batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchRequest {
    /// Unique identifier for this request within the batch
    pub id: String,

    /// Method to call
    pub method: String,

    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// Dependencies on other requests in the batch (by their IDs)
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Timeout for this specific request in milliseconds
    #[serde(skip_serializing_if = "Option::is_none", rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,

    /// Priority for request execution (higher values = higher priority)
    #[serde(default)]
    pub priority: i32,

    /// Additional request metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Batch execution mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum BatchExecutionMode {
    /// Execute all requests in parallel
    #[default]
    Parallel,
    /// Execute requests sequentially in order
    Sequential,
    /// Execute based on dependency graph
    Dependency,
    /// Execute based on priority and dependency
    PriorityDependency,
}

/// Result of batch method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchResult {
    /// Results for each request in the batch
    pub results: Vec<BatchItemResult>,

    /// Batch execution statistics
    pub stats: BatchStats,

    /// Correlation token if provided in request
    #[serde(skip_serializing_if = "Option::is_none", rename = "correlationToken")]
    pub correlation_token: Option<String>,

    /// Additional result metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Result for an individual request in a batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchItemResult {
    /// Request ID from the batch request
    pub id: String,

    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error information (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<super::JsonRpcError>,

    /// Execution time in milliseconds
    #[serde(rename = "executionTimeMs")]
    pub execution_time_ms: u64,

    /// Whether this request was skipped due to dependency failure
    #[serde(default)]
    pub skipped: bool,

    /// Additional item metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

/// Batch execution statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchStats {
    /// Total number of requests
    #[serde(rename = "totalRequests")]
    pub total_requests: u32,

    /// Number of successful requests
    #[serde(rename = "successfulRequests")]
    pub successful_requests: u32,

    /// Number of failed requests
    #[serde(rename = "failedRequests")]
    pub failed_requests: u32,

    /// Number of skipped requests
    #[serde(rename = "skippedRequests")]
    pub skipped_requests: u32,

    /// Total execution time in milliseconds
    #[serde(rename = "totalExecutionTimeMs")]
    pub total_execution_time_ms: u64,

    /// Average execution time per request in milliseconds
    #[serde(rename = "averageExecutionTimeMs")]
    pub average_execution_time_ms: f64,

    /// Maximum parallel executions achieved
    #[serde(rename = "maxParallelExecuted")]
    pub max_parallel_executed: u32,
}

/// Batch progress notification for long-running batch operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchProgressNotification {
    /// Correlation token for the batch
    #[serde(rename = "correlationToken")]
    pub correlation_token: String,

    /// Number of completed requests
    #[serde(rename = "completedRequests")]
    pub completed_requests: u32,

    /// Total number of requests in the batch
    #[serde(rename = "totalRequests")]
    pub total_requests: u32,

    /// Currently executing request IDs
    #[serde(rename = "executingRequests")]
    pub executing_requests: Vec<String>,

    /// Timestamp of progress update
    pub timestamp: String,

    /// Additional progress data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
