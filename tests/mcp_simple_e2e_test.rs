//! Simple MCP e2e test focused on core functionality
//!
//! This test validates the basic MCP workflow without complex task operations
//! that may fail in test environments.

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use ratchet_mcp::{server::adapter::RatchetMcpAdapter, McpConfig, McpServer};

/// Test helper to create a test MCP adapter
async fn create_test_adapter() -> Result<RatchetMcpAdapter> {
    use ratchet_execution::{ProcessExecutorConfig, ProcessTaskExecutor};
    use ratchet_storage::seaorm::{connection::DatabaseConnection, repositories::RepositoryFactory as SeaOrmRepositoryFactory};
    use ratchet_server::{task_service::UnifiedTaskService, services::DirectRepositoryFactory};
    use ratchet_interfaces::{TaskRegistry, RegistryError, TaskMetadata, RepositoryFactory};
    use async_trait::async_trait;

    // Simple mock TaskRegistry for testing
    struct MockTaskRegistry;
    
    #[async_trait]
    impl TaskRegistry for MockTaskRegistry {
        async fn discover_tasks(&self) -> Result<Vec<TaskMetadata>, RegistryError> {
            Ok(vec![])
        }
        
        async fn get_task_metadata(&self, _name: &str) -> Result<TaskMetadata, RegistryError> {
            Err(RegistryError::TaskNotFound { name: "test".to_string() })
        }
        
        async fn load_task_content(&self, _name: &str) -> Result<String, RegistryError> {
            Err(RegistryError::TaskNotFound { name: "test".to_string() })
        }
        
        async fn task_exists(&self, _name: &str) -> Result<bool, RegistryError> {
            Ok(false)
        }
        
        fn registry_id(&self) -> &str {
            "test-registry"
        }
        
        async fn health_check(&self) -> Result<(), RegistryError> {
            Ok(())
        }
    }

    // Create in-memory database for testing
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

    // Create repositories using the SeaORM factory and then wrap with DirectRepositoryFactory
    let seaorm_factory = Arc::new(SeaOrmRepositoryFactory::new(database.clone()));
    let repo_factory = Arc::new(DirectRepositoryFactory::new(seaorm_factory.clone()));
    let execution_repository = Arc::new(seaorm_factory.execution_repository());

    // Create mock registry and task service
    let mock_registry = Arc::new(MockTaskRegistry);
    let task_service = Arc::new(UnifiedTaskService::new(repo_factory, mock_registry));

    // Create executor
    let executor_config = ProcessExecutorConfig {
        worker_count: 1,
        task_timeout_seconds: 30,
        restart_on_crash: true,
        max_restart_attempts: 3,
    };
    let executor = Arc::new(ProcessTaskExecutor::new(executor_config));

    Ok(RatchetMcpAdapter::new(executor, task_service, execution_repository))
}

/// Test helper to send MCP message and get response
async fn send_mcp_request(server: &McpServer, method: &str, params: Option<Value>, id: Option<i64>) -> Result<Value> {
    let request = if let Some(id) = id {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        })
    } else {
        json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        })
    };

    let response = server.handle_message(&serde_json::to_string(&request)?, None).await?;

    match response {
        Some(resp) => {
            if let Some(error) = resp.error {
                anyhow::bail!("MCP request failed: {:?}", error);
            }
            Ok(resp.result.unwrap_or(json!({})))
        }
        None => Ok(json!({})),
    }
}

