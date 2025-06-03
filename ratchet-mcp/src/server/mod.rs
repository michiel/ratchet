//! MCP server implementation for exposing Ratchet capabilities to LLMs

pub mod config;
pub mod tools;
pub mod handler;

pub use config::{McpServerConfig, McpServerTransport};
pub use tools::{McpTool, ToolRegistry, RatchetToolRegistry};
pub use handler::McpRequestHandler;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{McpError, McpResult};
use crate::protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    InitializeParams, InitializeResult, ServerInfo, ServerCapabilities,
};
use crate::security::{McpAuthManager, SecurityContext, AuditLogger};

/// MCP server for exposing Ratchet capabilities to LLMs
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
    sessions: RwLock<HashMap<String, SecurityContext>>,
    
    /// Whether the server is initialized
    initialized: RwLock<bool>,
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
            sessions: RwLock::new(HashMap::new()),
            initialized: RwLock::new(false),
        }
    }
    
    /// Start the MCP server
    pub async fn start(&self) -> McpResult<()> {
        match &self.config.transport {
            McpServerTransport::Stdio => {
                self.start_stdio_server().await
            }
            McpServerTransport::Sse { port, host, .. } => {
                let bind_address = format!("{}:{}", host, port);
                self.start_sse_server(&bind_address).await
            }
        }
    }
    
    /// Start stdio-based MCP server
    async fn start_stdio_server(&self) -> McpResult<()> {
        tracing::info!("Starting MCP server with stdio transport");
        
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    // Process the request
                    match self.handle_message(line, None).await {
                        Ok(Some(response)) => {
                            let response_json = serde_json::to_string(&response)?;
                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Ok(None) => {
                            // No response needed (notification)
                        }
                        Err(e) => {
                            tracing::error!("Error handling MCP request: {}", e);
                            // Send error response if possible
                            let error_response = JsonRpcResponse::error(
                                JsonRpcError::internal_error(e.to_string()),
                                None,
                            );
                            let response_json = serde_json::to_string(&error_response)?;
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
        
        Ok(())
    }
    
    /// Start SSE-based MCP server
    async fn start_sse_server(&self, bind_address: &str) -> McpResult<()> {
        tracing::info!("Starting MCP server with SSE transport on {}", bind_address);
        
        // This would be implemented with full SSE support
        // For now, return an error indicating it's not yet implemented
        Err(McpError::Generic {
            message: "SSE transport not yet implemented. Use stdio transport instead.".to_string(),
        })
    }
    
    /// Handle an incoming message
    pub async fn handle_message(
        &self,
        message: &str,
        auth_header: Option<&str>,
    ) -> McpResult<Option<JsonRpcResponse>> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_str(message)
            .map_err(|e| McpError::InvalidJsonRpc {
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
                    McpError::MethodNotFound { method } => {
                        JsonRpcError::method_not_found(&method)
                    }
                    McpError::InvalidParams { details, .. } => {
                        JsonRpcError::invalid_params(details)
                    }
                    McpError::AuthenticationFailed { reason } => {
                        JsonRpcError::server_error(-32001, "Authentication failed", Some(serde_json::Value::String(reason)))
                    }
                    McpError::AuthorizationDenied { reason } => {
                        JsonRpcError::server_error(-32002, "Authorization denied", Some(serde_json::Value::String(reason)))
                    }
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
                let security_ctx = self.authenticate_and_authorize(&request, auth_header, "tools/list").await?;
                handler.handle_tools_list(request.params, &security_ctx).await
            }
            
            "tools/call" => {
                let security_ctx = self.authenticate_and_authorize(&request, auth_header, "tools/call").await?;
                handler.handle_tools_call(request.params, &security_ctx).await
            }
            
            "resources/list" => {
                let security_ctx = self.authenticate_and_authorize(&request, auth_header, "resources/list").await?;
                handler.handle_resources_list(request.params, &security_ctx).await
            }
            
            "resources/read" => {
                let security_ctx = self.authenticate_and_authorize(&request, auth_header, "resources/read").await?;
                handler.handle_resources_read(request.params, &security_ctx).await
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
                *initialized = true;
                tracing::info!("MCP server initialized");
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
        tracing::info!("Initializing MCP server with client: {}", params.client_info.name);
        
        // Validate protocol version
        if !crate::protocol::validate_protocol_version(&params.protocol_version) {
            return Err(McpError::Protocol {
                message: format!("Unsupported protocol version: {}", params.protocol_version),
            });
        }
        
        // Build server capabilities
        let capabilities = ServerCapabilities {
            experimental: HashMap::new(),
            logging: None, // TODO: Add logging capability
            prompts: None, // TODO: Add prompts capability
            resources: None, // TODO: Add resources capability
            tools: Some(crate::protocol::ToolsCapability {
                list_changed: false,
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
        self.audit_logger.log_authorization(
            &security_context.client.id,
            operation,
            "mcp_request",
            true, // TODO: Add actual authorization checks
            request.id_as_string(),
        ).await;
        
        Ok(security_context)
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