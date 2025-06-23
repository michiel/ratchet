//! MCP endpoint handlers supporting both SSE and StreamableHTTP transports

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, warn};

use crate::config::{McpApiConfig, McpTransportMode};

#[cfg(feature = "mcp")]
use ratchet_mcp::{
    server::{McpServer, RatchetToolRegistry},
    transport::streamable_http::{
        EventStore, InMemoryEventStore, SessionManager, StreamableHttpTransport,
    },
    security::{AuditLogger, McpAuth, McpAuthManager},
    server::McpServerConfig,
};

/// MCP endpoint state for handling both SSE and StreamableHTTP
#[derive(Clone)]
pub struct McpEndpointState {
    pub config: McpApiConfig,
    #[cfg(feature = "mcp")]
    pub mcp_server: Arc<McpServer>,
    #[cfg(feature = "mcp")]
    pub session_manager: Option<Arc<SessionManager>>,
    #[cfg(feature = "mcp")]
    pub streamable_transport: Option<Arc<tokio::sync::Mutex<StreamableHttpTransport>>>,
}

impl McpEndpointState {
    #[cfg(feature = "mcp")]
    pub fn new(config: McpApiConfig) -> anyhow::Result<Self> {
        // Create MCP server
        let mcp_server_config = McpServerConfig::sse_with_host(config.port, &config.host);
        let tool_registry = RatchetToolRegistry::new();
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::default()));
        let audit_logger = Arc::new(AuditLogger::new(false));

        let mcp_server = Arc::new(McpServer::new(
            mcp_server_config,
            Arc::new(tool_registry),
            auth_manager,
            audit_logger,
        ));

        // Create session manager for StreamableHTTP if needed
        let (session_manager, streamable_transport) = match config.transport {
            McpTransportMode::StreamableHttp | McpTransportMode::Both => {
                let event_store = Arc::new(InMemoryEventStore::new(
                    config.max_events_per_session,
                    Duration::from_secs(config.session_timeout_minutes as u64 * 60),
                )) as Arc<dyn EventStore>;

                let session_manager = Arc::new(SessionManager::new(
                    event_store,
                    Duration::from_secs(config.session_timeout_minutes as u64 * 60),
                    Duration::from_secs(60), // cleanup interval
                ));

                let streamable_transport = Arc::new(tokio::sync::Mutex::new(
                    StreamableHttpTransport::new(Arc::clone(&session_manager)),
                ));

                (Some(session_manager), Some(streamable_transport))
            }
            McpTransportMode::Sse => (None, None),
        };

        Ok(Self {
            config,
            mcp_server,
            session_manager,
            streamable_transport,
        })
    }

    #[cfg(not(feature = "mcp"))]
    pub fn new(config: McpApiConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }
}

/// GET handler for MCP endpoint
pub async fn mcp_get_handler(
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    State(state): State<McpEndpointState>,
) -> Result<Response, StatusCode> {
    handle_mcp_request(axum::http::Method::GET, headers, query, state, None).await
}

/// POST handler for MCP endpoint  
pub async fn mcp_post_handler(
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    State(state): State<McpEndpointState>,
    body: axum::body::Bytes,
) -> Result<Response, StatusCode> {
    handle_mcp_request(axum::http::Method::POST, headers, query, state, Some(body.to_vec())).await
}

/// DELETE handler for MCP endpoint
pub async fn mcp_delete_handler(
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    State(state): State<McpEndpointState>,
) -> Result<Response, StatusCode> {
    handle_mcp_request(axum::http::Method::DELETE, headers, query, state, None).await
}

