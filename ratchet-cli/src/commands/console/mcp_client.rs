//! MCP client integration for console commands

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use reqwest::Client;
use tokio::time::{timeout, Duration};

use super::ConsoleConfig;

/// MCP client for console operations
pub struct ConsoleMcpClient {
    config: ConsoleConfig,
    http_client: Client,
    connected: bool,
    server_url: String,
}

impl ConsoleMcpClient {
    /// Create a new MCP client
    pub fn new(config: ConsoleConfig) -> Self {
        let server_url = if let Some(connect_url) = &config.connect_url {
            connect_url.clone()
        } else {
            format!("http://{}:{}", config.host, config.port)
        };

        Self {
            config,
            http_client: Client::new(),
            connected: false,
            server_url,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<String> {
        // Test connection by trying to list tools
        match self.list_available_tools().await {
            Ok(_) => {
                self.connected = true;
                Ok(format!("ratchet-server@{}", self.server_url))
            }
            Err(e) => {
                self.connected = false;
                Err(anyhow!("Failed to connect to MCP server: {}", e))
            }
        }
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&mut self) {
        self.connected = false;
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Check server health
    pub async fn check_health(&self) -> Result<String> {
        if !self.connected {
            return Err(anyhow!("Not connected to server"));
        }

        // Try to make a simple GraphQL health query
        let health_query = r#"
            query {
                health {
                    database
                    message
                }
            }
        "#;

        let response = timeout(
            Duration::from_secs(10),
            self.http_client
                .post(format!("{}/graphql", self.server_url))
                .json(&serde_json::json!({
                    "query": health_query
                }))
                .send()
        ).await??;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            if result.get("errors").is_none() {
                Ok("healthy".to_string())
            } else {
                Ok("degraded".to_string())
            }
        } else {
            Err(anyhow!("Server returned status: {}", response.status()))
        }
    }

    /// List available MCP tools
    pub async fn list_available_tools(&self) -> Result<Vec<String>> {
        // For now, return the known tools from the registry
        // In a full implementation, this would query the MCP server
        Ok(vec![
            "ratchet.create_task".to_string(),
            "ratchet.validate_task".to_string(),
            "ratchet.debug_task_execution".to_string(),
            "ratchet.run_task_tests".to_string(),
            "ratchet.create_task_version".to_string(),
            "ratchet.edit_task".to_string(),
            "ratchet.delete_task".to_string(),
            "ratchet.import_tasks".to_string(),
            "ratchet.export_tasks".to_string(),
            "ratchet.store_result".to_string(),
            "ratchet.get_results".to_string(),
        ])
    }

    /// Execute a GraphQL query
    pub async fn execute_graphql_query(&self, query: &str, variables: Option<Value>) -> Result<Value> {
        if !self.connected {
            return Err(anyhow!("Not connected to server"));
        }

        let mut request_body = serde_json::json!({
            "query": query
        });

        if let Some(vars) = variables {
            request_body["variables"] = vars;
        }

        let response = timeout(
            Duration::from_secs(30),
            self.http_client
                .post(format!("{}/graphql", self.server_url))
                .json(&request_body)
                .send()
        ).await??;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            Err(anyhow!("GraphQL request failed with status: {}", response.status()))
        }
    }

    /// Execute an MCP tool (placeholder for future implementation)
    pub async fn execute_mcp_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        if !self.connected {
            return Err(anyhow!("Not connected to server"));
        }

        // For now, return a mock response indicating MCP tools aren't fully integrated yet
        Ok(serde_json::json!({
            "tool": tool_name,
            "status": "mock_response",
            "message": "MCP tool integration not yet implemented",
            "arguments": arguments
        }))
    }

    /// Get repository list via GraphQL (uses tasks endpoint as repository info isn't directly available)
    pub async fn get_repositories(&self) -> Result<Value> {
        let query = r#"
            query {
                taskStats {
                    totalTasks
                    enabledTasks
                    disabledTasks
                }
            }
        "#;

        self.execute_graphql_query(query, None).await
    }

    /// Get task list via GraphQL
    pub async fn get_tasks(&self, _filters: Option<HashMap<String, Value>>) -> Result<Value> {
        let query = r#"
            query {
                tasks {
                    items {
                        id
                        name
                        description
                        version
                        enabled
                        inputSchema
                        outputSchema
                        metadata
                        createdAt
                        updatedAt
                    }
                    meta {
                        total
                        page
                        limit
                    }
                }
            }
        "#;

        self.execute_graphql_query(query, None).await
    }

    /// Get executions via GraphQL
    pub async fn get_executions(&self, filters: Option<HashMap<String, Value>>) -> Result<Value> {
        let query = r#"
            query GetExecutions($filters: ExecutionFilters) {
                executions(filters: $filters) {
                    items {
                        id
                        uuid
                        taskId
                        input
                        output
                        status
                        errorMessage
                        errorDetails
                        queuedAt
                        startedAt
                        completedAt
                        durationMs
                        httpRequests
                        recordingPath
                        canRetry
                        canCancel
                        progress
                    }
                    meta {
                        page
                        limit
                        total
                        totalPages
                        hasNext
                        hasPrevious
                    }
                }
            }
        "#;

        let variables = filters.map(|f| serde_json::to_value(f).unwrap_or_default());
        self.execute_graphql_query(query, variables).await
    }

    /// Get jobs via GraphQL
    pub async fn get_jobs(&self, filters: Option<HashMap<String, Value>>) -> Result<Value> {
        let query = r#"
            query GetJobs($filters: JobFilters) {
                jobs(filters: $filters) {
                    items {
                        id
                        taskId
                        priority
                        status
                        retryCount
                        maxRetries
                        queuedAt
                        scheduledFor
                        errorMessage
                        outputDestinations {
                            destinationType
                            template
                        }
                    }
                    meta {
                        page
                        limit
                        total
                        totalPages
                        hasNext
                        hasPrevious
                    }
                }
            }
        "#;

        let variables = filters.map(|f| serde_json::to_value(f).unwrap_or_default());
        self.execute_graphql_query(query, variables).await
    }

    /// Execute a task via GraphQL
    pub async fn execute_task(&self, task_id: &str, input_data: Value, webhook_url: Option<String>) -> Result<Value> {
        let mut output_destinations = Vec::new();
        
        if let Some(webhook) = webhook_url {
            output_destinations.push(serde_json::json!({
                "destinationType": "WEBHOOK",
                "webhook": {
                    "url": webhook,
                    "method": "POST",
                    "contentType": "application/json",
                    "retryPolicy": {
                        "maxAttempts": 3,
                        "initialDelayMs": 1000,
                        "maxDelayMs": 5000,
                        "backoffMultiplier": 2.0
                    }
                }
            }));
        }

        let mutation = r#"
            mutation ExecuteTask($input: ExecuteTaskInput!) {
                executeTask(input: $input) {
                    id
                    taskId
                    priority
                    status
                    queuedAt
                    outputDestinations {
                        destinationType
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "input": {
                "taskId": task_id,
                "inputData": input_data,
                "priority": "NORMAL",
                "outputDestinations": output_destinations
            }
        });

        self.execute_graphql_query(mutation, Some(variables)).await
    }

    /// Get server statistics via GraphQL
    pub async fn get_server_stats(&self) -> Result<Value> {
        let query = r#"
            query SystemStats {
                taskStats {
                    totalTasks
                    enabledTasks
                    disabledTasks
                }
                workerStats {
                    totalWorkers
                    activeWorkers
                    idleWorkers
                }
                health {
                    database
                    message
                }
            }
        "#;

        self.execute_graphql_query(query, None).await
    }
}