//! Streamable HTTP transport implementation for MCP
//!
//! This transport combines HTTP POST for JSON-RPC requests with Server-Sent Events (SSE)
//! for streaming responses, providing session management and resumability features.

use async_trait::async_trait;
use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    error::{McpError, McpResult},
    protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcError},
    transport::{McpTransport, TransportHealth},
};

/// Event store trait for supporting session resumability
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Store an event for a session
    async fn store_event(&self, session_id: &str, event: McpEvent) -> McpResult<()>;
    
    /// Get events since a specific event ID
    async fn get_events_since(&self, session_id: &str, last_event_id: Option<&str>) -> McpResult<Vec<McpEvent>>;
    
    /// Clean up expired events
    async fn cleanup_expired(&self) -> McpResult<()>;
    
    /// Remove all events for a session  
    async fn remove_session(&self, session_id: &str) -> McpResult<()>;
}

/// MCP event for the event store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEvent {
    pub id: String,
    pub session_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

impl McpEvent {
    pub fn new(session_id: String, event_type: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            event_type,
            data,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// In-memory event store implementation
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<String, Vec<McpEvent>>>>,
    max_events_per_session: usize,
    max_session_age: Duration,
}

impl InMemoryEventStore {
    pub fn new(max_events_per_session: usize, max_session_age: Duration) -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            max_events_per_session,
            max_session_age,
        }
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn store_event(&self, session_id: &str, event: McpEvent) -> McpResult<()> {
        let mut events = self.events.write().await;
        let session_events = events.entry(session_id.to_string()).or_insert_with(Vec::new);
        
        session_events.push(event);
        
        // Trim to max events per session
        if session_events.len() > self.max_events_per_session {
            session_events.drain(0..session_events.len() - self.max_events_per_session);
        }
        
        Ok(())
    }
    
    async fn get_events_since(&self, session_id: &str, last_event_id: Option<&str>) -> McpResult<Vec<McpEvent>> {
        let events = self.events.read().await;
        let session_events = events.get(session_id).map(|v| v.as_slice()).unwrap_or(&[]);
        
        if let Some(last_id) = last_event_id {
            // Find events after the last event ID
            if let Some(pos) = session_events.iter().position(|e| e.id == last_id) {
                Ok(session_events[pos + 1..].to_vec())
            } else {
                // Last event ID not found, return all events
                Ok(session_events.to_vec())
            }
        } else {
            // No last event ID, return all events
            Ok(session_events.to_vec())
        }
    }
    
    async fn cleanup_expired(&self) -> McpResult<()> {
        let mut events = self.events.write().await;
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.max_session_age.as_secs());
        
        events.retain(|_session_id, session_events| {
            session_events.retain(|event| event.timestamp > cutoff);
            !session_events.is_empty()
        });
        
        Ok(())
    }
    
    async fn remove_session(&self, session_id: &str) -> McpResult<()> {
        let mut events = self.events.write().await;
        events.remove(session_id);
        Ok(())
    }
}

/// Streamable HTTP transport session
#[derive(Debug)]
pub struct StreamableHttpSession {
    pub session_id: String,
    pub created_at: SystemTime,
    pub last_activity: Arc<RwLock<SystemTime>>,
    pub event_sender: mpsc::UnboundedSender<McpEvent>,
    pub cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl StreamableHttpSession {
    pub fn new(session_id: String) -> (Self, mpsc::UnboundedReceiver<McpEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let session = Self {
            session_id,
            created_at: SystemTime::now(),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            event_sender,
            cleanup_handle: None,
        };
        (session, event_receiver)
    }
    
    pub async fn update_activity(&self) {
        *self.last_activity.write().await = SystemTime::now();
    }
    
    pub async fn send_event(&self, event: McpEvent) -> McpResult<()> {
        self.event_sender.send(event).map_err(|_| McpError::Transport {
            message: "Failed to send event to session".to_string(),
        })?;
        self.update_activity().await;
        Ok(())
    }
}

/// Session manager for handling multiple streamable HTTP sessions
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<StreamableHttpSession>>>>,
    event_store: Arc<dyn EventStore>,
    session_timeout: Duration,
    cleanup_interval: Duration,
}

