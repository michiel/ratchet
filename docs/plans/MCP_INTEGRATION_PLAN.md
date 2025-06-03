# LLM MCP Integration Design Proposal

## Executive Summary

This proposal outlines the design and implementation plan for integrating Model Context Protocol (MCP) support into Ratchet, enabling JavaScript tasks to interact with Large Language Models (LLMs) and their associated tools through a standardized protocol.

MCP is an open protocol that standardizes how applications communicate with LLM servers, providing a consistent interface for tool invocation, context management, and model interactions. By integrating MCP into Ratchet, we can enable tasks to leverage AI capabilities while maintaining Ratchet's process isolation and security model.

## Goals & Benefits

### Primary Goals

1. **Enable AI-Powered Tasks**: Allow Ratchet tasks to interact with LLMs for text generation, analysis, and decision-making
2. **Tool Integration**: Provide access to LLM tools (web search, code execution, data analysis) through MCP
3. **Provider Agnostic**: Support multiple LLM providers (Anthropic, OpenAI, local models) through a single protocol
4. **Maintain Security**: Preserve Ratchet's process isolation and security boundaries
5. **Developer Experience**: Provide a simple, intuitive JavaScript API for MCP interactions

### Key Benefits

- **Standardization**: Use industry-standard MCP protocol instead of proprietary integrations
- **Flexibility**: Switch between LLM providers without changing task code
- **Extensibility**: Easy to add new MCP servers and capabilities
- **Performance**: Connection pooling and efficient transport mechanisms
- **Observability**: Built-in logging and monitoring for AI interactions

## Architecture Overview

### High-Level Architecture

```
┌─────────────────────┐
│   Ratchet Task      │
│   (JavaScript)      │
│  ┌───────────────┐  │
│  │ mcp.invoke()  │  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
┌──────────▼──────────┐
│   Worker Process    │
│  ┌───────────────┐  │
│  │  MCP Client   │  │
│  └───────┬───────┘  │
└──────────┼──────────┘
           │
    ┌──────┴──────┐
    │   Transport │
    │ (stdio/SSE) │
    └──────┬──────┘
           │
┌──────────▼──────────┐
│    MCP Server       │
│  (Anthropic/OpenAI) │
└─────────────────────┘
```

### Component Design

#### 1. MCP Service Layer

Create a new service following Ratchet's service pattern:

```rust
// ratchet-lib/src/services/mcp_service.rs

#[async_trait(?Send)]
pub trait McpService {
    /// List configured MCP servers
    async fn list_servers(&self) -> ServiceResult<Vec<McpServerInfo>>;
    
    /// Connect to an MCP server
    async fn connect(&self, server_id: &str) -> ServiceResult<McpConnection>;
    
    /// List available tools on a server
    async fn list_tools(&self, conn: &McpConnection) -> ServiceResult<Vec<McpTool>>;
    
    /// Invoke a tool on the server
    async fn invoke_tool(
        &self, 
        conn: &McpConnection,
        tool: &str,
        args: serde_json::Value
    ) -> ServiceResult<serde_json::Value>;
    
    /// Send a completion request
    async fn complete(
        &self,
        conn: &McpConnection, 
        request: CompletionRequest
    ) -> ServiceResult<CompletionResponse>;
}

pub struct McpServiceImpl {
    config: McpConfig,
    connections: Arc<Mutex<HashMap<String, McpConnection>>>,
    http_client: HttpManager,
}
```

#### 2. MCP Protocol Types

Define MCP protocol types based on the specification:

```rust
// ratchet-lib/src/mcp/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub transport: McpTransport,
    pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Sse {
        url: String,
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub model_preferences: Option<ModelPreferences>,
    pub system_prompt: Option<String>,
    pub include_context: Option<IncludeContext>,
    pub max_tokens: Option<u32>,
}
```

#### 3. JavaScript API

Extend the JavaScript environment with an MCP global object:

```rust
// ratchet-lib/src/js_executor/mcp_integration.rs

pub fn setup_mcp_api(context: &mut boa::Context, mcp_service: Arc<dyn McpService>) {
    let mcp = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(mcp_list_servers),
            "listServers",
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(mcp_invoke_tool),
            "invokeTool", 
            3,
        )
        .function(
            NativeFunction::from_fn_ptr(mcp_complete),
            "complete",
            2,
        )
        .build();
    
    context.register_global_property("mcp", mcp, Attribute::all());
}
```

JavaScript task usage:

