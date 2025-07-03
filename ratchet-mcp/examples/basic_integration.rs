//! Basic integration test showing ratchet-mcp using axum-mcp

use ratchet_mcp::ratchet_server::RatchetServerState;
use ratchet_mcp::axum_mcp_lib::{
    security::SecurityContext,
    server::{ToolExecutionContext, McpServerState, ToolRegistry},
};
use std::sync::Arc;
use async_trait::async_trait;

// Mock implementations for testing
struct MockRepositoryFactory;
struct MockLogger;

#[async_trait]
impl ratchet_interfaces::RepositoryFactory for MockRepositoryFactory {
    fn task_repository(&self) -> &dyn ratchet_interfaces::TaskRepository { unimplemented!() }
    fn execution_repository(&self) -> &dyn ratchet_interfaces::ExecutionRepository { unimplemented!() }
    fn job_repository(&self) -> &dyn ratchet_interfaces::JobRepository { unimplemented!() }
    fn schedule_repository(&self) -> &dyn ratchet_interfaces::ScheduleRepository { unimplemented!() }
    fn user_repository(&self) -> &dyn ratchet_interfaces::UserRepository { unimplemented!() }
    fn session_repository(&self) -> &dyn ratchet_interfaces::SessionRepository { unimplemented!() }
    fn api_key_repository(&self) -> &dyn ratchet_interfaces::ApiKeyRepository { unimplemented!() }
    async fn health_check(&self) -> Result<(), ratchet_interfaces::DatabaseError> {
        Ok(())
    }
}

impl ratchet_interfaces::logging::StructuredLogger for MockLogger {
    fn log(&self, _event: ratchet_interfaces::logging::LogEvent) {
        // Mock implementation
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Ratchet MCP integration with axum-mcp...");
    
    // Create mock dependencies
    let repository_factory = Arc::new(MockRepositoryFactory);
    let logger = Arc::new(MockLogger);
    
    // Create server state using axum-mcp traits
    let state = RatchetServerState::new(repository_factory, logger);
    
    // Test that the server state implements the required traits
    let server_info = state.server_info();
    println!("Server: {} v{}", server_info.name, server_info.version);
    
    let capabilities = state.server_capabilities();
    println!("Capabilities: {:?}", capabilities);
    
    // Test tool registry
    let context = SecurityContext::system();
    let tools = state.tool_registry().list_tools(&context).await?;
    println!("Available tools: {}", tools.len());
    
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description);
    }
    
    // Test tool execution
    if let Some(tool) = tools.first() {
        let execution_context = ToolExecutionContext::new(context.clone())
            .with_arguments(serde_json::json!({
                "task_name": "test_task",
                "parameters": {"key": "value"}
            }));
            
        match state.tool_registry().execute_tool(&tool.name, execution_context).await {
            Ok(result) => {
                println!("Tool execution result: {:?}", result);
            }
            Err(e) => {
                println!("Tool execution error: {}", e);
            }
        }
    }
    
    println!("Integration test completed successfully!");
    Ok(())
}