//! Template system commands for task generation and management

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::commands::console::{
    command_trait::{ConsoleCommand, CommandArgs, CommandOutput},
    enhanced_mcp_client::EnhancedMcpClient,
};

/// Template management command
pub struct TemplateCommand;

impl TemplateCommand {
    pub fn new() -> Self {
        Self
    }

    /// List available templates
    async fn list_templates(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let category = args.get_flag("category");
        let detailed = args.has_flag("detailed");

        let mut list_args = json!({
            "include_metadata": detailed
        });

        if let Some(cat) = category {
            list_args["category"] = json!(cat);
        }

        let result = mcp_client
            .execute_tool("ratchet_list_templates", list_args)
            .await?;

        if let Some(templates) = result.get("templates").and_then(|v| v.as_array()) {
            let mut headers = vec![
                "Name".to_string(),
                "Category".to_string(),
                "Description".to_string(),
            ];

            if detailed {
                headers.extend(vec![
                    "Version".to_string(),
                    "Parameters".to_string(),
                    "Tags".to_string(),
                ]);
            }

            let rows: Vec<Vec<String>> = templates
                .iter()
                .map(|template| {
                    let mut row = vec![
                        template.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        template.get("category").and_then(|v| v.as_str()).unwrap_or("general").to_string(),
                        template.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    ];

                    if detailed {
                        row.push(
                            template.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string()
                        );
                        
                        let params = template.get("parameters")
                            .and_then(|v| v.as_object())
                            .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(", "))
                            .unwrap_or_else(|| "None".to_string());
                        row.push(params);

                        let tags = template.get("tags")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_else(|| "None".to_string());
                        row.push(tags);
                    }

                    row
                })
                .collect();

            Ok(CommandOutput::table_with_title("Available Templates", headers, rows))
        } else {
            Ok(CommandOutput::error("No templates found"))
        }
    }

    /// Generate task from template
    async fn generate_from_template(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let template_name = args.require_positional(0, "template name")?;
        let task_name = args.require_positional(1, "task name")?;
        let description = args.get_flag("description")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Generated from {} template", template_name));

        // Parse parameters from flags
        let mut parameters = json!({});
        
        // Common template parameters
        if let Some(api_url) = args.get_flag("api-url") {
            parameters["api_url"] = json!(api_url);
        }
        if let Some(auth_type) = args.get_flag("auth-type") {
            parameters["auth_type"] = json!(auth_type);
        }
        if let Some(timeout) = args.get_flag("timeout") {
            if let Ok(timeout_num) = timeout.parse::<u32>() {
                parameters["timeout"] = json!(timeout_num);
            }
        }
        if let Some(method) = args.get_flag("method") {
            parameters["method"] = json!(method);
        }

        // Parse custom parameters from --param key=value flags
        if let Some(params_str) = args.get_flag("params") {
            for param in params_str.split(',') {
                if let Some((key, value)) = param.split_once('=') {
                    parameters[key.trim()] = json!(value.trim());
                }
            }
        }

        let generate_args = json!({
            "template": template_name,
            "name": task_name,
            "description": description,
            "parameters": parameters
        });

        let result = mcp_client
            .execute_tool("ratchet_generate_from_template", generate_args)
            .await?;

        if let Some(task_id) = result.get("task_id") {
            Ok(CommandOutput::success_with_data(
                format!("Task '{}' generated from template '{}'", task_name, template_name),
                json!({
                    "task_id": task_id,
                    "template": template_name,
                    "name": task_name,
                    "parameters": parameters
                }),
            ))
        } else {
            Ok(CommandOutput::error_with_context(
                "Failed to generate task from template",
                result,
            ))
        }
    }

    /// Show template details
    async fn show_template(
        &self,
        args: &CommandArgs,
        mcp_client: &EnhancedMcpClient,
    ) -> Result<CommandOutput> {
        let template_name = args.require_positional(0, "template name")?;

        // Get detailed template information
        let list_args = json!({
            "include_metadata": true,
            "template_name": template_name
        });

        let result = mcp_client
            .execute_tool("ratchet_list_templates", list_args)
            .await?;

        if let Some(templates) = result.get("templates").and_then(|v| v.as_array()) {
            if let Some(template) = templates.first() {
                // Format template details
                let mut details = Vec::new();
                
                details.push(vec!["Name".to_string(), 
                    template.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string()]);
                details.push(vec!["Category".to_string(), 
                    template.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string()]);
                details.push(vec!["Description".to_string(), 
                    template.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string()]);
                details.push(vec!["Version".to_string(), 
                    template.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string()]);

                if let Some(params) = template.get("parameters").and_then(|v| v.as_object()) {
                    for (key, param_info) in params {
                        let param_desc = param_info.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let param_type = param_info.get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("string");
                        let required = param_info.get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        
                        details.push(vec![
                            format!("Parameter: {}", key),
                            format!("{} ({}) {}", param_desc, param_type, 
                                if required { "[required]" } else { "[optional]" })
                        ]);
                    }
                }

                Ok(CommandOutput::table_with_title(
                    &format!("Template: {}", template_name),
                    vec!["Property".to_string(), "Value".to_string()],
                    details,
                ))
            } else {
                Ok(CommandOutput::error(format!("Template '{}' not found", template_name)))
            }
        } else {
            Ok(CommandOutput::error("Failed to get template information"))
        }
    }
}

