//! Integration tests for Ratchet-MCP framework integration
//!
//! This module tests the complete integration between axum-mcp and Ratchet,
//! including tools, resources, prompts, and server capabilities.

use crate::ratchet_server::*;
use crate::axum_mcp_lib::server::McpServerState;
use ratchet_interfaces::{logging::StructuredLogger, RepositoryFactory};
use async_trait::async_trait;
use std::sync::Arc;

// Mock implementations for testing
pub struct MockRepositoryFactory;
pub struct MockLogger;

#[async_trait]
impl RepositoryFactory for MockRepositoryFactory {
    fn task_repository(&self) -> &dyn ratchet_interfaces::TaskRepository { 
        unimplemented!("Mock implementation") 
    }
    fn execution_repository(&self) -> &dyn ratchet_interfaces::ExecutionRepository { 
        unimplemented!("Mock implementation") 
    }
    fn job_repository(&self) -> &dyn ratchet_interfaces::JobRepository { 
        unimplemented!("Mock implementation") 
    }
    fn schedule_repository(&self) -> &dyn ratchet_interfaces::ScheduleRepository { 
        unimplemented!("Mock implementation") 
    }
    fn user_repository(&self) -> &dyn ratchet_interfaces::UserRepository { 
        unimplemented!("Mock implementation") 
    }
    fn session_repository(&self) -> &dyn ratchet_interfaces::SessionRepository { 
        unimplemented!("Mock implementation") 
    }
    fn api_key_repository(&self) -> &dyn ratchet_interfaces::ApiKeyRepository { 
        unimplemented!("Mock implementation") 
    }
    async fn health_check(&self) -> Result<(), ratchet_interfaces::DatabaseError> {
        Ok(())
    }
}

