//! Axum HTTP handlers for MCP endpoints

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response, sse::{Event, Sse}},
    Json,
};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    error::{McpError, McpResult},
    protocol::{JsonRpcRequest, JsonRpcResponse},
    security::{SecurityContext, ClientContext},
    server::{service::McpServer, McpServerState},
    transport::{
        streamable_http::{SessionManager, McpEvent},
        TransportHealth,
    },
};

/// Handler state for MCP endpoints
pub trait McpHandlerState: Send + Sync + Clone + 'static {
    /// Server state implementation
    type ServerState: McpServerState;
    
    /// Get the MCP server instance
    fn mcp_server(&self) -> &McpServer<Self::ServerState>;
    
    /// Get the session manager (if using StreamableHTTP)
    fn session_manager(&self) -> Option<&SessionManager>;
    
    /// Get transport health information
    async fn transport_health(&self) -> TransportHealth {
        TransportHealth::healthy()
    }
    
    /// Create security context from request headers
    fn create_security_context(&self, headers: &HeaderMap) -> SecurityContext {
        // Extract client information from headers
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
            
        let client_context = ClientContext {
            user_agent: user_agent.to_string(),
            client_id: None,
            session_id: None,
            metadata: HashMap::new(),
        };
        
        // For now, create a basic authenticated context
        // Real implementations would validate JWT tokens, API keys, etc.
        SecurityContext::authenticated(client_context, Vec::new())
    }
}

/// Query parameters for MCP endpoints
#[derive(Debug, Deserialize)]
pub struct McpQueryParams {
    /// Session ID for resumable sessions
    pub session_id: Option<String>,
    /// Last event ID for SSE resumption
    pub last_event_id: Option<String>,
    /// Transport type preference
    pub transport: Option<String>,
}

/// MCP endpoint response information
#[derive(Debug, Serialize)]
pub struct McpEndpointInfo {
    /// Server name
    pub name: String,
    /// Supported protocol versions
    pub protocol_versions: Vec<String>,
    /// Available transports
    pub transports: Vec<String>,
    /// Server capabilities summary
    pub capabilities: Vec<String>,
    /// Session support
    pub session_support: bool,
}

/// Handle GET requests to MCP endpoint (discovery and health)
pub async fn mcp_get_handler<S>(
    State(state): State<S>,
    Query(params): Query<McpQueryParams>,
    headers: HeaderMap,
) -> impl IntoResponse
where
    S: McpHandlerState,
{
    debug!("MCP GET request with params: {:?}", params);
    
    let server_config = state.mcp_server().config();
    let transport_health = state.transport_health().await;
    
    // If this is a health check request
    if params.transport.as_deref() == Some("health") {
        let health = state.mcp_server().get_health().await;
        return Json(serde_json::json!({
            "status": if health.healthy { "healthy" } else { "unhealthy" },
            "message": health.status,
            "uptime_seconds": health.uptime_seconds,
            "active_connections": health.active_connections,
            "transport_health": transport_health
        })).into_response();
    }
    
    // Return endpoint information
    let info = McpEndpointInfo {
        name: server_config.name.clone(),
        protocol_versions: crate::protocol::SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .map(|v| v.to_string())
            .collect(),
        transports: vec!["sse".to_string(), "streamable_http".to_string()],
        capabilities: vec!["tools".to_string(), "batch".to_string()],
        session_support: state.session_manager().is_some(),
    };
    
    Json(info).into_response()
}

