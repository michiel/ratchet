//! Enhanced task development commands with full MCP integration

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::commands::console::{
    command_trait::{ConsoleCommand, CommandArgs, CommandOutput, InteractiveCommand},
    enhanced_mcp_client::EnhancedMcpClient,
};

/// Enhanced task command with comprehensive MCP integration
pub struct EnhancedTaskCommand;

impl EnhancedTaskCommand {
    pub fn new() -> Self {
        Self
    }

    /// Interactive task creation wizard
    async fn create_task_interactive(&self, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        // For now, return a simulated interactive creation
        // In a full implementation, this would use a proper TUI library
        Ok(CommandOutput::success("Interactive task creation not yet implemented. Use: task create <name> --description \"desc\""))
    }

    /// Create task with template support
    async fn create_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let name = args.require_positional(0, "task name")?;
        let description = args.get_flag("description").unwrap_or("Generated task");
        let version = args.get_flag("version").unwrap_or("1.0.0");
        let template = args.get_flag("template");

        let mut create_args = json!({
            "name": name,
            "description": description,
            "version": version,
            "enabled": true
        });

        // If template specified, generate from template first
        if let Some(template_name) = template {
            let template_args = json!({
                "template": template_name,
                "name": name,
                "description": description,
                "parameters": {}
            });

            let template_result = mcp_client
                .execute_tool("ratchet_generate_from_template", template_args)
                .await?;

            if let Some(code) = template_result.get("code") {
                create_args["code"] = code.clone();
            }
            if let Some(input_schema) = template_result.get("input_schema") {
                create_args["input_schema"] = input_schema.clone();
            }
            if let Some(output_schema) = template_result.get("output_schema") {
                create_args["output_schema"] = output_schema.clone();
            }
        } else {
            // Default task implementation
            create_args["code"] = json!(format!(
                "async function main(input) {{\n  // TODO: Implement {} logic\n  return {{ message: \"Hello from {}!\" }};\n}}",
                name, name
            ));
            create_args["input_schema"] = json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            });
            create_args["output_schema"] = json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            });
        }

        let result = mcp_client
            .execute_tool("ratchet_create_task", create_args)
            .await?;

        if let Some(task_id) = result.get("task_id") {
            Ok(CommandOutput::success_with_data(
                format!("Task '{}' created successfully", name),
                json!({
                    "task_id": task_id,
                    "name": name,
                    "template_used": template
                }),
            ))
        } else {
            Ok(CommandOutput::error("Failed to create task"))
        }
    }

    /// Edit existing task
    async fn edit_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;

        let mut edit_args = json!({
            "task_id": task_id
        });

        // Add optional edit parameters
        if let Some(code) = args.get_flag("code") {
            edit_args["code"] = json!(code);
        }
        if let Some(description) = args.get_flag("description") {
            edit_args["description"] = json!(description);
        }
        if let Some(input_schema) = args.get_flag("input-schema") {
            edit_args["input_schema"] = args.parse_json(input_schema)?;
        }
        if let Some(output_schema) = args.get_flag("output-schema") {
            edit_args["output_schema"] = args.parse_json(output_schema)?;
        }

        let result = mcp_client
            .execute_tool("ratchet_edit_task", edit_args)
            .await?;

        Ok(CommandOutput::success_with_data(
            format!("Task '{}' updated successfully", task_id),
            result,
        ))
    }

    /// Validate task with comprehensive checks
    async fn validate_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;
        let run_tests = args.has_flag("run-tests");
        let fix_issues = args.has_flag("fix");

        let validate_args = json!({
            "task_id": task_id,
            "run_tests": run_tests,
            "fix_issues": fix_issues,
            "syntax_only": false
        });

        let result = mcp_client
            .execute_tool("ratchet_validate_task", validate_args)
            .await?;

        let is_valid = result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
        
        if is_valid {
            Ok(CommandOutput::success_with_data("Task validation passed", result))
        } else {
            let errors = result.get("errors").cloned().unwrap_or(json!([]));
            Ok(CommandOutput::error_with_context(
                "Task validation failed",
                json!({
                    "task_id": task_id,
                    "errors": errors,
                    "validation_result": result
                }),
            ))
        }
    }

    /// Test task execution
    async fn test_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;
        let test_names = args.get_flag("test-names");
        let parallel = args.get_bool_flag("parallel", false);
        let input_data = args.get_flag("input");

        let mut test_args = json!({
            "task_id": task_id,
            "parallel": parallel,
            "stop_on_failure": false,
            "include_traces": true
        });

        if let Some(names) = test_names {
            let names: Vec<&str> = names.split(',').collect();
            test_args["test_names"] = json!(names);
        }

        if let Some(input) = input_data {
            test_args["custom_input"] = args.parse_json(input)?;
        }

        let result = mcp_client
            .execute_tool("ratchet_run_task_tests", test_args)
            .await?;

        let passed = result.get("passed").and_then(|v| v.as_u64()).unwrap_or(0);
        let failed = result.get("failed").and_then(|v| v.as_u64()).unwrap_or(0);
        let total = passed + failed;

        if failed == 0 {
            Ok(CommandOutput::success_with_data(
                format!("All {} tests passed", total),
                result,
            ))
        } else {
            Ok(CommandOutput::error_with_context(
                format!("{} of {} tests failed", failed, total),
                result,
            ))
        }
    }

    /// Debug task execution with breakpoints
    async fn debug_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;
        let input_data = args.get_flag("input").unwrap_or("{}");
        let breakpoints_str = args.get_flag("breakpoints");
        let step_mode = args.has_flag("step");

        let mut debug_args = json!({
            "task_id": task_id,
            "input": args.parse_json(input_data)?,
            "capture_variables": true,
            "timeout_ms": 60000
        });

        if let Some(bp_str) = breakpoints_str {
            let breakpoints: Result<Vec<u32>, _> = bp_str
                .split(',')
                .map(|s| s.trim().parse())
                .collect();
            
            if let Ok(bp_lines) = breakpoints {
                debug_args["breakpoints"] = json!(bp_lines);
            }
        }

        if step_mode {
            debug_args["step_mode"] = json!(true);
        }

        let result = mcp_client
            .execute_tool("ratchet_debug_task_execution", debug_args)
            .await?;

        Ok(CommandOutput::success_with_data(
            "Debug session completed",
            result,
        ))
    }

    /// Create new task version
    async fn create_version(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;
        let new_version = args.require_positional(1, "new version")?;
        let description = args.get_flag("description").unwrap_or("New version");
        let breaking_change = args.has_flag("breaking");
        let make_active = args.get_bool_flag("make-active", true);

        let version_args = json!({
            "task_id": task_id,
            "new_version": new_version,
            "description": description,
            "breaking_change": breaking_change,
            "make_active": make_active
        });

        let result = mcp_client
            .execute_tool("ratchet_create_task_version", version_args)
            .await?;

        Ok(CommandOutput::success_with_data(
            format!("Created version {} for task {}", new_version, task_id),
            result,
        ))
    }

    /// Execute task with enhanced options
    async fn execute_task(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let task_id = args.require_positional(0, "task ID")?;
        let input_data = args.get_flag("input").unwrap_or("{}");
        let trace = args.has_flag("trace");
        let stream_progress = args.has_flag("stream");
        let timeout = args.get_number_flag("timeout", 30u32);

        let execute_args = json!({
            "task_id": task_id,
            "input": args.parse_json(input_data)?,
            "trace": trace,
            "stream_progress": stream_progress,
            "timeout": timeout
        });

        if stream_progress && mcp_client.supports_streaming() {
            // Use streaming execution
            let _stream = mcp_client.execute_task_stream(task_id, args.parse_json(input_data)?).await?;
            
            // For now, just execute normally and return result
            // In full implementation, this would return the stream
            let result = mcp_client
                .execute_tool("ratchet_execute_task", execute_args)
                .await?;

            Ok(CommandOutput::success_with_data("Task executed with streaming", result))
        } else {
            // Regular execution
            let result = mcp_client
                .execute_tool("ratchet_execute_task", execute_args)
                .await?;

            let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
            
            if status == "completed" {
                Ok(CommandOutput::success_with_data("Task executed successfully", result))
            } else if status == "failed" {
                Ok(CommandOutput::error_with_context("Task execution failed", result))
            } else {
                Ok(CommandOutput::success_with_data(
                    format!("Task execution {}", status),
                    result,
                ))
            }
        }
    }

    /// List tasks with enhanced filtering
    async fn list_tasks(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let limit = args.get_number_flag("limit", 20usize);
        let enabled_only = args.has_flag("enabled");
        let include_schemas = args.has_flag("include-schemas");

        let list_args = json!({
            "include_schemas": include_schemas,
            "limit": limit,
            "sort_by": "name",
            "filters": {
                "enabled": if enabled_only { Some(true) } else { None }
            }
        });

        let result = mcp_client
            .execute_tool("ratchet_list_available_tasks", list_args)
            .await?;

        if let Some(tasks) = result.get("tasks").and_then(|v| v.as_array()) {
            let mut headers = vec![
                "Name".to_string(),
                "Description".to_string(),
                "Version".to_string(),
                "Status".to_string(),
            ];

            if include_schemas {
                headers.push("Input Schema".to_string());
                headers.push("Output Schema".to_string());
            }

            let rows: Vec<Vec<String>> = tasks
                .iter()
                .map(|task| {
                    let mut row = vec![
                        task.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        task.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        task.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        if task.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                            "Enabled".to_string()
                        } else {
                            "Disabled".to_string()
                        },
                    ];

                    if include_schemas {
                        row.push(
                            task.get("input_schema")
                                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "Invalid".to_string()))
                                .unwrap_or_else(|| "None".to_string())
                        );
                        row.push(
                            task.get("output_schema")
                                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "Invalid".to_string()))
                                .unwrap_or_else(|| "None".to_string())
                        );
                    }

                    row
                })
                .collect();

            Ok(CommandOutput::table_with_title("Available Tasks", headers, rows))
        } else {
            Ok(CommandOutput::error("No tasks found in response"))
        }
    }
}

