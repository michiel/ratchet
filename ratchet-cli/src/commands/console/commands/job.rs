//! Job management commands for scheduling and managing recurring tasks

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::commands::console::{
    command_trait::{ConsoleCommand, CommandArgs, CommandOutput},
    enhanced_mcp_client::EnhancedMcpClient,
};

/// Job management command
pub struct JobCommand;

impl JobCommand {
    pub fn new() -> Self {
        Self
    }

    /// List jobs with filtering and pagination
    async fn list_jobs(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let status = args.get_flag("status");
        let task_id = args.get_flag("task-id");
        let schedule_type = args.get_flag("schedule-type");
        let limit = args.get_number_flag("limit", 20);
        let offset = args.get_number_flag("offset", 0);
        let sort_by = args.get_flag("sort-by").unwrap_or("created_at");
        let sort_order = args.get_flag("sort-order").unwrap_or("desc");

        let mut list_args = json!({
            "limit": limit,
            "offset": offset,
            "sort_by": sort_by,
            "sort_order": sort_order,
            "include_metadata": args.has_flag("detailed"),
            "include_schedule": true
        });

        if let Some(status) = status {
            list_args["status"] = json!(status);
        }

        if let Some(task_id) = task_id {
            list_args["task_id"] = json!(task_id);
        }

        if let Some(schedule_type) = schedule_type {
            list_args["schedule_type"] = json!(schedule_type);
        }

        let result = mcp_client
            .execute_tool("ratchet_list_jobs", list_args)
            .await?;

        if let Some(jobs) = result.get("jobs").and_then(|j| j.as_array()) {
            let mut headers = vec![
                "ID".to_string(),
                "Name".to_string(),
                "Task ID".to_string(),
                "Status".to_string(),
                "Schedule".to_string(),
                "Next Run".to_string(),
                "Last Run".to_string(),
            ];

            if args.has_flag("detailed") {
                headers.extend_from_slice(&[
                    "Success Rate".to_string(),
                    "Total Runs".to_string(),
                    "Description".to_string(),
                ]);
            }

            let rows: Vec<Vec<String>> = jobs
                .iter()
                .map(|job| {
                    let mut row = vec![
                        job.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        job.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        job.get("task_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        job.get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        job.get("schedule")
                            .and_then(|s| s.get("expression"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A")
                            .to_string(),
                        job.get("next_run_at")
                            .and_then(|v| v.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s).to_string())
                            .unwrap_or_else(|| "N/A".to_string()),
                        job.get("last_run_at")
                            .and_then(|v| v.as_str())
                            .map(|s| s.split('T').next().unwrap_or(s).to_string())
                            .unwrap_or_else(|| "None".to_string()),
                    ];

                    if args.has_flag("detailed") {
                        let total_runs = job.get("total_runs").and_then(|v| v.as_u64()).unwrap_or(0);
                        let successful_runs = job.get("successful_runs").and_then(|v| v.as_u64()).unwrap_or(0);
                        let success_rate = if total_runs > 0 {
                            (successful_runs as f64 / total_runs as f64) * 100.0
                        } else {
                            0.0
                        };

                        row.push(format!("{:.1}%", success_rate));
                        row.push(total_runs.to_string());
                        row.push(
                            job.get("description")
                                .and_then(|v| v.as_str())
                                .map(|s| if s.len() > 40 { format!("{}...", &s[..37]) } else { s.to_string() })
                                .unwrap_or_else(|| "None".to_string())
                        );
                    }

                    row
                })
                .collect();

            let title = if let Some(status) = status {
                format!("Jobs (Status: {})", status)
            } else {
                "Scheduled Jobs".to_string()
            };

            Ok(CommandOutput::table_with_title(title, headers, rows))
        } else {
            Ok(CommandOutput::error("No jobs found in response"))
        }
    }

    /// Show detailed job information
    async fn show_job(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let job_id = args.require_positional(0, "job ID")?;
        let include_history = args.has_flag("history");
        let include_schedule = args.has_flag("schedule");

        let show_args = json!({
            "job_id": job_id,
            "include_metadata": true,
            "include_schedule_details": include_schedule
        });

        let result = mcp_client
            .execute_tool("ratchet_get_job_status", show_args)
            .await?;

        if let Some(job) = result.get("job") {
            let mut output = vec![
                format!("Job ID: {}", job.get("id").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Name: {}", job.get("name").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Task ID: {}", job.get("task_id").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Status: {}", job.get("status").and_then(|v| v.as_str()).unwrap_or("unknown")),
                format!("Created: {}", job.get("created_at").and_then(|v| v.as_str()).unwrap_or("N/A")),
                format!("Updated: {}", job.get("updated_at").and_then(|v| v.as_str()).unwrap_or("N/A")),
            ];

            if let Some(description) = job.get("description").and_then(|v| v.as_str()) {
                output.push(format!("Description: {}", description));
            }

            // Schedule Information
            if let Some(schedule) = job.get("schedule") {
                output.push("".to_string());
                output.push("Schedule Information:".to_string());
                output.push("─".repeat(30));
                
                if let Some(expression) = schedule.get("expression").and_then(|v| v.as_str()) {
                    output.push(format!("Expression: {}", expression));
                }
                
                if let Some(timezone) = schedule.get("timezone").and_then(|v| v.as_str()) {
                    output.push(format!("Timezone: {}", timezone));
                }
                
                if let Some(next_run) = job.get("next_run_at").and_then(|v| v.as_str()) {
                    output.push(format!("Next Run: {}", next_run));
                }
                
                if let Some(last_run) = job.get("last_run_at").and_then(|v| v.as_str()) {
                    output.push(format!("Last Run: {}", last_run));
                }
            }

            // Statistics
            output.push("".to_string());
            output.push("Statistics:".to_string());
            output.push("─".repeat(30));
            
            let total_runs = job.get("total_runs").and_then(|v| v.as_u64()).unwrap_or(0);
            let successful_runs = job.get("successful_runs").and_then(|v| v.as_u64()).unwrap_or(0);
            let failed_runs = job.get("failed_runs").and_then(|v| v.as_u64()).unwrap_or(0);
            let success_rate = if total_runs > 0 {
                (successful_runs as f64 / total_runs as f64) * 100.0
            } else {
                0.0
            };

            output.push(format!("Total Runs: {}", total_runs));
            output.push(format!("Successful Runs: {}", successful_runs));
            output.push(format!("Failed Runs: {}", failed_runs));
            output.push(format!("Success Rate: {:.1}%", success_rate));

            if let Some(avg_duration) = job.get("average_duration_ms").and_then(|v| v.as_u64()) {
                output.push(format!("Average Duration: {}ms", avg_duration));
            }

            // Job Configuration
            if let Some(config) = job.get("config") {
                output.push("".to_string());
                output.push("Configuration:".to_string());
                output.push("─".repeat(30));
                output.push(serde_json::to_string_pretty(&config).unwrap_or_else(|_| "N/A".to_string()));
            }

            // Recent execution history
            if include_history {
                output.push("".to_string());
                output.push("Recent Execution History:".to_string());
                output.push("─".repeat(30));
                
                let history_args = json!({"job_id": job_id, "limit": 10});
                match mcp_client.execute_tool("ratchet_get_job_history", history_args).await {
                    Ok(history_result) => {
                        if let Some(executions) = history_result.get("executions").and_then(|e| e.as_array()) {
                            for execution in executions {
                                let exec_id = execution.get("id").and_then(|i| i.as_str()).unwrap_or("N/A");
                                let status = execution.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                                let started = execution.get("started_at")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("N/A");
                                let duration = execution.get("duration_ms")
                                    .and_then(|d| d.as_u64())
                                    .map(|ms| format!("{}ms", ms))
                                    .unwrap_or_else(|| "N/A".to_string());
                                
                                output.push(format!("  {} | {} | {} | {}", 
                                    &exec_id[..8.min(exec_id.len())], 
                                    status, 
                                    started.split('T').next().unwrap_or(started),
                                    duration
                                ));
                            }
                        } else {
                            output.push("No execution history available".to_string());
                        }
                    }
                    Err(e) => {
                        output.push(format!("Failed to fetch execution history: {}", e));
                    }
                }
            }

            Ok(CommandOutput::text(output.join("\n")))
        } else {
            Ok(CommandOutput::error(format!("Job '{}' not found", job_id)))
        }
    }

    /// Create a new scheduled job
    async fn create_job(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let name = args.require_positional(0, "job name")?;
        let task_id = args.require_positional(1, "task ID")?;
        let schedule = args.require_positional(2, "schedule expression")?;
        
        let description = args.get_flag("description");
        let timezone = args.get_flag("timezone").unwrap_or("UTC");
        let enabled = args.get_bool_flag("enabled", true);
        let max_retries = args.get_number_flag("max-retries", 3);
        let config = args.get_flag("config");

        let mut create_args = json!({
            "name": name,
            "task_id": task_id,
            "schedule": {
                "expression": schedule,
                "timezone": timezone
            },
            "enabled": enabled,
            "max_retries": max_retries
        });

        if let Some(description) = description {
            create_args["description"] = json!(description);
        }

        if let Some(config_str) = config {
            create_args["config"] = args.parse_json(config_str)?;
        }

        let result = mcp_client
            .execute_tool("ratchet_create_job", create_args)
            .await?;

        if let Some(job_id) = result.get("job_id").and_then(|id| id.as_str()) {
            let next_run = result.get("next_run_at")
                .and_then(|nr| nr.as_str())
                .unwrap_or("Not scheduled");
            
            Ok(CommandOutput::success_with_data(
                format!("Job '{}' created successfully. ID: {}, Next run: {}", name, job_id, next_run),
                result,
            ))
        } else if let Some(error) = result.get("error") {
            Ok(CommandOutput::error_with_context(
                format!("Failed to create job '{}'", name),
                error.clone(),
            ))
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }

    /// Update an existing job
    async fn update_job(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let job_id = args.require_positional(0, "job ID")?;
        
        let mut update_args = json!({"job_id": job_id});
        let mut updates = Vec::new();

        if let Some(name) = args.get_flag("name") {
            update_args["name"] = json!(name);
            updates.push(format!("name -> {}", name));
        }

        if let Some(description) = args.get_flag("description") {
            update_args["description"] = json!(description);
            updates.push(format!("description -> {}", description));
        }

        if let Some(schedule) = args.get_flag("schedule") {
            update_args["schedule_expression"] = json!(schedule);
            updates.push(format!("schedule -> {}", schedule));
        }

        if let Some(timezone) = args.get_flag("timezone") {
            update_args["timezone"] = json!(timezone);
            updates.push(format!("timezone -> {}", timezone));
        }

        if args.has_flag("enable") {
            update_args["enabled"] = json!(true);
            updates.push("enabled -> true".to_string());
        }

        if args.has_flag("disable") {
            update_args["enabled"] = json!(false);
            updates.push("enabled -> false".to_string());
        }

        if let Some(max_retries) = args.get_flag("max-retries") {
            let retries: u32 = max_retries.parse()
                .map_err(|_| anyhow!("Invalid max-retries value: {}", max_retries))?;
            update_args["max_retries"] = json!(retries);
            updates.push(format!("max_retries -> {}", retries));
        }

        if let Some(config_str) = args.get_flag("config") {
            update_args["config"] = args.parse_json(config_str)?;
            updates.push("config -> <updated>".to_string());
        }

        if updates.is_empty() {
            return Ok(CommandOutput::error("No updates specified. Use --name, --description, --schedule, etc."));
        }

        let result = mcp_client
            .execute_tool("ratchet_update_job", update_args)
            .await?;

        if let Some(success) = result.get("success").and_then(|s| s.as_bool()) {
            if success {
                Ok(CommandOutput::success_with_data(
                    format!("Job '{}' updated successfully. Changes: {}", job_id, updates.join(", ")),
                    result,
                ))
            } else {
                let error_msg = result.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                Ok(CommandOutput::error_with_context(
                    format!("Failed to update job '{}': {}", job_id, error_msg),
                    result,
                ))
            }
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }

    /// Delete a job
    async fn delete_job(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let job_id = args.require_positional(0, "job ID")?;
        let force = args.has_flag("force");

        let delete_args = json!({
            "job_id": job_id,
            "force": force
        });

        let result = mcp_client
            .execute_tool("ratchet_delete_job", delete_args)
            .await?;

        if let Some(success) = result.get("success").and_then(|s| s.as_bool()) {
            if success {
                Ok(CommandOutput::success_with_data(
                    format!("Job '{}' deleted successfully", job_id),
                    result,
                ))
            } else {
                let error_msg = result.get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error");
                Ok(CommandOutput::error_with_context(
                    format!("Failed to delete job '{}': {}", job_id, error_msg),
                    result,
                ))
            }
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }

    /// Trigger a job to run immediately
    async fn trigger_job(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let job_id = args.require_positional(0, "job ID")?;
        let override_config = args.get_flag("config");
        let wait_for_completion = args.has_flag("wait");

        let mut trigger_args = json!({
            "job_id": job_id,
            "wait_for_completion": wait_for_completion
        });

        if let Some(config_str) = override_config {
            trigger_args["override_config"] = args.parse_json(config_str)?;
        }

        let result = mcp_client
            .execute_tool("ratchet_trigger_job", trigger_args)
            .await?;

        if let Some(execution_id) = result.get("execution_id").and_then(|id| id.as_str()) {
            if wait_for_completion {
                if let Some(execution_result) = result.get("execution_result") {
                    let status = execution_result.get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown");
                    
                    match status {
                        "completed" => Ok(CommandOutput::success_with_data(
                            format!("Job '{}' executed successfully. Execution ID: {}", job_id, execution_id),
                            result,
                        )),
                        "failed" => Ok(CommandOutput::error_with_context(
                            format!("Job '{}' execution failed. Execution ID: {}", job_id, execution_id),
                            execution_result.clone(),
                        )),
                        _ => Ok(CommandOutput::text(format!("Job '{}' triggered. Execution ID: {}, Status: {}", job_id, execution_id, status))),
                    }
                } else {
                    Ok(CommandOutput::error("Execution completed but no result returned"))
                }
            } else {
                Ok(CommandOutput::success_with_data(
                    format!("Job '{}' triggered successfully. Execution ID: {}", job_id, execution_id),
                    result,
                ))
            }
        } else if let Some(error) = result.get("error") {
            Ok(CommandOutput::error_with_context(
                format!("Failed to trigger job '{}'", job_id),
                error.clone(),
            ))
        } else {
            Ok(CommandOutput::error("Invalid response from server"))
        }
    }
}

#[async_trait]
impl ConsoleCommand for JobCommand {
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match args.action.as_str() {
            "list" | "ls" => self.list_jobs(&args, mcp_client).await,
            "show" | "info" | "get" => self.show_job(&args, mcp_client).await,
            "create" | "add" | "new" => self.create_job(&args, mcp_client).await,
            "update" | "edit" | "modify" => self.update_job(&args, mcp_client).await,
            "delete" | "remove" | "rm" => self.delete_job(&args, mcp_client).await,
            "trigger" | "run" | "execute" => self.trigger_job(&args, mcp_client).await,
            "help" | _ => Ok(CommandOutput::text(self.help_text().to_string())),
        }
    }

    fn completion_hints(&self, partial: &str) -> Vec<String> {
        let commands = vec!["list", "show", "create", "update", "delete", "trigger", "help"];
        commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| cmd.to_string())
            .collect()
    }

    fn help_text(&self) -> &'static str {
        "Job Management Commands:
  job list [--status <status>] [--task-id <id>] [--schedule-type <type>] [--detailed]
    List scheduled jobs with optional filtering
    
  job show <job-id> [--history] [--schedule]
    Show detailed job information and execution history
    
  job create <name> <task-id> <schedule> [--description <text>] [--timezone <tz>] 
    [--enabled] [--max-retries <n>] [--config <json>]
    Create a new scheduled job
    
  job update <job-id> [--name <name>] [--description <text>] [--schedule <cron>]
    [--timezone <tz>] [--enable|--disable] [--max-retries <n>] [--config <json>]
    Update an existing job
    
  job delete <job-id> [--force]
    Delete a scheduled job
    
  job trigger <job-id> [--config <json>] [--wait]
    Trigger a job to run immediately

Examples:
  job list --status active --detailed
  job show abc123 --history
  job create 'daily-backup' task-456 '0 2 * * *' --description 'Daily backup job'
  job update xyz789 --schedule '0 3 * * *' --timezone 'America/New_York'
  job trigger abc123 --wait
  job delete old-job --force"
    }

    fn usage_examples(&self) -> Vec<&'static str> {
        vec![
            "job list",
            "job list --status active --detailed",
            "job show abc123 --history",
            "job create 'hourly-sync' task-123 '0 * * * *'",
            "job update job-456 --enable --schedule '0 */2 * * *'",
            "job trigger job-789 --wait",
            "job delete old-job",
        ]
    }

    fn category(&self) -> &'static str {
        "job"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["jobs", "schedule", "cron"]
    }

    fn requires_connection(&self) -> bool {
        true
    }

    fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        match args.action.as_str() {
            "show" | "update" | "delete" | "trigger" => {
                if args.positional.is_empty() {
                    return Err(anyhow!("Job ID is required for {} command", args.action));
                }
            }
            "create" => {
                if args.positional.len() < 3 {
                    return Err(anyhow!("Create command requires: <name> <task-id> <schedule>"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}