```javascript
// Task's main.js
(async function(input) {
    // List available MCP servers
    const servers = await mcp.listServers();
    
    // Invoke a tool
    const searchResults = await mcp.invokeTool('anthropic-server', 'web_search', {
        query: input.searchQuery,
        max_results: 5
    });
    
    // Get LLM completion
    const analysis = await mcp.complete('anthropic-server', {
        messages: [
            {
                role: 'user',
                content: `Analyze these search results: ${JSON.stringify(searchResults)}`
            }
        ],
        max_tokens: 1000
    });
    
    return {
        results: searchResults,
        analysis: analysis.content
    };
})
```

#### 4. Transport Implementation

Implement MCP transports:

```rust
// ratchet-lib/src/mcp/transport/mod.rs

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn connect(&mut self) -> Result<(), McpError>;
    async fn send(&mut self, message: McpMessage) -> Result<(), McpError>;
    async fn receive(&mut self) -> Result<McpMessage, McpError>;
    async fn close(&mut self) -> Result<(), McpError>;
}

// ratchet-lib/src/mcp/transport/stdio.rs
pub struct StdioTransport {
    child: tokio::process::Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

// ratchet-lib/src/mcp/transport/sse.rs
pub struct SseTransport {
    client: reqwest::Client,
    event_stream: Option<EventStream>,
    url: String,
}
```

#### 5. Configuration

Extend Ratchet's configuration:

```rust
// ratchet-lib/src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // ... existing fields
    pub mcp: Option<McpConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
    pub connection_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
    pub auth: Option<McpAuthConfig>,
    pub capabilities: Option<ServerCapabilities>,
}
```

Example configuration:

```yaml
mcp:
  connection_timeout: 30s
  request_timeout: 120s
  servers:
    - name: "claude"
      transport:
        type: stdio
        command: "claude-mcp-server"
        args: ["--api-key-env", "ANTHROPIC_API_KEY"]
      capabilities:
        tools: true
        sampling: true
        
    - name: "gpt4"
      transport:
        type: sse
        url: "https://api.openai.com/v1/mcp/sse"
      auth:
        type: bearer
        token: "${OPENAI_API_KEY}"
```

#### 6. IPC Extension

Extend IPC messages for MCP operations:

```rust
// ratchet-lib/src/execution/ipc.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerMessage {
    // ... existing variants
    McpRequest(McpRequest),
    McpResponse(McpResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub id: String,
    pub server: String,
    pub operation: McpOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpOperation {
    ListTools,
    InvokeTool { tool: String, args: Value },
    Complete { request: CompletionRequest },
}
```

## Implementation Plan

**UPDATED PRIORITIES**: Focus shifted to MCP Server implementation first, with JavaScript client integration deprioritized.

### Phase 1: Architecture Foundation (3-4 weeks) - **HIGHEST PRIORITY**

1. **Enhanced Worker Architecture**
   - Complete modularization with ratchet-mcp crate
   - Implement bidirectional IPC for MCP message routing
   - Add support for persistent connections in worker processes
   - Create connection pooling and health monitoring

2. **Core Protocol Implementation**
   - Define all MCP message types for bidirectional communication
   - Implement JSON-RPC 2.0 handling with proper error propagation
   - Create protocol state machine for connection lifecycle
   - Add request/response correlation system

3. **Transport Layer**
   - Implement enhanced stdio transport for MCP JSON-RPC
   - Implement SSE transport for HTTP-based MCP connections
   - Add connection pooling and reconnection logic
   - Create transport abstraction for multiple connection types

### Phase 2: MCP Server Implementation (4-5 weeks) - **HIGHEST PRIORITY**

1. **Tool Registry & Execution**
   - Implement MCP server framework with tool registry
   - Add task execution tool (`ratchet.execute_task`)
   - Add monitoring tools (`ratchet.get_execution_status`, `ratchet.get_execution_logs`)
   - Add debugging tools (`ratchet.analyze_execution_error`)

2. **Security & Access Control**
   - Authentication for MCP connections (API keys, OAuth2)
   - Fine-grained permissions for LLM tool access
   - Rate limiting per client and tool
   - Audit logging for all MCP operations

3. **Performance Optimization**
   - High-performance message handling with batching
   - Streaming responses for long-running tasks
   - Real-time progress updates
   - Binary encoding options for performance-critical paths

### Phase 3: Configuration & Management (2-3 weeks) - **HIGH PRIORITY**

1. **Enhanced Configuration System**
   - MCP server configuration with security settings
   - Authentication and authorization configuration
   - Performance tuning parameters
   - Resource quotas and limits

2. **Production Features**
   - Comprehensive audit logging
   - Monitoring and alerting integration
   - Health checks and status endpoints
   - Documentation and examples