#[async_trait]
impl ConsoleCommand for EnhancedTaskCommand {
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match args.action.as_str() {
            "create" => {
                if args.has_flag("interactive") {
                    self.create_task_interactive(mcp_client).await
                } else {
                    self.create_task(&args, mcp_client).await
                }
            }
            "edit" => self.edit_task(&args, mcp_client).await,
            "validate" => self.validate_task(&args, mcp_client).await,
            "test" => self.test_task(&args, mcp_client).await,
            "debug" => self.debug_task(&args, mcp_client).await,
            "version" => self.create_version(&args, mcp_client).await,
            "execute" => self.execute_task(&args, mcp_client).await,
            "list" => self.list_tasks(&args, mcp_client).await,
            _ => Err(anyhow!("Unknown task action: {}", args.action)),
        }
    }

    fn completion_hints(&self, partial: &str) -> Vec<String> {
        let actions = vec![
            "create", "edit", "validate", "test", "debug", "version", "execute", "list"
        ];

        actions
            .into_iter()
            .filter(|action| action.starts_with(partial))
            .map(|s| s.to_string())
            .collect()
    }

    fn help_text(&self) -> &'static str {
        r#"Enhanced task development commands with full MCP integration

USAGE:
    task create <name> [OPTIONS]     Create a new task
    task edit <id> [OPTIONS]         Edit existing task
    task validate <id> [OPTIONS]     Validate task code and schema
    task test <id> [OPTIONS]         Run task tests
    task debug <id> [OPTIONS]        Debug task execution
    task version <id> <version>      Create new task version
    task execute <id> [OPTIONS]      Execute task with options
    task list [OPTIONS]              List available tasks