impl StructuredLogger for MockLogger {
    fn log(&self, _event: ratchet_interfaces::logging::LogEvent) {
        // Mock implementation - in real usage this would use Ratchet's logging system
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axum_mcp_lib::{
        server::ToolRegistry,
        security::SecurityContext,
        GetPromptRequest,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_ratchet_server_state_creation() {
        let repository_factory = Arc::new(MockRepositoryFactory);
        let logger = Arc::new(MockLogger);
        
        let state = RatchetServerState::new(repository_factory, logger);
        
        // Test that server info is correctly set
        let server_info = state.server_info();
        assert_eq!(server_info.name, "Ratchet MCP Server");
        assert!(server_info.metadata.contains_key("provider"));
        assert!(server_info.metadata.contains_key("uri_scheme"));
        
        // Test that capabilities are correctly configured
        let capabilities = state.server_capabilities();
        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());
    }

    #[tokio::test]
    async fn test_ratchet_tool_registry() {
        let repository_factory = Arc::new(MockRepositoryFactory);
        let logger = Arc::new(MockLogger);
        
        let registry = RatchetToolRegistry::new(repository_factory, logger);
        let context = SecurityContext::system();
        
        // Test tool listing
        let tools = registry.list_tools(&context).await.unwrap();
        assert!(!tools.is_empty());
        
        // Check for specific Ratchet tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"ratchet_execute_task"));
        assert!(tool_names.contains(&"ratchet_list_executions"));
        assert!(tool_names.contains(&"ratchet_get_execution_logs"));
        assert!(tool_names.contains(&"ratchet_list_schedules"));
        
        // Test tool access
        assert!(registry.can_access_tool("ratchet_execute_task", &context).await);
        assert!(!registry.can_access_tool("non_existent_tool", &context).await);
        
        // Test categories
        let categories = registry.get_categories(&context).await.unwrap();
        assert!(categories.contains(&"execution".to_string()));
        assert!(categories.contains(&"monitoring".to_string()));
        assert!(categories.contains(&"scheduling".to_string()));
    }

    #[tokio::test]
    async fn test_ratchet_resource_registry() {
        let repository_factory = Arc::new(MockRepositoryFactory);
        let logger = Arc::new(MockLogger);
        
        let state = RatchetServerState::new(repository_factory, logger);
        let resource_registry = state.resource_registry().unwrap();
        let context = SecurityContext::system();
        
        // Test resource templates listing
        let templates = resource_registry.list_resource_templates(&context).await.unwrap();
        assert!(!templates.is_empty());
        
        // Check for ratchet:// URI scheme
        let has_ratchet_scheme = templates.iter().any(|t| t.uri_template.starts_with("ratchet://"));
        assert!(has_ratchet_scheme);
        
        // Test resource reading
        let web_scraper_resource = resource_registry
            .get_resource("ratchet://tasks/web-scraper", &context)
            .await
            .unwrap();
        
        assert_eq!(web_scraper_resource.uri, "ratchet://tasks/web-scraper");
        assert_eq!(web_scraper_resource.name, "Web Scraper Task");
        assert!(web_scraper_resource.mime_type == Some("application/json".to_string()));
        
        // Verify the content is valid JSON
        let content_text = match &web_scraper_resource.content {
            crate::axum_mcp_lib::server::resource::ResourceContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };
        let _parsed: serde_json::Value = serde_json::from_str(&content_text).unwrap();
    }

    #[tokio::test]
    async fn test_ratchet_prompt_registry() {
        let repository_factory = Arc::new(MockRepositoryFactory);
        let logger = Arc::new(MockLogger);
        
        let state = RatchetServerState::new(repository_factory, logger);
        let prompt_registry = state.prompt_registry().unwrap();
        let context = SecurityContext::system();
        
        // Test prompt listing
        let prompts = prompt_registry.list_prompts(&context).await.unwrap();
        assert!(!prompts.is_empty());
        
        // Check for specific Ratchet prompts
        let prompt_names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
        assert!(prompt_names.contains(&"ratchet_task_analyzer"));
        assert!(prompt_names.contains(&"ratchet_execution_debugger"));
        assert!(prompt_names.contains(&"ratchet_schedule_optimizer"));
        
        // Test prompt retrieval without parameters
        let task_analyzer = prompt_registry
            .get_prompt("ratchet_task_analyzer", &context)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(task_analyzer.name, "ratchet_task_analyzer");
        assert!(!task_analyzer.parameters.is_empty());
        
        // Test prompt with parameters
        let mut arguments = HashMap::new();
        arguments.insert("task_config".to_string(), serde_json::json!("{\"name\": \"test-task\"}"));
        
        let request = GetPromptRequest {
            name: "ratchet_task_analyzer".to_string(),
            arguments: Some(arguments),
        };
        
        let rendered_prompt = prompt_registry
            .get_prompt_with_args(request, &context)
            .await
            .unwrap();
        
        assert_eq!(rendered_prompt.name, "ratchet_task_analyzer");
        assert!(!rendered_prompt.messages.is_empty());
        
        // Verify parameter substitution occurred
        let user_message = rendered_prompt.messages.iter()
            .find(|m| matches!(m.role, crate::axum_mcp_lib::server::prompt::MessageRole::User))
            .unwrap();
        
        match &user_message.content {
            crate::axum_mcp_lib::server::prompt::PromptContent::Text { text } => {
                assert!(text.contains("{\"name\": \"test-task\"}"));
                assert!(!text.contains("{{task_config}}"));
            },
            _ => panic!("Expected text content"),
        }
        
        // Test categories
        let categories = prompt_registry.list_categories(&context).await.unwrap();
        assert!(!categories.is_empty());
        
        let ratchet_ops = categories.iter()
            .find(|c| c.id == "ratchet_operations")
            .unwrap();
        
        assert_eq!(ratchet_ops.name, "Ratchet Operations");
        assert!(ratchet_ops.prompts.contains(&"ratchet_task_analyzer".to_string()));
    }

    #[tokio::test]
    async fn test_complete_integration() {
        // This test verifies that all components work together
        let repository_factory = Arc::new(MockRepositoryFactory);
        let logger = Arc::new(MockLogger);
        
        let state = RatchetServerState::new(repository_factory, logger);
        
        // Verify all registries are available
        assert!(state.resource_registry().is_some());
        assert!(state.prompt_registry().is_some());
        
        // Verify server capabilities reflect all features
        let capabilities = state.server_capabilities();
        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());
        
        // Verify server info contains expected metadata
        let info = state.server_info();
        let capabilities_list = info.metadata.get("capabilities").unwrap();
        let caps_array = capabilities_list.as_array().unwrap();
        
        assert!(caps_array.iter().any(|v| v.as_str() == Some("task_execution")));
        assert!(caps_array.iter().any(|v| v.as_str() == Some("resource_access")));
        assert!(caps_array.iter().any(|v| v.as_str() == Some("ai_workflows")));
        
        let uri_scheme = info.metadata.get("uri_scheme").unwrap();
        assert_eq!(uri_scheme.as_str().unwrap(), "ratchet://");
    }
}