impl SessionManager {
    pub fn new(
        event_store: Arc<dyn EventStore>,
        session_timeout: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_store,
            session_timeout,
            cleanup_interval,
        }
    }
    
    /// Create a new session
    pub async fn create_session(&self) -> McpResult<String> {
        let session_id = Uuid::new_v4().to_string();
        let (session, _event_receiver) = StreamableHttpSession::new(session_id.clone());
        let session = Arc::new(session);
        
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }
        
        info!("Created new streamable HTTP session: {}", session_id);
        Ok(session_id)
    }
    
    /// Get an existing session
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<StreamableHttpSession>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }
    
    /// Remove a session  
    pub async fn remove_session(&self, session_id: &str) -> McpResult<()> {
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id);
        }
        
        // Clean up event store
        self.event_store.remove_session(session_id).await?;
        
        info!("Removed streamable HTTP session: {}", session_id);
        Ok(())
    }
    
    /// Store an event for a session
    pub async fn store_event(&self, event: &McpEvent) -> McpResult<()> {
        self.event_store.store_event(&event.session_id, event.clone()).await
    }
    
    /// Get events since a specific event ID
    pub async fn get_events_since(&self, session_id: &str, last_event_id: Option<&str>) -> McpResult<Vec<McpEvent>> {
        self.event_store.get_events_since(session_id, last_event_id).await
    }
    
    /// Subscribe to session events (returns a stream)
    pub async fn subscribe_to_session(&self, session_id: &str) -> impl futures_util::Stream<Item = McpEvent> {
        use futures_util::stream;
        
        // For now, return an empty stream as a placeholder
        // In a real implementation, this would use a broadcast channel or similar
        stream::empty()
    }
    
    /// Start background cleanup task
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let sessions = Arc::clone(&self.sessions);
        let event_store = Arc::clone(&self.event_store);
        let session_timeout = self.session_timeout;
        let cleanup_interval = self.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                
                // Clean up expired sessions
                let cutoff = SystemTime::now()
                    .checked_sub(session_timeout)
                    .unwrap_or(UNIX_EPOCH);
                
                let expired_sessions: Vec<String> = {
                    let sessions_guard = sessions.read().await;
                    let mut expired = Vec::new();
                    
                    for (session_id, session) in sessions_guard.iter() {
                        let last_activity = *session.last_activity.read().await;
                        if last_activity < cutoff {
                            expired.push(session_id.clone());
                        }
                    }
                    expired
                };
                
                if !expired_sessions.is_empty() {
                    let mut sessions_guard = sessions.write().await;
                    for session_id in expired_sessions {
                        sessions_guard.remove(&session_id);
                        if let Err(e) = event_store.remove_session(&session_id).await {
                            warn!("Failed to clean up event store for session {}: {}", session_id, e);
                        } else {
                            debug!("Cleaned up expired session: {}", session_id);
                        }
                    }
                }
                
                // Clean up event store
                if let Err(e) = event_store.cleanup_expired().await {
                    warn!("Failed to clean up expired events: {}", e);
                }
            }
        })
    }
}

/// Streamable HTTP transport implementation
pub struct StreamableHttpTransport {
    session_manager: Arc<SessionManager>,
    current_session_id: Option<String>,
    health: Arc<RwLock<TransportHealth>>,
}

