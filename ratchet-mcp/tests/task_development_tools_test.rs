//! Integration tests for MCP task development tools

use ratchet_mcp::server::tools::{RatchetToolRegistry, ToolRegistry};
use ratchet_mcp::security::{ClientContext, ClientPermissions, SecurityContext, SecurityConfig};
use serde_json::json;

fn create_test_context() -> SecurityContext {
    let client = ClientContext {
        id: "test-client".to_string(),
        name: "Test Client".to_string(),
        permissions: ClientPermissions::default(),
        authenticated_at: chrono::Utc::now(),
        session_id: "session-123".to_string(),
    };

    SecurityContext::new(client, SecurityConfig::default())
}

#[tokio::test]
async fn test_task_development_tools_registered() {
    let registry = RatchetToolRegistry::new();
    let context = create_test_context();
    
    // List all tools
    let tools = registry.list_tools(&context).await.unwrap();
    
    // Verify task development tools are registered
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    
    assert!(tool_names.contains(&"ratchet.create_task"));
    assert!(tool_names.contains(&"ratchet.validate_task"));
    assert!(tool_names.contains(&"ratchet.debug_task_execution"));
    assert!(tool_names.contains(&"ratchet.run_task_tests"));
    assert!(tool_names.contains(&"ratchet.create_task_version"));
}

#[tokio::test]
async fn test_create_task_tool_schema() {
    let registry = RatchetToolRegistry::new();
    let context = create_test_context();
    
    // Get the create_task tool
    let tool = registry.get_tool("ratchet.create_task", &context).await.unwrap();
    assert!(tool.is_some());
    
    let create_tool = tool.unwrap();
    assert_eq!(create_tool.tool.name, "ratchet.create_task");
    assert_eq!(create_tool.category, "development");
    
    // Verify schema has required fields
    let schema = &create_tool.tool.input_schema;
    assert!(schema["properties"]["name"].is_object());
    assert!(schema["properties"]["description"].is_object());
    assert!(schema["properties"]["code"].is_object());
    assert!(schema["properties"]["input_schema"].is_object());
    assert!(schema["properties"]["output_schema"].is_object());
    
    // Verify required fields
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("name")));
    assert!(required.contains(&json!("code")));
}

#[tokio::test]
async fn test_validate_task_tool_schema() {
    let registry = RatchetToolRegistry::new();
    let context = create_test_context();
    
    // Get the validate_task tool
    let tool = registry.get_tool("ratchet.validate_task", &context).await.unwrap();
    assert!(tool.is_some());
    
    let validate_tool = tool.unwrap();
    assert_eq!(validate_tool.tool.name, "ratchet.validate_task");
    
    // Verify schema
    let schema = &validate_tool.tool.input_schema;
    assert!(schema["properties"]["task_id"].is_object());
    assert!(schema["properties"]["run_tests"]["default"].as_bool().unwrap());
}

#[tokio::test]
async fn test_task_dev_tools_without_service() {
    use ratchet_mcp::server::tools::{ToolExecutionContext, McpTaskExecutor};
    use std::sync::Arc;
    
    // Create registry without task development service
    let mut registry = RatchetToolRegistry::new();
    
    // Set a mock executor for other tools
    struct MockExecutor;
    
    #[async_trait::async_trait]
    impl McpTaskExecutor for MockExecutor {
        async fn execute_task(&self, _task_path: &str, _input: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(json!({"result": "mock"}))
        }
        
        async fn execute_task_with_progress(
            &self,
            _task_path: &str,
            _input: serde_json::Value,
            _progress_manager: Option<Arc<ratchet_mcp::server::progress::ProgressNotificationManager>>,
            _connection: Option<Arc<dyn ratchet_mcp::transport::connection::TransportConnection>>,
            _filter: Option<ratchet_mcp::server::progress::ProgressFilter>,
        ) -> Result<(String, serde_json::Value), String> {
            Ok(("exec-123".to_string(), json!({"result": "mock"})))
        }
        
        async fn list_tasks(&self, _filter: Option<&str>) -> Result<Vec<ratchet_mcp::server::tools::McpTaskInfo>, String> {
            Ok(vec![])
        }
        
        async fn get_execution_logs(
            &self,
            _execution_id: &str,
            _level: &str,
            _limit: usize,
        ) -> Result<String, String> {
            Ok("[]".to_string())
        }
        
        async fn get_execution_status(&self, _execution_id: &str) -> Result<ratchet_mcp::server::tools::McpExecutionStatus, String> {
            Err("Not implemented".to_string())
        }
    }
    
    registry.set_executor(Arc::new(MockExecutor));
    
    let context = create_test_context();
    let execution_context = ToolExecutionContext {
        security: context,
        arguments: Some(json!({
            "name": "test-task",
            "description": "A test task",
            "code": "function process(input) { return input; }",
            "input_schema": {"type": "object"},
            "output_schema": {"type": "object"}
        })),
        request_id: Some("req-123".to_string()),
    };
    
    // Try to execute create_task without service configured
    let result = registry.execute_tool("ratchet.create_task", execution_context).await;
    assert!(result.is_ok());
    
    let tool_result = result.unwrap();
    assert!(tool_result.is_error);
    
    // Verify error message
    if let ratchet_mcp::protocol::ToolContent::Text { text } = &tool_result.content[0] {
        assert!(text.contains("Task development service not configured"));
    } else {
        panic!("Expected text content");
    }
}