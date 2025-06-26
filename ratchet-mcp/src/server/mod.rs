//! MCP server implementation for exposing Ratchet capabilities to LLMs

pub mod adapter;
pub mod batch;
pub mod config;
pub mod handler;
pub mod progress;
pub mod service;
pub mod task_dev_tools;
pub mod tools;

pub use adapter::{RatchetMcpAdapter, RatchetMcpAdapterBuilder};
pub use batch::BatchProcessor;
pub use config::{McpServerConfig, McpServerTransport};
pub use handler::McpRequestHandler;
pub use service::{McpService, McpServiceBuilder, McpServiceConfig};
pub use tools::{McpTaskExecutor, McpTaskInfo, McpTool, RatchetToolRegistry, ToolRegistry};

// Main server types are defined in this module, no need to re-export

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{
    InitializeParams, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerCapabilities, ServerInfo,
};
use crate::security::{AuditLogger, McpAuthManager, SecurityContext};
use crate::correlation::{CorrelationManager, CorrelationConfig};
use crate::metrics::{McpMetrics, MetricsConfig};
use crate::{McpAuth, McpError, McpResult};

/// MCP server for exposing Ratchet capabilities to LLMs
#[derive(Clone)]
pub struct McpServer {
    /// Server configuration
    config: McpServerConfig,

    /// Tool registry containing available tools
    tool_registry: Arc<dyn ToolRegistry>,

    /// Authentication manager
    auth_manager: Arc<McpAuthManager>,

    /// Audit logger
    audit_logger: Arc<AuditLogger>,

    /// Request correlation manager
    correlation_manager: Arc<CorrelationManager>,

    /// Performance metrics system
    metrics: Arc<McpMetrics>,

    /// Active client sessions
    _sessions: Arc<RwLock<HashMap<String, SecurityContext>>>,

    /// Whether the server is initialized
    initialized: Arc<RwLock<bool>>,

    /// Server-issued session IDs (for validation)
    server_issued_sessions: Arc<RwLock<HashSet<String>>>,

