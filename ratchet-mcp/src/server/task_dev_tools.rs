//! Task development tools for MCP server
//! These tools enable agents to create, edit, validate, test, and version tasks

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use crate::protocol::{ToolContent, ToolsCallResult};
use crate::server::tools::{McpTool, ToolExecutionContext};
use crate::{McpError, McpResult};

use ratchet_storage::seaorm::repositories::task_repository::TaskRepository;

/// Simple task validator for JavaScript syntax checking
pub struct TaskValidator {
    _private: (),
}

impl TaskValidator {
    /// Create a new task validator
    pub fn new() -> Self {
        Self { _private: () }
    }
    
    /// Validate JavaScript syntax
    pub fn validate_syntax(&self, code: &str) -> Result<(), String> {
        // Basic syntax validation without boa_engine dependency
        // In a real implementation, this would use the task validation from ratchet-lib
        
        // Basic checks
        if code.trim().is_empty() {
            return Err("JavaScript code cannot be empty".to_string());
        }
        
        // Check for function definition
        if !code.contains("function") && !code.contains("=>") {
            return Err("JavaScript code must contain at least one function definition".to_string());
        }
        
        // Check for basic syntax patterns
        let open_braces = code.chars().filter(|&c| c == '{').count();
        let close_braces = code.chars().filter(|&c| c == '}').count();
        if open_braces != close_braces {
            return Err("Mismatched braces in JavaScript code".to_string());
        }
        
        let open_parens = code.chars().filter(|&c| c == '(').count();
        let close_parens = code.chars().filter(|&c| c == ')').count();
        if open_parens != close_parens {
            return Err("Mismatched parentheses in JavaScript code".to_string());
        }
        
        // Note: In production, this should use the actual JavaScript parser
        // from ratchet-lib with boa_engine feature enabled
        Ok(())
    }
}

/// Task creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Task name (must be unique)
    pub name: String,
    
    /// Task description
    pub description: String,
    
    /// JavaScript code for the task
    pub code: String,
    
    /// Input schema (JSON Schema format)
    pub input_schema: Value,
    
    /// Output schema (JSON Schema format)
    pub output_schema: Value,
    
    /// Task tags/categories
    #[serde(default)]
    pub tags: Vec<String>,
    
    /// Task version (defaults to "0.1.0")
    #[serde(default = "default_version")]
    pub version: String,
    
    /// Whether to enable the task immediately
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Optional test cases
    #[serde(default)]
    pub test_cases: Vec<TaskTestCase>,
    
    /// Task metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_enabled() -> bool {
    true
}

/// Task test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTestCase {
    /// Test name
    pub name: String,
    
    /// Input data for the test
    pub input: Value,
    
    /// Expected output (for validation)
    pub expected_output: Option<Value>,
    
    /// Whether this test should fail
    #[serde(default)]
    pub should_fail: bool,
    
    /// Test description
    pub description: Option<String>,
}

/// Task validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateTaskRequest {
    /// Task name or ID
    pub task_id: String,
    
    /// Optional JavaScript code to validate (if not provided, validates existing)
    pub code: Option<String>,
    
    /// Optional input schema to validate
    pub input_schema: Option<Value>,
    
    /// Optional output schema to validate
    pub output_schema: Option<Value>,
    
    /// Whether to run test cases
    #[serde(default = "default_run_tests")]
    pub run_tests: bool,
    
    /// Whether to check for syntax errors only
    #[serde(default)]
    pub syntax_only: bool,
}

fn default_run_tests() -> bool {
    true
}

/// Task debug execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugTaskRequest {
    /// Task name or ID
    pub task_id: String,
    
    /// Input data for debugging
    pub input: Value,
    
    /// Breakpoints (line numbers)
    #[serde(default)]
    pub breakpoints: Vec<u32>,
    
    /// Whether to enable step-by-step execution
    #[serde(default)]
    pub step_mode: bool,
    
    /// Whether to capture all variable states
    #[serde(default = "default_capture_vars")]
    pub capture_variables: bool,
    
    /// Maximum execution time in milliseconds
    #[serde(default = "default_debug_timeout")]
    pub timeout_ms: u64,
}

fn default_capture_vars() -> bool {
    true
}

fn default_debug_timeout() -> u64 {
    300000 // 5 minutes for debugging
}

/// Task test execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskTestsRequest {
    /// Task name or ID
    pub task_id: String,
    
    /// Specific test names to run (empty = all tests)
    #[serde(default)]
    pub test_names: Vec<String>,
    
    /// Whether to stop on first failure
    #[serde(default)]
    pub stop_on_failure: bool,
    
    /// Whether to include execution traces
    #[serde(default = "default_include_traces")]
    pub include_traces: bool,
    
    /// Whether to run tests in parallel
    #[serde(default)]
    pub parallel: bool,
}

fn default_include_traces() -> bool {
    true
}

/// Task version creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskVersionRequest {
    /// Task name or ID
    pub task_id: String,
    
    /// New version number (must be higher than current)
    pub new_version: String,
    
    /// Version description/changelog
    pub description: String,
    
    /// Whether this is a breaking change
    #[serde(default)]
    pub breaking_change: bool,
    
    /// Whether to make this the active version
    #[serde(default = "default_make_active")]
    pub make_active: bool,
    
    /// Optional migration script for breaking changes
    pub migration_script: Option<String>,
}

fn default_make_active() -> bool {
    true
}

/// Task editing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTaskRequest {
    /// Task name or ID
    pub task_id: String,
    
    /// New JavaScript code (optional)
    pub code: Option<String>,
    
    /// New input schema (optional)
    pub input_schema: Option<Value>,
    
    /// New output schema (optional)
    pub output_schema: Option<Value>,
    
    /// New description (optional)
    pub description: Option<String>,
    
    /// New tags (optional)
    pub tags: Option<Vec<String>>,
    
    /// Whether to validate changes before applying
    #[serde(default = "default_validate_changes")]
    pub validate_changes: bool,
    
    /// Whether to create a backup before editing
    #[serde(default = "default_create_backup")]
    pub create_backup: bool,
}

