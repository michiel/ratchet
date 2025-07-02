//! Execution management commands for monitoring and controlling task executions

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::commands::console::{
    command_trait::{ConsoleCommand, CommandArgs, CommandOutput},
    enhanced_mcp_client::EnhancedMcpClient,
};

/// Execution management command
pub struct ExecutionCommand;

impl ExecutionCommand {
    pub fn new() -> Self {
        Self
    }

    /// List executions with filtering and pagination
    async fn list_executions(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let status = args.get_flag("status");
        let task_id = args.get_flag("task-id");
        let limit = args.get_number_flag("limit", 20);
        let offset = args.get_number_flag("offset", 0);
        let sort_by = args.get_flag("sort-by").unwrap_or("created_at");
        let sort_order = args.get_flag("sort-order").unwrap_or("desc");

        let mut list_args = json!({
            "limit": limit,
            "offset": offset,
            "sort_by": sort_by,
            "sort_order": sort_order,
            "include_output": args.has_flag("include-output"),
            "include_metadata": args.has_flag("detailed")
        });

        if let Some(status) = status {
            list_args["status"] = json!(status);
        }

        if let Some(task_id) = task_id {
            list_args["task_id"] = json!(task_id);
        }

        // Add date filtering if provided
        if let Some(since) = args.get_flag("since") {
            list_args["since"] = json!(since);
        }

        if let Some(until) = args.get_flag("until") {
            list_args["until"] = json!(until);
        }

        let result = mcp_client
            .execute_tool("ratchet_list_executions", list_args)
            .await?;

        if let Some(executions) = result.get("executions").and_then(|e| e.as_array()) {
            let mut headers = vec![
                "ID".to_string(),
                "Task ID".to_string(),
                "Status".to_string(),
                "Created".to_string(),
                "Duration".to_string(),
            ];

            if args.has_flag("detailed") {
                headers.extend_from_slice(&[
                    "Progress".to_string(),
                    "Worker".to_string(),
                    "Error".to_string(),
                ]);
            }

            let rows: Vec<Vec<String>> = executions
                .iter()
                .map(|execution| {
                    let mut row = vec![
                        execution.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        execution.get("task_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        execution.get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        execution.get("created_at")
                            .and_then(|v| v.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s).to_string())
                            .unwrap_or_else(|| "N/A".to_string()),
                        execution.get("duration_ms")
                            .and_then(|v| v.as_u64())
                            .map(|ms| format!("{}ms", ms))
                            .unwrap_or_else(|| "N/A".to_string()),
                    ];

                    if args.has_flag("detailed") {
                        row.push(
                            execution.get("progress")
                                .and_then(|v| v.as_f64())
                                .map(|p| format!("{:.1}%", p * 100.0))
                                .unwrap_or_else(|| "N/A".to_string())
                        );
                        row.push(
                            execution.get("worker_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A")
                                .to_string()
                        );
                        row.push(
                            execution.get("error")
                                .and_then(|v| v.get("message"))
                                .and_then(|v| v.as_str())
                                .map(|s| if s.len() > 50 { format!("{}...", &s[..47]) } else { s.to_string() })
                                .unwrap_or_else(|| "None".to_string())
                        );
                    }

                    row
                })
                .collect();

            let title = if let Some(status) = status {
                format!("Executions (Status: {})", status)
            } else {
                "Recent Executions".to_string()
            };

            Ok(CommandOutput::table_with_title(title, headers, rows))
        } else {
            Ok(CommandOutput::error("No executions found in response"))
        }
    }

