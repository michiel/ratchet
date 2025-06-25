//! MCP client integration for console commands

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use ratchet_mcp::protocol::{
    Tool, ToolsListResult
};

use super::ConsoleConfig;

/// MCP client for console operations
pub struct ConsoleMcpClient {
    config: ConsoleConfig,
    http_client: Client,
    connected: bool,
    server_url: String,
    pub mcp_capabilities: Option<Value>,
    session_initialized: bool,
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
            mcp_capabilities: None,
            session_initialized: false,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<String> {
        // For now, use direct HTTP JSON-RPC instead of transport layer
        let mcp_url = if self.server_url.ends_with("/mcp") {
            self.server_url.clone()
        } else {
            format!("{}/mcp", self.server_url)
        };

        // Test connection with a simple initialize request
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "experimental": {},
                    "sampling": null
                },
                "clientInfo": {
                    "name": "ratchet-console",
                    "version": env!("CARGO_PKG_VERSION"),
                    "metadata": {}
                }
            },
            "id": "1"
        });

        let response = self.http_client
            .post(&mcp_url)
            .header("Content-Type", "application/json")
            .json(&init_request)
            .send()
            .await
            .map_err(|e| anyhow!("Connection failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("Connection failed with status: {}", response.status()));
        }

        let response_json: Value = response.json().await.map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        // Check if we got a valid initialize response
        if let Some(result) = response_json.get("result") {
            // Store capabilities as raw JSON for now
            if let Some(capabilities) = result.get("capabilities") {
                self.mcp_capabilities = Some(capabilities.clone());
            }
            
            self.connected = true;
            self.session_initialized = true;
            Ok(format!("ratchet-server@{}", mcp_url))
        } else {
            Err(anyhow!("Invalid initialize response: {}", response_json))
        }
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&mut self) {
        self.connected = false;
        self.session_initialized = false;
        self.mcp_capabilities = None;
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
                .send(),
        )
        .await??;

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
    pub async fn list_available_tools(&mut self) -> Result<Vec<Tool>> {
        if !self.session_initialized {
            return Err(anyhow!("MCP session not initialized"));
        }

        let mcp_url = if self.server_url.ends_with("/mcp") {
            self.server_url.clone()
        } else {
            format!("{}/mcp", self.server_url)
        };

        // Create tools/list request using direct HTTP
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": "list-tools"
        });

        let response = self.http_client
            .post(&mcp_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to list tools: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("Tools list failed with status: {}", response.status()));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse tools list response: {}", e))?;

        if let Some(result) = response_json.get("result") {
            let tools_result: ToolsListResult = serde_json::from_value(result.clone())
                .map_err(|e| anyhow!("Failed to parse tools list: {}", e))?;
            Ok(tools_result.tools)
        } else if let Some(error) = response_json.get("error") {
            Err(anyhow!("Tools list failed: {}", error))
        } else {
            Err(anyhow!("Tools list response missing result and error"))
        }
    }

    /// Get available tool names
    pub async fn get_tool_names(&mut self) -> Result<Vec<String>> {
        let tools = self.list_available_tools().await?;
        Ok(tools.into_iter().map(|tool| tool.name).collect())
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
                .send(),
        )
        .await??;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            Err(anyhow!("GraphQL request failed with status: {}", response.status()))
        }
    }

    /// Execute an MCP tool
    pub async fn execute_mcp_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        if !self.session_initialized {
            return Err(anyhow!("MCP session not initialized"));
        }

        let mcp_url = if self.server_url.ends_with("/mcp") {
            self.server_url.clone()
        } else {
            format!("{}/mcp", self.server_url)
        };

        // Create tools/call request using direct HTTP
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            },
            "id": format!("tool-{}", Uuid::new_v4())
        });

        let response = self.http_client
            .post(&mcp_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute tool: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("Tool execution failed with status: {}", response.status()));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse tool response: {}", e))?;

        if let Some(result) = response_json.get("result") {
            Ok(result.clone())
        } else if let Some(error) = response_json.get("error") {
            Err(anyhow!("Tool execution failed: {}", error))
        } else {
            Err(anyhow!("Tool execution response missing result and error"))
        }
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
