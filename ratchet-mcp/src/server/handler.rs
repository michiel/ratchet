//! Request handler for MCP server operations

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::tools::ToolExecutionContext;
use super::{BatchProcessor, McpServerConfig, ToolRegistry};
use crate::protocol::{
    BatchParams, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ResourcesListParams, ResourcesListResult,
    ResourcesReadParams, ResourcesReadResult, ToolsCallParams, ToolsListParams, ToolsListResult,
};
use crate::security::{AuditLogger, McpAuthManager, PermissionChecker, SecurityContext};
use crate::{McpError, McpResult};

/// Request handler for MCP operations
#[derive(Clone)]
pub struct McpRequestHandler {
    /// Tool registry for executing tools
    tool_registry: Arc<dyn ToolRegistry>,

    /// Authentication manager
    _auth_manager: Arc<McpAuthManager>,

    /// Audit logger
    audit_logger: Arc<AuditLogger>,

    /// Server configuration
    _config: McpServerConfig,

    /// Batch processor for handling batch requests
    batch_processor: Option<Arc<BatchProcessor>>,
}

impl McpRequestHandler {
    /// Create a new request handler
    pub fn new(
        tool_registry: Arc<dyn ToolRegistry>,
        auth_manager: Arc<McpAuthManager>,
        audit_logger: Arc<AuditLogger>,
        config: &McpServerConfig,
    ) -> Self {
        Self {
            tool_registry,
            _auth_manager: auth_manager,
            audit_logger,
            _config: config.clone(),
            batch_processor: None,
        }
    }

    /// Create a new request handler with batch processing
    pub fn with_batch_processor(
        tool_registry: Arc<dyn ToolRegistry>,
        auth_manager: Arc<McpAuthManager>,
        audit_logger: Arc<AuditLogger>,
        config: &McpServerConfig,
        batch_processor: Arc<BatchProcessor>,
    ) -> Self {
        Self {
            tool_registry,
            _auth_manager: auth_manager,
            audit_logger,
            _config: config.clone(),
            batch_processor: Some(batch_processor),
        }
    }

    /// Handle tools/list request
    pub async fn handle_tools_list(&self, params: Option<Value>, security_ctx: &SecurityContext) -> McpResult<Value> {
        let _params: Option<ToolsListParams> = if let Some(p) = params {
            Some(serde_json::from_value(p)?)
        } else {
            None
        };

        // Check permissions
        if !PermissionChecker::can_read_logs(&security_ctx.client.permissions) {
            // For tools/list, we use a less restrictive check
            // In a real implementation, this might have its own permission
        }

        // Get available tools
        let tools = self.tool_registry.list_tools(security_ctx).await?;

        let result = ToolsListResult {
            tools,
            next_cursor: None, // TODO: Implement pagination
        };

        // Audit log the request
        self.audit_logger
            .log_tool_execution(
                &security_ctx.client.id,
                "tools/list",
                true,
                0, // No execution time for list operation
                None,
            )
            .await;

        Ok(serde_json::to_value(result)?)
    }

    /// Handle tools/call request
    pub async fn handle_tools_call(&self, params: Option<Value>, security_ctx: &SecurityContext) -> McpResult<Value> {
        let params: ToolsCallParams = TryFromValue::try_into(params.ok_or_else(|| McpError::InvalidParams {
            method: "tools/call".to_string(),
            details: "Missing parameters".to_string(),
        })?)
        .map_err(|e: serde_json::Error| McpError::InvalidParams {
            method: "tools/call".to_string(),
            details: e.to_string(),
        })?;

        // Check if client can access this tool
        if !self.tool_registry.can_access_tool(&params.name, security_ctx).await {
            return Err(McpError::AuthorizationDenied {
                reason: format!("Access denied to tool: {}", params.name),
            });
        }

        let start_time = std::time::Instant::now();

        // Create execution context
        let execution_context = ToolExecutionContext {
            security: security_ctx.clone(),
            arguments: params.arguments,
            request_id: None, // TODO: Extract from request context
        };

        // Execute the tool
        let result = self.tool_registry.execute_tool(&params.name, execution_context).await;

        let duration = start_time.elapsed();

        // Audit log the execution
        self.audit_logger
            .log_tool_execution(
                &security_ctx.client.id,
                &params.name,
                result.is_ok(),
                duration.as_millis() as u64,
                None,
            )
            .await;

        let tool_result = result?;
        Ok(serde_json::to_value(tool_result)?)
    }