    /// Show detailed execution information
    async fn show_execution(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let execution_id = args.require_positional(0, "execution ID")?;
        let include_logs = args.has_flag("logs");
        let include_trace = args.has_flag("trace");

        let show_args = json!({
            "execution_id": execution_id,
            "include_output": true,
            "include_metadata": true
        });

        let result = mcp_client
            .execute_tool("ratchet_get_execution_status", show_args)
            .await?;

        if let Some(execution) = result.get("execution") {
            let mut output = vec![
                format!("Execution ID: {}", execution.get("id").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Task ID: {}", execution.get("task_id").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Status: {}", execution.get("status").and_then(|v| v.as_str()).unwrap_or("unknown")),
                format!("Created: {}", execution.get("created_at").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Updated: {}", execution.get("updated_at").and_then(|v| v.as_str()).unwrap_or("N/A")),
            ];

            if let Some(duration) = execution.get("duration_ms").and_then(|v| v.as_u64()) {
                output.push(format!("Duration: {}ms", duration));
            }

            if let Some(progress) = execution.get("progress").and_then(|v| v.as_f64()) {
                output.push(format!("Progress: {:.1}%", progress * 100.0));
            }

            if let Some(worker) = execution.get("worker_id").and_then(|v| v.as_str()) {
                output.push(format!("Worker: {}", worker));
            }

            if let Some(input) = execution.get("input") {
                output.push(format!("Input: {}", serde_json::to_string_pretty(&input).unwrap_or_else(|_| "N/A".to_string())));
            }

            if let Some(output_data) = execution.get("output") {
                output.push(format!("Output: {}", serde_json::to_string_pretty(&output_data).unwrap_or_else(|_| "N/A".to_string())));
            }

            if let Some(error) = execution.get("error") {
                output.push(format!("Error: {}", serde_json::to_string_pretty(&error).unwrap_or_else(|_| "N/A".to_string())));
            }

            // Fetch logs if requested
            if include_logs {
                output.push("\n--- Execution Logs ---".to_string());
                let log_args = json!({"execution_id": execution_id});
                
                match mcp_client.execute_tool("ratchet_get_execution_logs", log_args).await {
                    Ok(log_result) => {
                        if let Some(logs) = log_result.get("logs").and_then(|l| l.as_array()) {
                            for log in logs {
                                if let Some(message) = log.get("message").and_then(|m| m.as_str()) {
                                    let timestamp = log.get("timestamp")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("N/A");
                                    let level = log.get("level")
                                        .and_then(|l| l.as_str())
                                        .unwrap_or("INFO");
                                    output.push(format!("[{}] {}: {}", timestamp, level, message));
                                }
                            }
                        } else {
                            output.push("No logs available".to_string());
                        }
                    }
                    Err(e) => {
                        output.push(format!("Failed to fetch logs: {}", e));
                    }
                }
            }

            // Fetch trace if requested
            if include_trace {
                output.push("\n--- Execution Trace ---".to_string());
                let trace_args = json!({"execution_id": execution_id});
                
                match mcp_client.execute_tool("ratchet_get_execution_trace", trace_args).await {
                    Ok(trace_result) => {
                        if let Some(trace) = trace_result.get("trace") {
                            output.push(serde_json::to_string_pretty(&trace).unwrap_or_else(|_| "N/A".to_string()));
                        } else {
                            output.push("No trace available".to_string());
                        }
                    }
                    Err(e) => {
                        output.push(format!("Failed to fetch trace: {}", e));
                    }
                }
            }

            Ok(CommandOutput::text(output.join("\n")))
        } else {
            Ok(CommandOutput::error(format!("Execution '{}' not found", execution_id)))
        }
    }

    /// Cancel a running execution
    async fn cancel_execution(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let execution_id = args.require_positional(0, "execution ID")?;
        let reason = args.get_flag("reason").unwrap_or("User requested cancellation");
        let force = args.has_flag("force");

        let cancel_args = json!({
            "execution_id": execution_id,
            "reason": reason,
            "force": force
        });

        let result = mcp_client
            .execute_tool("ratchet_cancel_execution", cancel_args)
            .await?;

        if let Some(success) = result.get("success").and_then(|s| s.as_bool()) {
            if success {
                Ok(CommandOutput::success_with_data(
                    format!("Execution '{}' cancelled successfully", execution_id),
                    result,
                ))
            } else {
                let error_msg = result.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                Ok(CommandOutput::error_with_context(
                    format!("Failed to cancel execution '{}': {}", execution_id, error_msg),
                    result,
                ))
            }
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }

    /// Retry a failed execution
    async fn retry_execution(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let execution_id = args.require_positional(0, "execution ID")?;
        let new_input = args.get_flag("input");
        let reset_state = args.has_flag("reset-state");

        let mut retry_args = json!({
            "execution_id": execution_id,
            "reset_state": reset_state
        });

        if let Some(input) = new_input {
            retry_args["new_input"] = args.parse_json(input)?;
        }

        let result = mcp_client
            .execute_tool("ratchet_retry_execution", retry_args)
            .await?;

        if let Some(new_execution_id) = result.get("new_execution_id").and_then(|id| id.as_str()) {
            Ok(CommandOutput::success_with_data(
                format!("Execution retried successfully. New execution ID: {}", new_execution_id),
                result,
            ))
        } else if let Some(error) = result.get("error") {
            Ok(CommandOutput::error_with_context(
                format!("Failed to retry execution '{}'", execution_id),
                error.clone(),
            ))
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }

    /// Analyze execution errors with AI assistance
    async fn analyze_execution(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let execution_id = args.require_positional(0, "execution ID")?;
        let include_suggestions = args.has_flag("suggestions");

        let analyze_args = json!({
            "execution_id": execution_id,
            "include_context": true,
            "include_suggestions": include_suggestions,
            "analysis_depth": args.get_flag("depth").unwrap_or("standard")
        });

        let result = mcp_client
            .execute_tool("ratchet_analyze_execution_error", analyze_args)
            .await?;

        if let Some(analysis) = result.get("analysis") {
            let mut output = vec![
                format!("Error Analysis for Execution: {}", execution_id),
                "".to_string(),
            ];

            if let Some(error_type) = analysis.get("error_type").and_then(|t| t.as_str()) {
                output.push(format!("Error Type: {}", error_type));
            }

            if let Some(root_cause) = analysis.get("root_cause").and_then(|c| c.as_str()) {
                output.push(format!("Root Cause: {}", root_cause));
            }

            if let Some(description) = analysis.get("description").and_then(|d| d.as_str()) {
                output.push(format!("Description: {}", description));
            }

            if include_suggestions {
                if let Some(suggestions) = analysis.get("suggestions").and_then(|s| s.as_array()) {
                    output.push("".to_string());
                    output.push("Suggested Actions:".to_string());
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        if let Some(text) = suggestion.as_str() {
                            output.push(format!("  {}. {}", i + 1, text));
                        }
                    }
                }
            }

            if let Some(related_executions) = analysis.get("related_executions").and_then(|r| r.as_array()) {
                if !related_executions.is_empty() {
                    output.push("".to_string());
                    output.push("Related Failed Executions:".to_string());
                    for related in related_executions {
                        if let Some(id) = related.get("id").and_then(|i| i.as_str()) {
                            output.push(format!("  - {}", id));
                        }
                    }
                }
            }

            Ok(CommandOutput::text(output.join("\n")))
        } else {
            Ok(CommandOutput::error(format!("Failed to analyze execution '{}'", execution_id)))
        }
    }
}

#[async_trait]
impl ConsoleCommand for ExecutionCommand {
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match args.action.as_str() {
            "list" | "ls" => self.list_executions(&args, mcp_client).await,
            "show" | "info" | "get" => self.show_execution(&args, mcp_client).await,
            "cancel" | "stop" => self.cancel_execution(&args, mcp_client).await,
            "retry" | "restart" => self.retry_execution(&args, mcp_client).await,
            "analyze" | "debug" => self.analyze_execution(&args, mcp_client).await,
            "help" | _ => Ok(CommandOutput::text(self.help_text().to_string())),
        }
    }

    fn completion_hints(&self, partial: &str) -> Vec<String> {
        let commands = vec!["list", "show", "cancel", "retry", "analyze", "help"];
        commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| cmd.to_string())
            .collect()
    }