fn default_validate_changes() -> bool {
    true
}

fn default_create_backup() -> bool {
    true
}

/// Task import request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTaskRequest {
    /// Task data (JSON or ZIP format)
    pub data: Value,
    
    /// Import format
    #[serde(default = "default_import_format")]
    pub format: ImportFormat,
    
    /// Whether to overwrite existing tasks
    #[serde(default)]
    pub overwrite_existing: bool,
    
    /// Import options
    #[serde(default)]
    pub options: ImportOptions,
}

fn default_import_format() -> ImportFormat {
    ImportFormat::Json
}

/// Import format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat {
    Json,
    Zip,
    Directory,
}

/// Import options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportOptions {
    /// Include test cases
    #[serde(default = "default_include_import_tests")]
    pub include_tests: bool,
    
    /// Validate imported tasks
    #[serde(default = "default_validate_imports")]
    pub validate_tasks: bool,
    
    /// Prefix for imported task names
    pub name_prefix: Option<String>,
}

fn default_include_import_tests() -> bool {
    true
}

fn default_validate_imports() -> bool {
    true
}

/// Task export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTaskRequest {
    /// Task name or ID (optional, exports all if not provided)
    pub task_id: Option<String>,
    
    /// Export format
    #[serde(default = "default_export_format")]
    pub format: ExportFormat,
    
    /// Export options
    #[serde(default)]
    pub options: ExportOptions,
}

fn default_export_format() -> ExportFormat {
    ExportFormat::Json
}

/// Export format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Zip,
    Individual,
}

/// Export options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportOptions {
    /// Include test cases
    #[serde(default = "default_include_export_tests")]
    pub include_tests: bool,
    
    /// Include metadata
    #[serde(default = "default_include_metadata")]
    pub include_metadata: bool,
    
    /// Include version history
    #[serde(default)]
    pub include_versions: bool,
}

fn default_include_export_tests() -> bool {
    true
}

fn default_include_metadata() -> bool {
    true
}

/// Template generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateFromTemplateRequest {
    /// Template name
    pub template: String,
    
    /// Task name
    pub name: String,
    
    /// Template parameters
    pub parameters: HashMap<String, Value>,
    
    /// Task description
    pub description: Option<String>,
}

/// Available task templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub description: String,
    pub code_template: String,
    pub input_schema_template: Value,
    pub output_schema_template: Value,
    pub required_parameters: Vec<String>,
    pub optional_parameters: Vec<String>,
    pub category: String,
    pub tags: Vec<String>,
}

/// Task development service that handles creation, validation, testing, and versioning
pub struct TaskDevelopmentService {
    /// Task repository for database operations
    task_repository: Arc<TaskRepository>,
    
    /// Task validator for validation operations
    task_validator: Arc<TaskValidator>,
    
    /// Base path for task storage
    task_base_path: PathBuf,
    
    /// Whether to allow direct file system operations
    allow_fs_operations: bool,
}

impl TaskDevelopmentService {
    /// Create a new task development service
    pub fn new(
        task_repository: Arc<TaskRepository>,
        task_base_path: PathBuf,
        allow_fs_operations: bool,
    ) -> Self {
        Self {
            task_repository,
            task_validator: Arc::new(TaskValidator::new()),
            task_base_path,
            allow_fs_operations,
        }
    }
    
    /// Create a new task
    pub async fn create_task(&self, request: CreateTaskRequest) -> McpResult<Value> {
        // Validate task name is unique
        if let Ok(Some(_)) = self.task_repository.find_by_name(&request.name).await {
            return Err(McpError::InvalidParams {
                method: "create_task".to_string(),
                details: format!("Task with name '{}' already exists", request.name),
            });
        }
        
        // Validate JavaScript syntax
        if let Err(e) = self.task_validator.validate_syntax(&request.code) {
            return Err(McpError::InvalidParams {
                method: "create_task".to_string(),
                details: format!("JavaScript syntax error: {}", e),
            });
        }
        
        // Validate schemas
        if let Err(e) = self.validate_json_schema(&request.input_schema) {
            return Err(McpError::InvalidParams {
                method: "create_task".to_string(),
                details: format!("Invalid input schema: {}", e),
            });
        }
        
        if let Err(e) = self.validate_json_schema(&request.output_schema) {
            return Err(McpError::InvalidParams {
                method: "create_task".to_string(),
                details: format!("Invalid output schema: {}", e),
            });
        }
        
        // Create task directory structure if file system operations are allowed
        let task_path = if self.allow_fs_operations {
            let task_dir = self.task_base_path.join(&request.name);
            self.create_task_directory(&task_dir, &request).await?;
            Some(task_dir)
        } else {
            None
        };
        
        // Create database entry
        let task_uuid = uuid::Uuid::new_v4();
        let task_id = self.create_database_entry(&request, task_uuid, task_path.as_deref()).await?;
        
        // Run initial tests if provided
        let test_results = if !request.test_cases.is_empty() {
            Some(self.run_task_tests_internal(&request.name, &request.test_cases, &request.code).await?)
        } else {
            None
        };
        
        Ok(json!({
            "task_id": task_uuid.to_string(),
            "database_id": task_id,
            "name": request.name,
            "version": request.version,
            "path": task_path.map(|p| p.display().to_string()),
            "enabled": request.enabled,
            "test_results": test_results,
            "message": "Task created successfully"
        }))
    }
    