impl StreamableHttpTransport {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self {
            session_manager,
            current_session_id: None,
            health: Arc::new(RwLock::new(TransportHealth::healthy())),
        }
    }
    
    /// Handle HTTP POST request (initialization or JSON-RPC)
    pub async fn handle_post_request(
        &mut self,
        headers: &HeaderMap,
        request_body: Vec<u8>,
    ) -> McpResult<Response> {
        let session_id = headers
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        if let Some(session_id) = session_id {
            // Existing session request
            if let Some(session) = self.session_manager.get_session(&session_id).await {
                session.update_activity().await; 
                self.handle_jsonrpc_request(&session_id, request_body).await
            } else {
                self.error_response(StatusCode::BAD_REQUEST, -32000, "Invalid session ID")
            }
        } else {
            // New session initialization
            let session_id = self.session_manager.create_session().await?;
            self.current_session_id = Some(session_id.clone());
            
            let response = self.handle_initialization_request(&session_id, request_body).await?;
            
            // Add session ID to response headers
            let mut response = response.into_response();
            response.headers_mut().insert(
                "mcp-session-id",
                HeaderValue::from_str(&session_id).unwrap(),
            );
            
            Ok(response)
        }
    }
    
    /// Handle session GET request (SSE stream)
    pub async fn handle_get_request(
        &self,
        headers: &HeaderMap,
        query: &HashMap<String, String>,
    ) -> McpResult<Response> {
        let session_id = headers
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok())
            .or_else(|| query.get("session_id").map(|s| s.as_str()));
        
        if let Some(session_id) = session_id {
            if let Some(_session) = self.session_manager.get_session(session_id).await {
                let last_event_id = headers
                    .get("last-event-id")
                    .and_then(|h| h.to_str().ok());
                
                self.establish_sse_stream(session_id, last_event_id).await
            } else {
                self.error_response(StatusCode::BAD_REQUEST, -32000, "Invalid session ID")
            }
        } else {
            self.error_response(StatusCode::BAD_REQUEST, -32000, "No session ID provided")  
        }
    }
    
    /// Handle session DELETE request (termination)
    pub async fn handle_delete_request(
        &self,
        headers: &HeaderMap,
    ) -> McpResult<Response> {
        let session_id = headers
            .get("mcp-session-id")
            .and_then(|h| h.to_str().ok());
        
        if let Some(session_id) = session_id {
            self.session_manager.remove_session(session_id).await?;
            Ok(Json(serde_json::json!({
                "jsonrpc": "2.0",
                "result": { "status": "terminated" }
            })).into_response())
        } else {
            self.error_response(StatusCode::BAD_REQUEST, -32000, "No session ID provided")
        }
    }
    
    async fn handle_initialization_request(
        &self,
        session_id: &str,
        request_body: Vec<u8>,
    ) -> McpResult<Response> {
        // Parse the initialization request
        let request: JsonRpcRequest = serde_json::from_slice(&request_body)
            .map_err(|e| McpError::Protocol {
                message: format!("Invalid JSON-RPC request: {}", e),
            })?;
        
        // Store initialization event
        let event = McpEvent::new(
            session_id.to_string(),
            "initialization".to_string(),
            serde_json::to_value(&request).unwrap(),
        );
        self.session_manager.event_store.store_event(session_id, event).await?;
        
        // Handle initialization - this would normally go through your MCP server
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::json!({
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
            })),
            error: None,
        };
        
        Ok(Json(response).into_response())
    }
    
    async fn handle_jsonrpc_request(
        &self,
        session_id: &str,
        request_body: Vec<u8>,
    ) -> McpResult<Response> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_slice(&request_body)
            .map_err(|e| McpError::Protocol {
                message: format!("Invalid JSON-RPC request: {}", e),
            })?;
        
        // Store request event
        let event = McpEvent::new(
            session_id.to_string(),
            "request".to_string(),
            serde_json::to_value(&request).unwrap(),
        );
        self.session_manager.event_store.store_event(session_id, event).await?;
        
        // Handle specific MCP methods
        let response = match request.method.as_str() {
            "tools/list" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({
                        "tools": [
                            {
                                "name": "ratchet_execute_task",
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
                                "name": "ratchet_list_available_tasks",
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
                    })),
                    error: None,
                }
            }
            "resources/list" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({
                        "resources": []
                    })),
                    error: None,
                }
            }
            "tools/call" => {
                // Handle tool execution
                let params = request.params.as_ref().unwrap_or(&serde_json::Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let _arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                
                match tool_name {
                    "ratchet_list_available_tasks" => {
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": "Available Ratchet tasks:\n\n1. **heartbeat** - System heartbeat task\n2. **HTTP-enabled tasks** - Any JavaScript task can make HTTP requests using fetch() API\n\nFor HTTP functionality in Ratchet:\n- Use JavaScript tasks with the built-in fetch() API\n- HTTP client library (ratchet-http) provides request recording and mock support\n- Tasks can make REST API calls, handle authentication, and process responses\n\nExample task patterns:\n- weather-api: External API consumption\n- rest-call-sample: REST API integration\n- test-fetch: HTTP GET requests with headers"
                                    }
                                ]
                            })),
                            error: None,
                        }
                    }
                    "ratchet_execute_task" => {
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": "Task execution would require a full task runtime integration. Currently, this MCP server provides task discovery and management capabilities.\n\nTo execute HTTP-related tasks:\n1. Use the Ratchet CLI: `ratchet execute <task-name>`\n2. Tasks have access to fetch() API for HTTP requests\n3. Check available tasks with the list command"
                                    }
                                ]
                            })),
                            error: None,
                        }
                    }
                    _ => {
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(JsonRpcError::method_not_found(&format!("Tool '{}'", tool_name))),
                        }
                    }
                }
            }
            _ => {
                // Method not implemented yet
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError::method_not_found(&request.method)),
                }
            }
        };
        
        // Store response event
        let event = McpEvent::new(
            session_id.to_string(),
            "response".to_string(),
            serde_json::to_value(&response).unwrap(),
        );
        self.session_manager.event_store.store_event(session_id, event).await?;
        
        Ok(Json(response).into_response())
    }
    
    async fn establish_sse_stream(
        &self,
        session_id: &str,
        last_event_id: Option<&str>,
    ) -> McpResult<Response> {
        // Get historical events if resuming
        let historical_events = self
            .session_manager
            .event_store
            .get_events_since(session_id, last_event_id)
            .await?;
        
        let _session_id = session_id.to_string();
        let _event_store = Arc::clone(&self.session_manager.event_store);
        
        let stream = async_stream::stream! {
            // Send historical events first
            for event in historical_events {
                let sse_event = Event::default()
                    .id(event.id)
                    .event(event.event_type)
                    .data(serde_json::to_string(&event.data).unwrap_or_default());
                yield Ok::<_, Infallible>(sse_event);
            }
            
            // TODO: Subscribe to live events from session
            // This would typically involve getting the event receiver from the session
            // and yielding events as they come in
            
            // Keep-alive
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let keep_alive = Event::default()
                    .event("keep-alive")
                    .data("ping");
                yield Ok(keep_alive);
            }
        };
        
        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response())
    }
    
    fn error_response(&self, status: StatusCode, code: i32, message: &str) -> McpResult<Response> {
        let error_body = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message
            },
            "id": null
        });
        
        Ok((status, Json(error_body)).into_response())
    }
}

