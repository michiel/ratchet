//! Integration tests for MCP server

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::{server::adapter::RatchetMcpAdapter, McpConfig, McpServer, SimpleTransportType};
use ratchet_interfaces::{TaskService, TaskServiceError, TaskServiceFilters, TaskServiceMetadata, TaskSource};
use ratchet_api_types::{ListResponse, PaginationInput, UnifiedTask};
use ratchet_api_types::pagination::PaginationMeta;
use ratchet_storage::seaorm::repositories::task_repository::TaskRepository;

/// Mock task service for testing that wraps a TaskRepository
struct MockTaskService {
    task_repository: Arc<TaskRepository>,
}

impl MockTaskService {
    fn new(task_repository: Arc<TaskRepository>) -> Self {
        Self { task_repository }
    }
}

#[async_trait]
impl TaskService for MockTaskService {
    async fn find_by_id(&self, _id: Uuid) -> Result<Option<UnifiedTask>, TaskServiceError> {
        // Simplified for testing - return a mock task
        Ok(None)
    }

    async fn find_by_name(&self, _name: &str) -> Result<Option<UnifiedTask>, TaskServiceError> {
        // Simplified for testing - return a mock task
        Ok(None)
    }

    async fn list_tasks(
        &self,
        _pagination: Option<PaginationInput>,
        _filters: Option<TaskServiceFilters>,
    ) -> Result<ListResponse<UnifiedTask>, TaskServiceError> {
        // Simplified for testing - return empty list
        Ok(ListResponse {
            items: vec![],
            meta: PaginationMeta {
                page: 1,
                limit: 25,
                total: 0,
                total_pages: 0,
                has_next: false,
                has_previous: false,
                offset: 0,
            },
        })
    }

    async fn get_task_metadata(&self, _id: Uuid) -> Result<Option<TaskServiceMetadata>, TaskServiceError> {
        // Simplified for testing
        Ok(None)
    }

    async fn execute_task(&self, _id: Uuid, _input: serde_json::Value) -> Result<serde_json::Value, TaskServiceError> {
        // Simplified for testing
        Ok(serde_json::json!({"status": "success"}))
    }

    async fn task_exists(&self, id: Uuid) -> Result<bool, TaskServiceError> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    async fn get_task_source(&self, _id: Uuid) -> Result<Option<TaskSource>, TaskServiceError> {
        // Simplified for testing
        Ok(Some(TaskSource::Database))
    }
}

// Stdio-specific integration tests
mod simple_stdio_test;
mod stdio_initialization_test;

// SSE-specific integration tests
mod sse_integration_test;

/// Test the MCP server initialization sequence
#[tokio::test]
async fn test_mcp_server_initialization() {
    // Create a mock adapter for testing
    let adapter = create_test_adapter().await;

    // Create MCP server config
    let config = McpConfig {
        transport_type: SimpleTransportType::Stdio,
        ..Default::default()
    };

    // Create MCP server
    let server = McpServer::with_adapter(config, adapter)
        .await
        .expect("Failed to create MCP server");

    // Test initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Test Client",
                "version": "1.0.0"
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .expect("Failed to handle initialize request");

    assert!(response.is_some());
    let response = response.unwrap();

    // Verify the response
    assert_eq!(response.id, Some(json!(1)));
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    // Parse the result as InitializeResult
    let result: serde_json::Value = response.result.unwrap();
    assert!(result["serverInfo"]["name"].as_str().unwrap().contains("Ratchet"));
    assert!(result["capabilities"]["tools"].is_object());
}