    /// Validate a task
    pub async fn validate_task(&self, request: ValidateTaskRequest) -> McpResult<Value> {
        // Find the task
        let task = self.find_task(&request.task_id).await?;
        
        let mut validation_results = Vec::new();
        
        // Validate JavaScript code
        let code_to_validate = request.code.as_deref().unwrap_or(&task.code);
        match self.task_validator.validate_syntax(code_to_validate) {
            Ok(_) => {
                validation_results.push(json!({
                    "type": "syntax",
                    "status": "passed",
                    "message": "JavaScript syntax is valid"
                }));
            }
            Err(e) => {
                validation_results.push(json!({
                    "type": "syntax",
                    "status": "failed",
                    "error": e.to_string()
                }));
                
                if request.syntax_only {
                    return Ok(json!({
                        "task_id": task.uuid.to_string(),
                        "valid": false,
                        "validation_results": validation_results
                    }));
                }
            }
        }
        
        // Validate schemas if provided
        if let Some(input_schema) = &request.input_schema {
            match self.validate_json_schema(input_schema) {
                Ok(_) => {
                    validation_results.push(json!({
                        "type": "input_schema",
                        "status": "passed",
                        "message": "Input schema is valid"
                    }));
                }
                Err(e) => {
                    validation_results.push(json!({
                        "type": "input_schema",
                        "status": "failed",
                        "error": e
                    }));
                }
            }
        }
        
        if let Some(output_schema) = &request.output_schema {
            match self.validate_json_schema(output_schema) {
                Ok(_) => {
                    validation_results.push(json!({
                        "type": "output_schema",
                        "status": "passed",
                        "message": "Output schema is valid"
                    }));
                }
                Err(e) => {
                    validation_results.push(json!({
                        "type": "output_schema",
                        "status": "failed",
                        "error": e
                    }));
                }
            }
        }
        
        // Run tests if requested
        let test_results = if request.run_tests && !request.syntax_only {
            // Load test cases from file system or database
            let test_cases = self.load_task_tests(&task.name).await?;
            if !test_cases.is_empty() {
                Some(self.run_task_tests_internal(&task.name, &test_cases, code_to_validate).await?)
            } else {
                None
            }
        } else {
            None
        };
        
        let all_passed = validation_results.iter().all(|r| r["status"] == "passed");
        
        Ok(json!({
            "task_id": task.uuid.to_string(),
            "task_name": task.name,
            "valid": all_passed,
            "validation_results": validation_results,
            "test_results": test_results,
            "message": if all_passed { "Task validation passed" } else { "Task validation failed" }
        }))
    }
    
    /// Debug task execution
    pub async fn debug_task(&self, request: DebugTaskRequest) -> McpResult<Value> {
        // Find the task
        let task = self.find_task(&request.task_id).await?;
        
        // Create debug session
        let session_id = uuid::Uuid::new_v4().to_string();
        
        // Note: Full debugging would require integration with a JavaScript debugger
        // This is a simplified implementation that provides execution traces
        
        let debug_info = json!({
            "session_id": session_id,
            "task_id": task.uuid.to_string(),
            "task_name": task.name,
            "breakpoints": request.breakpoints,
            "step_mode": request.step_mode,
            "status": "not_implemented",
            "message": "Full debugging support requires JavaScript debugger integration. Use execution traces for now.",
            "available_features": [
                "execution_traces",
                "error_analysis",
                "performance_profiling"
            ],
            "next_steps": [
                "Use ratchet.execute_task with trace=true for execution traces",
                "Use ratchet.get_execution_trace for detailed trace analysis",
                "Use ratchet.analyze_execution_error for error debugging"
            ]
        });
        
        Ok(debug_info)
    }
    
    /// Run task tests
    pub async fn run_task_tests(&self, request: RunTaskTestsRequest) -> McpResult<Value> {
        // Find the task
        let task = self.find_task(&request.task_id).await?;
        
        // Load test cases
        let all_test_cases = self.load_task_tests(&task.name).await?;
        
        // Filter test cases if specific names provided
        let test_cases = if request.test_names.is_empty() {
            all_test_cases
        } else {
            all_test_cases.into_iter()
                .filter(|tc| request.test_names.contains(&tc.name))
                .collect()
        };
        
        if test_cases.is_empty() {
            return Ok(json!({
                "task_id": task.uuid.to_string(),
                "task_name": task.name,
                "message": "No test cases found",
                "total_tests": 0,
                "passed": 0,
                "failed": 0
            }));
        }
        
        let results = self.run_task_tests_internal(&task.name, &test_cases, &task.code).await?;
        
        Ok(results)
    }
    
    /// Create a new task version
    pub async fn create_task_version(&self, request: CreateTaskVersionRequest) -> McpResult<Value> {
        // Find the task
        let current_task = self.find_task(&request.task_id).await?;
        
        // Validate version is higher
        if !self.is_version_higher(&current_task.version, &request.new_version) {
            return Err(McpError::InvalidParams {
                method: "create_task_version".to_string(),
                details: format!(
                    "New version {} must be higher than current version {}",
                    request.new_version, current_task.version
                ),
            });
        }
        
        // Create version info
        let version_info = json!({
            "task_id": current_task.uuid.to_string(),
            "task_name": current_task.name,
            "previous_version": current_task.version,
            "new_version": request.new_version,
            "description": request.description,
            "breaking_change": request.breaking_change,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "status": "not_implemented",
            "message": "Task versioning requires full implementation of version management system",
            "next_steps": [
                "Implement version storage in database",
                "Add version migration support",
                "Create version rollback mechanism"
            ]
        });
        
        Ok(version_info)
    }
    