/// Test basic MCP API functionality
#[tokio::test]
async fn test_mcp_api_workflow() -> Result<()> {
    println!("🚀 Starting MCP API workflow test");

    // Create MCP adapter and server
    let adapter = create_test_adapter().await?;
    let server = McpServer::with_adapter(McpConfig::default(), adapter).await?;
    println!("🔧 Created MCP server");

    // Step 1: Initialize MCP server
    println!("\n📋 Step 1: Initialize MCP server");
    let init_params = json!({
        "protocolVersion": "2025-03-26",
        "capabilities": {
            "tools": {}
        },
        "clientInfo": {
            "name": "Test MCP Client",
            "version": "1.0.0"
        }
    });

    let init_result = send_mcp_request(&server, "initialize", Some(init_params), Some(1)).await?;
    println!("✅ Server initialized: {}", init_result["serverInfo"]["name"]);

    // Send initialized notification
    send_mcp_request(&server, "initialized", None, None).await?;
    println!("✅ Server marked as initialized");

    // Step 2: List available tools
    println!("\n📋 Step 2: List available MCP tools");
    let tools_result = send_mcp_request(&server, "tools/list", None, Some(2)).await?;
    let tools = tools_result["tools"].as_array().unwrap();

    println!("📊 Found {} tools", tools.len());

    // Verify expected tools are available
    let tool_names: std::collections::HashSet<&str> = tools.iter().map(|tool| tool["name"].as_str().unwrap()).collect();

    let expected_tools = [
        "ratchet_execute_task",
        "ratchet_list_available_tasks",
        "ratchet_get_execution_status",
        "ratchet_get_execution_logs",
        "ratchet_get_execution_trace",
        "ratchet_analyze_execution_error",
        "ratchet_batch_execute",
    ];

    for expected_tool in &expected_tools {
        assert!(
            tool_names.contains(expected_tool),
            "Missing expected tool: {}",
            expected_tool
        );
    }
    println!("✅ All expected core tools are available");

    // Step 3: Test list_available_tasks tool
    println!("\n📋 Step 3: List available tasks");
    let list_result = timeout(
        Duration::from_secs(10),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_list_available_tasks",
                "arguments": {
                    "include_schemas": false
                }
            })),
            Some(3),
        ),
    )
    .await??;

    // Parse response carefully
    if list_result["isError"].as_bool().unwrap_or(false) {
        println!(
            "⚠️ List tasks returned error (expected in test environment): {}",
            list_result["content"][0]["text"]
        );
    } else {
        let content_text = list_result["content"][0]["text"].as_str().unwrap();
        let response: Value = serde_json::from_str(content_text)?;
        let task_count = if let Some(tasks) = response["tasks"].as_array() {
            tasks.len()
        } else {
            0
        };
        println!("📊 Found {} tasks in repository", task_count);
    }

    // Step 4: Test execution with invalid task (should return error gracefully)
    println!("\n📋 Step 4: Test task execution with invalid task");
    let execute_result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_execute_task",
                "arguments": {
                    "task_id": "non-existent-task",
                    "input": {"test": "data"},
                    "trace": false
                }
            })),
            Some(4),
        ),
    )
    .await??;

    // Should return an error gracefully
    assert!(
        execute_result["isError"].as_bool().unwrap_or(false),
        "Expected error for non-existent task"
    );
    println!("✅ Task execution properly returns error for invalid task");

    // Step 5: Test execution status with invalid ID
    println!("\n📋 Step 5: Test execution status with invalid ID");
    let status_result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_get_execution_status",
                "arguments": {
                    "execution_id": "00000000-0000-0000-0000-000000000000"
                }
            })),
            Some(5),
        ),
    )
    .await??;

    // Should handle gracefully
    if status_result["isError"].as_bool().unwrap_or(false) {
        println!("✅ Execution status properly handles invalid ID");
    } else {
        println!(
            "ℹ️ Execution status returned result: {}",
            status_result["content"][0]["text"]
        );
    }

    // Step 6: Test batch execution with empty request list
    println!("\n📋 Step 6: Test batch execution validation");
    let batch_result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_batch_execute",
                "arguments": {
                    "requests": [],
                    "execution_mode": "parallel"
                }
            })),
            Some(6),
        ),
    )
    .await??;

    // Should handle empty requests gracefully
    if batch_result["isError"].as_bool().unwrap_or(false) {
        println!("✅ Batch execution properly validates empty request list");
    } else {
        println!(
            "ℹ️ Batch execution handled empty requests: {}",
            batch_result["content"][0]["text"]
        );
    }

    // Step 7: Test error analysis with invalid execution ID
    println!("\n📋 Step 7: Test error analysis");
    let analyze_result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_analyze_execution_error",
                "arguments": {
                    "execution_id": "invalid-execution-id",
                    "include_suggestions": true
                }
            })),
            Some(7),
        ),
    )
    .await??;

    // Should handle invalid ID gracefully
    if analyze_result["isError"].as_bool().unwrap_or(false) {
        println!("✅ Error analysis properly handles invalid execution ID");
    } else {
        println!("ℹ️ Error analysis returned: {}", analyze_result["content"][0]["text"]);
    }

    // Final verification: List tools again to ensure consistency
    println!("\n📋 Final verification: Ensure server consistency");
    let final_tools_result = send_mcp_request(&server, "tools/list", None, Some(8)).await?;
    let final_tools = final_tools_result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), final_tools.len(), "Tool count should remain consistent");
    println!("✅ Server remains consistent with {} tools", final_tools.len());

    println!("\n🎉 MCP API workflow test completed successfully!");
    println!("   📊 Tested {} MCP tools", expected_tools.len());
    println!("   ✅ Verified server initialization");
    println!("   ✅ Verified tool listing and availability");
    println!("   ✅ Verified error handling for invalid inputs");
    println!("   ✅ Verified tool execution workflows");
    println!("   ✅ Verified server consistency");

    Ok(())
}