    fn help_text(&self) -> &'static str {
        "Execution Management Commands:
  execution list [--status <status>] [--task-id <id>] [--limit <n>] [--detailed]
    List executions with optional filtering
    
  execution show <execution-id> [--logs] [--trace]
    Show detailed execution information
    
  execution cancel <execution-id> [--reason <text>] [--force]
    Cancel a running execution
    
  execution retry <execution-id> [--input <json>] [--reset-state]
    Retry a failed execution
    
  execution analyze <execution-id> [--suggestions] [--depth <level>]
    Analyze execution errors with AI assistance

Examples:
  execution list --status failed --limit 10
  execution show abc123 --logs --trace
  execution cancel xyz789 --reason \"User request\"
  execution retry abc123 --input '{\"param\": \"new_value\"}'
  execution analyze failed-exec --suggestions"
    }

    fn usage_examples(&self) -> Vec<&'static str> {
        vec![
            "execution list",
            "execution list --status running --detailed",
            "execution show abc123 --logs",
            "execution cancel xyz789",
            "execution retry failed-exec --reset-state",
            "execution analyze error-exec --suggestions",
        ]
    }

    fn category(&self) -> &'static str {
        "execution"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["exec", "run"]
    }

    fn requires_connection(&self) -> bool {
        true
    }

    fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        match args.action.as_str() {
            "show" | "cancel" | "retry" | "analyze" => {
                if args.positional.is_empty() {
                    return Err(anyhow!("Execution ID is required for {} command", args.action));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