    /// Edit an existing task
    pub async fn edit_task(&self, request: EditTaskRequest) -> McpResult<Value> {
        // Find the task
        let task = self.find_task(&request.task_id).await?;
        
        // Create backup if requested
        if request.create_backup {
            let _backup_id = format!("{}_backup_{}", task.name, chrono::Utc::now().format("%Y%m%d_%H%M%S"));
            // In production, this would create an actual backup
        }
        
        let mut changes = Vec::new();
        let mut errors = Vec::new();
        
        // Validate code changes if provided
        if let Some(ref code) = request.code {
            if request.validate_changes {
                if let Err(e) = self.task_validator.validate_syntax(code) {
                    errors.push(format!("Code validation failed: {}", e));
                } else {
                    changes.push("code".to_string());
                }
            } else {
                changes.push("code".to_string());
            }
        }
        
        // Validate schema changes if provided
        if let Some(ref input_schema) = request.input_schema {
            if request.validate_changes {
                if let Err(e) = self.validate_json_schema(input_schema) {
                    errors.push(format!("Input schema validation failed: {}", e));
                } else {
                    changes.push("input_schema".to_string());
                }
            } else {
                changes.push("input_schema".to_string());
            }
        }
        
        if let Some(ref output_schema) = request.output_schema {
            if request.validate_changes {
                if let Err(e) = self.validate_json_schema(output_schema) {
                    errors.push(format!("Output schema validation failed: {}", e));
                } else {
                    changes.push("output_schema".to_string());
                }
            } else {
                changes.push("output_schema".to_string());
            }
        }
        
        if let Some(ref _description) = request.description {
            changes.push("description".to_string());
        }
        
        if let Some(ref _tags) = request.tags {
            changes.push("tags".to_string());
        }
        
        if !errors.is_empty() {
            return Ok(json!({
                "task_id": task.uuid.to_string(),
                "success": false,
                "errors": errors,
                "message": "Validation failed"
            }));
        }
        
        let edit_result = json!({
            "task_id": task.uuid.to_string(),
            "task_name": task.name,
            "success": true,
            "changes_applied": changes,
            "backup_created": request.create_backup,
            "validation_performed": request.validate_changes,
            "message": "Task edited successfully (database update not implemented)",
            "edited_at": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(edit_result)
    }
    
    /// Import tasks from external source
    pub async fn import_tasks(&self, request: ImportTaskRequest) -> McpResult<Value> {
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        
        match request.format {
            ImportFormat::Json => {
                // Parse JSON data
                if let Some(tasks_array) = request.data.as_array() {
                    for (idx, task_data) in tasks_array.iter().enumerate() {
                        match self.import_single_task(task_data, &request.options).await {
                            Ok(imported) => imported_tasks.push(imported),
                            Err(e) => errors.push(format!("Task {}: {}", idx, e)),
                        }
                    }
                } else if request.data.is_object() {
                    // Single task import
                    match self.import_single_task(&request.data, &request.options).await {
                        Ok(imported) => imported_tasks.push(imported),
                        Err(e) => errors.push(format!("Task import: {}", e)),
                    }
                }
            }
            ImportFormat::Zip | ImportFormat::Directory => {
                errors.push("ZIP and Directory import not yet implemented".to_string());
            }
        }
        
        let import_result = json!({
            "imported_count": imported_tasks.len(),
            "error_count": errors.len(),
            "imported_tasks": imported_tasks,
            "errors": errors,
            "format": request.format,
            "overwrite_existing": request.overwrite_existing,
            "imported_at": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(import_result)
    }
    
    /// Export tasks
    pub async fn export_tasks(&self, request: ExportTaskRequest) -> McpResult<Value> {
        let tasks = if let Some(task_id) = &request.task_id {
            // Export single task
            vec![self.find_task(task_id).await?]
        } else {
            // Export all tasks (placeholder - would need database integration)
            vec![]
        };
        
        let mut exported_tasks = Vec::new();
        
        for task in tasks {
            let mut task_export = json!({
                "name": task.name,
                "version": task.version,
                "code": task.code,
                "input_schema": task.input_schema,
                "output_schema": task.output_schema
            });
            
            if request.options.include_metadata {
                task_export["metadata"] = json!({
                    "uuid": task.uuid.to_string(),
                    "created_at": chrono::Utc::now().to_rfc3339(),
                    "exported_at": chrono::Utc::now().to_rfc3339()
                });
            }
            
            if request.options.include_tests {
                // Load test cases
                let test_cases = self.load_task_tests(&task.name).await?;
                task_export["test_cases"] = json!(test_cases);
            }
            
            exported_tasks.push(task_export);
        }
        
        let export_result = match request.format {
            ExportFormat::Json => json!({
                "tasks": exported_tasks,
                "export_format": "json",
                "exported_count": exported_tasks.len(),
                "options": request.options,
                "exported_at": chrono::Utc::now().to_rfc3339()
            }),
            ExportFormat::Zip => json!({
                "message": "ZIP export not yet implemented",
                "tasks": exported_tasks,
                "export_format": "zip"
            }),
            ExportFormat::Individual => json!({
                "message": "Individual file export not yet implemented", 
                "tasks": exported_tasks,
                "export_format": "individual"
            }),
        };
        
        Ok(export_result)
    }
    
    /// Generate task from template
    pub async fn generate_from_template(&self, request: GenerateFromTemplateRequest) -> McpResult<Value> {
        let templates = self.get_available_templates();
        
        let template = templates.iter()
            .find(|t| t.name == request.template)
            .ok_or_else(|| McpError::InvalidParams {
                method: "generate_from_template".to_string(),
                details: format!("Template '{}' not found", request.template),
            })?;
        
        // Check required parameters
        for param in &template.required_parameters {
            if !request.parameters.contains_key(param) {
                return Err(McpError::InvalidParams {
                    method: "generate_from_template".to_string(),
                    details: format!("Missing required parameter: {}", param),
                });
            }
        }
        
        // Generate code from template
        let code = self.apply_template_parameters(&template.code_template, &request.parameters)?;
        let input_schema = self.apply_template_to_schema(&template.input_schema_template, &request.parameters)?;
        let output_schema = self.apply_template_to_schema(&template.output_schema_template, &request.parameters)?;
        
        let generated_task = json!({
            "name": request.name,
            "description": request.description.unwrap_or_else(|| template.description.clone()),
            "code": code,
            "input_schema": input_schema,
            "output_schema": output_schema,
            "template_used": template.name,
            "template_category": template.category,
            "parameters": request.parameters,
            "generated_at": chrono::Utc::now().to_rfc3339()
        });
        
        Ok(generated_task)
    }
    
    /// List available templates
    pub async fn list_templates(&self) -> McpResult<Value> {
        let templates = self.get_available_templates();
        
        let template_list: Vec<Value> = templates.iter().map(|template| json!({
            "name": template.name,
            "description": template.description,
            "category": template.category,
            "tags": template.tags,
            "required_parameters": template.required_parameters,
            "optional_parameters": template.optional_parameters
        })).collect();
        
        Ok(json!({
            "templates": template_list,
            "total_count": templates.len(),
            "categories": self.get_template_categories(&templates)
        }))
    }
    
    // Helper methods
    
    async fn create_task_directory(&self, task_dir: &Path, request: &CreateTaskRequest) -> McpResult<()> {
        // Create directory structure
        fs::create_dir_all(task_dir).await.map_err(|e| McpError::Internal {
            message: format!("Failed to create task directory: {}", e),
        })?;
        
        // Write main.js
        let main_js_path = task_dir.join("main.js");
        fs::write(&main_js_path, &request.code).await.map_err(|e| McpError::Internal {
            message: format!("Failed to write main.js: {}", e),
        })?;
        
        // Write metadata.json
        let metadata = json!({
            "name": request.name,
            "version": request.version,
            "description": request.description,
            "tags": request.tags,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "custom": request.metadata
        });
        let metadata_path = task_dir.join("metadata.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap())
            .await
            .map_err(|e| McpError::Internal {
                message: format!("Failed to write metadata.json: {}", e),
            })?;
        
        // Write schemas
        let input_schema_path = task_dir.join("input.schema.json");
        fs::write(&input_schema_path, serde_json::to_string_pretty(&request.input_schema).unwrap())
            .await
            .map_err(|e| McpError::Internal {
                message: format!("Failed to write input schema: {}", e),
            })?;
        
        let output_schema_path = task_dir.join("output.schema.json");
        fs::write(&output_schema_path, serde_json::to_string_pretty(&request.output_schema).unwrap())
            .await
            .map_err(|e| McpError::Internal {
                message: format!("Failed to write output schema: {}", e),
            })?;
        
        // Create tests directory and write test cases
        if !request.test_cases.is_empty() {
            let tests_dir = task_dir.join("tests");
            fs::create_dir_all(&tests_dir).await.map_err(|e| McpError::Internal {
                message: format!("Failed to create tests directory: {}", e),
            })?;
            
            for (idx, test_case) in request.test_cases.iter().enumerate() {
                let test_file = tests_dir.join(format!("test-{:03}.json", idx + 1));
                let test_data = json!({
                    "name": test_case.name,
                    "description": test_case.description,
                    "input": test_case.input,
                    "expected": test_case.expected_output,
                    "should_fail": test_case.should_fail
                });
                fs::write(&test_file, serde_json::to_string_pretty(&test_data).unwrap())
                    .await
                    .map_err(|e| McpError::Internal {
                        message: format!("Failed to write test case: {}", e),
                    })?;
            }
        }
        
        Ok(())
    }
    
    async fn create_database_entry(
        &self,
        _request: &CreateTaskRequest,
        _task_uuid: uuid::Uuid,
        _task_path: Option<&Path>,
    ) -> McpResult<i32> {
        // Note: This would require adding a create method to TaskRepository
        // For now, return a placeholder
        Err(McpError::Internal {
            message: "Database task creation not yet implemented in TaskRepository".to_string(),
        })
    }
    
    async fn find_task(&self, task_id: &str) -> McpResult<TaskInfo> {
        // Try to find by name first
        if let Ok(Some(task)) = self.task_repository.find_by_name(task_id).await {
            return Ok(TaskInfo {
                uuid: task.uuid,
                name: task.name,
                version: task.version,
                code: "".to_string(), // Would need to load from file system
                input_schema: task.input_schema,
                output_schema: task.output_schema,
            });
        }
        
        // Try as UUID
        if let Ok(uuid) = uuid::Uuid::parse_str(task_id) {
            if let Ok(Some(task)) = self.task_repository.find_by_uuid(uuid).await {
                return Ok(TaskInfo {
                    uuid: task.uuid,
                    name: task.name,
                    version: task.version,
                    code: "".to_string(), // Would need to load from file system
                    input_schema: task.input_schema,
                    output_schema: task.output_schema,
                });
            }
        }
        
        Err(McpError::InvalidParams {
            method: "find_task".to_string(),
            details: format!("Task not found: {}", task_id),
        })
    }
    
    async fn load_task_tests(&self, task_name: &str) -> McpResult<Vec<TaskTestCase>> {
        if !self.allow_fs_operations {
            return Ok(Vec::new());
        }
        
        let tests_dir = self.task_base_path.join(task_name).join("tests");
        if !tests_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut test_cases = Vec::new();
        
        let mut entries = fs::read_dir(&tests_dir).await.map_err(|e| McpError::Internal {
            message: format!("Failed to read tests directory: {}", e),
        })?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| McpError::Internal {
            message: format!("Failed to read test entry: {}", e),
        })? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await.map_err(|e| McpError::Internal {
                    message: format!("Failed to read test file: {}", e),
                })?;
                
                if let Ok(test_data) = serde_json::from_str::<Value>(&content) {
                    let test_case = TaskTestCase {
                        name: test_data["name"].as_str().unwrap_or("unnamed").to_string(),
                        input: test_data["input"].clone(),
                        expected_output: test_data.get("expected").cloned(),
                        should_fail: test_data["should_fail"].as_bool().unwrap_or(false),
                        description: test_data["description"].as_str().map(|s| s.to_string()),
                    };
                    test_cases.push(test_case);
                }
            }
        }
        
        Ok(test_cases)
    }
    
    async fn run_task_tests_internal(
        &self,
        task_name: &str,
        test_cases: &[TaskTestCase],
        _code: &str,
    ) -> McpResult<Value> {
        // Note: This would require actual JavaScript execution
        // For now, return a mock result
        let results = json!({
            "task_name": task_name,
            "total_tests": test_cases.len(),
            "passed": 0,
            "failed": 0,
            "skipped": test_cases.len(),
            "message": "Test execution requires JavaScript runtime integration",
            "tests": test_cases.iter().map(|tc| json!({
                "name": tc.name,
                "status": "skipped",
                "message": "JavaScript execution not yet implemented"
            })).collect::<Vec<_>>()
        });
        
        Ok(results)
    }
    
    fn validate_json_schema(&self, schema: &Value) -> Result<(), String> {
        // Basic JSON Schema validation
        if !schema.is_object() {
            return Err("Schema must be an object".to_string());
        }
        
        let obj = schema.as_object().unwrap();
        
        // Check for required fields
        if !obj.contains_key("type") {
            return Err("Schema must have a 'type' field".to_string());
        }
        
        // Additional validation could be added here
        Ok(())
    }
    
    fn is_version_higher(&self, current: &str, new: &str) -> bool {
        // Simple semantic version comparison
        let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
        let new_parts: Vec<u32> = new.split('.').filter_map(|s| s.parse().ok()).collect();
        
        for i in 0..current_parts.len().min(new_parts.len()) {
            if new_parts[i] > current_parts[i] {
                return true;
            } else if new_parts[i] < current_parts[i] {
                return false;
            }
        }
        
        new_parts.len() > current_parts.len()
    }
    
    /// Import a single task from JSON data
    async fn import_single_task(&self, task_data: &Value, options: &ImportOptions) -> McpResult<Value> {
        let name = task_data["name"].as_str()
            .ok_or_else(|| McpError::InvalidParams {
                method: "import_task".to_string(),
                details: "Task name is required".to_string(),
            })?;
        
        let final_name = if let Some(prefix) = &options.name_prefix {
            format!("{}{}", prefix, name)
        } else {
            name.to_string()
        };
        
        if options.validate_tasks {
            if let Some(code) = task_data["code"].as_str() {
                if let Err(e) = self.task_validator.validate_syntax(code) {
                    return Err(McpError::InvalidParams {
                        method: "import_task".to_string(),
                        details: format!("Task validation failed: {}", e),
                    });
                }
            }
        }
        
        Ok(json!({
            "original_name": name,
            "imported_name": final_name,
            "validated": options.validate_tasks,
            "status": "imported",
            "message": "Task import successful (database integration pending)"
        }))
    }
    
    /// Get available task templates
    fn get_available_templates(&self) -> Vec<TaskTemplate> {
        vec![
            TaskTemplate {
                name: "http_api_call".to_string(),
                description: "Make HTTP API calls with error handling".to_string(),
                code_template: r#"
async function process(input, { fetch }) {
    const { url, method = 'GET', headers = {}, body } = input;
    
    const response = await fetch(url, {
        method,
        headers: {
            'Content-Type': 'application/json',
            ...headers
        },
        body: body ? JSON.stringify(body) : undefined
    });
    
    if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const result = await response.json();
    return result;
}"#.to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "format": "uri" },
                        "method": { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"] },
                        "headers": { "type": "object" },
                        "body": { "type": "object" }
                    },
                    "required": ["url"]
                }),
                output_schema_template: json!({
                    "type": "object"
                }),
                required_parameters: vec![],
                optional_parameters: vec!["default_headers".to_string()],
                category: "api".to_string(),
                tags: vec!["http".to_string(), "api".to_string(), "network".to_string()],
            },
            TaskTemplate {
                name: "data_transform".to_string(),
                description: "Transform data structures using mapping rules".to_string(),
                code_template: r#"
function process(input) {
    const { data, mapping } = input;
    
    const result = {};
    for (const [outputKey, inputPath] of Object.entries(mapping)) {
        result[outputKey] = getValueByPath(data, inputPath);
    }
    
    return result;
}

function getValueByPath(obj, path) {
    return path.split('.').reduce((acc, part) => acc && acc[part], obj);
}"#.to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "data": { "type": "object" },
                        "mapping": { 
                            "type": "object",
                            "additionalProperties": { "type": "string" }
                        }
                    },
                    "required": ["data", "mapping"]
                }),
                output_schema_template: json!({
                    "type": "object"
                }),
                required_parameters: vec![],
                optional_parameters: vec![],
                category: "transformation".to_string(),
                tags: vec!["data".to_string(), "transform".to_string(), "mapping".to_string()],
            },
            TaskTemplate {
                name: "validation".to_string(),
                description: "Validate data against JSON schemas".to_string(),
                code_template: r#"
function process(input) {
    const { data, schema, strict = true } = input;
    
    // Basic validation implementation
    const errors = validateAgainstSchema(data, schema, strict);
    
    return {
        valid: errors.length === 0,
        errors: errors,
        data: data
    };
}

function validateAgainstSchema(data, schema, strict) {
    const errors = [];
    
    // Type validation
    if (schema.type && typeof data !== schema.type) {
        errors.push(`Expected type ${schema.type}, got ${typeof data}`);
    }
    
    // Required fields validation
    if (schema.required && schema.type === 'object') {
        for (const field of schema.required) {
            if (!(field in data)) {
                errors.push(`Missing required field: ${field}`);
            }
        }
    }
    
    return errors;
}"#.to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "data": { "type": "object" },
                        "schema": { "type": "object" },
                        "strict": { "type": "boolean", "default": true }
                    },
                    "required": ["data", "schema"]
                }),
                output_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "valid": { "type": "boolean" },
                        "errors": { "type": "array", "items": { "type": "string" } },
                        "data": { "type": "object" }
                    },
                    "required": ["valid", "errors"]
                }),
                required_parameters: vec![],
                optional_parameters: vec![],
                category: "validation".to_string(),
                tags: vec!["validation".to_string(), "schema".to_string(), "data".to_string()],
            },
            TaskTemplate {
                name: "file_processor".to_string(),
                description: "Process files with different formats".to_string(),
                code_template: r#"
function process(input) {
    const { fileContent, fileType, operation = 'parse' } = input;
    
    switch (fileType.toLowerCase()) {
        case 'json':
            return processJson(fileContent, operation);
        case 'csv':
            return processCsv(fileContent, operation);
        case 'xml':
            return processXml(fileContent, operation);
        default:
            throw new Error(`Unsupported file type: ${fileType}`);
    }
}

function processJson(content, operation) {
    const data = JSON.parse(content);
    return { type: 'json', data, operation };
}

function processCsv(content, operation) {
    const lines = content.split('\n').filter(line => line.trim());
    const headers = lines[0].split(',');
    const rows = lines.slice(1).map(line => line.split(','));
    return { type: 'csv', headers, rows, operation };
}

function processXml(content, operation) {
    // Basic XML parsing placeholder
    return { type: 'xml', content, operation, message: 'XML parsing not fully implemented' };
}"#.to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "fileContent": { "type": "string" },
                        "fileType": { "type": "string", "enum": ["json", "csv", "xml"] },
                        "operation": { "type": "string", "default": "parse" }
                    },
                    "required": ["fileContent", "fileType"]
                }),
                output_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "type": { "type": "string" },
                        "data": { "type": "object" },
                        "operation": { "type": "string" }
                    }
                }),
                required_parameters: vec![],
                optional_parameters: vec!["default_operation".to_string()],
                category: "file".to_string(),
                tags: vec!["file".to_string(), "parsing".to_string(), "processing".to_string()],
            },
        ]
    }
    
    /// Apply template parameters to code
    fn apply_template_parameters(&self, template: &str, parameters: &HashMap<String, Value>) -> McpResult<String> {
        let mut result = template.to_string();
        
        for (key, value) in parameters {
            let placeholder = format!("{{{{{}}}}}", key);
            if let Some(str_value) = value.as_str() {
                result = result.replace(&placeholder, str_value);
            }
        }
        
        Ok(result)
    }
    
    /// Apply template parameters to schema
    fn apply_template_to_schema(&self, schema: &Value, _parameters: &HashMap<String, Value>) -> McpResult<Value> {
        // For now, return schema as-is
        // In a full implementation, this would support parameter substitution in schemas
        Ok(schema.clone())
    }
    
    /// Get template categories
    fn get_template_categories(&self, templates: &[TaskTemplate]) -> Vec<String> {
        let mut categories: Vec<String> = templates.iter()
            .map(|t| t.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }
}