#[async_trait]
impl ConsoleCommand for TemplateCommand {
    async fn execute(&self, args: CommandArgs, mcp_client: &EnhancedMcpClient) -> Result<CommandOutput> {
        match args.action.as_str() {
            "list" => self.list_templates(&args, mcp_client).await,
            "generate" => self.generate_from_template(&args, mcp_client).await,
            "show" => self.show_template(&args, mcp_client).await,
            _ => Err(anyhow!("Unknown template action: {}", args.action)),
        }
    }

    fn completion_hints(&self, partial: &str) -> Vec<String> {
        let actions = vec!["list", "generate", "show"];

        actions
            .into_iter()
            .filter(|action| action.starts_with(partial))
            .map(|s| s.to_string())
            .collect()
    }

    fn help_text(&self) -> &'static str {
        r#"Template system commands for task generation and management

USAGE:
    template list [OPTIONS]               List available templates
    template generate <template> <name>   Generate task from template
    template show <template>              Show template details

LIST OPTIONS:
    --category <category>                 Filter by template category
    --detailed                           Show detailed information

GENERATE OPTIONS:
    --description <text>                  Task description
    --api-url <url>                      API endpoint URL (for HTTP templates)
    --auth-type <type>                   Authentication type (none, api-key, bearer, basic)
    --timeout <seconds>                  Request timeout in seconds
    --method <method>                    HTTP method (GET, POST, PUT, DELETE)
    --params <key=value,key=value>       Custom template parameters

SHOW OPTIONS:
    (no additional options)

EXAMPLES:
    template list --category http
    template list --detailed
    template generate http-client weather-api --description "Weather API client"
    template generate http-client github-api --api-url "https://api.github.com" --auth-type bearer
    template generate data-processor csv-parser --params "delimiter=comma,headers=true"
    template show http-client

AVAILABLE TEMPLATE CATEGORIES:
    http         - HTTP client templates
    data         - Data processing templates
    database     - Database operation templates
    file         - File system operation templates
    notification - Notification and messaging templates
    basic        - Basic task templates

COMMON TEMPLATE PARAMETERS:
    api_url      - Base URL for API endpoints
    auth_type    - Authentication method
    timeout      - Operation timeout in seconds
    retry_count  - Number of retry attempts
    headers      - Default HTTP headers
    format       - Data format (json, xml, csv, etc.)
    delimiter    - Field delimiter for CSV processing
    encoding     - Character encoding (utf-8, ascii, etc.)"#
    }

    fn usage_examples(&self) -> Vec<&'static str> {
        vec![
            "template list --category http",
            "template generate http-client my-api --api-url https://api.example.com",
            "template show http-client",
        ]
    }

    fn category(&self) -> &'static str {
        "development"
    }

    fn aliases(&self) -> Vec<&'static str> {
        vec!["tmpl"]
    }

    fn validate_args(&self, args: &CommandArgs) -> Result<()> {
        match args.action.as_str() {
            "generate" => {
                if args.positional.len() < 2 {
                    return Err(anyhow!("Template name and task name are required"));
                }
            }
            "show" => {
                if args.positional.is_empty() {
                    return Err(anyhow!("Template name is required"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}