/// Handle POST requests to MCP endpoint (JSON-RPC)
pub async fn mcp_post_handler<S>(
    State(state): State<S>,
    Query(params): Query<McpQueryParams>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse
where
    S: McpHandlerState,
{
    debug!("MCP POST request: {} (id: {:?})", request.method, request.id);
    
    // Create security context from headers
    let security_context = state.create_security_context(&headers);
    
    // Handle the request
    let response = state.mcp_server().handle_request(request, security_context).await;
    
    // For StreamableHTTP transport, store the response as an event
    if let Some(session_manager) = state.session_manager() {
        if let Some(session_id) = &params.session_id {
            let event = McpEvent::new(
                session_id.clone(),
                "response".to_string(),
                serde_json::to_value(&response).unwrap_or_default(),
            );
            
            if let Err(e) = session_manager.store_event(&event).await {
                warn!("Failed to store response event: {}", e);
            }
        }
    }
    
    Json(response).into_response()
}

/// Handle SSE endpoint for streaming responses
pub async fn mcp_sse_handler<S>(
    State(state): State<S>,
    Query(params): Query<McpQueryParams>,
    headers: HeaderMap,
) -> impl IntoResponse
where
    S: McpHandlerState,
{
    debug!("MCP SSE request with params: {:?}", params);
    
    // Detect if this is Claude Desktop by checking user-agent
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let is_claude_desktop = user_agent.contains("Claude");
    
    if is_claude_desktop {
        info!("Claude Desktop client detected, using StreamableHTTP transport");
        return handle_streamable_http_sse(state, params, headers).await;
    } else {
        info!("Standard SSE client detected");
        return handle_standard_sse(state, params, headers).await;
    }
}

/// Handle standard SSE streaming
async fn handle_standard_sse<S>(
    state: S,
    _params: McpQueryParams,
    _headers: HeaderMap,
) -> Response
where
    S: McpHandlerState,
{
    // Create a stream of server events
    let progress_receiver = state.mcp_server().progress_reporter().subscribe();
    let progress_stream = BroadcastStream::new(progress_receiver);
    
    let event_stream = progress_stream.map(|result| {
        match result {
            Ok(progress) => {
                let data = serde_json::to_string(&progress).unwrap_or_default();
                Ok(Event::default()
                    .event("progress")
                    .data(data)
                    .id(progress.operation_id))
            }
            Err(e) => {
                error!("Progress stream error: {}", e);
                Ok(Event::default()
                    .event("error")
                    .data(format!("Stream error: {}", e)))
            }
        }
    });
    
    Sse::new(event_stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive")
        )
        .into_response()
}

/// Handle StreamableHTTP SSE for Claude Desktop
async fn handle_streamable_http_sse<S>(
    state: S,
    params: McpQueryParams,
    _headers: HeaderMap,
) -> Response
where
    S: McpHandlerState,
{
    let session_manager = match state.session_manager() {
        Some(sm) => sm,
        None => {
            error!("StreamableHTTP requested but no session manager available");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "StreamableHTTP transport not available"
            ).into_response();
        }
    };
    
    // Get or create session
    let session_id = params.session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Get events since last event ID
    let events = if let Some(last_event_id) = &params.last_event_id {
        match session_manager.get_events_since(&session_id, Some(last_event_id)).await {
            Ok(events) => events,
            Err(e) => {
                error!("Failed to get events since {}: {}", last_event_id, e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get session events").into_response();
            }
        }
    } else {
        // No last event ID, start fresh
        Vec::new()
    };
    
    // Create event stream from stored events and new events
    let stored_events = stream::iter(events.into_iter().map(|event| {
        Ok(Event::default()
            .id(event.id)
            .event(event.event_type)
            .data(serde_json::to_string(&event.data).unwrap_or_default()))
    }));
    
    // Subscribe to new events for this session
    let session_stream = session_manager.subscribe_to_session(&session_id).await;
    let new_events = session_stream.map(|event| {
        Ok(Event::default()
            .id(event.id)
            .event(event.event_type)
            .data(serde_json::to_string(&event.data).unwrap_or_default()))
    });
    
    // Combine stored and new events
    let combined_stream = stored_events.chain(new_events);
    
    Sse::new(combined_stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive")
        )
        .into_response()
}

/// Handle DELETE requests to MCP endpoint (session cleanup)
pub async fn mcp_delete_handler<S>(
    State(state): State<S>,
    Query(params): Query<McpQueryParams>,
    _headers: HeaderMap,
) -> impl IntoResponse
where
    S: McpHandlerState,
{
    debug!("MCP DELETE request with params: {:?}", params);
    
    if let Some(session_id) = &params.session_id {
        if let Some(session_manager) = state.session_manager() {
            match session_manager.remove_session(session_id).await {
                Ok(_) => {
                    info!("Session {} removed successfully", session_id);
                    (StatusCode::OK, "Session removed").into_response()
                }
                Err(e) => {
                    error!("Failed to remove session {}: {}", session_id, e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to remove session").into_response()
                }
            }
        } else {
            (StatusCode::NOT_FOUND, "Session management not available").into_response()
        }
    } else {
        (StatusCode::BAD_REQUEST, "Session ID required").into_response()
    }
}

/// Create MCP routes for Axum router
pub fn mcp_routes<S>() -> axum::Router<S>
where
    S: McpHandlerState,
{
    axum::Router::new()
        .route("/mcp", axum::routing::get(mcp_get_handler::<S>))
        .route("/mcp", axum::routing::post(mcp_post_handler::<S>))
        .route("/mcp", axum::routing::delete(mcp_delete_handler::<S>))
        .route("/mcp/sse", axum::routing::get(mcp_sse_handler::<S>))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        server::{config::McpServerConfig, service::McpServer, registry::InMemoryToolRegistry},
        security::McpAuth,
    };

    // Test implementations
    #[derive(Clone)]
    struct TestHandlerState {
        server: McpServer<TestServerState>,
    }

    #[derive(Clone)]
    struct TestServerState {
        tools: InMemoryToolRegistry,
        auth: TestAuth,
    }

    #[derive(Clone)]
    struct TestAuth;

    #[async_trait::async_trait]
    impl McpAuth for TestAuth {
        async fn authenticate(&self, _client_info: &ClientContext) -> McpResult<SecurityContext> {
            Ok(SecurityContext::system())
        }

        async fn authorize(&self, _context: &SecurityContext, _resource: &str, _action: &str) -> bool {
            true
        }
    }

    impl crate::server::McpServerState for TestServerState {
        type ToolRegistry = InMemoryToolRegistry;
        type AuthManager = TestAuth;

        fn tool_registry(&self) -> &Self::ToolRegistry {
            &self.tools
        }

        fn auth_manager(&self) -> &Self::AuthManager {
            &self.auth
        }
    }

    impl McpHandlerState for TestHandlerState {
        type ServerState = TestServerState;

        fn mcp_server(&self) -> &McpServer<Self::ServerState> {
            &self.server
        }

        fn session_manager(&self) -> Option<&SessionManager> {
            None
        }
    }

    #[tokio::test]
    async fn test_mcp_routes_creation() {
        let config = McpServerConfig::default();
        let state = TestServerState {
            tools: InMemoryToolRegistry::new(),
            auth: TestAuth,
        };
        let server = McpServer::new(config, state);
        let handler_state = TestHandlerState { server };

        let router = mcp_routes().with_state(handler_state);
        
        // Router should compile without errors
        assert_eq!(format!("{:?}", router).contains("Router"), true);
    }
}