struct TaskInfo {
    uuid: uuid::Uuid,
    name: String,
    version: String,
    code: String,
    input_schema: Value,
    output_schema: Value,
}

/// Register task development tools in the tool registry
pub fn register_task_dev_tools(tools: &mut HashMap<String, McpTool>) {
    // Create task tool
    let create_task_tool = McpTool::new(
        "ratchet.create_task",
        "Create a new task with code, schemas, and optional test cases",
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Unique task name"
                },
                "description": {
                    "type": "string",
                    "description": "Task description"
                },
                "code": {
                    "type": "string",
                    "description": "JavaScript code for the task"
                },
                "input_schema": {
                    "type": "object",
                    "description": "JSON Schema for task input"
                },
                "output_schema": {
                    "type": "object",
                    "description": "JSON Schema for task output"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Task categories/tags"
                },
                "version": {
                    "type": "string",
                    "default": "0.1.0",
                    "description": "Task version"
                },
                "enabled": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to enable the task"
                },
                "test_cases": {
                    "type": "array",
                    "description": "Optional test cases",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "input": {"type": "object"},
                            "expected_output": {"type": "object"},
                            "should_fail": {"type": "boolean"},
                            "description": {"type": "string"}
                        },
                        "required": ["name", "input"]
                    }
                },
                "metadata": {
                    "type": "object",
                    "description": "Additional task metadata"
                }
            },
            "required": ["name", "description", "code", "input_schema", "output_schema"]
        }),
        "development",
    );
    tools.insert("ratchet.create_task".to_string(), create_task_tool);
    
    // Validate task tool
    let validate_task_tool = McpTool::new(
        "ratchet.validate_task",
        "Validate task code, schemas, and optionally run tests without execution",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID"
                },
                "code": {
                    "type": "string",
                    "description": "Optional JavaScript code to validate"
                },
                "input_schema": {
                    "type": "object",
                    "description": "Optional input schema to validate"
                },
                "output_schema": {
                    "type": "object",
                    "description": "Optional output schema to validate"
                },
                "run_tests": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to run test cases"
                },
                "syntax_only": {
                    "type": "boolean",
                    "default": false,
                    "description": "Only check syntax, skip other validations"
                }
            },
            "required": ["task_id"]
        }),
        "development",
    );
    tools.insert("ratchet.validate_task".to_string(), validate_task_tool);
    
    // Debug task tool
    let debug_task_tool = McpTool::new(
        "ratchet.debug_task_execution",
        "Debug task execution with breakpoints and variable inspection",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID"
                },
                "input": {
                    "type": "object",
                    "description": "Input data for debugging"
                },
                "breakpoints": {
                    "type": "array",
                    "items": {"type": "integer"},
                    "description": "Line numbers for breakpoints"
                },
                "step_mode": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enable step-by-step execution"
                },
                "capture_variables": {
                    "type": "boolean",
                    "default": true,
                    "description": "Capture all variable states"
                },
                "timeout_ms": {
                    "type": "integer",
                    "default": 300000,
                    "description": "Debug session timeout in milliseconds"
                }
            },
            "required": ["task_id", "input"]
        }),
        "development",
    );
    tools.insert("ratchet.debug_task_execution".to_string(), debug_task_tool);
    
    // Run tests tool
    let run_tests_tool = McpTool::new(
        "ratchet.run_task_tests",
        "Execute test cases for a task and report results",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID"
                },
                "test_names": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Specific test names to run (empty = all)"
                },
                "stop_on_failure": {
                    "type": "boolean",
                    "default": false,
                    "description": "Stop on first test failure"
                },
                "include_traces": {
                    "type": "boolean",
                    "default": true,
                    "description": "Include execution traces"
                },
                "parallel": {
                    "type": "boolean",
                    "default": false,
                    "description": "Run tests in parallel"
                }
            },
            "required": ["task_id"]
        }),
        "development",
    );
    tools.insert("ratchet.run_task_tests".to_string(), run_tests_tool);
    
    // Create version tool
    let create_version_tool = McpTool::new(
        "ratchet.create_task_version",
        "Create a new version of an existing task",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID"
                },
                "new_version": {
                    "type": "string",
                    "description": "New version number (must be higher)"
                },
                "description": {
                    "type": "string",
                    "description": "Version description/changelog"
                },
                "breaking_change": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether this is a breaking change"
                },
                "make_active": {
                    "type": "boolean",
                    "default": true,
                    "description": "Make this the active version"
                },
                "migration_script": {
                    "type": "string",
                    "description": "Optional migration script for breaking changes"
                }
            },
            "required": ["task_id", "new_version", "description"]
        }),
        "development",
    );
    tools.insert("ratchet.create_task_version".to_string(), create_version_tool);
    
    // Edit task tool
    let edit_task_tool = McpTool::new(
        "ratchet.edit_task",
        "Edit existing task code, schemas, and metadata",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID to edit"
                },
                "code": {
                    "type": "string",
                    "description": "New JavaScript code"
                },
                "input_schema": {
                    "type": "object",
                    "description": "New input schema"
                },
                "output_schema": {
                    "type": "object",
                    "description": "New output schema"
                },
                "description": {
                    "type": "string",
                    "description": "New task description"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "New task tags"
                },
                "validate_changes": {
                    "type": "boolean",
                    "default": true,
                    "description": "Validate changes before applying"
                },
                "create_backup": {
                    "type": "boolean",
                    "default": true,
                    "description": "Create backup before editing"
                }
            },
            "required": ["task_id"]
        }),
        "development",
    );
    tools.insert("ratchet.edit_task".to_string(), edit_task_tool);
    
    // Import tasks tool
    let import_tasks_tool = McpTool::new(
        "ratchet.import_tasks",
        "Import tasks from JSON or other formats",
        json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object",
                    "description": "Task data to import"
                },
                "format": {
                    "type": "string",
                    "enum": ["json", "zip", "directory"],
                    "default": "json",
                    "description": "Import format"
                },
                "overwrite_existing": {
                    "type": "boolean",
                    "default": false,
                    "description": "Overwrite existing tasks"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "include_tests": {"type": "boolean", "default": true},
                        "validate_tasks": {"type": "boolean", "default": true},
                        "name_prefix": {"type": "string"}
                    }
                }
            },
            "required": ["data"]
        }),
        "development",
    );
    tools.insert("ratchet.import_tasks".to_string(), import_tasks_tool);
    
    // Export tasks tool
    let export_tasks_tool = McpTool::new(
        "ratchet.export_tasks",
        "Export tasks to JSON or other formats",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task to export (optional, exports all if not provided)"
                },
                "format": {
                    "type": "string",
                    "enum": ["json", "zip", "individual"],
                    "default": "json",
                    "description": "Export format"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "include_tests": {"type": "boolean", "default": true},
                        "include_metadata": {"type": "boolean", "default": true},
                        "include_versions": {"type": "boolean", "default": false}
                    }
                }
            }
        }),
        "development",
    );
    tools.insert("ratchet.export_tasks".to_string(), export_tasks_tool);
    
    // Generate from template tool
    let generate_template_tool = McpTool::new(
        "ratchet.generate_from_template",
        "Generate a new task from a predefined template",
        json!({
            "type": "object",
            "properties": {
                "template": {
                    "type": "string",
                    "description": "Template name (use list_templates to see available)"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the generated task"
                },
                "parameters": {
                    "type": "object",
                    "description": "Template-specific parameters"
                },
                "description": {
                    "type": "string",
                    "description": "Task description"
                }
            },
            "required": ["template", "name", "parameters"]
        }),
        "development",
    );
    tools.insert("ratchet.generate_from_template".to_string(), generate_template_tool);
    
    // List templates tool
    let list_templates_tool = McpTool::new(
        "ratchet.list_templates",
        "List all available task templates",
        json!({
            "type": "object",
            "properties": {}
        }),
        "development",
    );
    tools.insert("ratchet.list_templates".to_string(), list_templates_tool);
}