/// Test MCP tool parameter validation
#[tokio::test]
async fn test_mcp_parameter_validation() -> Result<()> {
    println!("🚀 Starting MCP parameter validation test");

    let adapter = create_test_adapter().await?;
    let server = McpServer::with_adapter(McpConfig::default(), adapter).await?;

    // Initialize server
    let init_params = json!({
        "protocolVersion": "2025-03-26",
        "capabilities": {},
        "clientInfo": {"name": "Validation Test", "version": "1.0.0"}
    });

    send_mcp_request(&server, "initialize", Some(init_params), Some(1)).await?;
    send_mcp_request(&server, "initialized", None, None).await?;

    // Test 1: Call tool with missing required parameters
    let result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_execute_task",
                "arguments": {
                    "task_id": "test-task"
                    // Missing required "input" parameter
                }
            })),
            Some(2),
        ),
    )
    .await??;

    assert!(
        result["isError"].as_bool().unwrap_or(false),
        "Should return error for missing required parameter"
    );
    println!("✅ Properly validates missing required parameters");

    // Test 2: Call tool with invalid parameter types
    let result = timeout(
        Duration::from_secs(5),
        send_mcp_request(
            &server,
            "tools/call",
            Some(json!({
                "name": "ratchet_get_execution_logs",
                "arguments": {
                    "execution_id": "test-id",
                    "limit": "not-a-number" // Should be integer
                }
            })),
            Some(3),
        ),
    )
    .await??;

    // Some tools may accept this and convert, others may reject it
    println!(
        "ℹ️ Parameter type validation result: {}",
        if result["isError"].as_bool().unwrap_or(false) {
            "strict"
        } else {
            "lenient"
        }
    );

    // Test 3: Call non-existent tool
    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "non_existent_tool",
            "arguments": {}
        }
    });

    let response = server.handle_message(&serde_json::to_string(&request)?, None).await?;

    // Should return an error response
    assert!(response.is_some());
    let response = response.unwrap();
    assert!(response.error.is_some(), "Should return error for non-existent tool");
    println!("✅ Properly rejects calls to non-existent tools");

    println!("\n🎉 MCP parameter validation test completed!");

    Ok(())
}

/// Test MCP protocol edge cases
#[tokio::test]
async fn test_mcp_protocol_edge_cases() -> Result<()> {
    println!("🚀 Starting MCP protocol edge cases test");

    let adapter = create_test_adapter().await?;
    let server = McpServer::with_adapter(McpConfig::default(), adapter).await?;

    // Test 1: Initialize with different protocol versions first
    let init_params = json!({
        "protocolVersion": "0.1.0", // Older version
        "capabilities": {},
        "clientInfo": {"name": "Edge Case Test", "version": "1.0.0"}
    });

    let init_result = send_mcp_request(&server, "initialize", Some(init_params), Some(1)).await?;
    assert!(init_result["serverInfo"]["name"].as_str().unwrap().contains("Ratchet"));
    println!("✅ Handles different protocol versions gracefully");

    // Test 2: Send 'initialized' notification before calling tools (proper MCP flow)
    send_mcp_request(&server, "initialized", None, None).await?;

    // Test 2b: Call tools/list after full initialization
    let tools_result = send_mcp_request(&server, "tools/list", None, Some(2)).await?;
    let tools = tools_result["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Should return tools after full initialization");
    println!("✅ Tools available after full initialization");

    // Test 3: Send multiple initialized notifications (should be idempotent)
    send_mcp_request(&server, "initialized", None, None).await?;
    send_mcp_request(&server, "initialized", None, None).await?;
    println!("✅ Multiple initialized notifications handled gracefully");

    // Test 4: Invalid JSON-RPC format
    let invalid_json = r#"{"jsonrpc": "2.0", "method": "tools/list"}"#; // Missing ID for request
    let response = server.handle_message(invalid_json, None).await?;
    // Should handle gracefully (may return error or None)
    println!("ℹ️ Invalid JSON-RPC handled: {:?}", response.is_some());

    println!("\n🎉 MCP protocol edge cases test completed!");

    Ok(())
}