/// Internal handler that routes to appropriate transport
async fn handle_mcp_request(
    method: axum::http::Method,
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    state: McpEndpointState,
    body: Option<Vec<u8>>,
) -> Result<Response, StatusCode> {
    match determine_transport_type(&method, &headers, &state.config) {
        TransportType::Sse => handle_sse_request(method, headers, query, state, body).await,
        TransportType::StreamableHttp => {
            handle_streamable_http_request(method, headers, query, state, body).await
        }
        TransportType::Unsupported => {
            error!("Unsupported transport type for method: {}", method);
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
    }
}

#[derive(Debug)]
enum TransportType {
    Sse,
    StreamableHttp,
    Unsupported,
}

fn determine_transport_type(
    method: &axum::http::Method,
    headers: &HeaderMap,
    config: &McpApiConfig,
) -> TransportType {
    // Check for StreamableHTTP indicators
    let has_session_id = headers.contains_key("mcp-session-id");
    let has_last_event_id = headers.contains_key("last-event-id");
    let is_claude_request = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|ua| ua.contains("Claude"))
        .unwrap_or(false);

    match (&config.transport, method.as_str()) {
        // StreamableHTTP mode
        (McpTransportMode::StreamableHttp, "GET") if has_session_id || has_last_event_id => {
            TransportType::StreamableHttp
        }
        (McpTransportMode::StreamableHttp, "POST") => TransportType::StreamableHttp,
        (McpTransportMode::StreamableHttp, "DELETE") if has_session_id => {
            TransportType::StreamableHttp
        }

        // SSE mode  
        (McpTransportMode::Sse, "GET") => TransportType::Sse,
        (McpTransportMode::Sse, "POST") => TransportType::Sse,
        (McpTransportMode::Sse, "DELETE") => TransportType::Sse,

        // Both mode - prefer StreamableHTTP for Claude/session-based requests
        (McpTransportMode::Both, "GET") if has_session_id || has_last_event_id || is_claude_request => {
            TransportType::StreamableHttp
        }
        (McpTransportMode::Both, "POST") if has_session_id || is_claude_request => {
            TransportType::StreamableHttp
        }
        (McpTransportMode::Both, "DELETE") if has_session_id => TransportType::StreamableHttp,
        (McpTransportMode::Both, _) => TransportType::Sse, // Default to SSE

        _ => TransportType::Unsupported,
    }
}

#[cfg(feature = "mcp")]
async fn handle_sse_request(
    method: axum::http::Method,
    _headers: HeaderMap,
    _query: Query<HashMap<String, String>>,
    _state: McpEndpointState,
    body: Option<Vec<u8>>,
) -> Result<Response, StatusCode> {
    debug!("Handling SSE request with method: {}", method);
    
    // For now, we need to implement proper JSON-RPC handling
    // Since we can't easily delegate to the routes here, let's handle the basic cases
    
    match method.as_str() {
        "GET" => {
            // For GET requests, we should establish an SSE connection
            // This is a simplified implementation - in production, you'd want to 
            // properly handle the SSE streaming
            use axum::response::sse::{Event, Sse};
            use futures_util::stream;
            
            let stream = stream::iter(vec![
                Ok::<_, std::convert::Infallible>(Event::default().data("SSE connection established"))
            ]);
            
            Ok(Sse::new(stream).into_response())
        }
        "POST" => {
            // For POST requests, parse and handle JSON-RPC
            let request_body = body.unwrap_or_default();
            
            if request_body.is_empty() {
                // Return JSON-RPC error for empty request
                return Ok(Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32600,
                        "message": "Invalid Request"
                    },
                    "id": null
                })).into_response());
            }
            
            // Try to parse as JSON-RPC request
            match serde_json::from_slice::<serde_json::Value>(&request_body) {
                Ok(request_json) => {
                    // Extract the request ID for the response
                    let request_id = request_json.get("id").cloned().unwrap_or(serde_json::Value::Null);
                    let method_name = request_json.get("method").and_then(|m| m.as_str()).unwrap_or("");
                    
                    match method_name {
                        "initialize" => {
                            // Return proper initialization response
                            Ok(Json(serde_json::json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "protocolVersion": "2024-11-05",
                                    "capabilities": {
                                        "tools": {},
                                        "resources": {},
                                        "logging": {}
                                    },
                                    "serverInfo": {
                                        "name": "ratchet-mcp-server",
                                        "version": env!("CARGO_PKG_VERSION")
                                    }
                                },
                                "id": request_id
                            })).into_response())
                        }
                        "tools/list" => {
                            // Return available tools
                            Ok(Json(serde_json::json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "tools": [
                                        {
                                            "name": "ratchet.execute_task",
                                            "description": "Execute a Ratchet task with given input and optional progress streaming",
                                            "inputSchema": {
                                                "type": "object",
                                                "properties": {
                                                    "task_id": {
                                                        "type": "string",
                                                        "description": "ID or name of the task to execute"
                                                    },
                                                    "input": {
                                                        "type": "object",
                                                        "description": "Input data for the task"
                                                    }
                                                },
                                                "required": ["task_id", "input"]
                                            }
                                        },
                                        {
                                            "name": "ratchet.list_available_tasks",
                                            "description": "List all available tasks with their schemas and pagination support",
                                            "inputSchema": {
                                                "type": "object",
                                                "properties": {
                                                    "limit": {
                                                        "type": "integer",
                                                        "description": "Maximum number of tasks to return",
                                                        "default": 50
                                                    },
                                                    "page": {
                                                        "type": "integer", 
                                                        "description": "Page number for pagination",
                                                        "default": 0
                                                    }
                                                }
                                            }
                                        }
                                    ]
                                },
                                "id": request_id
                            })).into_response())
                        }
                        "resources/list" => {
                            // Return available resources
                            Ok(Json(serde_json::json!({
                                "jsonrpc": "2.0",
                                "result": {
                                    "resources": []
                                },
                                "id": request_id
                            })).into_response())
                        }
                        _ => {
                            // Return JSON-RPC method not found error
                            Ok(Json(serde_json::json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32601,
                                    "message": "Method not found"
                                },
                                "id": request_id
                            })).into_response())
                        }
                    }
                }
                Err(_) => {
                    // Return JSON-RPC parse error
                    Ok(Json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": "Parse error"
                        },
                        "id": null
                    })).into_response())
                }
            }
        }
        "DELETE" => {
            // Return proper JSON-RPC response for session termination
            Ok(Json(serde_json::json!({
                "jsonrpc": "2.0",
                "result": {
                    "status": "terminated"
                },
                "id": null
            })).into_response())
        }
        _ => {
            warn!("Unsupported SSE method: {}", method);
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
    }
}