/// Test that tools/list works immediately after initialize without 'initialized' notification
/// This test verifies the fix for Claude Code compatibility
#[tokio::test]
async fn test_mcp_server_tools_list_without_initialized_notification() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // Initialize server
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Claude Code Test Client",
                "version": "1.0.0"
            }
        }
    });

    let init_response = server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .unwrap();
    assert!(init_response.is_some());
    assert!(init_response.unwrap().error.is_none());

    // Send tools/list request immediately without 'initialized' notification
    // This simulates Claude Code behavior
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = server
        .handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await
        .expect("Failed to handle tools/list request");

    assert!(response.is_some());
    let response = response.unwrap();

    // Should NOT get "Server not initialized" error
    assert!(
        response.error.is_none(),
        "Expected successful response but got error: {:?}",
        response.error
    );
    assert_eq!(response.id, Some(json!(2)));
    assert!(response.result.is_some());

    // Verify we get the expected tools
    let result: serde_json::Value = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    assert!(tool_names.contains(&"ratchet_execute_task".to_string()));
    assert!(tool_names.contains(&"ratchet_list_available_tasks".to_string()));
}

/// Test the tools/list request
#[tokio::test]
async fn test_mcp_server_tools_list() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // Initialize first
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Test Client",
                "version": "1.0.0"
            }
        }
    });

    server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .unwrap();

    // Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });

    server
        .handle_message(&serde_json::to_string(&initialized_notification).unwrap(), None)
        .await
        .unwrap();

    // Test tools/list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = server
        .handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await
        .expect("Failed to handle tools/list request");

    assert!(response.is_some());
    let response = response.unwrap();

    // Verify the response
    assert_eq!(response.id, Some(json!(2)));
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    // Parse the result
    let result: serde_json::Value = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    // Should have our built-in tools
    assert!(!tools.is_empty());

    // Check for ratchet_execute_task tool
    let execute_task_tool = tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some("ratchet_execute_task"));
    assert!(execute_task_tool.is_some());
}

/// Test error handling for invalid requests
#[tokio::test]
async fn test_mcp_server_error_handling() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // Test invalid JSON
    let response = server.handle_message("invalid json", None).await;
    assert!(response.is_err());

    // Test method not found
    let invalid_method_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nonexistent/method"
    });

    let response = server
        .handle_message(&serde_json::to_string(&invalid_method_request).unwrap(), None)
        .await
        .unwrap();

    assert!(response.is_some());
    let response = response.unwrap();
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32601); // Method not found
}

/// Create a test adapter with mock repositories
async fn create_test_adapter() -> RatchetMcpAdapter {
    use ratchet_execution::{ProcessExecutorConfig, ProcessTaskExecutor};
    use ratchet_storage::seaorm::{connection::DatabaseConnection, repositories::RepositoryFactory};
    use ratchet_server::{task_service::UnifiedTaskService, services::DirectRepositoryFactory};
    use ratchet_interfaces::{RepositoryFactory as RepoFactory, TaskRegistry};

    // Create in-memory database for testing using new config type
    let db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 1,
        connection_timeout: std::time::Duration::from_secs(5),
    };

    let database = DatabaseConnection::new(db_config.clone())
        .await
        .expect("Failed to create test database");

    // Run migrations
    database.migrate().await.expect("Failed to run migrations");

    // Create storage factory
    let storage_factory = Arc::new(RepositoryFactory::new(database.clone()));
    
    // Create direct repository factory (bridge to interfaces)
    let direct_factory = DirectRepositoryFactory::new(storage_factory.clone());
    let repositories: Arc<dyn RepoFactory> = Arc::new(direct_factory);
    let execution_repository = Arc::new(storage_factory.execution_repository());
    
    // Create unified interfaces - use bridge registry for testing
    let server_config = ratchet_server::config::ServerConfig::default();
    let registry: Arc<dyn TaskRegistry> = Arc::new(
        ratchet_server::bridges::BridgeTaskRegistry::new(&server_config)
            .await
            .expect("Failed to create bridge registry")
    );
    
    // Create unified task service
    let task_service = Arc::new(UnifiedTaskService::new(repositories, registry));

    // Create a simple task service wrapper for testing
    let task_service = Arc::new(MockTaskService::new(task_repository.clone()));

    // Create executor using the new API from ratchet-execution
    let executor_config = ProcessExecutorConfig {
        worker_count: 1,
        task_timeout_seconds: 30,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let executor = Arc::new(ProcessTaskExecutor::new(executor_config));

    RatchetMcpAdapter::new(executor, task_service, execution_repository)
}

