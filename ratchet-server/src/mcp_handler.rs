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
    server::{McpServer, RatchetToolRegistry, tools::{ToolExecutionContext, ToolRegistry}, task_dev_tools::TaskDevelopmentService},
    transport::streamable_http::{
        EventStore, InMemoryEventStore, SessionManager, StreamableHttpTransport,
    },
    security::{AuditLogger, McpAuth, McpAuthManager, SecurityContext, SecurityConfig, ClientContext, permissions::ClientPermissions},
    server::McpServerConfig,
};
use ratchet_interfaces::RepositoryFactory;
use ratchet_execution::{ExecutionBridge, ProcessTaskExecutor};

/// MCP endpoint state for handling both SSE and StreamableHTTP
#[derive(Clone)]
pub struct McpEndpointState {
    pub config: McpApiConfig,
    #[cfg(feature = "mcp")]
    pub mcp_server: Arc<McpServer>,
    #[cfg(feature = "mcp")]
    pub tool_registry: Arc<RatchetToolRegistry>,
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
        let tool_registry = Arc::new(RatchetToolRegistry::new());
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::default()));
        let audit_logger = Arc::new(AuditLogger::new(false));

        let mcp_server = Arc::new(McpServer::new(
            mcp_server_config,
            Arc::clone(&tool_registry) as Arc<dyn ToolRegistry>,
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
            tool_registry,
            session_manager,
            streamable_transport,
        })
    }

    #[cfg(feature = "mcp")]
    pub fn new_with_dependencies(
        config: McpApiConfig,
        repositories: Arc<dyn RepositoryFactory>,
        mcp_task_service: Option<Arc<TaskDevelopmentService>>,
        storage_factory: Option<Arc<ratchet_storage::seaorm::repositories::RepositoryFactory>>,
    ) -> anyhow::Result<Self> {
        // Create MCP server
        let mcp_server_config = McpServerConfig::sse_with_host(config.port, &config.host);
        let tool_registry = Arc::new(
            RatchetToolRegistry::new()
                .with_repositories(repositories)
        );
        
        // Configure tool registry with task development service if available
        let tool_registry = if let Some(task_dev_service) = mcp_task_service {
            Arc::new(
                Arc::try_unwrap(tool_registry)
                    .map_err(|_| anyhow::anyhow!("Failed to unwrap tool registry"))?
                    .with_task_dev_service(task_dev_service)
            )
        } else {
            tool_registry
        };
        
        // Create MCP task executor if storage factory is available
        let tool_registry = if let Some(storage_fact) = storage_factory {
            // Create an ExecutionBridge as the task executor
            let executor_config = ratchet_execution::ProcessExecutorConfig::default();
            let execution_bridge = Arc::new(ExecutionBridge::new(executor_config));
            
            // Create the MCP adapter using the ExecutionBridge
            let mcp_adapter = ratchet_mcp::server::adapter::RatchetMcpAdapter::with_bridge_executor(
                execution_bridge,
                Arc::new(storage_fact.task_repository()),
                Arc::new(storage_fact.execution_repository()),
            );
            
            // Configure tool registry with the MCP adapter as task executor
            Arc::new(
                Arc::try_unwrap(tool_registry)
                    .map_err(|_| anyhow::anyhow!("Failed to unwrap tool registry for executor configuration"))?
                    .with_task_executor(Arc::new(mcp_adapter))
            )
        } else {
            tool_registry
        };
        
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::default()));
        let audit_logger = Arc::new(AuditLogger::new(false));

        let mcp_server = Arc::new(McpServer::new(
            mcp_server_config,
            Arc::clone(&tool_registry) as Arc<dyn ToolRegistry>,
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
            tool_registry,
            session_manager,
            streamable_transport,
        })
    }

    #[cfg(not(feature = "mcp"))]
    pub fn new(config: McpApiConfig) -> anyhow::Result<Self> {
        Ok(Self { config })
    }
}

/// Create a default security context for MCP operations
#[cfg(feature = "mcp")]
fn create_default_security_context() -> SecurityContext {
    let client = ClientContext {
        id: "default-client".to_string(),
        name: "Default MCP Client".to_string(),
        permissions: ClientPermissions::default(),
        authenticated_at: chrono::Utc::now(),
        session_id: "default-session".to_string(),
    };
    SecurityContext::new(client, SecurityConfig::default())
}