### Phase 4: JavaScript Client Integration (2-3 weeks) - **DEPRIORITIZED**

1. **JavaScript API** *(Lower Priority)*
   - Create mcp global object in Boa context
   - Implement async operations (mcp.invokeTool, mcp.complete)
   - Add proper error propagation

2. **IPC Integration** *(Lower Priority)*
   - Extend worker messages for MCP client operations
   - Handle MCP requests in worker process
   - Implement response routing

3. **Testing Framework**
   - Create mock MCP server for testing
   - Add integration tests for both server and client
   - Document JavaScript API

**Priority Rationale**: The MCP server implementation provides immediate value by enabling LLMs to control Ratchet directly for AI-powered debugging, workflow orchestration, and automated operations. JavaScript client integration can be added later once the core server infrastructure is stable.

## Security Considerations

### Process Isolation

- MCP connections are created per worker process
- No shared state between workers
- Connections terminated on worker exit

### Authentication & Authorization

- Support for API key authentication
- OAuth2 support for remote servers
- Per-server access control

### Input Validation

- Validate all tool inputs against schemas
- Sanitize prompts before sending
- Limit request sizes

### Network Security

- TLS required for remote connections
- Certificate validation
- Proxy support for corporate environments

## Performance Considerations

### Connection Pooling

- Reuse connections within worker processes
- Lazy connection establishment
- Connection timeout management

### Caching

- Cache tool schemas per server
- Cache authentication tokens
- Optional response caching

### Resource Limits

- Maximum concurrent MCP operations
- Request size limits
- Response streaming for large outputs

## Testing Strategy

### Unit Tests

- Protocol message parsing
- Transport implementations
- Service layer logic

### Integration Tests

- End-to-end task execution with MCP
- Multiple server scenarios
- Error handling paths

### Mock MCP Server

Create a mock MCP server for testing:

```rust
// ratchet-lib/src/testing/mock_mcp_server.rs

pub struct MockMcpServer {
    tools: HashMap<String, MockTool>,
    responses: HashMap<String, Value>,
}

impl MockMcpServer {
    pub fn with_tool(mut self, name: &str, handler: MockToolHandler) -> Self {
        // ...
    }
    
    pub fn start(&self) -> McpServerHandle {
        // ...
    }
}
```

## Migration & Compatibility

### Backward Compatibility

- Existing tasks continue to work unchanged
- MCP is opt-in per task
- No breaking changes to existing APIs

### Migration Path

1. Deploy MCP support in Ratchet
2. Configure MCP servers
3. Update tasks to use MCP as needed
4. Monitor and optimize

## Documentation Plan

### User Documentation

1. **MCP Overview** - What is MCP and why use it
2. **Configuration Guide** - How to configure MCP servers
3. **JavaScript API Reference** - Complete API documentation
4. **Examples** - Common use cases and patterns
5. **Troubleshooting** - Common issues and solutions

### Developer Documentation

1. **Architecture Guide** - Internal design and flow
2. **Protocol Reference** - MCP protocol details
3. **Extension Guide** - Adding new transports or features
4. **Testing Guide** - How to test MCP integrations

## Success Metrics

- Number of tasks using MCP capabilities
- Average response time for MCP operations
- Error rate and types
- User adoption and feedback
- Performance impact on task execution

## Open Questions

1. **Provider-Specific Extensions**: How to handle provider-specific MCP extensions?
2. **Cost Management**: How to track and limit API costs?
3. **Prompt Engineering**: Should we provide prompt templates or helpers?
4. **Model Selection**: How to handle model selection and fallbacks?
5. **Context Windows**: How to manage large context windows efficiently?

## Conclusion

Integrating MCP into Ratchet provides a powerful, standardized way for tasks to leverage LLM capabilities while maintaining Ratchet's security and isolation guarantees. The phased implementation approach allows for incremental delivery and validation of functionality.

The design leverages Ratchet's existing patterns and architecture, making it a natural extension of the platform's capabilities. With proper implementation, this will enable a new class of AI-powered automation tasks while remaining provider-agnostic and maintainable.

## Extension: Bidirectional MCP Integration

This design can be extended to support bidirectional communication, where Ratchet itself becomes an MCP server that LLMs can connect to. This would enable:

- LLMs executing Ratchet tasks with full observability
- AI-powered debugging using execution logs and traces
- Intelligent workflow orchestration by AI agents
- Automated error recovery and system monitoring

See [MCP Bidirectional Design](MCP_BIDIRECTIONAL_DESIGN.md) for the complete extension proposal that explores exposing Ratchet's capabilities as MCP tools for LLM consumption.