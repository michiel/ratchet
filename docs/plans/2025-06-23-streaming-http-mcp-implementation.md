# Streaming HTTP MCP Implementation Plan

**Date:** 2025-06-23  
**Objective:** Implement streaming HTTP transport for Model Context Protocol (MCP) in `ratchet serve` command using the reference server as a guide.

## Current State Analysis

### Current 'ratchet serve' Implementation
- **Location:** `ratchet-server/src/startup.rs`
- **Architecture:** Unified server combining REST API, GraphQL, MCP SSE, and Admin UI
- **MCP Integration:** Already has SSE-based MCP at `/mcp` endpoint (port 8090)
- **Transport:** Currently uses Server-Sent Events (SSE) for MCP communication
- **Status:** Production-ready with TLS, CORS, graceful shutdown, health checks

### Current 'ratchet mcp-serve' Implementation  
- **Location:** `ratchet-mcp/src/server/mod.rs`
- **Transport:** Stdio-based JSON-RPC communication
- **Purpose:** Spawnable process for direct LLM integration
- **Features:** Tool registry, batch execution, progress streaming

### Reference Server Analysis
- **Location:** `tmp/servers/src/everything/streamableHttp.ts`
- **Transport:** StreamableHTTPServerTransport from MCP SDK
- **Key Features:**
  - Session management with unique IDs
  - POST `/mcp` for initialization and JSON-RPC requests
  - GET `/mcp` for SSE stream establishment
  - DELETE `/mcp` for session termination
  - Resumability support with Last-Event-ID
  - In-memory event store for connection recovery

## Implementation Plan

### Phase 1: Rust MCP SDK Integration

#### 1.1 Dependencies
- Add Model Context Protocol Rust SDK dependency
- Research equivalent of `@modelcontextprotocol/sdk/server/streamableHttp.js`
- Identify if Rust StreamableHTTP transport exists or needs custom implementation

#### 1.2 Transport Implementation
**New File:** `ratchet-mcp/src/transport/streamable_http.rs`

```rust
pub struct StreamableHttpTransport {
    session_id: String,
    event_store: Arc<dyn EventStore>,
    server: Arc<McpServer>,
}

impl StreamableHttpTransport {
    pub async fn handle_request(&self, req: &HttpRequest) -> HttpResponse {
        match req.method() {
            "POST" => self.handle_post_request(req).await,
            "GET" => self.handle_sse_stream(req).await,
            "DELETE" => self.handle_session_termination(req).await,
            _ => HttpResponse::MethodNotAllowed(),
        }
    }
}
```

#### 1.3 Session Management
```rust
pub struct SessionManager {
    transports: Arc<RwLock<HashMap<String, Arc<StreamableHttpTransport>>>>,
}

impl SessionManager {
    pub async fn create_session(&self) -> String;
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<StreamableHttpTransport>>;
    pub async fn remove_session(&self, session_id: &str);
}
```

### Phase 2: Server Integration

#### 2.1 Modify Existing MCP SSE Endpoint
**File:** `ratchet-server/src/startup.rs`

Update the existing `/mcp` endpoint to support both SSE and StreamableHTTP protocols:

```rust
// Replace existing SSE-only implementation
async fn mcp_handler(
    req: HttpRequest,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse, Error> {
    match req.method() {
        &Method::POST => handle_mcp_post(req, session_manager).await,
        &Method::GET => handle_mcp_get(req, session_manager).await,
        &Method::DELETE => handle_mcp_delete(req, session_manager).await,
        _ => Ok(HttpResponse::MethodNotAllowed().finish()),
    }
}
```

#### 2.2 Configuration Updates
**File:** `ratchet-server/src/config.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApiConfig {
    pub enabled: bool,
    pub transport: McpTransportType, // Add enum for SSE, StreamableHTTP, Both
    pub endpoint: String,
    pub cors_origins: Vec<String>,
    pub max_sessions: u32,
    pub session_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransportType {
    Sse,
    StreamableHttp,
    Both,
}
```

### Phase 3: Event Store Implementation

#### 3.1 In-Memory Event Store
**New File:** `ratchet-mcp/src/event_store/memory.rs`

```rust
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<String, Vec<McpEvent>>>>,
    cleanup_interval: Duration,
}

impl EventStore for InMemoryEventStore {
    async fn store_event(&self, session_id: &str, event: McpEvent);
    async fn get_events_since(&self, session_id: &str, last_event_id: Option<&str>) -> Vec<McpEvent>;
    async fn cleanup_expired(&self);
}
```

#### 3.2 Optional Database Event Store
**New File:** `ratchet-mcp/src/event_store/database.rs`

For production deployments with persistence requirements:

```rust 
pub struct DatabaseEventStore {
    pool: Arc<SqlitePool>,
}

impl EventStore for DatabaseEventStore {
    // Implement persistent event storage for session resumability
}
```

### Phase 4: Request/Response Handling

#### 4.1 POST Request Handler
Handle JSON-RPC initialization and regular requests:

```rust
async fn handle_mcp_post(
    req: HttpRequest,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse, Error> {
    let session_id = req.headers().get("mcp-session-id");
    
    if session_id.is_none() {
        // New session initialization
        let transport = create_new_session(&session_manager).await?;
        return transport.handle_initialization(req).await;
    }
    
    // Existing session request
    let session_id = session_id.unwrap().to_str()?;
    if let Some(transport) = session_manager.get_session(session_id).await {
        transport.handle_request(req).await
    } else {
        Ok(HttpResponse::BadRequest().json(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32000,
                "message": "Invalid session ID"
            }
        })))
    }
}
```

#### 4.2 GET Request Handler (SSE Stream)
Handle Server-Sent Events stream establishment:

```rust
async fn handle_mcp_get(
    req: HttpRequest,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse, Error> {
    let session_id = get_session_id_from_headers(&req)?;
    let last_event_id = req.headers().get("last-event-id");
    
    if let Some(transport) = session_manager.get_session(&session_id).await {
        transport.establish_sse_stream(req, last_event_id).await
    } else {
        Ok(HttpResponse::BadRequest().json(json!({
            "error": "Invalid session ID"
        })))
    }
}
```

### Phase 5: Testing & Validation

#### 5.1 Unit Tests
**New File:** `ratchet-mcp/src/transport/streamable_http/tests.rs`

- Session creation and management
- Event store functionality
- Request/response serialization
- Error handling scenarios

#### 5.2 Integration Tests
**New File:** `ratchet-server/tests/mcp_streamable_http_test.rs`

- Full HTTP request/response cycle
- Session resumability with Last-Event-ID
- Multiple simultaneous sessions
- Graceful session termination

#### 5.3 Reference Compatibility Test
Create test that validates compatibility with the reference TypeScript server behavior:

```rust
#[tokio::test]
async fn test_reference_server_compatibility() {
    // Test against expected MCP client behavior
    // Validate session management matches reference implementation
}
```

### Phase 6: Documentation & Configuration

#### 6.1 Configuration Example
**Update:** `config/example.yaml`

```yaml
server:
  mcp_api:
    enabled: true
    transport: streamable_http  # or "sse" or "both"
    endpoint: "/mcp"
    max_sessions: 100
    session_timeout: "30m"
    cors_origins:
      - "https://claude.ai"
      - "http://localhost:3000"
```

#### 6.2 Documentation Updates
- Update MCP documentation with new transport option
- Add examples for client integration
- Document session management and resumability features

## Migration Strategy

### Backward Compatibility
- Keep existing SSE implementation as default
- Add configuration option to enable StreamableHTTP
- Support both transports simultaneously during transition period

### Rollout Plan
1. **Alpha:** StreamableHTTP available as opt-in feature
2. **Beta:** Default to StreamableHTTP for new installations  
3. **Stable:** Deprecate SSE-only mode, recommend StreamableHTTP

## Risk Assessment

### Technical Risks
- **Rust MCP SDK Availability:** May need custom StreamableHTTP implementation if SDK doesn't exist
- **Session State Management:** Memory usage growth with long-lived sessions
- **Connection Handling:** Complex error scenarios with dual POST/GET patterns

### Mitigation Strategies
- **Custom Implementation:** Create Rust equivalent of TypeScript StreamableHTTP transport
- **Session Cleanup:** Implement aggressive cleanup policies and monitoring
- **Connection Monitoring:** Add comprehensive logging and health checks

## Success Criteria

1. **Functional:** StreamableHTTP transport works identically to reference server
2. **Performance:** No degradation compared to existing SSE implementation
3. **Compatibility:** Works with existing MCP clients (Claude, etc.)
4. **Reliability:** Session resumability functions correctly under network interruptions
5. **Configuration:** Seamless migration path for existing users

## Timeline Estimate

- **Phase 1-2:** 1-2 weeks (Transport implementation and server integration)
- **Phase 3-4:** 1 week (Event store and request handling) 
- **Phase 5:** 1 week (Testing and validation)
- **Phase 6:** 3-5 days (Documentation and configuration)

**Total:** ~4 weeks for complete implementation and testing

## Next Steps

1. Research availability of Rust MCP SDK with StreamableHTTP support
2. Begin Phase 1 implementation with dependency analysis
3. Create proof-of-concept transport implementation
4. Validate approach with minimal working example