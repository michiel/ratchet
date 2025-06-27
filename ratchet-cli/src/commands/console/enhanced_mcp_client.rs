//! Enhanced MCP client with streaming support for console commands

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use futures::Stream;
use tokio::sync::mpsc;
use ratchet_mcp::protocol::{Tool, ToolsListResult};

use super::ConsoleConfig;

/// Enhanced MCP client with streaming capabilities
pub struct EnhancedMcpClient {
    config: ConsoleConfig,
    http_client: Client,
    connected: bool,
    server_url: String,
    mcp_capabilities: Option<Value>,
    session_initialized: bool,
}

/// Execution update for streaming progress
#[derive(Debug, Clone)]
pub struct ExecutionUpdate {
    pub execution_id: String,
    pub status: String,
    pub progress: Option<f64>,
    pub message: Option<String>,
    pub data: Option<Value>,
}

/// Batch execution request
#[derive(Debug, Clone)]
pub struct BatchRequest {
    pub id: String,
    pub task_id: String,
    pub input: Value,
    pub dependencies: Vec<String>,
}

/// Batch execution result
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub requests: Vec<BatchRequest>,
    pub results: HashMap<String, Result<Value, String>>,
    pub execution_order: Vec<String>,
    pub total_duration_ms: u64,
}

/// Execution filter for monitoring
#[derive(Debug, Clone)]
pub struct ExecutionFilter {
    pub status: Option<String>,
    pub task_id: Option<String>,
    pub since: Option<String>,
    pub limit: Option<usize>,
}

impl EnhancedMcpClient {
    /// Create a new enhanced MCP client
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