    /// Handle resources/list request
    pub async fn handle_resources_list(
        &self,
        params: Option<Value>,
        security_ctx: &SecurityContext,
    ) -> McpResult<Value> {
        let _params: Option<ResourcesListParams> = if let Some(p) = params {
            Some(serde_json::from_value(p)?)
        } else {
            None
        };

        // For now, return an empty resource list
        // In a full implementation, this would list available Ratchet resources
        let result = ResourcesListResult {
            resources: vec![],
            next_cursor: None,
        };

        self.audit_logger
            .log_authorization(&security_ctx.client.id, "resources", "list", true, None)
            .await;

        Ok(serde_json::to_value(result)?)
    }

    /// Handle resources/read request
    pub async fn handle_resources_read(
        &self,
        params: Option<Value>,
        security_ctx: &SecurityContext,
    ) -> McpResult<Value> {
        let params: ResourcesReadParams = TryFromValue::try_into(params.ok_or_else(|| McpError::InvalidParams {
            method: "resources/read".to_string(),
            details: "Missing parameters".to_string(),
        })?)
        .map_err(|e: serde_json::Error| McpError::InvalidParams {
            method: "resources/read".to_string(),
            details: e.to_string(),
        })?;

        // Validate the resource URI
        if !crate::security::InputSanitizer::validate_resource_uri(&params.uri) {
            return Err(McpError::Validation {
                field: "uri".to_string(),
                message: "Invalid or unsafe resource URI".to_string(),
            });
        }

        // For now, return an empty result
        // In a full implementation, this would read Ratchet resources
        let result = ResourcesReadResult { contents: vec![] };

        self.audit_logger
            .log_authorization(&security_ctx.client.id, &params.uri, "read", true, None)
            .await;

        Ok(serde_json::to_value(result)?)
    }

    /// Handle batch request
    pub async fn handle_batch(&self, params: Option<Value>, security_ctx: &SecurityContext) -> McpResult<Value> {
        // Check if batch processing is enabled
        let batch_processor = self.batch_processor.as_ref().ok_or_else(|| McpError::MethodNotFound {
            method: "batch".to_string(),
        })?;

        let params: BatchParams = TryFromValue::try_into(params.ok_or_else(|| McpError::InvalidParams {
            method: "batch".to_string(),
            details: "Missing parameters".to_string(),
        })?)
        .map_err(|e: serde_json::Error| McpError::InvalidParams {
            method: "batch".to_string(),
            details: e.to_string(),
        })?;

        // Validate batch size against client permissions
        let batch_size = params.requests.len() as u64;
        PermissionChecker::validate_request_size(&security_ctx.client.permissions, batch_size).map_err(|msg| {
            McpError::Validation {
                field: "batch_size".to_string(),
                message: msg,
            }
        })?;

        // Create a request handler for the batch processor
        let handler = self.clone();
        let security_ctx_clone = security_ctx.clone();

        let handler_fn: Arc<super::batch::BatchRequestHandler> = Arc::new(move |request: JsonRpcRequest| {
            let handler = handler.clone();
            let security_ctx = security_ctx_clone.clone();
            Box::pin(async move {
                match handler.handle_single_request(&request, &security_ctx).await {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(result),
                        error: None,
                        id: request.id,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(e.into()),
                        id: request.id,
                    },
                }
            }) as Pin<Box<dyn Future<Output = JsonRpcResponse> + Send>>
        });

        // Process the batch
        let result = batch_processor
            .process_batch_with_handler(params, &handler_fn)
            .await
            .map_err(|e| McpError::Internal {
                message: format!("Batch processing failed: {}", e),
            })?;

        // Log batch completion
        self.audit_logger
            .log_authorization(
                &security_ctx.client.id,
                &format!("batch:{}", result.stats.total_requests),
                "batch_execute",
                result.stats.failed_requests == 0,
                Some(format!(
                    "success:{}, failed:{}, skipped:{}",
                    result.stats.successful_requests, result.stats.failed_requests, result.stats.skipped_requests
                )),
            )
            .await;