/// Execute a tool from the registry and convert result to JSON-RPC format
#[cfg(feature = "mcp")]
async fn execute_tool_from_registry(
    registry: &RatchetToolRegistry,
    tool_name: &str,
    arguments: serde_json::Value,
    request_id: serde_json::Value,
) -> Result<serde_json::Value, StatusCode> {
    // Create security context (for now, using default - could be enhanced with actual auth)
    let security_context = create_default_security_context();
    
    // Create tool execution context
    let execution_context = ToolExecutionContext {
        security: security_context.clone(),
        arguments: Some(arguments),
        request_id: request_id.as_str().map(|s| s.to_string()),
    };
    
    // Check if tool exists and is accessible
    if !registry.can_access_tool(tool_name, &security_context).await {
        error!("Tool '{}' not found or not accessible", tool_name);
        return Ok(serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32601,
                "message": format!("Tool '{}' not found", tool_name)
            },
            "id": request_id
        }));
    }
    
    // Execute the tool
    match registry.execute_tool(tool_name, execution_context).await {
        Ok(result) => {
            // Convert ToolsCallResult to JSON-RPC response
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "result": result,
                "id": request_id
            }))
        }
        Err(e) => {
            error!("Tool execution failed for '{}': {}", tool_name, e);
            Ok(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32603,
                    "message": format!("Tool execution failed: {}", e)
                },
                "id": request_id
            }))
        }
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
    state: McpEndpointState,
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
                            // Return available tools from registry
                            let security_context = create_default_security_context();
                            match state.tool_registry.list_tools(&security_context).await {
                                Ok(tools) => {
                                    Ok(Json(serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "result": {
                                            "tools": tools
                                        },
                                        "id": request_id
                                    })).into_response())
                                }
                                Err(e) => {
                                    error!("Failed to list tools: {}", e);
                                    Ok(Json(serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "error": {
                                            "code": -32603,
                                            "message": format!("Failed to list tools: {}", e)
                                        },
                                        "id": request_id
                                    })).into_response())
                                }
                            }
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
                        "tools/call" => {
                            // Handle tool execution using registry
                            let params = request_json.get("params").cloned().unwrap_or(serde_json::Value::Null);
                            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                            
                            match execute_tool_from_registry(&state.tool_registry, tool_name, arguments, request_id.clone()).await {
                                Ok(response) => Ok(Json(response).into_response()),
                                Err(status_code) => Err(status_code),
                            }
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
            
            // Check if this is a tools/list or tools/call request that we should handle here
            if let Ok(request_json) = serde_json::from_slice::<serde_json::Value>(&request_body) {
                let method_name = request_json.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let request_id = request_json.get("id").cloned().unwrap_or(serde_json::Value::Null);
                
                match method_name {
                    "tools/list" => {
                        // Handle tools/list using registry
                        let security_context = create_default_security_context();
                        return match state.tool_registry.list_tools(&security_context).await {
                            Ok(tools) => {
                                Ok(Json(serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "result": {
                                        "tools": tools
                                    },
                                    "id": request_id
                                })).into_response())
                            }
                            Err(e) => {
                                error!("Failed to list tools: {}", e);
                                Ok(Json(serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "error": {
                                        "code": -32603,
                                        "message": format!("Failed to list tools: {}", e)
                                    },
                                    "id": request_id
                                })).into_response())
                            }
                        };
                    }
                    "tools/call" => {
                        // Handle tools/call using registry
                        let params = request_json.get("params").cloned().unwrap_or(serde_json::Value::Null);
                        let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                        
                        return match execute_tool_from_registry(&state.tool_registry, tool_name, arguments, request_id).await {
                            Ok(response) => Ok(Json(response).into_response()),
                            Err(status_code) => Err(status_code),
                        };
                    }
                    _ => {
                        // For other methods, delegate to the transport
                    }
                }
            }
            
            // Fall back to transport handling for non-tool requests
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