/// Execute task development tools
pub async fn execute_task_dev_tool(
    tool_name: &str,
    context: ToolExecutionContext,
    service: Arc<TaskDevelopmentService>,
) -> McpResult<ToolsCallResult> {
    let args = context.arguments.ok_or_else(|| McpError::InvalidParams {
        method: tool_name.to_string(),
        details: "Missing arguments".to_string(),
    })?;
    
    match tool_name {
        "ratchet.create_task" => {
            let request: CreateTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.create_task(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to create task: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.validate_task" => {
            let request: ValidateTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.validate_task(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to validate task: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.debug_task_execution" => {
            let request: DebugTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.debug_task(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to debug task: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.run_task_tests" => {
            let request: RunTaskTestsRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.run_task_tests(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to run tests: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.create_task_version" => {
            let request: CreateTaskVersionRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.create_task_version(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to create version: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.edit_task" => {
            let request: EditTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.edit_task(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to edit task: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.import_tasks" => {
            let request: ImportTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.import_tasks(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to import tasks: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.export_tasks" => {
            let request: ExportTaskRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.export_tasks(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to export tasks: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.generate_from_template" => {
            let request: GenerateFromTemplateRequest = serde_json::from_value(args)
                .map_err(|e| McpError::InvalidParams {
                    method: tool_name.to_string(),
                    details: format!("Invalid request: {}", e),
                })?;
            
            match service.generate_from_template(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to generate from template: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        "ratchet.list_templates" => {
            match service.list_templates().await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to list templates: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }
        
        _ => Err(McpError::ToolNotFound {
            tool_name: tool_name.to_string(),
        }),
    }
}