#[cfg(not(feature = "mcp"))]
async fn handle_sse_request(
    _method: axum::http::Method,
    _headers: HeaderMap,
    _query: Query<HashMap<String, String>>,
    _state: McpEndpointState,
    _body: Option<Vec<u8>>,
) -> Result<Response, StatusCode> {
    warn!("MCP feature not enabled - SSE not available");
    Err(StatusCode::NOT_IMPLEMENTED)
}

#[cfg(feature = "mcp")]
async fn handle_streamable_http_request(
    method: axum::http::Method,
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    state: McpEndpointState,
    body: Option<Vec<u8>>,
) -> Result<Response, StatusCode> {
    debug!("Handling StreamableHTTP request with method: {}", method);

    let streamable_transport = match &state.streamable_transport {
        Some(transport) => transport,
        None => {
            error!("StreamableHTTP transport not configured");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut transport = streamable_transport.lock().await;

    match method.as_str() {
        "POST" => {
            let request_body = body.unwrap_or_default();
            match transport.handle_post_request(&headers, request_body).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    error!("StreamableHTTP POST error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        "GET" => {
            let query_map = query.0;
            match transport.handle_get_request(&headers, &query_map).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    error!("StreamableHTTP GET error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        "DELETE" => match transport.handle_delete_request(&headers).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!("StreamableHTTP DELETE error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        _ => {
            warn!("Unsupported StreamableHTTP method: {}", method);
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
    }
}

#[cfg(not(feature = "mcp"))]
async fn handle_streamable_http_request(
    _method: axum::http::Method,
    _headers: HeaderMap,
    _query: Query<HashMap<String, String>>,
    _state: McpEndpointState,
    _body: Option<Vec<u8>>,
) -> Result<Response, StatusCode> {
    warn!("MCP feature not enabled - StreamableHTTP not available");
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Health check endpoint for MCP
pub async fn mcp_health_handler(State(state): State<McpEndpointState>) -> impl IntoResponse {
    let transport_info = match state.config.transport {
        McpTransportMode::Sse => "sse",
        McpTransportMode::StreamableHttp => "streamable_http",
        McpTransportMode::Both => "both",
    };

    Json(serde_json::json!({
        "status": "healthy",
        "transport": transport_info,
        "endpoint": state.config.endpoint,
        "max_sessions": state.config.max_sessions,
        "session_timeout_minutes": state.config.session_timeout_minutes
    }))
}

/// Placeholder handler for when MCP is not available
#[cfg(not(feature = "mcp"))]
pub async fn mcp_placeholder_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "MCP feature not enabled at compile time"
        })),
    )
}