#[async_trait]
impl McpTransport for StreamableHttpTransport {
    async fn connect(&mut self) -> McpResult<()> {
        // For HTTP transport, connection is established per-request
        self.health.write().await.mark_success(None);
        Ok(())
    }
    
    async fn send(&mut self, _message: JsonRpcRequest) -> McpResult<()> {
        // HTTP transport doesn't have persistent sending - handled by HTTP handlers
        Err(McpError::Transport {
            message: "Direct sending not supported for StreamableHttp transport".to_string(),
        })
    }
    
    async fn receive(&mut self) -> McpResult<JsonRpcResponse> {
        // HTTP transport doesn't have persistent receiving - handled by HTTP handlers  
        Err(McpError::Transport {
            message: "Direct receiving not supported for StreamableHttp transport".to_string(),
        })
    }
    
    async fn is_connected(&self) -> bool {
        self.current_session_id.is_some()
    }
    
    async fn health(&self) -> TransportHealth {
        self.health.read().await.clone()
    }
    
    async fn close(&mut self) -> McpResult<()> {
        if let Some(session_id) = &self.current_session_id {
            self.session_manager.remove_session(session_id).await?;
            self.current_session_id = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_in_memory_event_store() {
        let store = InMemoryEventStore::new(100, Duration::from_secs(3600));
        let session_id = "test-session";
        
        let event1 = McpEvent::new(
            session_id.to_string(),
            "test".to_string(),
            serde_json::json!({"data": "test1"}),
        );
        let event_id = event1.id.clone();
        
        // Store event
        store.store_event(session_id, event1).await.unwrap();
        
        // Get all events
        let events = store.get_events_since(session_id, None).await.unwrap();
        assert_eq!(events.len(), 1);
        
        // Store another event
        let event2 = McpEvent::new(
            session_id.to_string(),
            "test".to_string(),
            serde_json::json!({"data": "test2"}),
        );
        store.store_event(session_id, event2).await.unwrap();
        
        // Get events since first event
        let events = store.get_events_since(session_id, Some(&event_id)).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data["data"], "test2");
        
        // Clean up session
        store.remove_session(session_id).await.unwrap();
        let events = store.get_events_since(session_id, None).await.unwrap();
        assert_eq!(events.len(), 0);
    }
    
    #[tokio::test]
    async fn test_session_manager() {
        let event_store = Arc::new(InMemoryEventStore::new(100, Duration::from_secs(3600)));
        let manager = SessionManager::new(
            event_store,
            Duration::from_secs(300),
            Duration::from_secs(60),
        );
        
        // Create session
        let session_id = manager.create_session().await.unwrap();
        assert!(manager.get_session(&session_id).await.is_some());
        
        // Remove session
        manager.remove_session(&session_id).await.unwrap();
        assert!(manager.get_session(&session_id).await.is_none());
    }
}