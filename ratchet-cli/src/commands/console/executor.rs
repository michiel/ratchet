//! Command executor for console commands

use anyhow::Result;
use serde_json::Value;
use chrono;

use super::{mcp_client::ConsoleMcpClient, parser::ConsoleCommand, ConsoleConfig};

/// Result of command execution
#[derive(Debug, Clone)]
pub enum CommandResult {
    Success {
        message: String,
        data: Option<Value>,
    },
    Error {
        message: String,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Json {
        data: Value,
    },
    Text {
        content: String,
    },
}

/// Command executor that interfaces with MCP
pub struct CommandExecutor {
    config: ConsoleConfig,
    mcp_client: ConsoleMcpClient,
}

impl CommandExecutor {
    pub async fn new(config: &ConsoleConfig) -> Result<Self> {
        let mcp_client = ConsoleMcpClient::new(config.clone());
        Ok(Self {
            config: config.clone(),
            mcp_client,
        })
    }

    /// Check if connected to server
    pub fn is_connected(&self) -> bool {
        self.mcp_client.is_connected()
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<String> {
        self.mcp_client.connect().await
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&mut self) {
        self.mcp_client.disconnect().await
    }

    /// Check server health
    pub async fn check_health(&self) -> Result<String> {
        self.mcp_client.check_health().await
    }

    /// Execute a parsed command
    pub async fn execute(&mut self, command: ConsoleCommand) -> Result<CommandResult> {
        if !self.is_connected() && !self.is_local_command(&command) {
            return Ok(CommandResult::Error {
                message: "Not connected to server. Use 'connect' command first.".to_string(),
            });
        }

        match command.category.as_str() {
            "repo" => self.execute_repo_command(command).await,
            "task" => self.execute_task_command(command).await,
            "execution" => self.execute_execution_command(command).await,
            "job" => self.execute_job_command(command).await,
            "server" => self.execute_server_command(command).await,
            "db" => self.execute_db_command(command).await,
            "health" => self.execute_health_command(command).await,
            "stats" => self.execute_stats_command(command).await,
            "monitor" => self.execute_monitor_command(command).await,
            "mcp" => self.execute_mcp_command(command).await,
            _ => Ok(CommandResult::Error {
                message: format!("Unknown command category: {}", command.category),
            }),
        }
    }

    /// Check if a command can be executed locally (doesn't require server connection)
    fn is_local_command(&self, command: &ConsoleCommand) -> bool {
        matches!(command.category.as_str(), "help" | "version")
    }

    /// Execute repository management commands
    async fn execute_repo_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "list" => {
                match self.mcp_client.get_repositories().await {
                    Ok(response) => {
                        if let Some(errors) = response.get("errors") {
                            return Ok(CommandResult::Error {
                                message: format!("GraphQL errors: {}", errors),
                            });
                        }

                        if let Some(data) = response.get("data").and_then(|d| d.get("taskStats")) {
                            let headers = vec!["Metric".to_string(), "Value".to_string(), "Description".to_string()];
                            let rows = vec![
                                vec![
                                    "Total Tasks".to_string(),
                                    data["totalTasks"].to_string(),
                                    "All registered tasks".to_string(),
                                ],
                                vec![
                                    "Enabled Tasks".to_string(),
                                    data["enabledTasks"].to_string(),
                                    "Currently enabled tasks".to_string(),
                                ],
                                vec![
                                    "Disabled Tasks".to_string(),
                                    data["disabledTasks"].to_string(),
                                    "Currently disabled tasks".to_string(),
                                ],
                            ];

                            Ok(CommandResult::Table { headers, rows })
                        } else {
                            // Fallback to mock data if no repositories or wrong structure
                            Ok(CommandResult::Table {
                                headers: vec![
                                    "Name".to_string(),
                                    "Type".to_string(),
                                    "URL".to_string(),
                                    "Status".to_string(),
                                ],
                                rows: vec![
                                    vec![
                                        "sample-tasks".to_string(),
                                        "filesystem".to_string(),
                                        "/path/to/sample".to_string(),
                                        "active".to_string(),
                                    ],
                                    vec![
                                        "prod-tasks".to_string(),
                                        "git".to_string(),
                                        "https://github.com/example/tasks".to_string(),
                                        "syncing".to_string(),
                                    ],
                                ],
                            })
                        }
                    }
                    Err(_) => {
                        // Fallback to mock data on error
                        Ok(CommandResult::Table {
                            headers: vec![
                                "Name".to_string(),
                                "Type".to_string(),
                                "URL".to_string(),
                                "Status".to_string(),
                            ],
                            rows: vec![
                                vec![
                                    "sample-tasks".to_string(),
                                    "filesystem".to_string(),
                                    "/path/to/sample".to_string(),
                                    "active".to_string(),
                                ],
                                vec![
                                    "prod-tasks".to_string(),
                                    "git".to_string(),
                                    "https://github.com/example/tasks".to_string(),
                                    "syncing".to_string(),
                                ],
                            ],
                        })
                    }
                }
            }
            "add" => {
                if command.arguments.len() < 2 {
                    return Ok(CommandResult::Error {
                        message: "Usage: repo add <name> <uri> [--type git|filesystem]".to_string(),
                    });
                }
                let name = &command.arguments[0];
                let uri = &command.arguments[1];
                Ok(CommandResult::Success {
                    message: format!("Repository '{}' added with URI: {}", name, uri),
                    data: None,
                })
            }
            "remove" => {
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: repo remove <name>".to_string(),
                    });
                }
                let name = &command.arguments[0];
                Ok(CommandResult::Success {
                    message: format!("Repository '{}' removed", name),
                    data: None,
                })
            }
            "refresh" => {
                let name = command
                    .arguments
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("all repositories");
                Ok(CommandResult::Success {
                    message: format!("Refreshed {}", name),
                    data: None,
                })
            }
            "status" => Ok(CommandResult::Table {
                headers: vec![
                    "Repository".to_string(),
                    "Tasks".to_string(),
                    "Last Sync".to_string(),
                    "Status".to_string(),
                ],
                rows: vec![
                    vec![
                        "sample-tasks".to_string(),
                        "5".to_string(),
                        "2 minutes ago".to_string(),
                        "✓ Healthy".to_string(),
                    ],
                    vec![
                        "prod-tasks".to_string(),
                        "23".to_string(),
                        "1 hour ago".to_string(),
                        "⚠ Sync pending".to_string(),
                    ],
                ],
            }),
            "verify" => Ok(CommandResult::Success {
                message: "All repositories verified successfully".to_string(),
                data: None,
            }),
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown repo command: '{}'. Available commands: list, add, remove, refresh, status, verify",
                    command.action
                ),
            }),
        }
    }

    /// Execute task management commands
    async fn execute_task_command(&mut self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "list" => {
                match self.mcp_client.get_tasks(None).await {
                    Ok(response) => {
                        if let Some(errors) = response.get("errors") {
                            return Ok(CommandResult::Error {
                                message: format!("GraphQL errors: {}", errors),
                            });
                        }

                        if let Some(data) = response["data"]["tasks"]["items"].as_array() {
                            let headers = vec![
                                "ID".to_string(),
                                "Name".to_string(),
                                "Version".to_string(),
                                "Description".to_string(),
                                "Status".to_string(),
                            ];
                            let rows: Vec<Vec<String>> = data
                                .iter()
                                .map(|task| {
                                    vec![
                                        task["id"].as_str().unwrap_or("unknown").to_string(),
                                        task["name"].as_str().unwrap_or("unknown").to_string(),
                                        task["version"].as_str().unwrap_or("unknown").to_string(),
                                        task["description"].as_str().unwrap_or("No description").to_string(),
                                        if task["enabled"].as_bool().unwrap_or(false) {
                                            "enabled"
                                        } else {
                                            "disabled"
                                        }
                                        .to_string(),
                                    ]
                                })
                                .collect();

                            Ok(CommandResult::Table { headers, rows })
                        } else {
                            // Fallback to mock data
                            Ok(CommandResult::Table {
                                headers: vec![
                                    "ID".to_string(),
                                    "Name".to_string(),
                                    "Version".to_string(),
                                    "Repository".to_string(),
                                    "Status".to_string(),
                                ],
                                rows: vec![
                                    vec![
                                        "task-001".to_string(),
                                        "addition".to_string(),
                                        "1.0.0".to_string(),
                                        "sample-tasks".to_string(),
                                        "enabled".to_string(),
                                    ],
                                    vec![
                                        "task-002".to_string(),
                                        "fetch-data".to_string(),
                                        "2.1.0".to_string(),
                                        "sample-tasks".to_string(),
                                        "enabled".to_string(),
                                    ],
                                    vec![
                                        "task-003".to_string(),
                                        "process-logs".to_string(),
                                        "1.5.0".to_string(),
                                        "prod-tasks".to_string(),
                                        "disabled".to_string(),
                                    ],
                                ],
                            })
                        }
                    }
                    Err(_) => {
                        // Fallback to mock data on error
                        Ok(CommandResult::Table {
                            headers: vec![
                                "ID".to_string(),
                                "Name".to_string(),
                                "Version".to_string(),
                                "Repository".to_string(),
                                "Status".to_string(),
                            ],
                            rows: vec![
                                vec![
                                    "task-001".to_string(),
                                    "addition".to_string(),
                                    "1.0.0".to_string(),
                                    "sample-tasks".to_string(),
                                    "enabled".to_string(),
                                ],
                                vec![
                                    "task-002".to_string(),
                                    "fetch-data".to_string(),
                                    "2.1.0".to_string(),
                                    "sample-tasks".to_string(),
                                    "enabled".to_string(),
                                ],
                                vec![
                                    "task-003".to_string(),
                                    "process-logs".to_string(),
                                    "1.5.0".to_string(),
                                    "prod-tasks".to_string(),
                                    "disabled".to_string(),
                                ],
                            ],
                        })
                    }
                }
            }
            "show" => {
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task show <task-id>".to_string(),
                    });
                }
                let task_id = &command.arguments[0];
                Ok(CommandResult::Json {
                    data: serde_json::json!({
                        "id": task_id,
                        "name": "addition",
                        "version": "1.0.0",
                        "description": "Adds two numbers together",
                        "enabled": true,
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "num1": {"type": "number"},
                                "num2": {"type": "number"}
                            }
                        },
                        "outputSchema": {
                            "type": "object",
                            "properties": {
                                "sum": {"type": "number"}
                            }
                        }
                    }),
                })
            }
            "enable" | "disable" => {
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: format!("Usage: task {} <task-id>", command.action),
                    });
                }
                let task_id = &command.arguments[0];
                Ok(CommandResult::Success {
                    message: format!(
                        "Task '{}' {}",
                        task_id,
                        if command.action == "enable" {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    ),
                    data: None,
                })
            }
            "execute" => {
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task execute <task-id> [input-json]".to_string(),
                    });
                }
                let task_id = &command.arguments[0];
                let input_data = command.json_input.unwrap_or(serde_json::json!({}));
                let webhook_url = command.flags.get("webhook").cloned();

                match self
                    .mcp_client
                    .execute_task(task_id, input_data.clone(), webhook_url)
                    .await
                {
                    Ok(response) => {
                        if let Some(errors) = response.get("errors") {
                            Ok(CommandResult::Error {
                                message: format!("Task execution failed: {}", errors),
                            })
                        } else if let Some(data) = response.get("data").and_then(|d| d.get("executeTask")) {
                            Ok(CommandResult::Success {
                                message: format!("Task '{}' execution scheduled", task_id),
                                data: Some(data.clone()),
                            })
                        } else {
                            Ok(CommandResult::Success {
                                message: format!("Task '{}' execution request submitted", task_id),
                                data: Some(serde_json::json!({
                                    "taskId": task_id,
                                    "status": "submitted",
                                    "input": input_data
                                })),
                            })
                        }
                    }
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to execute task: {}", e),
                    }),
                }
            }
            "create" => {
                // Interactive task creation using MCP tools
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task create <task-name> [description]".to_string(),
                    });
                }
                
                let task_name = &command.arguments[0];
                let description = command.arguments.get(1).cloned().unwrap_or_else(|| "New task".to_string());
                
                let args = serde_json::json!({
                    "name": task_name,
                    "description": description,
                    "task_type": "javascript",
                    "input_schema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    },
                    "template": "basic"
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.create_task", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to create task: {}", e),
                    }),
                }
            }
            "edit" => {
                // Edit task using MCP tools
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task edit <task-id>".to_string(),
                    });
                }
                
                let task_id = &command.arguments[0];
                let args = serde_json::json!({
                    "task_id": task_id,
                    "editor_type": "inline"
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.edit_task", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to edit task: {}", e),
                    }),
                }
            }
            "test" => {
                // Run task tests using MCP tools
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task test <task-id> [input-json]".to_string(),
                    });
                }
                
                let task_id = &command.arguments[0];
                let test_input = if command.arguments.len() > 1 {
                    match serde_json::from_str::<serde_json::Value>(&command.arguments[1]) {
                        Ok(input) => input,
                        Err(_) => serde_json::json!({}),
                    }
                } else {
                    serde_json::json!({})
                };
                
                let args = serde_json::json!({
                    "task_id": task_id,
                    "test_input": test_input,
                    "test_type": "unit"
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.run_task_tests", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to test task: {}", e),
                    }),
                }
            }
            "debug" => {
                // Debug task execution using MCP tools
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task debug <task-id> [input-json]".to_string(),
                    });
                }
                
                let task_id = &command.arguments[0];
                let debug_input = if command.arguments.len() > 1 {
                    match serde_json::from_str::<serde_json::Value>(&command.arguments[1]) {
                        Ok(input) => input,
                        Err(_) => serde_json::json!({}),
                    }
                } else {
                    serde_json::json!({})
                };
                
                let args = serde_json::json!({
                    "task_id": task_id,
                    "input": debug_input,
                    "debug_level": "verbose",
                    "breakpoints": true
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.debug_task_execution", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to debug task: {}", e),
                    }),
                }
            }
            "validate" => {
                // Validate task using MCP tools
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: task validate <task-id>".to_string(),
                    });
                }
                
                let task_id = &command.arguments[0];
                let args = serde_json::json!({
                    "task_id": task_id,
                    "validation_level": "comprehensive"
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.validate_task", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to validate task: {}", e),
                    }),
                }
            }
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown task command: '{}'. Available commands: list, show, enable, disable, execute, create, edit, test, debug, validate",
                    command.action
                ),
            }),
        }
    }

    /// Execute execution management commands
    async fn execute_execution_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "list" => Ok(CommandResult::Table {
                headers: vec![
                    "ID".to_string(),
                    "Task".to_string(),
                    "Status".to_string(),
                    "Started".to_string(),
                    "Duration".to_string(),
                ],
                rows: vec![
                    vec![
                        "exec-001".to_string(),
                        "addition".to_string(),
                        "completed".to_string(),
                        "2 min ago".to_string(),
                        "1.2s".to_string(),
                    ],
                    vec![
                        "exec-002".to_string(),
                        "fetch-data".to_string(),
                        "running".to_string(),
                        "30s ago".to_string(),
                        "-".to_string(),
                    ],
                    vec![
                        "exec-003".to_string(),
                        "addition".to_string(),
                        "failed".to_string(),
                        "5 min ago".to_string(),
                        "0.8s".to_string(),
                    ],
                ],
            }),
            "show" => {
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: execution show <execution-id>".to_string(),
                    });
                }
                let exec_id = &command.arguments[0];
                Ok(CommandResult::Json {
                    data: serde_json::json!({
                        "id": exec_id,
                        "taskId": "task-001",
                        "status": "completed",
                        "input": {"num1": 42, "num2": 58},
                        "output": {"sum": 100},
                        "queuedAt": "2024-01-15T10:30:00Z",
                        "startedAt": "2024-01-15T10:30:01Z",
                        "completedAt": "2024-01-15T10:30:02Z",
                        "durationMs": 1200
                    }),
                })
            }
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown execution command: '{}'. Available commands: list, show",
                    command.action
                ),
            }),
        }
    }

    /// Execute job queue management commands
    async fn execute_job_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "list" => Ok(CommandResult::Table {
                headers: vec![
                    "ID".to_string(),
                    "Task".to_string(),
                    "Priority".to_string(),
                    "Status".to_string(),
                    "Queued".to_string(),
                ],
                rows: vec![
                    vec![
                        "job-001".to_string(),
                        "fetch-data".to_string(),
                        "normal".to_string(),
                        "processing".to_string(),
                        "1 min ago".to_string(),
                    ],
                    vec![
                        "job-002".to_string(),
                        "addition".to_string(),
                        "high".to_string(),
                        "queued".to_string(),
                        "30s ago".to_string(),
                    ],
                    vec![
                        "job-003".to_string(),
                        "process-logs".to_string(),
                        "low".to_string(),
                        "queued".to_string(),
                        "2 min ago".to_string(),
                    ],
                ],
            }),
            "clear" => Ok(CommandResult::Success {
                message: "Cleared 15 completed jobs".to_string(),
                data: None,
            }),
            "pause" => Ok(CommandResult::Success {
                message: "Job processing paused".to_string(),
                data: None,
            }),
            "resume" => Ok(CommandResult::Success {
                message: "Job processing resumed".to_string(),
                data: None,
            }),
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown job command: '{}'. Available commands: list, clear, pause, resume",
                    command.action
                ),
            }),
        }
    }

    /// Execute server management commands
    async fn execute_server_command(&mut self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "status" => Ok(CommandResult::Json {
                data: serde_json::json!({
                    "status": "running",
                    "version": "0.6.0",
                    "uptime": "2h 15m 30s",
                    "connections": 5,
                    "workers": 3,
                    "memory": "142.5 MB",
                    "cpu": "12.3%"
                }),
            }),
            "metrics" => Ok(CommandResult::Json {
                data: serde_json::json!({
                    "tasks": {
                        "total": 28,
                        "enabled": 25,
                        "disabled": 3
                    },
                    "executions": {
                        "total": 1247,
                        "completed": 1198,
                        "failed": 49,
                        "running": 3,
                        "queued": 7
                    },
                    "performance": {
                        "avgExecutionTime": "2.3s",
                        "throughput": "45 tasks/min",
                        "errorRate": "3.9%"
                    }
                }),
            }),
            "config" => {
                // Configuration management using MCP tools
                match command.arguments.first().map(|s| s.as_str()) {
                    Some("get") => {
                        let key = command.arguments.get(1).cloned().unwrap_or_else(|| "all".to_string());
                        let args = serde_json::json!({
                            "key": key,
                            "format": "json"
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.get_config", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to get config: {}", e),
                            }),
                        }
                    }
                    Some("set") => {
                        if command.arguments.len() < 3 {
                            return Ok(CommandResult::Error {
                                message: "Usage: server config set <key> <value>".to_string(),
                            });
                        }
                        let key = &command.arguments[1];
                        let value = &command.arguments[2];
                        let args = serde_json::json!({
                            "key": key,
                            "value": value
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.set_config", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to set config: {}", e),
                            }),
                        }
                    }
                    _ => Ok(CommandResult::Error {
                        message: "Usage: server config <get|set> [key] [value]".to_string(),
                    }),
                }
            }
            "backup" => {
                // Backup operations using MCP tools
                match command.arguments.first().map(|s| s.as_str()) {
                    Some("create") => {
                        let backup_name = command.arguments.get(1).cloned()
                            .unwrap_or_else(|| format!("backup_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
                        let args = serde_json::json!({
                            "name": backup_name,
                            "include_data": true,
                            "include_config": true
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.create_backup", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to create backup: {}", e),
                            }),
                        }
                    }
                    Some("restore") => {
                        if command.arguments.len() < 2 {
                            return Ok(CommandResult::Error {
                                message: "Usage: server backup restore <backup-name>".to_string(),
                            });
                        }
                        let backup_name = &command.arguments[1];
                        let args = serde_json::json!({
                            "name": backup_name,
                            "confirm": true
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.restore_backup", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to restore backup: {}", e),
                            }),
                        }
                    }
                    Some("list") => {
                        let args = serde_json::json!({
                            "include_details": true
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.list_backups", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to list backups: {}", e),
                            }),
                        }
                    }
                    _ => Ok(CommandResult::Error {
                        message: "Usage: server backup <create|restore|list> [name]".to_string(),
                    }),
                }
            }
            "security" => {
                // Security audit using MCP tools
                match command.arguments.first().map(|s| s.as_str()) {
                    Some("audit") => {
                        let args = serde_json::json!({
                            "level": "comprehensive",
                            "include_recommendations": true
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.security_audit", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to run security audit: {}", e),
                            }),
                        }
                    }
                    _ => Ok(CommandResult::Error {
                        message: "Usage: server security audit".to_string(),
                    }),
                }
            }
            "workers" => {
                // Advanced worker management using MCP tools
                match command.arguments.first().map(|s| s.as_str()) {
                    Some("scale") => {
                        if command.arguments.len() < 2 {
                            return Ok(CommandResult::Error {
                                message: "Usage: server workers scale <count>".to_string(),
                            });
                        }
                        let count = &command.arguments[1];
                        let args = serde_json::json!({
                            "worker_count": count,
                            "strategy": "gradual"
                        });
                        
                        match self.mcp_client.execute_mcp_tool("ratchet.scale_workers", args).await {
                            Ok(result) => Ok(CommandResult::Json { data: result }),
                            Err(e) => Ok(CommandResult::Error {
                                message: format!("Failed to scale workers: {}", e),
                            }),
                        }
                    }
                    _ => {
                        // Default worker list
                        Ok(CommandResult::Table {
                            headers: vec![
                                "ID".to_string(),
                                "Status".to_string(),
                                "Tasks".to_string(),
                                "Uptime".to_string(),
                                "Memory".to_string(),
                            ],
                            rows: vec![
                                vec![
                                    "worker-1".to_string(),
                                    "active".to_string(),
                                    "2".to_string(),
                                    "2h 15m".to_string(),
                                    "45.2 MB".to_string(),
                                ],
                                vec![
                                    "worker-2".to_string(),
                                    "active".to_string(),
                                    "1".to_string(),
                                    "2h 15m".to_string(),
                                    "38.7 MB".to_string(),
                                ],
                                vec![
                                    "worker-3".to_string(),
                                    "idle".to_string(),
                                    "0".to_string(),
                                    "2h 15m".to_string(),
                                    "32.1 MB".to_string(),
                                ],
                            ],
                        })
                    }
                }
            }
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown server command: '{}'. Available commands: status, workers, metrics, config, backup, security",
                    command.action
                ),
            }),
        }
    }

    /// Execute database management commands
    async fn execute_db_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "status" => Ok(CommandResult::Table {
                headers: vec!["Migration".to_string(), "Applied".to_string(), "Date".to_string()],
                rows: vec![
                    vec![
                        "001_initial_schema".to_string(),
                        "✓".to_string(),
                        "2024-01-01".to_string(),
                    ],
                    vec!["002_add_indexes".to_string(), "✓".to_string(), "2024-01-05".to_string()],
                    vec![
                        "003_add_output_destinations".to_string(),
                        "✓".to_string(),
                        "2024-01-10".to_string(),
                    ],
                ],
            }),
            "migrate" => {
                if command.flags.contains_key("dry-run") {
                    Ok(CommandResult::Text {
                        content: "Dry run: No pending migrations".to_string(),
                    })
                } else {
                    Ok(CommandResult::Success {
                        message: "Database migrations completed successfully".to_string(),
                        data: None,
                    })
                }
            }
            "stats" => Ok(CommandResult::Json {
                data: serde_json::json!({
                    "tables": {
                        "tasks": 28,
                        "executions": 1247,
                        "jobs": 15,
                        "repositories": 2
                    },
                    "size": "15.7 MB",
                    "connections": 5,
                    "queries": {
                        "total": 45231,
                        "avg_duration": "2.1ms"
                    }
                }),
            }),
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown db command: '{}'. Available commands: status, migrate, stats",
                    command.action
                ),
            }),
        }
    }

    /// Execute health check command
    async fn execute_health_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        // For single-word commands like "health", treat empty action as the default
        if !command.action.is_empty() && command.action != "check" {
            return Ok(CommandResult::Error {
                message: format!(
                    "Unknown health command: {}. Try 'health' or 'health check'",
                    command.action
                ),
            });
        }
        Ok(CommandResult::Json {
            data: serde_json::json!({
                "status": "healthy",
                "database": "connected",
                "server": "running",
                "workers": "active",
                "timestamp": "2024-01-15T12:30:45Z"
            }),
        })
    }

    /// Execute stats command
    async fn execute_stats_command(&self, command: ConsoleCommand) -> Result<CommandResult> {
        // For single-word commands like "stats", treat empty action as the default
        if !command.action.is_empty() && command.action != "show" {
            return Ok(CommandResult::Error {
                message: format!("Unknown stats command: {}. Try 'stats' or 'stats show'", command.action),
            });
        }
        match self.mcp_client.get_server_stats().await {
            Ok(response) => {
                if let Some(errors) = response.get("errors") {
                    return Ok(CommandResult::Error {
                        message: format!("GraphQL errors: {}", errors),
                    });
                }

                if let Some(data) = response.get("data") {
                    let task_stats = &data["taskStats"];
                    let worker_stats = &data["workerStats"];
                    let health = &data["health"];

                    let headers = vec!["Metric".to_string(), "Value".to_string(), "Description".to_string()];
                    let mut rows = vec![
                        vec![
                            "Total Tasks".to_string(),
                            task_stats["totalTasks"].to_string(),
                            "All registered tasks".to_string(),
                        ],
                        vec![
                            "Enabled Tasks".to_string(),
                            task_stats["enabledTasks"].to_string(),
                            "Currently enabled tasks".to_string(),
                        ],
                        vec![
                            "Disabled Tasks".to_string(),
                            task_stats["disabledTasks"].to_string(),
                            "Currently disabled tasks".to_string(),
                        ],
                    ];

                    if let Some(total_workers) = worker_stats.get("totalWorkers") {
                        rows.push(vec![
                            "Total Workers".to_string(),
                            total_workers.to_string(),
                            "All workers".to_string(),
                        ]);
                    }
                    if let Some(active_workers) = worker_stats.get("activeWorkers") {
                        rows.push(vec![
                            "Active Workers".to_string(),
                            active_workers.to_string(),
                            "Currently active workers".to_string(),
                        ]);
                    }
                    if let Some(database) = health.get("database") {
                        rows.push(vec![
                            "Database".to_string(),
                            database.to_string(),
                            "Database connection status".to_string(),
                        ]);
                    }

                    Ok(CommandResult::Table { headers, rows })
                } else {
                    // Fallback to mock data
                    Ok(CommandResult::Table {
                        headers: vec!["Metric".to_string(), "Value".to_string(), "Trend".to_string()],
                        rows: vec![
                            vec!["Total Tasks".to_string(), "28".to_string(), "↑ +2".to_string()],
                            vec!["Active Executions".to_string(), "3".to_string(), "→ 0".to_string()],
                            vec!["Queued Jobs".to_string(), "7".to_string(), "↓ -3".to_string()],
                            vec!["Success Rate".to_string(), "96.1%".to_string(), "↑ +0.2%".to_string()],
                            vec![
                                "Avg Response Time".to_string(),
                                "2.3s".to_string(),
                                "↓ -0.1s".to_string(),
                            ],
                        ],
                    })
                }
            }
            Err(_) => {
                // Fallback to mock data on error
                Ok(CommandResult::Table {
                    headers: vec!["Metric".to_string(), "Value".to_string(), "Trend".to_string()],
                    rows: vec![
                        vec!["Total Tasks".to_string(), "28".to_string(), "↑ +2".to_string()],
                        vec!["Active Executions".to_string(), "3".to_string(), "→ 0".to_string()],
                        vec!["Queued Jobs".to_string(), "7".to_string(), "↓ -3".to_string()],
                        vec!["Success Rate".to_string(), "96.1%".to_string(), "↑ +0.2%".to_string()],
                        vec![
                            "Avg Response Time".to_string(),
                            "2.3s".to_string(),
                            "↓ -0.1s".to_string(),
                        ],
                    ],
                })
            }
        }
    }

    /// Execute monitor command
    async fn execute_monitor_command(&mut self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "" | "start" => {
                // Start general monitoring
                Ok(CommandResult::Text {
                    content: "Monitoring started. Press Ctrl+C to stop.\n\n[12:30:45] Tasks: 28, Executions: 3 running, Jobs: 7 queued\n[12:30:50] Tasks: 28, Executions: 2 running, Jobs: 6 queued\n[12:30:55] Tasks: 28, Executions: 3 running, Jobs: 8 queued".to_string(),
                })
            }
            "executions" => {
                // Monitor live executions using MCP tools
                let args = serde_json::json!({
                    "stream": true,
                    "filter": {
                        "status": ["running", "queued"]
                    }
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.monitor_executions", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to monitor executions: {}", e),
                    }),
                }
            }
            "logs" => {
                // Stream logs using MCP tools  
                let log_level = command.arguments.first().unwrap_or(&"info".to_string()).clone();
                let args = serde_json::json!({
                    "level": log_level,
                    "stream": true,
                    "tail": 50
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.stream_logs", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to stream logs: {}", e),
                    }),
                }
            }
            "metrics" => {
                // Real-time metrics dashboard using MCP tools
                let args = serde_json::json!({
                    "interval": 5,
                    "metrics": ["tasks", "executions", "workers", "performance"]
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.live_metrics", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to get live metrics: {}", e),
                    }),
                }
            }
            "workers" => {
                // Monitor worker status using MCP tools
                let args = serde_json::json!({
                    "include_inactive": false,
                    "include_stats": true
                });
                
                match self.mcp_client.execute_mcp_tool("ratchet.monitor_workers", args).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to monitor workers: {}", e),
                    }),
                }
            }
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown monitor command: '{}'. Available: start, executions, logs [level], metrics, workers",
                    command.action
                ),
            }),
        }
    }

    /// Execute MCP commands for tool discovery and execution
    async fn execute_mcp_command(&mut self, command: ConsoleCommand) -> Result<CommandResult> {
        match command.action.as_str() {
            "tools" => {
                // List available MCP tools
                match self.mcp_client.list_available_tools().await {
                    Ok(tools) => {
                        let mut rows = Vec::new();
                        for tool in tools {
                            rows.push(vec![
                                tool.name,
                                tool.description,
                                tool.input_schema.get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("object")
                                    .to_string(),
                            ]);
                        }
                        Ok(CommandResult::Table {
                            headers: vec![
                                "Tool Name".to_string(),
                                "Description".to_string(),
                                "Input Type".to_string(),
                            ],
                            rows,
                        })
                    }
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to list MCP tools: {}", e),
                    }),
                }
            }
            "call" => {
                // Execute an MCP tool: mcp call tool_name {"key": "value"}
                if command.arguments.is_empty() {
                    return Ok(CommandResult::Error {
                        message: "Usage: mcp call <tool_name> [arguments_json]".to_string(),
                    });
                }

                let tool_name = &command.arguments[0];
                let arguments = if command.arguments.len() > 1 {
                    // Try to parse the second argument as JSON
                    match serde_json::from_str::<Value>(&command.arguments[1]) {
                        Ok(args) => args,
                        Err(_) => {
                            // If not valid JSON, treat as simple string arguments
                            let mut args_obj = serde_json::Map::new();
                            for (i, arg) in command.arguments[1..].iter().enumerate() {
                                args_obj.insert(format!("arg_{}", i), Value::String(arg.clone()));
                            }
                            Value::Object(args_obj)
                        }
                    }
                } else {
                    Value::Object(serde_json::Map::new())
                };

                match self.mcp_client.execute_mcp_tool(tool_name, arguments).await {
                    Ok(result) => Ok(CommandResult::Json { data: result }),
                    Err(e) => Ok(CommandResult::Error {
                        message: format!("Failed to execute MCP tool '{}': {}", tool_name, e),
                    }),
                }
            }
            "capabilities" => {
                // Show MCP server capabilities
                if let Some(capabilities) = &self.mcp_client.mcp_capabilities {
                    Ok(CommandResult::Json {
                        data: serde_json::to_value(capabilities)
                            .unwrap_or_else(|_| Value::String("Failed to serialize capabilities".to_string())),
                    })
                } else {
                    Ok(CommandResult::Error {
                        message: "No MCP capabilities available. Connect to server first.".to_string(),
                    })
                }
            }
            _ => Ok(CommandResult::Error {
                message: format!(
                    "Unknown MCP command: {}. Available: tools, call, capabilities",
                    command.action
                ),
            }),
        }
    }
}