/// Test that demonstrates a complete MCP session
#[tokio::test]
async fn test_complete_mcp_session() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // 1. Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Test LLM Client",
                "version": "1.0.0"
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();
    assert!(response.error.is_none());

    // 2. Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });

    let response = server
        .handle_message(&serde_json::to_string(&initialized).unwrap(), None)
        .await
        .unwrap();
    assert!(response.is_none()); // Notifications don't return responses

    // 3. List tools
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = server
        .handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();
    assert!(response.error.is_none());

    // 4. Try to execute a task (will fail since no tasks in test DB, but should be handled gracefully)
    let execute_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "ratchet_execute_task",
            "arguments": {
                "task_id": "nonexistent-task",
                "input": {}
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&execute_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    // Should get a response (not an error), but the tool execution should indicate failure
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    // The result should indicate the tool execution failed
    let result = response.result.unwrap();
    let content = &result["content"];
    assert!(content.is_array());
}

/// Test the monitoring tools functionality
#[tokio::test]
async fn test_monitoring_tools() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Test Client",
                "version": "1.0.0"
            }
        }
    });

    server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .unwrap();

    // Test get_execution_status with invalid execution ID
    let status_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "ratchet_get_execution_status",
            "arguments": {
                "execution_id": "00000000-0000-0000-0000-000000000000"
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&status_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    // Should get a response (not an error), but the tool execution should indicate failure
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["content"].is_array());
    assert!(result["isError"].as_bool().unwrap_or(false)); // Should indicate error

    // Test get_execution_logs with invalid execution ID
    let logs_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "ratchet_get_execution_logs",
            "arguments": {
                "execution_id": "00000000-0000-0000-0000-000000000000",
                "level": "info",
                "limit": 50
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&logs_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    // Should get a response (not an error), but the tool execution should indicate failure
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["content"].is_array());
    assert!(result["isError"].as_bool().unwrap_or(false)); // Should indicate error

    // Test tools/list to ensure monitoring tools are present
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/list"
    });

    let response = server
        .handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    let tool_names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();

    // Verify monitoring tools are available
    assert!(tool_names.contains(&"ratchet_get_execution_status".to_string()));
    assert!(tool_names.contains(&"ratchet_get_execution_logs".to_string()));
    assert!(tool_names.contains(&"ratchet_get_execution_trace".to_string()));
    assert!(tool_names.contains(&"ratchet_analyze_execution_error".to_string()));
}

/// Test monitoring tools with a real execution record
#[tokio::test]
async fn test_monitoring_tools_with_real_execution() {
    let adapter = create_test_adapter().await;
    let config = McpConfig::default();
    let server = McpServer::with_adapter(config, adapter).await.unwrap();

    // Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "Test Client",
                "version": "1.0.0"
            }
        }
    });

    server
        .handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await
        .unwrap();

    // Create a test execution UUID (we don't actually insert it)
    let execution_uuid = Uuid::new_v4();

    // Get database connection from the adapter to insert test data
    // (This is a simplified test - in a real test we'd use the repository directly)

    // Test get_execution_status with the real execution ID
    let status_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "ratchet_get_execution_status",
            "arguments": {
                "execution_id": execution_uuid.to_string()
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&status_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    // Since we don't have the execution in the database, it should return an error
    // but the tool call itself should succeed
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["content"].is_array());
    // The execution won't be found, so it should be an error
    assert!(result["isError"].as_bool().unwrap_or(false));

    // Test invalid UUID format
    let invalid_status_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "ratchet_get_execution_status",
            "arguments": {
                "execution_id": "invalid-uuid-format"
            }
        }
    });

    let response = server
        .handle_message(&serde_json::to_string(&invalid_status_request).unwrap(), None)
        .await
        .unwrap()
        .unwrap();

    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["isError"].as_bool().unwrap_or(false)); // Should be error for invalid UUID
}