    /// Message history for resumability (session_id -> Vec<(event_id, message)>)
    message_history: Arc<RwLock<HashMap<String, Vec<(String, String)>>>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(
        config: McpServerConfig,
        tool_registry: Arc<dyn ToolRegistry>,
        auth_manager: Arc<McpAuthManager>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        let correlation_manager = Arc::new(CorrelationManager::new(CorrelationConfig::default()));
        let metrics = Arc::new(McpMetrics::new(MetricsConfig::default()));
        
        Self {
            config,
            tool_registry,
            auth_manager,
            audit_logger,
            correlation_manager,
            metrics,
            _sessions: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
            server_issued_sessions: Arc::new(RwLock::new(HashSet::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new MCP server with adapter
    pub async fn with_adapter(config: crate::config::McpConfig, adapter: RatchetMcpAdapter) -> McpResult<Self> {
        // Create tool registry from adapter
        let mut tool_registry = RatchetToolRegistry::new();
        tool_registry.set_executor(Arc::new(adapter));

        // Create security components
        let auth_manager = Arc::new(McpAuthManager::new(config.auth.clone()));
        let audit_logger = Arc::new(AuditLogger::new(false)); // TODO: Make configurable

        // Convert config to server config
        let server_config = McpServerConfig {
            transport: match config.transport_type {
                crate::config::SimpleTransportType::Stdio => McpServerTransport::Stdio,
                crate::config::SimpleTransportType::Sse => McpServerTransport::Sse {
                    host: config.host.clone(),
                    port: config.port,
                    tls: false,
                    cors: config::CorsConfig {
                        allowed_origins: vec!["*".to_string()],
                        allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
                        allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
                        allow_credentials: false,
                    },
                    timeout: config.timeouts.request_timeout,
                },
            },
            security: crate::security::SecurityConfig::default(),
            bind_address: Some(format!("{}:{}", config.host, config.port)),
        };

        let correlation_manager = Arc::new(CorrelationManager::new(CorrelationConfig::default()));
        let metrics = Arc::new(McpMetrics::new(MetricsConfig::default()));

        Ok(Self {
            config: server_config,
            tool_registry: Arc::new(tool_registry),
            auth_manager,
            audit_logger,
            correlation_manager,
            metrics,
            _sessions: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
            server_issued_sessions: Arc::new(RwLock::new(HashSet::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Run the server with stdio transport
    pub async fn run_stdio(&mut self) -> McpResult<()> {
        self.start_stdio_server().await
    }

    /// Run the server with SSE transport
    pub async fn run_sse(&mut self) -> McpResult<()> {
        match &self.config.transport {
            McpServerTransport::Sse { host, port, .. } => {
                let bind_address = format!("{}:{}", host, port);
                self.start_sse_server(&bind_address).await
            }
            _ => Err(McpError::Configuration {
                message: "Server not configured for SSE transport".to_string(),
            }),
        }
    }

    /// Run the server with a specific transport
    pub async fn run(&self, mut transport: Box<dyn crate::transport::McpTransport>) -> McpResult<()> {
        tracing::info!("Starting MCP server");

        loop {
            match transport.receive().await {
                Ok(request) => {
                    // Process the request
                    let request_str = serde_json::to_string(&request)?;
                    match self.handle_message(&request_str, None).await {
                        Ok(Some(response)) => {
                            // Convert response to request format for sending
                            // This is a workaround - ideally transport should accept JsonRpcResponse
                            let response_request = JsonRpcRequest {
                                jsonrpc: "2.0".to_string(),
                                method: "response".to_string(),
                                params: Some(serde_json::to_value(&response)?),
                                id: response.id.clone(),
                            };
                            transport.send(response_request).await?;
                        }
                        Ok(None) => {
                            // No response needed (notification)
                        }
                        Err(e) => {
                            tracing::error!("Error handling MCP request: {}", e);
                            // Send error response
                            let error_response =
                                JsonRpcResponse::error(JsonRpcError::internal_error(e.to_string()), request.id.clone());
                            let error_request = JsonRpcRequest {
                                jsonrpc: "2.0".to_string(),
                                method: "error".to_string(),
                                params: Some(serde_json::to_value(error_response)?),
                                id: request.id,
                            };
                            transport.send(error_request).await?;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Transport error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Start the MCP server
    pub async fn start(&self) -> McpResult<()> {
        match &self.config.transport {
            McpServerTransport::Stdio => self.start_stdio_server().await,
            McpServerTransport::Sse { port, host, .. } => {
                let bind_address = format!("{}:{}", host, port);
                self.start_sse_server(&bind_address).await
            }
        }
    }

    /// Start stdio-based MCP server
    async fn start_stdio_server(&self) -> McpResult<()> {
        tracing::info!("Starting MCP server with stdio transport");
        tracing::info!("Server ready to accept MCP requests via stdin/stdout");

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        let mut request_count = 0;

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    tracing::info!("Received EOF on stdin, shutting down MCP server");
                    break; // EOF
                }
                Ok(bytes_read) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    request_count += 1;
                    tracing::debug!(
                        "Received MCP request #{} ({} bytes): {}",
                        request_count,
                        bytes_read,
                        line
                    );

                    // Process the request
                    match self.handle_message(line, None).await {
                        Ok(Some(response)) => {
                            let response_json = serde_json::to_string(&response)?;
                            tracing::debug!("Sending MCP response #{}: {}", request_count, response_json);
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Ok(None) => {
                            tracing::debug!("MCP request #{} was a notification, no response sent", request_count);
                        }
                        Err(e) => {
                            tracing::error!("Error handling MCP request #{}: {}", request_count, e);
                            // Send error response if possible
                            let error_response =
                                JsonRpcResponse::error(JsonRpcError::internal_error(e.to_string()), None);
                            let response_json = serde_json::to_string(&error_response)?;
                            tracing::debug!("Sending MCP error response #{}: {}", request_count, response_json);
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        tracing::info!("MCP server stdio loop terminated after {} requests", request_count);
        Ok(())
    }

    /// Start SSE-based MCP server
    async fn start_sse_server(&self, bind_address: &str) -> McpResult<()> {
        tracing::info!("Starting MCP server with SSE transport on {}", bind_address);

        use axum::{
            extract::{Path, State},
            http::{HeaderMap, StatusCode},
            response::{sse::Event, Sse},
            routing::{get, post},
            Json, Router,
        };
        use serde_json::Value;
        use std::collections::HashMap;
        use std::convert::Infallible;
        use std::sync::Arc;
        use tokio::sync::{mpsc, RwLock};
        use tower_http::cors::{Any, CorsLayer};

        // Server state for managing SSE connections
        #[derive(Clone)]
        struct SseServerState {
            server: Arc<McpServer>,
            connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
            server_issued_sessions: Arc<RwLock<HashSet<String>>>,
            message_history: Arc<RwLock<HashMap<String, Vec<(String, String)>>>>,
        }

        impl SseServerState {
            fn new(server: Arc<McpServer>) -> Self {
                Self {
                    server: server.clone(),
                    connections: Arc::new(RwLock::new(HashMap::new())),
                    server_issued_sessions: server.server_issued_sessions.clone(),
                    message_history: server.message_history.clone(),
                }
            }
        }

        let state = SseServerState::new(Arc::new(self.clone()));

        // Create SSE endpoint handler
        async fn sse_handler(
            Path(session_id): Path<String>,
            State(state): State<SseServerState>,
        ) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
            tracing::info!("New SSE connection established for session: {}", session_id);

            let (tx, mut rx) = mpsc::unbounded_channel();

            // Store connection
            {
                let mut connections = state.connections.write().await;
                connections.insert(session_id.clone(), tx.clone());
            }

            // Send initial connection event
            let _ = tx.send("data: {\"type\":\"connection\",\"status\":\"connected\"}\n\n".to_string());

            let stream = async_stream::stream! {
                while let Some(data) = rx.recv().await {
                    yield Ok(Event::default().data(data));
                }
            };

            Sse::new(stream).keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(std::time::Duration::from_secs(30))
                    .text("keep-alive"),
            )
        }

        // Create message posting endpoint
        async fn post_message_handler(
            Path(session_id): Path<String>,
            State(state): State<SseServerState>,
            headers: HeaderMap,
            Json(payload): Json<Value>,
        ) -> Result<Json<Value>, StatusCode> {
            tracing::debug!("Received MCP message for session {}: {:?}", session_id, payload);

            // Extract auth header
            let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

            // Process the MCP request
            let message_str = serde_json::to_string(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;

            match state.server.handle_message(&message_str, auth_header).await {
                Ok(Some(response)) => {
                    // Send response via SSE
                    let response_data =
                        serde_json::to_string(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", response_data);
                        if tx.send(sse_data).is_err() {
                            tracing::warn!("Failed to send SSE response for session: {}", session_id);
                        }
                    }

                    Ok(Json(serde_json::json!({"status": "sent"})))
                }
                Ok(None) => {
                    // Notification, no response needed
                    Ok(Json(serde_json::json!({"status": "processed"})))
                }
                Err(e) => {
                    tracing::error!("Error processing MCP request: {}", e);

                    // Send error via SSE
                    let error_response = crate::protocol::JsonRpcResponse::error(
                        crate::protocol::JsonRpcError::internal_error(e.to_string()),
                        None,
                    );

                    let error_data =
                        serde_json::to_string(&error_response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", error_data);
                        let _ = tx.send(sse_data);
                    }

                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }

        // Health check endpoint
        async fn health_handler() -> Json<Value> {
            Json(serde_json::json!({
                "status": "healthy",
                "service": "ratchet-mcp-server",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        // CORS configuration
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION])
            .allow_credentials(false);

        // Build the app
        let app = Router::new()
            .route("/sse/:session_id", get(sse_handler))
            .route("/message/:session_id", post(post_message_handler))
            .route("/health", get(health_handler))
            .layer(cors)
            .with_state(state);

        // Start the server
        tracing::info!("SSE MCP server listening on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|e| McpError::ConnectionFailed {
                reason: format!("Failed to bind to {}: {}", bind_address, e),
            })?;

        // Axum 0.7 API for serving
        axum::serve(listener, app).await.map_err(|e| McpError::ServerError {
            message: format!("SSE server error: {}", e),
        })?;

        Ok(())
    }

    /// Create MCP SSE routes that can be nested into another Axum router
    pub fn create_sse_routes(&self) -> axum::Router {
        use axum::{
            extract::{Path, State},
            http::{HeaderMap, StatusCode},
            response::{sse::Event, Sse},
            routing::get,
            Json, Router,
        };
        use serde_json::Value;
        use std::collections::HashMap;
        use std::convert::Infallible;
        use std::sync::Arc;
        use tokio::sync::{mpsc, RwLock};

        // Server state for managing SSE connections
        #[derive(Clone)]
        struct SseServerState {
            server: Arc<McpServer>,
            connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
            server_issued_sessions: Arc<RwLock<HashSet<String>>>,
            message_history: Arc<RwLock<HashMap<String, Vec<(String, String)>>>>,
        }

        impl SseServerState {
            fn new(server: Arc<McpServer>) -> Self {
                Self {
                    server: server.clone(),
                    connections: Arc::new(RwLock::new(HashMap::new())),
                    server_issued_sessions: server.server_issued_sessions.clone(),
                    message_history: server.message_history.clone(),
                }
            }
        }

        let state = SseServerState::new(Arc::new(self.clone()));

        // Create SSE endpoint handler
        async fn sse_handler(
            Path(session_id): Path<String>,
            State(state): State<SseServerState>,
        ) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
            tracing::info!("New MCP SSE connection established for session: {}", session_id);

            let (tx, mut rx) = mpsc::unbounded_channel();

            // Store connection
            {
                let mut connections = state.connections.write().await;
                connections.insert(session_id.clone(), tx.clone());
            }

            // Send initial connection event
            let _ = tx.send("data: {\"type\":\"connection\",\"status\":\"connected\"}\n\n".to_string());

            let stream = async_stream::stream! {
                while let Some(data) = rx.recv().await {
                    yield Ok(Event::default().data(data));
                }
            };

            Sse::new(stream).keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(std::time::Duration::from_secs(30))
                    .text("keep-alive"),
            )
        }

        // Create message posting endpoint
        async fn post_message_handler(
            Path(session_id): Path<String>,
            State(state): State<SseServerState>,
            headers: HeaderMap,
            Json(payload): Json<Value>,
        ) -> Result<Json<Value>, StatusCode> {
            tracing::debug!("Received MCP message for session {}: {:?}", session_id, payload);

            // Extract auth header
            let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

            // Process the MCP request
            let message_str = serde_json::to_string(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;

            match state.server.handle_message(&message_str, auth_header).await {
                Ok(Some(response)) => {
                    // Send response via SSE
                    let response_data =
                        serde_json::to_string(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", response_data);
                        if tx.send(sse_data).is_err() {
                            tracing::warn!("Failed to send SSE response for session: {}", session_id);
                        }
                    }

                    Ok(Json(serde_json::json!({"status": "sent"})))
                }
                Ok(None) => {
                    // Notification, no response needed
                    Ok(Json(serde_json::json!({"status": "processed"})))
                }
                Err(e) => {
                    tracing::error!("Error processing MCP request: {}", e);

                    // Send error via SSE
                    let error_response = crate::protocol::JsonRpcResponse::error(
                        crate::protocol::JsonRpcError::internal_error(e.to_string()),
                        None,
                    );

                    let error_data =
                        serde_json::to_string(&error_response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", error_data);
                        let _ = tx.send(sse_data);
                    }

                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }

        // MCP health check endpoint
        async fn mcp_health_handler() -> Json<Value> {
            Json(serde_json::json!({
                "status": "healthy",
                "service": "ratchet-mcp-server",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        // Create new MCP session endpoint
        async fn create_session_handler() -> Json<Value> {
            let session_id = uuid::Uuid::new_v4().to_string();
            tracing::info!("Created new MCP session: {}", session_id);

            Json(serde_json::json!({
                "session_id": session_id,
                "sse_url": format!("/mcp/sse/{}", session_id),
                "message_url": format!("/mcp/message/{}", session_id),
                "created_at": chrono::Utc::now().to_rfc3339()
            }))
        }

        // MCP connection info endpoint (for clients that just want to connect)
        async fn connection_info_handler() -> Json<Value> {
            Json(serde_json::json!({
                "service": "ratchet-mcp-server",
                "version": env!("CARGO_PKG_VERSION"),
                "transport": "sse",
                "instructions": {
                    "step1": "GET /mcp/ with Accept: text/event-stream to establish SSE connection",
                    "step2": "POST /mcp/ with JSON-RPC messages and mcp-session-id header",
                    "step3": "Session ID is provided in response header for initialize requests"
                },
                "endpoints": {
                    "main": "GET/POST/DELETE /mcp/",
                    "health": "GET /mcp/health",
                    "info": "GET /mcp/info"
                },
                "claude_compatible": true
            }))
        }

        // MCP protocol compliant endpoint - handles GET, POST, DELETE
        async fn mcp_endpoint_handler(
            method: axum::http::Method,
            headers: HeaderMap,
            State(state): State<SseServerState>,
            body: Option<String>,
        ) -> impl axum::response::IntoResponse {
            use axum::http::{header, StatusCode};
            use axum::response::IntoResponse;

            // Validate Origin header for DNS rebinding protection
            if let Some(origin) = headers.get("origin").and_then(|h| h.to_str().ok()) {
                // Allow localhost origins and Claude application origins
                if !origin.starts_with("http://localhost")
                    && !origin.starts_with("https://localhost")
                    && !origin.starts_with("http://127.0.0.1")
                    && !origin.starts_with("https://127.0.0.1")
                    && !origin.starts_with("http://[::1]")
                    && !origin.starts_with("https://[::1]")
                    && !origin.starts_with("claude://")
                    && !origin.starts_with("vscode://")
                    && !origin.starts_with("code://")
                    && origin != "null" // Allow null origin for local applications
                {
                    tracing::warn!("Rejected request from disallowed origin: {}", origin);
                    return StatusCode::FORBIDDEN.into_response();
                }
            }

            // Extract session ID from headers if present
            let session_id = headers
                .get("mcp-session-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // Generate a new session ID if none provided
                    let new_id = uuid::Uuid::new_v4().to_string();
                    tracing::debug!("Generated new session ID: {}", new_id);
                    new_id
                });

            match method {
                // GET - Open SSE stream for server-to-client messages
                axum::http::Method::GET => {
                    // Check Accept header for text/event-stream
                    let accept_header = headers.get("accept").and_then(|h| h.to_str().ok()).unwrap_or("");
                    if !accept_header.contains("text/event-stream") {
                        return StatusCode::NOT_ACCEPTABLE.into_response();
                    }

                    // Check for Last-Event-ID header for resumability
                    let last_event_id = headers.get("last-event-id").and_then(|h| h.to_str().ok());

                    let (tx, mut rx) = mpsc::unbounded_channel();

                    // Store connection
                    {
                        let mut connections = state.connections.write().await;
                        connections.insert(session_id.clone(), tx.clone());
                    }

                    // Handle resumability - replay messages after last event ID
                    let mut starting_event_id = 0u64;
                    if let Some(last_id) = last_event_id {
                        tracing::debug!(
                            "Resuming MCP SSE connection from event ID: {} for session: {}",
                            last_id,
                            session_id
                        );

                        // Parse the last event ID
                        if let Ok(last_id_num) = last_id.parse::<u64>() {
                            starting_event_id = last_id_num;

                            // Replay messages from history
                            let history = state.message_history.read().await;
                            if let Some(messages) = history.get(&session_id) {
                                for (event_id, message) in messages {
                                    if let Ok(event_id_num) = event_id.parse::<u64>() {
                                        if event_id_num > last_id_num {
                                            // Send the historical message
                                            let _ = tx.send(message.clone());
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::info!("New MCP SSE connection established for session: {}", session_id);
                        
                        // Track this session as server-issued for future validation
                        {
                            let mut server_sessions = state.server_issued_sessions.write().await;
                            server_sessions.insert(session_id.clone());
                        }
                    }

                    let session_id_clone = session_id.clone();
                    let message_history = state.message_history.clone();

                    let stream = async_stream::stream! {
                        let mut event_counter = starting_event_id;
                        while let Some(data) = rx.recv().await {
                            event_counter += 1;
                            let event_id = event_counter.to_string();

                            // Store message in history for resumability
                            {
                                let mut history = message_history.write().await;
                                let session_history = history.entry(session_id_clone.clone()).or_insert_with(Vec::new);
                                session_history.push((event_id.clone(), data.clone()));

                                // Keep only last 1000 messages per session to prevent memory bloat
                                if session_history.len() > 1000 {
                                    session_history.drain(0..100);
                                }
                            }

                            // Add event ID for resumability support
                            let event = Event::default()
                                .data(data)
                                .id(event_id);
                            yield Ok::<Event, std::convert::Infallible>(event);
                        }
                    };

                    let sse_response = Sse::new(stream).keep_alive(
                        axum::response::sse::KeepAlive::new()
                            .interval(std::time::Duration::from_secs(30))
                            .text("keep-alive"),
                    );

                    sse_response.into_response()
                }

                // POST - Send JSON-RPC messages
                axum::http::Method::POST => {
                    let body = match body {
                        Some(b) => b,
                        None => return StatusCode::BAD_REQUEST.into_response(),
                    };

                    // Check Accept header - both application/json and text/event-stream should be supported
                    let accept_header = headers.get("accept").and_then(|h| h.to_str().ok()).unwrap_or("");
                    let accepts_json = accept_header.contains("application/json");
                    let accepts_sse = accept_header.contains("text/event-stream");

                    if !accepts_json && !accepts_sse {
                        return StatusCode::NOT_ACCEPTABLE.into_response();
                    }

                    tracing::debug!("MCP POST request received for session: {}, body: {}", session_id, body);

                    // Parse JSON-RPC message(s)
                    let json_value: Value = match serde_json::from_str(&body) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::error!("Failed to parse JSON-RPC message: {}", e);
                            return StatusCode::BAD_REQUEST.into_response();
                        }
                    };

                    // Check if this contains requests (vs only responses/notifications)
                    let contains_requests = if json_value.is_array() {
                        json_value
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|msg| msg.get("method").is_some() && msg.get("id").is_some())
                    } else {
                        json_value.get("method").is_some() && json_value.get("id").is_some()
                    };

                    // Check if this contains only responses or notifications
                    let only_responses_or_notifications = if json_value.is_array() {
                        json_value.as_array().unwrap().iter().all(|msg| {
                            // Response: has "result" or "error" field
                            // Notification: has "method" but no "id"
                            msg.get("result").is_some()
                                || msg.get("error").is_some()
                                || (msg.get("method").is_some() && msg.get("id").is_none())
                        })
                    } else {
                        json_value.get("result").is_some()
                            || json_value.get("error").is_some()
                            || (json_value.get("method").is_some() && json_value.get("id").is_none())
                    };

                    if only_responses_or_notifications && !contains_requests {
                        // Only responses/notifications - return 202 Accepted with no body
                        tracing::debug!("Received only responses/notifications for session: {}", session_id);
                        return StatusCode::ACCEPTED.into_response();
                    }

                    // Check if session ID is required for non-initialize requests
                    // For Claude compatibility, we'll be more lenient with session validation
                    let is_initialize = json_value.get("method").and_then(|m| m.as_str()) == Some("initialize");
                    
                    // For Claude compatibility, accept any session ID and track it
                    // This allows Claude to reconnect with the same session ID
                    if !is_initialize && headers.contains_key("mcp-session-id") {
                        let provided_session = headers
                            .get("mcp-session-id")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("");

                        // Always accept and track session IDs from Claude
                        if !provided_session.is_empty() {
                            let mut sessions = state.server_issued_sessions.write().await;
                            if !sessions.contains(provided_session) {
                                tracing::info!("Accepting new session ID from Claude: {}", provided_session);
                                sessions.insert(provided_session.to_string());
                            }
                        }
                    }

                    // Handle the message using the server (allow unauthenticated for development)
                    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
                    
                    // For development: accept connections without authentication
                    tracing::debug!("Processing MCP request (auth header present: {})", auth_header.is_some());

                    match state.server.handle_message(&body, auth_header).await {
                        Ok(Some(response)) => {
                            let response_data = match serde_json::to_string(&response) {
                                Ok(data) => data,
                                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                            };

                            if accepts_sse {
                                // Return SSE stream with response
                                let (tx, mut rx) = mpsc::unbounded_channel();

                                // Store connection
                                {
                                    let mut connections = state.connections.write().await;
                                    connections.insert(session_id.clone(), tx.clone());
                                }

                                // Send the response immediately with event ID
                                let _ = tx.send(format!("data: {}\n\n", response_data));

                                let stream = async_stream::stream! {
                                    let mut event_counter = 1u64;
                                    while let Some(data) = rx.recv().await {
                                        event_counter += 1;
                                        let event = Event::default()
                                            .data(data)
                                            .id(event_counter.to_string());
                                        yield Ok::<Event, std::convert::Infallible>(event);
                                    }
                                };

                                let sse_response = Sse::new(stream).keep_alive(
                                    axum::response::sse::KeepAlive::new()
                                        .interval(std::time::Duration::from_secs(30))
                                        .text("keep-alive"),
                                );

                                sse_response.into_response()
                            } else {
                                // Return JSON response with session header if this is an initialize request
                                let mut response_builder = axum::response::Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json");

                                // Check if this was an initialize request to add session header
                                if json_value.get("method").and_then(|m| m.as_str()) == Some("initialize") {
                                    response_builder = response_builder.header("mcp-session-id", &session_id);
                                    tracing::info!("Assigned session ID {} for initialize request", session_id);

                                    // Track that we issued this session ID
                                    {
                                        let mut server_sessions = state.server_issued_sessions.write().await;
                                        server_sessions.insert(session_id.clone());
                                    }
                                }

                                match response_builder.body(response_data) {
                                    Ok(response) => response.into_response(),
                                    Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                                }
                            }
                        }
                        Ok(None) => {
                            // No response expected - return 202
                            StatusCode::ACCEPTED.into_response()
                        }
                        Err(e) => {
                            tracing::error!("Error handling MCP message: {}", e);

                            let error_response = crate::protocol::JsonRpcResponse::error(
                                crate::protocol::JsonRpcError::internal_error(e.to_string()),
                                None,
                            );

                            let error_data = match serde_json::to_string(&error_response) {
                                Ok(data) => data,
                                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                            };

                            (
                                StatusCode::BAD_REQUEST,
                                [(header::CONTENT_TYPE, "application/json")],
                                error_data,
                            )
                                .into_response()
                        }
                    }
                }

                // DELETE - Terminate session
                axum::http::Method::DELETE => {
                    // Extract session ID from headers
                    let session_to_delete = headers
                        .get("mcp-session-id")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or(&session_id);

                    tracing::info!("MCP session termination requested for: {}", session_to_delete);

                    // Remove connection
                    {
                        let mut connections = state.connections.write().await;
                        connections.remove(session_to_delete);
                    }

                    // Remove from server-issued sessions
                    {
                        let mut server_sessions = state.server_issued_sessions.write().await;
                        server_sessions.remove(session_to_delete);
                    }

                    // Clean up message history
                    {
                        let mut history = state.message_history.write().await;
                        history.remove(session_to_delete);
                    }

                    StatusCode::NO_CONTENT.into_response()
                }

                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            }
        }

        // MCP endpoint handlers with proper method routing
        async fn mcp_get_handler(
            headers: HeaderMap,
            State(state): State<SseServerState>,
        ) -> impl axum::response::IntoResponse {
            mcp_endpoint_handler(axum::http::Method::GET, headers, State(state), None).await
        }

        async fn mcp_post_handler(
            headers: HeaderMap,
            State(state): State<SseServerState>,
            body: String,
        ) -> impl axum::response::IntoResponse {
            mcp_endpoint_handler(axum::http::Method::POST, headers, State(state), Some(body)).await
        }

        async fn mcp_delete_handler(
            headers: HeaderMap,
            State(state): State<SseServerState>,
        ) -> impl axum::response::IntoResponse {
            mcp_endpoint_handler(axum::http::Method::DELETE, headers, State(state), None).await
        }

        // OAuth endpoints for Claude compatibility
        async fn oauth_register_handler(
            Json(payload): Json<serde_json::Value>,
        ) -> Result<axum::response::Json<serde_json::Value>, StatusCode> {
            tracing::info!("OAuth2 Dynamic Client Registration request: {:?}", payload);
            
            // Generate a client ID for Claude
            let client_id = uuid::Uuid::new_v4().to_string();
            
            // Extract redirect_uris from the request payload, or provide defaults
            let redirect_uris = payload
                .get("redirect_uris")
                .and_then(|uris| uris.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<String>>())
                .unwrap_or_else(|| vec![
                    "http://localhost:60339/callback".to_string(),
                    "http://127.0.0.1:60339/callback".to_string(),
                    "claude://oauth/callback".to_string(),
                ]);
            
            // Return a successful registration response with all required fields
            Ok(axum::response::Json(serde_json::json!({
                "client_id": client_id,
                "client_id_issued_at": chrono::Utc::now().timestamp(),
                "grant_types": ["authorization_code"],
                "response_types": ["code"],
                "redirect_uris": redirect_uris,
                "scope": "mcp",
                "token_endpoint_auth_method": "none",
                "application_type": "native"
            })))
        }
        
        async fn oauth_token_handler(
            headers: HeaderMap,
            payload: String,
        ) -> Result<axum::response::Json<serde_json::Value>, StatusCode> {
            tracing::info!("OAuth2 token request headers: {:?}", headers);
            tracing::info!("OAuth2 token request body: {}", payload);
            
            // Parse the payload - OAuth2 token requests are typically form-encoded
            let parsed_data: std::collections::HashMap<String, String> = if headers
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .map(|ct| ct.contains("application/x-www-form-urlencoded"))
                .unwrap_or(false)
            {
                // Parse form-encoded data
                serde_urlencoded::from_str(&payload).map_err(|e| {
                    tracing::error!("Failed to parse form data: {}", e);
                    StatusCode::BAD_REQUEST
                })?
            } else {
                // Try to parse as JSON
                let json_payload: serde_json::Value = serde_json::from_str(&payload).map_err(|e| {
                    tracing::error!("Failed to parse JSON: {}", e);
                    StatusCode::BAD_REQUEST
                })?;
                
                // Convert JSON to HashMap
                match json_payload.as_object() {
                    Some(obj) => obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect(),
                    None => return Err(StatusCode::BAD_REQUEST),
                }
            };
            
            // Validate grant_type
            let grant_type = parsed_data.get("grant_type")
                .ok_or(StatusCode::BAD_REQUEST)?;
                
            if grant_type != "authorization_code" {
                tracing::error!("Invalid grant_type: {}", grant_type);
                return Err(StatusCode::BAD_REQUEST);
            }
            
            // Validate authorization code
            let code = parsed_data.get("code")
                .ok_or(StatusCode::BAD_REQUEST)?;
                
            if !code.starts_with("ratchet-auth-") {
                tracing::warn!("Invalid authorization code: {}", code);
                return Err(StatusCode::UNAUTHORIZED);
            }
            
            // PKCE validation (optional for now - in production you'd verify the code_verifier)
            if let Some(code_verifier) = parsed_data.get("code_verifier") {
                tracing::info!("Received PKCE code_verifier: {}", code_verifier);
                // In a real implementation, you would:
                // 1. Retrieve the stored code_challenge for this authorization code
                // 2. Hash the code_verifier using SHA256
                // 3. Base64-URL encode the result
                // 4. Compare with the stored code_challenge
                // For now, we'll just log it and continue
            }
            
            // Generate access token for Claude
            let access_token = format!("ratchet-token-{}", uuid::Uuid::new_v4());
            
            tracing::info!("Generated access token for authorization code: {}", code);
            
            // Return successful token response
            Ok(axum::response::Json(serde_json::json!({
                "access_token": access_token,
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "mcp"
            })))
        }
        
        async fn oauth_authorize_handler(
            query: axum::extract::Query<std::collections::HashMap<String, String>>,
        ) -> Result<axum::response::Redirect, StatusCode> {
            tracing::info!("OAuth2 authorization request: {:?}", query);
            
            // Generate authorization code
            let auth_code = format!("ratchet-auth-{}", uuid::Uuid::new_v4());
            
            // Get the redirect URI and state from query parameters
            let redirect_uri = query.get("redirect_uri")
                .ok_or(StatusCode::BAD_REQUEST)?;
            let state = query.get("state").cloned().unwrap_or_default();
            
            // Build redirect URL with authorization code
            let redirect_url = if redirect_uri.contains('?') {
                format!("{}&code={}&state={}", redirect_uri, auth_code, state)
            } else {
                format!("{}?code={}&state={}", redirect_uri, auth_code, state)
            };
            
            tracing::info!("Redirecting to: {}", redirect_url);
            Ok(axum::response::Redirect::temporary(&redirect_url))
        }

        // Build the MCP routes - single endpoint as per protocol
        Router::new()
            .route(
                "/",
                get(mcp_get_handler).post(mcp_post_handler).delete(mcp_delete_handler),
            )
            .route("/health", get(mcp_health_handler)) // Keep health for debugging
            .route("/info", get(connection_info_handler)) // Keep info for debugging
            // OAuth stub endpoints to prevent 404 errors
            .route("/oauth/register", axum::routing::post(oauth_register_handler))
            .route("/oauth/token", axum::routing::post(oauth_token_handler))
            .route("/oauth/authorize", get(oauth_authorize_handler))
            // Add a simple no-auth endpoint for development
            .route("/direct", get(mcp_get_handler).post(mcp_post_handler))
            .with_state(state)
    }

    /// Handle an incoming message
    pub async fn handle_message(&self, message: &str, auth_header: Option<&str>) -> McpResult<Option<JsonRpcResponse>> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_str(message).map_err(|e| McpError::InvalidJsonRpc {
            details: format!("Failed to parse JSON-RPC request: {}", e),
        })?;

        // Handle the request
        self.handle_request(request, auth_header).await
    }

    /// Handle a JSON-RPC request
    async fn handle_request(
        &self,
        request: JsonRpcRequest,
        auth_header: Option<&str>,
    ) -> McpResult<Option<JsonRpcResponse>> {
        let request_id = request.id.clone();

        // If this is a notification (no ID), don't send a response
        if request.is_notification() {
            self.handle_notification(request, auth_header).await?;
            return Ok(None);
        }

        // Handle the request and create response
        match self.process_request(request, auth_header).await {
            Ok(result) => Ok(Some(JsonRpcResponse::success(result, request_id))),
            Err(e) => {
                let json_rpc_error = match e {
                    McpError::MethodNotFound { method } => JsonRpcError::method_not_found(&method),
                    McpError::InvalidParams { details, .. } => JsonRpcError::invalid_params(details),
                    McpError::AuthenticationFailed { reason } => JsonRpcError::server_error(
                        -32001,
                        "Authentication failed",
                        Some(serde_json::Value::String(reason)),
                    ),
                    McpError::AuthorizationDenied { reason } => JsonRpcError::server_error(
                        -32002,
                        "Authorization denied",
                        Some(serde_json::Value::String(reason)),
                    ),
                    _ => JsonRpcError::internal_error(e.to_string()),
                };

                Ok(Some(JsonRpcResponse::error(json_rpc_error, request_id)))
            }
        }
    }

    /// Process a request (not a notification)
    async fn process_request(
        &self,
        request: JsonRpcRequest,
        auth_header: Option<&str>,
    ) -> McpResult<serde_json::Value> {
        // Create request handler
        let handler = McpRequestHandler::new(
            self.tool_registry.clone(),
            self.auth_manager.clone(),
            self.audit_logger.clone(),
            &self.config,
            self.correlation_manager.clone(),
            self.metrics.clone(),
        );

        // Handle the specific method
        match request.method.as_str() {
            "initialize" => {
                let params: InitializeParams = if let Some(params) = request.params {
                    serde_json::from_value(params)?
                } else {
                    return Err(McpError::InvalidParams {
                        method: "initialize".to_string(),
                        details: "Missing initialization parameters".to_string(),
                    });
                };

                let result = self.handle_initialize(params).await?;
                Ok(serde_json::to_value(result)?)
            }

            "tools/list" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "tools/list")
                    .await?;
                handler.handle_tools_list(request.params, &security_ctx).await
            }

            "tools/call" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "tools/call")
                    .await?;
                handler.handle_tools_call(request.params, &security_ctx).await
            }

            "resources/list" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "resources/list")
                    .await?;
                handler.handle_resources_list(request.params, &security_ctx).await
            }

            "resources/read" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "resources/read")
                    .await?;
                handler.handle_resources_read(request.params, &security_ctx).await
            }

            method => Err(McpError::MethodNotFound {
                method: method.to_string(),
            }),
        }
    }

    /// Handle a notification (no response expected)
    async fn handle_notification(&self, request: JsonRpcRequest, _auth_header: Option<&str>) -> McpResult<()> {
        match request.method.as_str() {
            "initialized" => {
                let mut initialized = self.initialized.write().await;
                if !*initialized {
                    *initialized = true;
                    tracing::info!("MCP server initialized via notification");
                } else {
                    tracing::debug!("Received initialized notification but server was already initialized");
                }
                Ok(())
            }
            "notifications/cancelled" => {
                // Handle cancellation notification
                tracing::debug!("Received cancellation notification");
                Ok(())
            }
            method => {
                tracing::warn!("Unknown notification method: {}", method);
                Ok(())
            }
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, params: InitializeParams) -> McpResult<InitializeResult> {
        let client_name = params
            .client_info
            .as_ref()
            .map(|info| info.name.as_str())
            .unwrap_or("Unknown Client");

        tracing::info!(
            "Initializing MCP server with client: {} (protocol: {})",
            client_name,
            params.protocol_version
        );

        // Validate protocol version
        if !crate::protocol::validate_protocol_version(&params.protocol_version) {
            return Err(McpError::Protocol {
                message: format!(
                    "Unsupported protocol version: {}. Supported versions: {:?}",
                    params.protocol_version,
                    crate::protocol::SUPPORTED_PROTOCOL_VERSIONS
                ),
            });
        }

        // Mark server as initialized immediately after successful initialize request
        // This is more compatible with clients that don't send the initialized notification
        {
            let mut initialized = self.initialized.write().await;
            *initialized = true;
            tracing::info!("MCP server marked as initialized after initialize request");
        }

        // Build server capabilities
        let capabilities = ServerCapabilities {
            experimental: HashMap::new(),
            logging: None,   // TODO: Add logging capability
            prompts: None,   // TODO: Add prompts capability
            resources: None, // TODO: Add resources capability
            tools: Some(crate::protocol::ToolsCapability { list_changed: false }),
            batch: Some(crate::protocol::BatchCapability {
                max_batch_size: 100,
                max_parallel: 10,
                supports_dependencies: true,
                supports_progress: true,
                supported_execution_modes: vec![
                    crate::protocol::BatchExecutionMode::Parallel,
                    crate::protocol::BatchExecutionMode::Sequential,
                    crate::protocol::BatchExecutionMode::Dependency,
                    crate::protocol::BatchExecutionMode::PriorityDependency,
                ],
            }),
        };

        let server_info = ServerInfo {
            name: "Ratchet MCP Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: HashMap::new(),
        };

        // Use the client's protocol version if supported, otherwise use our default
        let negotiated_version = crate::protocol::get_protocol_version_for_client(&params.protocol_version);

        Ok(InitializeResult {
            protocol_version: negotiated_version,
            capabilities,
            server_info,
        })
    }

    /// Authenticate and authorize a request
    async fn authenticate_and_authorize(
        &self,
        request: &JsonRpcRequest,
        auth_header: Option<&str>,
        operation: &str,
    ) -> McpResult<SecurityContext> {
        // Check if server is initialized
        let initialized = *self.initialized.read().await;
        if !initialized {
            return Err(McpError::Protocol {
                message: "Server not initialized. Send 'initialize' request first.".to_string(),
            });
        }

        // For development: allow unauthenticated access
        let client_context = if auth_header.is_some() {
            // Try to authenticate if auth header is present
            match self.auth_manager.authenticate(auth_header).await {
                Ok(ctx) => ctx,
                Err(_) => {
                    // If auth fails, create anonymous context for development
                    tracing::warn!("Authentication failed, using anonymous access for development");
                    crate::security::ClientContext {
                        id: "anonymous".to_string(),
                        name: "Anonymous User".to_string(),
                        permissions: crate::security::ClientPermissions::full_access(),
                        authenticated_at: chrono::Utc::now(),
                        session_id: uuid::Uuid::new_v4().to_string(),
                    }
                }
            }
        } else {
            // No auth header - create anonymous context for development
            tracing::debug!("No authentication provided, using anonymous access for development");
            crate::security::ClientContext {
                id: "anonymous".to_string(),
                name: "Anonymous User".to_string(),
                permissions: crate::security::ClientPermissions::full_access(),
                authenticated_at: chrono::Utc::now(),
                session_id: uuid::Uuid::new_v4().to_string(),
            }
        };

        // Create security context
        let security_context = SecurityContext::new(client_context, self.config.security.clone());

        // Log the operation
        self.audit_logger
            .log_authorization(
                &security_context.client.id,
                operation,
                "mcp_request",
                true, // TODO: Add actual authorization checks
                request.id_as_string(),
            )
            .await;

        Ok(security_context)
    }
}

/// Builder for creating MCP server with fluent API
pub struct McpServerBuilder {
    config: Option<McpServerConfig>,
    tool_registry: Option<Arc<dyn ToolRegistry>>,
    auth_manager: Option<Arc<McpAuthManager>>,
    audit_logger: Option<Arc<AuditLogger>>,
    security_config: Option<crate::security::SecurityConfig>,
}

impl McpServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            tool_registry: None,
            auth_manager: None,
            audit_logger: None,
            security_config: None,
        }
    }

    /// Set the server configuration
    pub fn with_config(mut self, config: McpServerConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the tool registry
    pub fn with_tool_registry(mut self, registry: Arc<dyn ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Set the authentication manager
    pub fn with_auth_manager(mut self, auth_manager: Arc<McpAuthManager>) -> Self {
        self.auth_manager = Some(auth_manager);
        self
    }

    /// Set the audit logger
    pub fn with_audit_logger(mut self, audit_logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(audit_logger);
        self
    }

    /// Set the security configuration
    pub fn with_security(mut self, security: crate::security::SecurityConfig) -> Self {
        self.security_config = Some(security);
        self
    }

    /// Build the MCP server
    pub fn build(self) -> McpResult<McpServer> {
        let config = self.config.unwrap_or_else(|| McpServerConfig {
            transport: McpServerTransport::Stdio,
            security: self.security_config.clone().unwrap_or_default(),
            bind_address: None,
        });

        let tool_registry = self.tool_registry.ok_or_else(|| McpError::Configuration {
            message: "Tool registry is required".to_string(),
        })?;

        let auth_manager = self
            .auth_manager
            .unwrap_or_else(|| Arc::new(McpAuthManager::new(McpAuth::None)));

        let audit_logger = self.audit_logger.unwrap_or_else(|| Arc::new(AuditLogger::new(false)));

        Ok(McpServer::new(config, tool_registry, auth_manager, audit_logger))
    }
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{McpAuth, SecurityConfig};
    use std::collections::HashMap;

    fn create_test_server() -> McpServer {
        let config = McpServerConfig {
            transport: McpServerTransport::Stdio,
            security: SecurityConfig::default(),
            bind_address: None,
        };

        let tool_registry = Arc::new(RatchetToolRegistry::new());
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::None));
        let audit_logger = Arc::new(AuditLogger::new(false));

        McpServer::new(config, tool_registry, auth_manager, audit_logger)
    }

    #[tokio::test]
    async fn test_initialize() {
        let server = create_test_server();

        let params = InitializeParams {
            protocol_version: crate::protocol::MCP_PROTOCOL_VERSION.to_string(),
            capabilities: crate::protocol::ClientCapabilities::default(),
            client_info: Some(crate::protocol::ClientInfo {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
                metadata: HashMap::new(),
            }),
        };

        let result = server.handle_initialize(params).await;
        assert!(result.is_ok());

        let init_result = result.unwrap();
        assert_eq!(init_result.server_info.name, "Ratchet MCP Server");
    }

    #[tokio::test]
    async fn test_invalid_protocol_version() {
        let server = create_test_server();

        let params = InitializeParams {
            protocol_version: "999.0.0".to_string(),
            capabilities: crate::protocol::ClientCapabilities::default(),
            client_info: Some(crate::protocol::ClientInfo {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
                metadata: HashMap::new(),
            }),
        };

        let result = server.handle_initialize(params).await;
        assert!(result.is_err());
    }
}