CREATE OPTIONS:
    --description <text>             Task description
    --version <version>              Task version (default: 1.0.0)
    --template <name>                Use template for generation
    --interactive                    Interactive creation wizard

EDIT OPTIONS:
    --code <code>                    Update task code
    --description <text>             Update description
    --input-schema <json>            Update input schema
    --output-schema <json>           Update output schema

VALIDATE OPTIONS:
    --run-tests                      Run tests during validation
    --fix                            Attempt to fix issues

TEST OPTIONS:
    --test-names <names>             Comma-separated test names
    --parallel                       Run tests in parallel
    --input <json>                   Custom input data

DEBUG OPTIONS:
    --input <json>                   Input data for debugging
    --breakpoints <lines>            Comma-separated line numbers
    --step                           Enable step-by-step mode

EXECUTE OPTIONS:
    --input <json>                   Input data
    --trace                          Enable execution tracing
    --stream                         Stream progress updates
    --timeout <seconds>              Execution timeout (default: 30)

LIST OPTIONS:
    --limit <n>                      Maximum number of tasks (default: 20)
    --enabled                        Show only enabled tasks
    --include-schemas                Include input/output schemas

VERSION OPTIONS:
    --description <text>             Version description
    --breaking                       Mark as breaking change
    --make-active <bool>             Make this version active (default: true)

EXAMPLES:
    task create weather-api --description "Weather API client" --template http-client
    task edit task123 --description "Updated weather API"
    task validate task123 --run-tests --fix
    task test task123 --test-names "basic,advanced" --parallel
    task debug task123 --input '{"city": "London"}' --breakpoints 10,15 --step
    task version task123 2.0.0 --description "Major update" --breaking
    task execute task123 --input '{"city": "Paris"}' --trace --stream
    task list --enabled --include-schemas --limit 50"#
    }

    fn usage_examples(&self) -> Vec<&'static str> {
        vec![
            "task create weather-api --template http-client",
            "task validate my-task --run-tests",
            "task execute my-task --input '{\"key\": \"value\"}' --trace",
            "task list --enabled --limit 10",
        ]
    }

    fn category(&self) -> &'static str {
        "development"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["t"]
    }

    fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        match args.action.as_str() {
            "create" => {
                if args.positional.is_empty() && !args.has_flag("interactive") {
                    return Err(anyhow!("Task name is required for create command"));
                }
            }
            "edit" | "validate" | "test" | "debug" | "execute" => {
                if args.positional.is_empty() {
                    return Err(anyhow!("Task ID is required"));
                }
            }
            "version" => {
                if args.positional.len() < 2 {
                    return Err(anyhow!("Task ID and new version are required"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
impl InteractiveCommand for EnhancedTaskCommand {
    async fn interactive_mode(&self, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        self.create_task_interactive(mcp_client).await
    }

    async fn handle_interactive_input(
        &self,
        input: &str,
        _mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        // Parse interactive input and handle accordingly
        // This would be expanded in a full implementation
        Ok(CommandOutput::text(format!("Interactive input received: {}", input)))
    }
}