    /// Connect to the MCP server with enhanced handshake
    pub async fn connect(&mut self) -> Result<String> {
        let mcp_url = if self.server_url.ends_with("/mcp") {
            self.server_url.clone()
        } else {
            format!("{}/mcp", self.server_url)
        };

        // Enhanced initialize request with streaming capabilities
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "experimental": {
                        "streaming": true,
                        "batch": true,
                        "progress": true
                    },
                    "sampling": null
                },
                "clientInfo": {
                    "name": "ratchet-console-enhanced",
                    "version": env!("CARGO_PKG_VERSION"),
                    "metadata": {
                        "features": ["streaming", "batch", "interactive"]
                    }
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

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        if let Some(result) = response_json.get("result") {
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

    /// Execute task with streaming progress updates
    pub async fn execute_task_stream(
        &self,
        task_id: &str,
        input: Value,
    ) -> Result<impl Stream<Item = ExecutionUpdate>> {
        if !self.session_initialized {
            return Err(anyhow!("MCP session not initialized"));
        }

        let (tx, rx) = mpsc::channel(100);
        let task_id = task_id.to_string();
        let input_clone = input.clone();
        let client = self.http_client.clone();
        let server_url = self.server_url.clone();

        tokio::spawn(async move {
            // Start execution with progress streaming enabled
            let execute_request = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "ratchet_execute_task",
                    "arguments": {
                        "task_id": task_id,
                        "input": input_clone,
                        "trace": true,
                        "stream_progress": true,
                        "progress_filter": {
                            "min_progress_delta": 0.1,
                            "max_frequency_ms": 1000,
                            "include_data": true
                        }
                    }
                },
                "id": format!("stream-exec-{}", Uuid::new_v4())
            });

            let mcp_url = if server_url.ends_with("/mcp") {
                server_url
            } else {
                format!("{}/mcp", server_url)
            };

            // Execute the task
            if let Ok(response) = client
                .post(&mcp_url)
                .header("Content-Type", "application/json")
                .json(&execute_request)
                .send()
                .await
            {
                if let Ok(response_json) = response.json::<Value>().await {
                    if let Some(result) = response_json.get("result") {
                        if let Some(execution_id) = result.get("execution_id").and_then(|v| v.as_str()) {
                            let execution_id = execution_id.to_string();
                            
                            // Send initial update
                            let _ = tx.send(ExecutionUpdate {
                                execution_id: execution_id.clone(),
                                status: "started".to_string(),
                                progress: Some(0.0),
                                message: Some("Task execution started".to_string()),
                                data: Some(result.clone()),
                            }).await;

                            // Poll for progress updates
                            let mut last_status = "started".to_string();
                            while last_status != "completed" && last_status != "failed" {
                                tokio::time::sleep(Duration::from_millis(500)).await;

                                let status_request = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "method": "tools/call",
                                    "params": {
                                        "name": "ratchet_get_execution_status",
                                        "arguments": {
                                            "execution_id": execution_id
                                        }
                                    },
                                    "id": format!("status-{}", Uuid::new_v4())
                                });

                                if let Ok(status_response) = client
                                    .post(&mcp_url)
                                    .header("Content-Type", "application/json")
                                    .json(&status_request)
                                    .send()
                                    .await
                                {
                                    if let Ok(status_json) = status_response.json::<Value>().await {
                                        if let Some(status_result) = status_json.get("result") {
                                            if let Some(status) = status_result.get("status").and_then(|v| v.as_str()) {
                                                last_status = status.to_string();
                                                
                                                let progress = status_result.get("progress")
                                                    .and_then(|v| v.as_f64());
                                                
                                                let message = status_result.get("message")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string());

                                                let _ = tx.send(ExecutionUpdate {
                                                    execution_id: execution_id.clone(),
                                                    status: last_status.clone(),
                                                    progress,
                                                    message,
                                                    data: Some(status_result.clone()),
                                                }).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        use tokio_stream::wrappers::ReceiverStream;
        Ok(ReceiverStream::new(rx))
    }

    /// Execute multiple tasks with dependency resolution
    pub async fn batch_execute(&self, requests: Vec<BatchRequest>) -> Result<BatchResult> {
        if !self.session_initialized {
            return Err(anyhow!("MCP session not initialized"));
        }

        let batch_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "ratchet_batch_execute",
                "arguments": {
                    "requests": requests.iter().map(|req| serde_json::json!({
                        "id": req.id,
                        "task_id": req.task_id,
                        "input": req.input
                    })).collect::<Vec<_>>(),
                    "execution_mode": "parallel",
                    "max_parallel": 4,
                    "stop_on_error": false,
                    "correlation_token": format!("batch-{}", Uuid::new_v4())
                }
            },
            "id": format!("batch-{}", Uuid::new_v4())
        });

        let result = self.execute_mcp_tool_direct(batch_request).await?;
        
        // Parse batch result
        let mut results = HashMap::new();
        let mut execution_order = Vec::new();
        let mut total_duration_ms = 0;

        if let Some(batch_results) = result.get("results").and_then(|v| v.as_array()) {
            for batch_result in batch_results {
                if let Some(id) = batch_result.get("id").and_then(|v| v.as_str()) {
                    execution_order.push(id.to_string());
                    
                    if let Some(success) = batch_result.get("success").and_then(|v| v.as_bool()) {
                        if success {
                            if let Some(output) = batch_result.get("output") {
                                results.insert(id.to_string(), Ok(output.clone()));
                            }
                        } else {
                            let error_msg = batch_result.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error")
                                .to_string();
                            results.insert(id.to_string(), Err(error_msg));
                        }
                    }
                }
            }
        }

        if let Some(duration) = result.get("total_duration_ms").and_then(|v| v.as_u64()) {
            total_duration_ms = duration;
        }

        Ok(BatchResult {
            requests,
            results,
            execution_order,
            total_duration_ms,
        })
    }

    /// Monitor executions with real-time updates
    pub async fn monitor_executions(
        &self,
        filter: ExecutionFilter,
    ) -> Result<impl Stream<Item = Value>> {
        let (tx, rx) = mpsc::channel(100);
        let client = self.http_client.clone();
        let server_url = self.server_url.clone();

        tokio::spawn(async move {
            let mcp_url = if server_url.ends_with("/mcp") {
                server_url
            } else {
                format!("{}/mcp", server_url)
            };

            loop {
                let list_request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "ratchet_list_executions",
                        "arguments": {
                            "status": filter.status,
                            "task_id": filter.task_id,
                            "limit": filter.limit.unwrap_or(20),
                            "sort_by": "created_at",
                            "sort_order": "desc",
                            "include_output": false
                        }
                    },
                    "id": format!("monitor-{}", Uuid::new_v4())
                });

                if let Ok(response) = client
                    .post(&mcp_url)
                    .header("Content-Type", "application/json")
                    .json(&list_request)
                    .send()
                    .await
                {
                    if let Ok(response_json) = response.json::<Value>().await {
                        if let Some(result) = response_json.get("result") {
                            let _ = tx.send(result.clone()).await;
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        use tokio_stream::wrappers::ReceiverStream;
        Ok(ReceiverStream::new(rx))
    }

    /// Get available tools with enhanced metadata
    pub async fn list_tools_enhanced(&self) -> Result<Vec<Tool>> {
        let tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": "list-enhanced"
        });

        let result = self.execute_mcp_tool_direct(tools_request).await?;
        
        if let Some(tools_data) = result.get("tools") {
            let tools_result: ToolsListResult = serde_json::from_value(tools_data.clone())
                .map_err(|e| anyhow!("Failed to parse tools list: {}", e))?;
            Ok(tools_result.tools)
        } else {
            Err(anyhow!("No tools found in response"))
        }
    }

    /// Execute MCP tool directly with JSON-RPC request
    async fn execute_mcp_tool_direct(&self, request: Value) -> Result<Value> {
        let mcp_url = if self.server_url.ends_with("/mcp") {
            self.server_url.clone()
        } else {
            format!("{}/mcp", self.server_url)
        };

        let response = self.http_client
            .post(&mcp_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute MCP request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("MCP request failed with status: {}", response.status()));
        }

        let response_json: Value = response.json().await
            .map_err(|e| anyhow!("Failed to parse MCP response: {}", e))?;

        if let Some(result) = response_json.get("result") {
            Ok(result.clone())
        } else if let Some(error) = response_json.get("error") {
            Err(anyhow!("MCP request failed: {}", error))
        } else {
            Err(anyhow!("MCP response missing result and error"))
        }
    }

    /// Execute MCP tool with arguments
    pub async fn execute_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            },
            "id": format!("tool-{}", Uuid::new_v4())
        });

        self.execute_mcp_tool_direct(request).await
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get server capabilities
    pub fn get_capabilities(&self) -> Option<&Value> {
        self.mcp_capabilities.as_ref()
    }

    /// Check if server supports streaming
    pub fn supports_streaming(&self) -> bool {
        if let Some(capabilities) = &self.mcp_capabilities {
            capabilities.get("experimental")
                .and_then(|exp| exp.get("streaming"))
                .and_then(|s| s.as_bool())
                .unwrap_or(false)
        } else {
            false
        }
    }

    /// Check if server supports batch operations
    pub fn supports_batch(&self) -> bool {
        if let Some(capabilities) = &self.mcp_capabilities {
            capabilities.get("experimental")
                .and_then(|exp| exp.get("batch"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false)
        } else {
            false
        }
    }
}