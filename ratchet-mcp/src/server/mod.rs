//! MCP server implementation for exposing Ratchet capabilities to LLMs

pub mod adapter;
pub mod batch;
pub mod config;
pub mod handler;
pub mod progress;
pub mod service;
pub mod tools;

pub use adapter::{RatchetMcpAdapter, RatchetMcpAdapterBuilder};
pub use batch::BatchProcessor;
pub use config::{McpServerConfig, McpServerTransport};
pub use handler::McpRequestHandler;
pub use service::{McpService, McpServiceBuilder, McpServiceConfig};
pub use tools::{McpTaskExecutor, McpTaskInfo, McpTool, RatchetToolRegistry, ToolRegistry};

// Main server types are defined in this module, no need to re-export

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{
    InitializeParams, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    ServerCapabilities, ServerInfo,
};
use crate::security::{AuditLogger, McpAuthManager, SecurityContext};
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

    /// Active client sessions
    _sessions: Arc<RwLock<HashMap<String, SecurityContext>>>,

    /// Whether the server is initialized
    initialized: Arc<RwLock<bool>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(
        config: McpServerConfig,
        tool_registry: Arc<dyn ToolRegistry>,
        auth_manager: Arc<McpAuthManager>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self {
            config,
            tool_registry,
            auth_manager,
            audit_logger,
            _sessions: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a new MCP server with adapter
    pub async fn with_adapter(
        config: crate::config::McpConfig,
        adapter: RatchetMcpAdapter,
    ) -> McpResult<Self> {
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
                        allowed_methods: vec![
                            "GET".to_string(),
                            "POST".to_string(),
                            "OPTIONS".to_string(),
                        ],
                        allowed_headers: vec![
                            "Content-Type".to_string(),
                            "Authorization".to_string(),
                        ],
                        allow_credentials: false,
                    },
                    timeout: config.timeouts.request_timeout,
                },
            },
            security: crate::security::SecurityConfig::default(),
            bind_address: Some(format!("{}:{}", config.host, config.port)),
        };

        Ok(Self {
            config: server_config,
            tool_registry: Arc::new(tool_registry),
            auth_manager,
            audit_logger,
            _sessions: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
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
    pub async fn run(
        &self,
        mut transport: Box<dyn crate::transport::McpTransport>,
    ) -> McpResult<()> {
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
                            let error_response = JsonRpcResponse::error(
                                JsonRpcError::internal_error(e.to_string()),
                                request.id.clone(),
                            );
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
                            tracing::debug!(
                                "Sending MCP response #{}: {}",
                                request_count,
                                response_json
                            );
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Ok(None) => {
                            tracing::debug!(
                                "MCP request #{} was a notification, no response sent",
                                request_count
                            );
                        }
                        Err(e) => {
                            tracing::error!("Error handling MCP request #{}: {}", request_count, e);
                            // Send error response if possible
                            let error_response = JsonRpcResponse::error(
                                JsonRpcError::internal_error(e.to_string()),
                                None,
                            );
                            let response_json = serde_json::to_string(&error_response)?;
                            tracing::debug!(
                                "Sending MCP error response #{}: {}",
                                request_count,
                                response_json
                            );
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

        tracing::info!(
            "MCP server stdio loop terminated after {} requests",
            request_count
        );
        Ok(())
    }

    /// Start SSE-based MCP server
    async fn start_sse_server(&self, bind_address: &str) -> McpResult<()> {
        tracing::info!("Starting MCP server with SSE transport on {}", bind_address);

        use axum::{
            extract::{Path, State},
            http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
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
        }

        impl SseServerState {
            fn new(server: Arc<McpServer>) -> Self {
                Self {
                    server,
                    connections: Arc::new(RwLock::new(HashMap::new())),
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
            let _ =
                tx.send("data: {\"type\":\"connection\",\"status\":\"connected\"}\n\n".to_string());

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
            tracing::debug!(
                "Received MCP message for session {}: {:?}",
                session_id,
                payload
            );

            // Extract auth header
            let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

            // Process the MCP request
            let message_str =
                serde_json::to_string(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;

            match state.server.handle_message(&message_str, auth_header).await {
                Ok(Some(response)) => {
                    // Send response via SSE
                    let response_data = serde_json::to_string(&response)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", response_data);
                        if tx.send(sse_data).is_err() {
                            tracing::warn!(
                                "Failed to send SSE response for session: {}",
                                session_id
                            );
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

                    let error_data = serde_json::to_string(&error_response)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
            .allow_headers([CONTENT_TYPE, axum::http::header::AUTHORIZATION])
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

        // Axum 0.6 API for serving
        axum::Server::from_tcp(listener.into_std().map_err(|e| McpError::ServerError {
            message: format!("Failed to convert listener: {}", e),
        })?)
        .map_err(|e| McpError::ServerError {
            message: format!("Failed to create server: {}", e),
        })?
        .serve(app.into_make_service())
        .await
        .map_err(|e| McpError::ServerError {
            message: format!("SSE server error: {}", e),
        })?;

        Ok(())
    }

    /// Create MCP SSE routes that can be nested into another Axum router
    pub fn create_sse_routes(&self) -> axum::Router {
        use axum::{
            extract::{Path, State},
            http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
            response::{sse::Event, Sse},
            routing::{get, post},
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
        }

        impl SseServerState {
            fn new(server: Arc<McpServer>) -> Self {
                Self {
                    server,
                    connections: Arc::new(RwLock::new(HashMap::new())),
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
            let _ =
                tx.send("data: {\"type\":\"connection\",\"status\":\"connected\"}\n\n".to_string());

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
            tracing::debug!(
                "Received MCP message for session {}: {:?}",
                session_id,
                payload
            );

            // Extract auth header
            let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

            // Process the MCP request
            let message_str =
                serde_json::to_string(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;

            match state.server.handle_message(&message_str, auth_header).await {
                Ok(Some(response)) => {
                    // Send response via SSE
                    let response_data = serde_json::to_string(&response)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    let connections = state.connections.read().await;
                    if let Some(tx) = connections.get(&session_id) {
                        let sse_data = format!("data: {}\n\n", response_data);
                        if tx.send(sse_data).is_err() {
                            tracing::warn!(
                                "Failed to send SSE response for session: {}",
                                session_id
                            );
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

                    let error_data = serde_json::to_string(&error_response)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

        // Build the MCP routes
        Router::new()
            .route("/sse/:session_id", get(sse_handler))
            .route("/message/:session_id", post(post_message_handler))
            .route("/health", get(mcp_health_handler))
            .with_state(state)
    }

    /// Handle an incoming message
    pub async fn handle_message(
        &self,
        message: &str,
        auth_header: Option<&str>,
    ) -> McpResult<Option<JsonRpcResponse>> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest =
            serde_json::from_str(message).map_err(|e| McpError::InvalidJsonRpc {
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
                    McpError::InvalidParams { details, .. } => {
                        JsonRpcError::invalid_params(details)
                    }
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
                handler
                    .handle_tools_list(request.params, &security_ctx)
                    .await
            }

            "tools/call" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "tools/call")
                    .await?;
                handler
                    .handle_tools_call(request.params, &security_ctx)
                    .await
            }

            "resources/list" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "resources/list")
                    .await?;
                handler
                    .handle_resources_list(request.params, &security_ctx)
                    .await
            }

            "resources/read" => {
                let security_ctx = self
                    .authenticate_and_authorize(&request, auth_header, "resources/read")
                    .await?;
                handler
                    .handle_resources_read(request.params, &security_ctx)
                    .await
            }

            method => Err(McpError::MethodNotFound {
                method: method.to_string(),
            }),
        }
    }

    /// Handle a notification (no response expected)
    async fn handle_notification(
        &self,
        request: JsonRpcRequest,
        _auth_header: Option<&str>,
    ) -> McpResult<()> {
        match request.method.as_str() {
            "initialized" => {
                let mut initialized = self.initialized.write().await;
                if !*initialized {
                    *initialized = true;
                    tracing::info!("MCP server initialized via notification");
                } else {
                    tracing::debug!(
                        "Received initialized notification but server was already initialized"
                    );
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
        tracing::info!(
            "Initializing MCP server with client: {}",
            params.client_info.name
        );

        // Validate protocol version
        if !crate::protocol::validate_protocol_version(&params.protocol_version) {
            return Err(McpError::Protocol {
                message: format!("Unsupported protocol version: {}", params.protocol_version),
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
            tools: Some(crate::protocol::ToolsCapability {
                list_changed: false,
            }),
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

        Ok(InitializeResult {
            protocol_version: crate::protocol::MCP_PROTOCOL_VERSION.to_string(),
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

        // Authenticate the client
        let client_context = self.auth_manager.authenticate(auth_header).await?;

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

        let audit_logger = self
            .audit_logger
            .unwrap_or_else(|| Arc::new(AuditLogger::new(false)));

        Ok(McpServer::new(
            config,
            tool_registry,
            auth_manager,
            audit_logger,
        ))
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
            client_info: crate::protocol::ClientInfo {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
                metadata: HashMap::new(),
            },
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
            client_info: crate::protocol::ClientInfo {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
                metadata: HashMap::new(),
            },
        };

        let result = server.handle_initialize(params).await;
        assert!(result.is_err());
    }
}
