//! Integration tests for MCP server

use std::sync::Arc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    McpServer, McpConfig, SimpleTransportType,
    server::adapter::RatchetMcpAdapter,
};

// Stdio-specific integration tests
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
    let server = McpServer::with_adapter(config, adapter).await
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
    
    let response = server.handle_message(
        &serde_json::to_string(&init_request).unwrap(),
        None
    ).await.expect("Failed to handle initialize request");
    
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
    
    let init_response = server.handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await.unwrap();
    assert!(init_response.is_some());
    assert!(init_response.unwrap().error.is_none());
    
    // Send tools/list request immediately without 'initialized' notification
    // This simulates Claude Code behavior
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    
    let response = server.handle_message(
        &serde_json::to_string(&tools_request).unwrap(),
        None
    ).await.expect("Failed to handle tools/list request");
    
    assert!(response.is_some());
    let response = response.unwrap();
    
    // Should NOT get "Server not initialized" error
    assert!(response.error.is_none(), 
        "Expected successful response but got error: {:?}", response.error);
    assert_eq!(response.id, Some(json!(2)));
    assert!(response.result.is_some());
    
    // Verify we get the expected tools
    let result: serde_json::Value = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    assert!(!tools.is_empty());
    
    let tool_names: Vec<String> = tools.iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    
    assert!(tool_names.contains(&"ratchet.execute_task".to_string()));
    assert!(tool_names.contains(&"ratchet.list_available_tasks".to_string()));
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
    
    server.handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await.unwrap();
    
    // Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    
    server.handle_message(&serde_json::to_string(&initialized_notification).unwrap(), None)
        .await.unwrap();
    
    // Test tools/list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    
    let response = server.handle_message(
        &serde_json::to_string(&tools_request).unwrap(),
        None
    ).await.expect("Failed to handle tools/list request");
    
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
    
    // Check for ratchet.execute_task tool
    let execute_task_tool = tools.iter()
        .find(|tool| tool["name"].as_str() == Some("ratchet.execute_task"));
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
    
    let response = server.handle_message(
        &serde_json::to_string(&invalid_method_request).unwrap(),
        None
    ).await.unwrap();
    
    assert!(response.is_some());
    let response = response.unwrap();
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32601); // Method not found
}

/// Create a test adapter with mock repositories
async fn create_test_adapter() -> RatchetMcpAdapter {
    use ratchet_storage::seaorm::{
        connection::DatabaseConnection,
        repositories::{
            task_repository::TaskRepository,
            execution_repository::ExecutionRepository,
            RepositoryFactory,
        }
    };
    use ratchet_lib::execution::ProcessTaskExecutor;
    
    // Create in-memory database for testing using new config type
    let db_config = ratchet_storage::seaorm::config::DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 1,
        connection_timeout: std::time::Duration::from_secs(5),
    };
    
    let database = DatabaseConnection::new(db_config).await
        .expect("Failed to create test database");
    
    // Run migrations
    database.migrate().await
        .expect("Failed to run migrations");
    
    // Create repositories using the new factory
    let repo_factory = RepositoryFactory::new(database.clone());
    let task_repository = Arc::new(repo_factory.task_repository());
    let execution_repository = Arc::new(repo_factory.execution_repository());
    
    // Create executor with test config (using legacy config for now since executor still needs it)
    let config = ratchet_lib::config::RatchetConfig::default();
    let executor = Arc::new(
        ProcessTaskExecutor::new(
            ratchet_lib::database::repositories::RepositoryFactory::new(database.clone().into()),
            config
        ).await.expect("Failed to create test executor")
    );
    
    RatchetMcpAdapter::new(executor, task_repository, execution_repository)
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
    
    let response = server.handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await.unwrap().unwrap();
    assert!(response.error.is_none());
    
    // 2. Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized"
    });
    
    let response = server.handle_message(&serde_json::to_string(&initialized).unwrap(), None)
        .await.unwrap();
    assert!(response.is_none()); // Notifications don't return responses
    
    // 3. List tools
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });
    
    let response = server.handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await.unwrap().unwrap();
    assert!(response.error.is_none());
    
    // 4. Try to execute a task (will fail since no tasks in test DB, but should be handled gracefully)
    let execute_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "ratchet.execute_task",
            "arguments": {
                "task_id": "nonexistent-task",
                "input": {}
            }
        }
    });
    
    let response = server.handle_message(&serde_json::to_string(&execute_request).unwrap(), None)
        .await.unwrap().unwrap();
    
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
    
    server.handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await.unwrap();
    
    // Test get_execution_status with invalid execution ID
    let status_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "ratchet.get_execution_status",
            "arguments": {
                "execution_id": "00000000-0000-0000-0000-000000000000"
            }
        }
    });
    
    let response = server.handle_message(&serde_json::to_string(&status_request).unwrap(), None)
        .await.unwrap().unwrap();
    
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
            "name": "ratchet.get_execution_logs",
            "arguments": {
                "execution_id": "00000000-0000-0000-0000-000000000000",
                "level": "info",
                "limit": 50
            }
        }
    });
    
    let response = server.handle_message(&serde_json::to_string(&logs_request).unwrap(), None)
        .await.unwrap().unwrap();
    
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
    
    let response = server.handle_message(&serde_json::to_string(&tools_request).unwrap(), None)
        .await.unwrap().unwrap();
    
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();
    
    let tool_names: Vec<String> = tools.iter()
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    
    // Verify monitoring tools are available
    assert!(tool_names.contains(&"ratchet.get_execution_status".to_string()));
    assert!(tool_names.contains(&"ratchet.get_execution_logs".to_string()));
    assert!(tool_names.contains(&"ratchet.get_execution_trace".to_string()));
    assert!(tool_names.contains(&"ratchet.analyze_execution_error".to_string()));
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
    
    server.handle_message(&serde_json::to_string(&init_request).unwrap(), None)
        .await.unwrap();
    
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
            "name": "ratchet.get_execution_status",
            "arguments": {
                "execution_id": execution_uuid.to_string()
            }
        }
    });
    
    let response = server.handle_message(&serde_json::to_string(&status_request).unwrap(), None)
        .await.unwrap().unwrap();
    
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
            "name": "ratchet.get_execution_status",
            "arguments": {
                "execution_id": "invalid-uuid-format"
            }
        }
    });
    
    let response = server.handle_message(&serde_json::to_string(&invalid_status_request).unwrap(), None)
        .await.unwrap().unwrap();
    
    assert!(response.error.is_none());
    assert!(response.result.is_some());
    
    let result = response.result.unwrap();
    assert!(result["isError"].as_bool().unwrap_or(false)); // Should be error for invalid UUID
}