        Ok(serde_json::to_value(result)?)
    }

    /// Handle a single request within a batch
    async fn handle_single_request(
        &self,
        request: &JsonRpcRequest,
        security_ctx: &SecurityContext,
    ) -> McpResult<Value> {
        match request.method.as_str() {
            "tools/list" => self.handle_tools_list(request.params.clone(), security_ctx).await,
            "tools/call" => self.handle_tools_call(request.params.clone(), security_ctx).await,
            "resources/list" => self.handle_resources_list(request.params.clone(), security_ctx).await,
            "resources/read" => self.handle_resources_read(request.params.clone(), security_ctx).await,
            _ => Err(McpError::MethodNotFound {
                method: request.method.clone(),
            }),
        }
    }

    /// Validate request size against quotas
    #[allow(dead_code)]
    fn validate_request_size(&self, security_ctx: &SecurityContext, params: &Value) -> McpResult<()> {
        let request_size = params.to_string().len() as u64;

        PermissionChecker::validate_request_size(&security_ctx.client.permissions, request_size).map_err(|msg| {
            McpError::Validation {
                field: "request_size".to_string(),
                message: msg,
            }
        })?;

        Ok(())
    }

    /// Check if the request has timed out
    #[allow(dead_code)]
    fn check_timeout(&self, security_ctx: &SecurityContext) -> McpResult<()> {
        if security_ctx.is_timed_out() {
            Err(McpError::ServerTimeout {
                timeout: security_ctx.config.max_execution_time,
            })
        } else {
            Ok(())
        }
    }
}

// Conversion from McpError to JsonRpcError
impl From<McpError> for JsonRpcError {
    fn from(err: McpError) -> Self {
        match err {
            McpError::MethodNotFound { method } => JsonRpcError::method_not_found(&method),
            McpError::InvalidParams { method: _, details } => JsonRpcError::invalid_params(details),
            McpError::Validation { field: _, message } => JsonRpcError::invalid_params(message),
            McpError::ServerTimeout { timeout: _ } => JsonRpcError::server_error(-32001, "Request timeout", None),
            McpError::Internal { message } => JsonRpcError::internal_error(message),
            _ => JsonRpcError::internal_error(err.to_string()),
        }
    }
}

// Helper trait for converting serde_json::Value to specific types
trait TryFromValue<T> {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

impl TryFromValue<ToolsCallParams> for Value {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<ToolsCallParams, Self::Error> {
        serde_json::from_value(self)
    }
}

impl TryFromValue<BatchParams> for Value {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<BatchParams, Self::Error> {
        serde_json::from_value(self)
    }
}

impl TryFromValue<ResourcesReadParams> for Value {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<ResourcesReadParams, Self::Error> {
        serde_json::from_value(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AuditLogger, McpAuth};
    use crate::security::{ClientContext, ClientPermissions, SecurityConfig};
    use crate::server::tools::RatchetToolRegistry;

    fn create_test_handler() -> McpRequestHandler {
        let tool_registry = Arc::new(RatchetToolRegistry::new());
        let auth_manager = Arc::new(McpAuthManager::new(McpAuth::None));
        let audit_logger = Arc::new(AuditLogger::new(false));
        let config = McpServerConfig::default();

        McpRequestHandler::new(tool_registry, auth_manager, audit_logger, &config)
    }

    fn create_test_security_context() -> SecurityContext {
        let client = ClientContext {
            id: "test-client".to_string(),
            name: "Test Client".to_string(),
            permissions: ClientPermissions::full_access(),
            authenticated_at: chrono::Utc::now(),
            session_id: "session-123".to_string(),
        };

        SecurityContext::new(client, SecurityConfig::default())
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let result = handler.handle_tools_list(None, &security_ctx).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let list_result: ToolsListResult = serde_json::from_value(value).unwrap();
        assert!(!list_result.tools.is_empty());
    }

    #[tokio::test]
    async fn test_handle_tools_call() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let params = serde_json::json!({
            "name": "ratchet_execute_task",
            "arguments": {
                "task_id": "test-task",
                "input": {"key": "value"}
            }
        });

        let result = handler.handle_tools_call(Some(params), &security_ctx).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let call_result: crate::protocol::ToolsCallResult = serde_json::from_value(value).unwrap();
        // Without an executor configured, the tool should return an error
        assert!(call_result.is_error);
    }

    #[tokio::test]
    async fn test_handle_tools_call_invalid_tool() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let params = serde_json::json!({
            "name": "nonexistent.tool",
            "arguments": {}
        });

        let result = handler.handle_tools_call(Some(params), &security_ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_resources_list() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let result = handler.handle_resources_list(None, &security_ctx).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let list_result: ResourcesListResult = serde_json::from_value(value).unwrap();
        // Empty for now since resources are not implemented
        assert!(list_result.resources.is_empty());
    }

    #[tokio::test]
    async fn test_handle_resources_read() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let params = serde_json::json!({
            "uri": "ratchet://config/settings"
        });

        let result = handler.handle_resources_read(Some(params), &security_ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_resources_read_invalid_uri() {
        let handler = create_test_handler();
        let security_ctx = create_test_security_context();

        let params = serde_json::json!({
            "uri": "../../../etc/passwd"
        });

        let result = handler.handle_resources_read(Some(params), &security_ctx).await;
        assert!(result.is_err());
    }
}
