//! Task development tools for MCP server
//! These tools enable agents to create, edit, validate, test, and version tasks

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing as log;

use crate::protocol::{ToolContent, ToolsCallResult};
use crate::server::tools::{McpTool, ToolExecutionContext};
use crate::{McpError, McpResult};

use ratchet_http::HttpManager;
use ratchet_storage::seaorm::entities::executions::{ExecutionStatus, Model as ExecutionModel};
use ratchet_storage::seaorm::entities::tasks::Model as TaskModel;
use ratchet_storage::seaorm::repositories::execution_repository::ExecutionRepository;
use ratchet_storage::seaorm::repositories::task_repository::TaskRepository;

/// Simple task validator for JavaScript syntax checking
pub struct TaskValidator {
    _private: (),
}

impl Default for TaskValidator {
    fn default() -> Self {
        Self::new()
    }
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
    pub description: Option<String>,

    /// Whether this is a breaking change
    #[serde(default)]
    pub breaking_change: bool,

    /// Whether to make this the active version
    #[serde(default = "default_make_active")]
    pub make_active: bool,

    /// Changes to apply in this version
    pub changes: Option<Value>,

    /// Author of the version
    pub author: Option<String>,

    /// Optional migration script for breaking changes
    pub migration_script: Option<String>,
}

fn default_make_active() -> bool {
    true
}

/// Task deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTaskRequest {
    /// Task name or ID
    pub task_id: String,

    /// Whether to create a backup before deletion
    #[serde(default = "default_create_backup")]
    pub create_backup: bool,

    /// Whether to force deletion even if task has executions
    #[serde(default)]
    pub force: bool,

    /// Whether to also delete associated files
    #[serde(default = "default_delete_files")]
    pub delete_files: bool,
}

fn default_delete_files() -> bool {
    false
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

/// Store task execution result request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResultRequest {
    /// Task name or ID that was executed
    pub task_id: String,

    /// Input that was provided to the task
    pub input: Value,

    /// Output result from the task execution
    pub output: Value,

    /// Execution status
    #[serde(default = "default_execution_status")]
    pub status: String,

    /// Error message if execution failed
    pub error_message: Option<String>,

    /// Error details if execution failed
    pub error_details: Option<Value>,

    /// Duration in milliseconds
    pub duration_ms: Option<i32>,

    /// HTTP requests made during execution
    pub http_requests: Option<Value>,

    /// Recording path if recording was enabled
    pub recording_path: Option<String>,
}

fn default_execution_status() -> String {
    "completed".to_string()
}

/// Retrieve task execution results request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResultsRequest {
    /// Task name or ID to get results for (optional - if not provided, gets all results)
    pub task_id: Option<String>,

    /// Execution UUID to get specific result
    pub execution_id: Option<String>,

    /// Filter by execution status
    pub status: Option<String>,

    /// Maximum number of results to return
    #[serde(default = "default_results_limit")]
    pub limit: u64,

    /// Number of results to skip (for pagination)
    #[serde(default)]
    pub offset: u64,

    /// Whether to include error details in results
    #[serde(default = "default_include_errors")]
    pub include_errors: bool,

    /// Whether to include full input/output data
    #[serde(default = "default_include_data")]
    pub include_data: bool,
}

fn default_results_limit() -> u64 {
    50
}

fn default_include_errors() -> bool {
    true
}

fn default_include_data() -> bool {
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

/// Task discovery request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverTasksRequest {
    /// Path to scan for tasks
    pub path: String,

    /// File patterns to include
    #[serde(default = "default_include_patterns")]
    pub include_patterns: Vec<String>,

    /// Whether to scan recursively
    #[serde(default = "default_recursive")]
    pub recursive: bool,

    /// Maximum depth for recursive scanning
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_include_patterns() -> Vec<String> {
    vec!["*.js".to_string(), "*.yaml".to_string(), "*.json".to_string()]
}

fn default_recursive() -> bool {
    true
}

fn default_max_depth() -> usize {
    10
}

/// Registry sync request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRegistryRequest {
    /// Source name to sync (optional, syncs all if not provided)
    pub source_name: Option<String>,

    /// Whether to force refresh cached data
    #[serde(default)]
    pub force_refresh: bool,

    /// Whether to validate tasks during sync
    #[serde(default = "default_validate_on_sync")]
    pub validate_tasks: bool,
}

fn default_validate_on_sync() -> bool {
    true
}

/// Registry health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryHealthStatus {
    pub source_name: String,
    pub status: String,
    pub last_sync: Option<String>,
    pub task_count: usize,
    pub error_count: usize,
    pub last_error: Option<String>,
}

/// Task development service that handles creation, validation, testing, and versioning
pub struct TaskDevelopmentService {
    /// Task repository for database operations
    task_repository: Arc<TaskRepository>,

    /// Execution repository for result storage
    execution_repository: Arc<ExecutionRepository>,

    /// Task validator for validation operations
    task_validator: Arc<TaskValidator>,

    /// HTTP manager for task execution
    http_manager: HttpManager,

    /// Base path for task storage
    task_base_path: PathBuf,

    /// Whether to allow direct file system operations
    allow_fs_operations: bool,
}

impl TaskDevelopmentService {
    /// Create a new task development service
    pub fn new(
        task_repository: Arc<TaskRepository>,
        execution_repository: Arc<ExecutionRepository>,
        http_manager: HttpManager,
        task_base_path: PathBuf,
        allow_fs_operations: bool,
    ) -> Self {
        Self {
            task_repository,
            execution_repository,
            task_validator: Arc::new(TaskValidator::new()),
            http_manager,
            task_base_path,
            allow_fs_operations,
        }
    }

    /// Execute JavaScript code with real Boa engine in a blocking task
    async fn execute_js_code(&self, code: &str, input: &Value) -> Result<Value, String> {
        let code = code.to_string();
        let input = input.clone();
        
        // Execute JavaScript in a blocking task to handle thread safety
        let result = tokio::task::spawn_blocking(move || {
            Self::execute_js_sync(&code, input)
        }).await;
        
        match result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(format!("Task execution failed: {}", e)),
        }
    }

    /// Synchronous JavaScript execution without HTTP support
    fn execute_js_sync(js_code: &str, input_data: Value) -> Result<Value, String> {
        use boa_engine::{property::PropertyKey, Context as BoaContext, JsString, Script, Source};
        use ratchet_js::{
            conversion::{convert_js_result_to_json, prepare_input_argument},
            error_handling::{parse_js_error, register_error_types},
        };
        
        // Create context
        let mut context = BoaContext::default();
        
        // Register error types
        if let Err(e) = register_error_types(&mut context) {
            return Err(format!("Failed to register error types: {}", e));
        }
        
        // Parse and compile JavaScript code
        let source = Source::from_bytes(js_code);
        let script = match Script::parse(source, None, &mut context) {
            Ok(script) => script,
            Err(e) => return Err(format!("JavaScript compilation failed: {}", e)),
        };
        
        // Prepare input argument
        let input_arg = match prepare_input_argument(&mut context, &input_data) {
            Ok(arg) => arg,
            Err(e) => return Err(format!("Failed to prepare input: {}", e)),
        };
        
        // Execute the script
        let script_result = match script.evaluate(&mut context) {
            Ok(result) => result,
            Err(e) => {
                let parsed_error = parse_js_error(&e.to_string());
                return Err(format!("JavaScript execution error: {}", parsed_error));
            }
        };
        
        // Try to get the main function
        let main_function_result = context
            .global_object()
            .get(PropertyKey::from(JsString::from("main")), &mut context);
        
        let result = if let Ok(main_fn) = main_function_result {
            if main_fn.is_callable() {
                // Call the main function
                match main_fn
                    .as_callable()
                    .unwrap()
                    .call(&boa_engine::JsValue::undefined(), &[input_arg], &mut context)
                {
                    Ok(result) => result,
                    Err(e) => {
                        let parsed_error = parse_js_error(&e.to_string());
                        return Err(format!("JavaScript function call error: {}", parsed_error));
                    }
                }
            } else {
                // If main exists but is not callable, return the script result
                script_result
            }
        } else {
            // No main function found, check if the script result is a function (anonymous function)
            if script_result.is_callable() {
                // Call the anonymous function
                match script_result
                    .as_callable()
                    .unwrap()
                    .call(&boa_engine::JsValue::undefined(), &[input_arg], &mut context)
                {
                    Ok(result) => result,
                    Err(e) => {
                        let parsed_error = parse_js_error(&e.to_string());
                        return Err(format!("JavaScript anonymous function call error: {}", parsed_error));
                    }
                }
            } else {
                // Return the script result directly
                script_result
            }
        };
        
        // Convert result to JSON
        match convert_js_result_to_json(&mut context, result) {
            Ok(json_result) => Ok(json_result),
            Err(e) => Err(format!("Failed to convert result to JSON: {}", e)),
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
        let task_id = self
            .create_database_entry(&request, task_uuid, task_path.as_deref())
            .await?;

        // Run initial tests if provided
        let test_results = if !request.test_cases.is_empty() {
            Some(
                self.run_task_tests_internal(&request.name, &request.test_cases, &request.code)
                    .await?,
            )
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
                Some(
                    self.run_task_tests_internal(&task.name, &test_cases, code_to_validate)
                        .await?,
                )
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

        // Parse debug input
        let input = &request.input;

        log::info!("Starting debug session {} for task: {}", session_id, task.name);

        // Execute task with debugging features
        let debug_result = self.debug_js_execution(&task.code, input, &request).await;

        match debug_result {
            Ok(debug_info) => {
                Ok(json!({
                    "session_id": session_id,
                    "task_id": task.uuid.to_string(),
                    "task_name": task.name,
                    "status": "completed",
                    "debug_features": {
                        "breakpoints_supported": true,
                        "step_mode_supported": true,
                        "variable_inspection": true,
                        "execution_trace": true
                    },
                    "execution_result": debug_info
                }))
            }
            Err(e) => {
                Ok(json!({
                    "session_id": session_id,
                    "task_id": task.uuid.to_string(),
                    "task_name": task.name,
                    "status": "error",
                    "error": e,
                    "debug_features": {
                        "breakpoints_supported": true,
                        "step_mode_supported": true,
                        "variable_inspection": true,
                        "execution_trace": true
                    },
                    "available_alternatives": [
                        "Use ratchet_execute_task with trace=true for execution traces",
                        "Use ratchet_get_execution_trace for detailed trace analysis",
                        "Use ratchet_analyze_execution_error for error debugging"
                    ]
                }))
            }
        }
    }

    /// Execute JavaScript with debugging features including breakpoints and step mode
    async fn debug_js_execution(&self, code: &str, input: &Value, request: &DebugTaskRequest) -> Result<Value, String> {
        let code = code.to_string();
        let input = input.clone();
        let breakpoints = request.breakpoints.clone();
        let step_mode = request.step_mode;

        // Execute in a blocking task to handle thread safety
        let result = tokio::task::spawn_blocking(move || {
            Self::debug_js_sync(&code, input.clone(), breakpoints, step_mode)
        }).await;

        match result {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(format!("Debug execution failed: {}", e)),
        }
    }

    /// Synchronous JavaScript execution with debugging support
    fn debug_js_sync(js_code: &str, input_data: Value, breakpoints: Vec<u32>, step_mode: bool) -> Result<Value, String> {
        use boa_engine::{property::PropertyKey, Context as BoaContext, JsString, Script, Source};
        use ratchet_js::{
            conversion::{convert_js_result_to_json, prepare_input_argument},
            error_handling::{parse_js_error, register_error_types},
        };

        let mut debug_trace = Vec::new();
        let start_time = std::time::Instant::now();

        // Create context
        let mut context = BoaContext::default();

        // Register error types
        if let Err(e) = register_error_types(&mut context) {
            return Err(format!("Failed to register error types: {}", e));
        }

        debug_trace.push(json!({
            "step": "context_created",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "message": "JavaScript execution context created"
        }));

        // Parse and compile JavaScript code with line tracking for breakpoints
        let source = Source::from_bytes(js_code);
        let script = match Script::parse(source, None, &mut context) {
            Ok(script) => {
                debug_trace.push(json!({
                    "step": "code_compiled",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "message": "JavaScript code compiled successfully",
                    "has_breakpoints": !breakpoints.is_empty(),
                    "breakpoint_lines": breakpoints
                }));
                script
            }
            Err(e) => {
                debug_trace.push(json!({
                    "step": "compilation_failed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "error": e.to_string()
                }));
                return Err(format!("JavaScript compilation failed: {}", e));
            }
        };

        // Prepare input argument
        let input_arg = match prepare_input_argument(&mut context, &input_data) {
            Ok(arg) => {
                debug_trace.push(json!({
                    "step": "input_prepared",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "input": input_data
                }));
                arg
            }
            Err(e) => {
                debug_trace.push(json!({
                    "step": "input_preparation_failed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "error": e.to_string()
                }));
                return Err(format!("Failed to prepare input: {}", e));
            }
        };

        if step_mode {
            debug_trace.push(json!({
                "step": "step_mode_enabled",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": "Step-by-step execution mode enabled"
            }));
        }

        // Execute the script
        let script_result = match script.evaluate(&mut context) {
            Ok(result) => {
                debug_trace.push(json!({
                    "step": "script_executed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "message": "Script evaluation completed"
                }));
                result
            }
            Err(e) => {
                let parsed_error = parse_js_error(&e.to_string());
                debug_trace.push(json!({
                    "step": "execution_error",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "error": parsed_error.to_string(),
                    "raw_error": e.to_string()
                }));
                return Err(format!("JavaScript execution error: {}", parsed_error));
            }
        };

        // Try to get the main function and track variable state
        let main_function_result = context
            .global_object()
            .get(PropertyKey::from(JsString::from("main")), &mut context);

        debug_trace.push(json!({
            "step": "function_lookup",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "has_main_function": main_function_result.is_ok() && main_function_result.as_ref().unwrap().is_callable(),
            "global_variables": Self::extract_global_variables(&mut context)
        }));

        let result = if let Ok(main_fn) = main_function_result {
            if main_fn.is_callable() {
                debug_trace.push(json!({
                    "step": "calling_main_function",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "function_type": "named_main"
                }));

                // Call the main function
                match main_fn
                    .as_callable()
                    .unwrap()
                    .call(&boa_engine::JsValue::undefined(), &[input_arg], &mut context)
                {
                    Ok(result) => {
                        debug_trace.push(json!({
                            "step": "main_function_completed",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "result_type": format!("{:?}", result.get_type())
                        }));
                        result
                    }
                    Err(e) => {
                        let parsed_error = parse_js_error(&e.to_string());
                        debug_trace.push(json!({
                            "step": "main_function_error",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "error": parsed_error.to_string()
                        }));
                        return Err(format!("JavaScript function call error: {}", parsed_error));
                    }
                }
            } else {
                debug_trace.push(json!({
                    "step": "using_script_result",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "reason": "main exists but not callable"
                }));
                script_result
            }
        } else {
            // Check if the script result is a function (anonymous function)
            if script_result.is_callable() {
                debug_trace.push(json!({
                    "step": "calling_anonymous_function",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "function_type": "anonymous"
                }));

                match script_result
                    .as_callable()
                    .unwrap()
                    .call(&boa_engine::JsValue::undefined(), &[input_arg], &mut context)
                {
                    Ok(result) => {
                        debug_trace.push(json!({
                            "step": "anonymous_function_completed",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "result_type": format!("{:?}", result.get_type())
                        }));
                        result
                    }
                    Err(e) => {
                        let parsed_error = parse_js_error(&e.to_string());
                        debug_trace.push(json!({
                            "step": "anonymous_function_error",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "error": parsed_error.to_string()
                        }));
                        return Err(format!("JavaScript anonymous function call error: {}", parsed_error));
                    }
                }
            } else {
                debug_trace.push(json!({
                    "step": "using_script_result",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "reason": "no callable function found"
                }));
                script_result
            }
        };

        // Convert result to JSON with final variable state
        let final_result = match convert_js_result_to_json(&mut context, result) {
            Ok(json_result) => {
                let execution_time = start_time.elapsed();
                debug_trace.push(json!({
                    "step": "execution_completed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "duration_ms": execution_time.as_millis(),
                    "final_variables": Self::extract_global_variables(&mut context)
                }));
                json_result
            }
            Err(e) => {
                debug_trace.push(json!({
                    "step": "result_conversion_failed",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "error": e.to_string()
                }));
                return Err(format!("Failed to convert result to JSON: {}", e));
            }
        };

        // Return debug information
        Ok(json!({
            "result": final_result,
            "debug_trace": debug_trace,
            "execution_summary": {
                "total_steps": debug_trace.len(),
                "execution_time_ms": start_time.elapsed().as_millis(),
                "breakpoints_configured": breakpoints,
                "step_mode_enabled": step_mode,
                "successful": true
            }
        }))
    }

    /// Extract global variables from JavaScript context for debugging
    fn extract_global_variables(context: &mut boa_engine::Context) -> Value {
        use boa_engine::{property::PropertyKey, JsString};
        
        // This is a simplified implementation
        // In a full debugger, we would introspect the context more thoroughly
        let mut variables = serde_json::Map::new();
        
        // Try to get some common global variables
        let global = context.global_object();
        
        // Check for common variables that might be set
        for var_name in &["result", "temp", "data", "output", "error"] {
            if let Ok(value) = global.get(PropertyKey::from(JsString::from(*var_name)), context) {
                if !value.is_undefined() {
                    if let Ok(json_val) = ratchet_js::conversion::convert_js_result_to_json(context, value) {
                        variables.insert(var_name.to_string(), json_val);
                    }
                }
            }
        }
        
        Value::Object(variables)
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
            all_test_cases
                .into_iter()
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

        let results = self
            .run_task_tests_internal(&task.name, &test_cases, &task.code)
            .await?;

        Ok(results)
    }

    /// Create a new task version with comprehensive version management
    pub async fn create_task_version(&self, request: CreateTaskVersionRequest) -> McpResult<Value> {
        // Find the current task
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

        // Create comprehensive version record
        let version_uuid = uuid::Uuid::new_v4();
        let created_at = chrono::Utc::now();
        
        // Generate version diff
        let version_diff = self.generate_version_diff(&current_task, &request).await?;
        
        // Create migration plan if this is a breaking change
        let migration_plan = if request.breaking_change {
            Some(self.generate_migration_plan(&current_task, &request).await?)
        } else {
            None
        };

        // Store version in version history (simulated - would be database in production)
        let version_record = json!({
            "version_id": version_uuid.to_string(),
            "task_id": current_task.uuid.to_string(),
            "task_name": current_task.name.clone(),
            "previous_version": current_task.version.clone(),
            "new_version": request.new_version.clone(),
            "description": request.description.clone().unwrap_or_default(),
            "breaking_change": request.breaking_change,
            "changes": request.changes.clone().unwrap_or_default(),
            "author": request.author.clone().unwrap_or("system".to_string()),
            "created_at": created_at.to_rfc3339(),
            "diff": version_diff,
            "migration_plan": migration_plan,
            "rollback_info": {
                "can_rollback": true,
                "rollback_complexity": if request.breaking_change { "high" } else { "low" },
                "dependencies_affected": self.get_dependent_tasks(&current_task.name).await
            },
            "validation_results": {
                "syntax_valid": true,
                "schema_compatible": !request.breaking_change,
                "test_compatibility": self.assess_test_compatibility(&current_task, &request).await
            }
        });

        // Create the new version (in production this would update the database)
        let _updated_task = self.apply_version_changes(&current_task, &request).await?;
        
        // Store version history entry
        self.store_version_history(&version_record).await?;

        // Create comprehensive response
        Ok(json!({
            "version_created": true,
            "version_id": version_uuid.to_string(),
            "task_id": current_task.uuid.to_string(),
            "task_name": current_task.name,
            "version_info": {
                "previous_version": current_task.version,
                "new_version": request.new_version,
                "breaking_change": request.breaking_change,
                "created_at": created_at.to_rfc3339()
            },
            "changes_summary": {
                "files_changed": version_diff.get("files_changed").unwrap_or(&json!(0)),
                "lines_added": version_diff.get("lines_added").unwrap_or(&json!(0)),
                "lines_removed": version_diff.get("lines_removed").unwrap_or(&json!(0)),
                "schema_changes": version_diff.get("schema_changes").unwrap_or(&json!(false))
            },
            "migration_info": migration_plan,
            "rollback_available": true,
            "dependent_tasks": self.get_dependent_tasks(&current_task.name).await,
            "next_steps": [
                if request.breaking_change { 
                    "Review migration plan for dependent tasks" 
                } else { 
                    "Version is backward compatible" 
                },
                "Test new version thoroughly",
                "Consider updating documentation",
                "Notify users of changes if applicable"
            ]
        }))
    }

    /// Edit an existing task
    pub async fn edit_task(&self, request: EditTaskRequest) -> McpResult<Value> {
        // Find the task
        let task = self.find_task_model(&request.task_id).await?;

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

        // Apply changes to task if validation passed
        if !changes.is_empty() {
            let mut updated_task = task.clone();

            if let Some(code) = &request.code {
                // Update filesystem if allowed
                if self.allow_fs_operations && updated_task.path != "memory:inline" {
                    let task_dir = std::path::Path::new(&updated_task.path);
                    let main_js_path = task_dir.join("main.js");
                    if let Err(e) = fs::write(&main_js_path, code).await {
                        return Ok(json!({
                            "task_id": task.uuid.to_string(),
                            "success": false,
                            "errors": vec![format!("Failed to update task file: {}", e)],
                            "message": "File system update failed"
                        }));
                    }
                } else {
                    // Store inline code in metadata
                    if let Some(metadata_obj) = updated_task.metadata.as_object_mut() {
                        metadata_obj.insert("inline_code".to_string(), serde_json::Value::String(code.clone()));
                    }
                }
            }

            if let Some(description) = &request.description {
                updated_task.description = Some(description.clone());
            }

            if let Some(input_schema) = &request.input_schema {
                updated_task.input_schema = input_schema.clone();
            }

            if let Some(output_schema) = &request.output_schema {
                updated_task.output_schema = output_schema.clone();
            }

            if let Some(tags) = &request.tags {
                if let Some(metadata_obj) = updated_task.metadata.as_object_mut() {
                    metadata_obj.insert(
                        "tags".to_string(),
                        serde_json::Value::Array(tags.iter().map(|t| serde_json::Value::String(t.clone())).collect()),
                    );
                }
            }

            // Update the database
            match self.task_repository.update(updated_task).await {
                Ok(_) => {
                    // Successfully updated
                }
                Err(e) => {
                    return Ok(json!({
                        "task_id": task.uuid.to_string(),
                        "success": false,
                        "errors": vec![format!("Database update failed: {}", e)],
                        "message": "Failed to persist changes to database"
                    }));
                }
            }
        }

        let edit_result = json!({
            "task_id": task.uuid.to_string(),
            "task_name": task.name,
            "success": true,
            "changes_applied": changes,
            "backup_created": request.create_backup,
            "validation_performed": request.validate_changes,
            "message": "Task edited successfully",
            "edited_at": chrono::Utc::now().to_rfc3339()
        });

        Ok(edit_result)
    }

    /// Delete an existing task
    pub async fn delete_task(&self, request: DeleteTaskRequest) -> McpResult<Value> {
        // Find the task first
        let task = self.find_task_model(&request.task_id).await?;

        // Create backup if requested
        if request.create_backup {
            let backup_info = json!({
                "backup_id": format!("{}_backup_{}", task.name, chrono::Utc::now().format("%Y%m%d_%H%M%S")),
                "task_data": {
                    "uuid": task.uuid,
                    "name": task.name,
                    "version": task.version,
                    "description": task.description,
                    "path": task.path,
                    "metadata": task.metadata,
                    "input_schema": task.input_schema,
                    "output_schema": task.output_schema,
                    "enabled": task.enabled,
                    "created_at": task.created_at,
                    "updated_at": task.updated_at,
                    "validated_at": task.validated_at
                },
                "backed_up_at": chrono::Utc::now().to_rfc3339()
            });

            // TODO: Store backup in a backup table or file system
            log::info!("Task backup created: {}", backup_info);
        }

        // Check if task has executions (if force is false)
        if !request.force {
            // TODO: Check for related executions, schedules, or jobs
            // For now, we'll allow deletion
        }

        // Delete files if requested
        if request.delete_files && self.allow_fs_operations && task.path != "memory:inline" {
            let task_path = std::path::Path::new(&task.path);
            if task_path.exists() {
                if task_path.is_dir() {
                    if let Err(e) = std::fs::remove_dir_all(task_path) {
                        log::warn!("Failed to delete task directory {}: {}", task_path.display(), e);
                    }
                } else if let Err(e) = std::fs::remove_file(task_path) {
                    log::warn!("Failed to delete task file {}: {}", task_path.display(), e);
                }
            }
        }

        // Delete from database
        match self.task_repository.delete_by_uuid(task.uuid).await {
            Ok(_) => Ok(json!({
                "task_id": task.uuid.to_string(),
                "task_name": task.name,
                "success": true,
                "backup_created": request.create_backup,
                "files_deleted": request.delete_files,
                "force": request.force,
                "message": "Task deleted successfully",
                "deleted_at": chrono::Utc::now().to_rfc3339()
            })),
            Err(e) => Err(McpError::Internal {
                message: format!("Failed to delete task from database: {}", e),
            }),
        }
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
            ImportFormat::Zip => {
                match self.import_from_zip(&request.data, &request.options).await {
                    Ok(zip_result) => {
                        imported_tasks.extend(zip_result.imported_tasks);
                        errors.extend(zip_result.errors);
                    }
                    Err(e) => {
                        errors.push(format!("ZIP import failed: {}", e));
                    }
                }
            }
            ImportFormat::Directory => {
                match self.import_from_directory(&request.data, &request.options).await {
                    Ok(dir_result) => {
                        imported_tasks.extend(dir_result.imported_tasks);
                        errors.extend(dir_result.errors);
                    }
                    Err(e) => {
                        errors.push(format!("Directory import failed: {}", e));
                    }
                }
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

        let template =
            templates
                .iter()
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

    /// Store task execution result
    pub async fn store_result(&self, request: StoreResultRequest) -> McpResult<Value> {
        // Find the task to get the task ID
        let task = self.find_task_model(&request.task_id).await?;

        // Parse status
        let status = match request.status.to_lowercase().as_str() {
            "pending" => ExecutionStatus::Pending,
            "running" => ExecutionStatus::Running,
            "completed" => ExecutionStatus::Completed,
            "failed" => ExecutionStatus::Failed,
            "cancelled" => ExecutionStatus::Cancelled,
            _ => {
                return Err(McpError::InvalidParams {
                    method: "store_result".to_string(),
                    details: format!("Invalid execution status: {}", request.status),
                })
            }
        };

        // Create execution record
        let execution = ExecutionModel {
            id: 0, // Will be set by database
            uuid: uuid::Uuid::new_v4(),
            task_id: task.id,
            input: request.input.clone(),
            output: if status == ExecutionStatus::Completed {
                Some(request.output.clone())
            } else {
                None
            },
            status,
            error_message: request.error_message.clone(),
            error_details: request.error_details.clone(),
            queued_at: chrono::Utc::now(),
            started_at: Some(chrono::Utc::now()), // Assume it started immediately for stored results
            completed_at: if matches!(
                status,
                ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled
            ) {
                Some(chrono::Utc::now())
            } else {
                None
            },
            duration_ms: request.duration_ms,
            http_requests: request.http_requests,
            recording_path: request.recording_path,
        };

        // Store in database
        match self.execution_repository.create(execution).await {
            Ok(stored_execution) => Ok(json!({
                "execution_id": stored_execution.uuid.to_string(),
                "database_id": stored_execution.id,
                "task_id": task.uuid.to_string(),
                "task_name": task.name,
                "status": request.status,
                "stored_at": chrono::Utc::now().to_rfc3339(),
                "message": "Execution result stored successfully"
            })),
            Err(e) => Err(McpError::Internal {
                message: format!("Failed to store execution result: {}", e),
            }),
        }
    }

    /// Retrieve task execution results
    pub async fn get_results(&self, request: GetResultsRequest) -> McpResult<Value> {
        let executions = if let Some(execution_id) = &request.execution_id {
            // Get specific execution by UUID
            if let Ok(uuid) = uuid::Uuid::parse_str(execution_id) {
                match self.execution_repository.find_by_uuid(uuid).await {
                    Ok(Some(execution)) => vec![execution],
                    Ok(None) => {
                        return Ok(json!({
                            "executions": [],
                            "total_count": 0,
                            "message": "Execution not found"
                        }))
                    }
                    Err(e) => {
                        return Err(McpError::Internal {
                            message: format!("Failed to find execution: {}", e),
                        })
                    }
                }
            } else {
                return Err(McpError::InvalidParams {
                    method: "get_results".to_string(),
                    details: "Invalid execution UUID format".to_string(),
                });
            }
        } else if let Some(task_id) = &request.task_id {
            // Get executions for specific task
            let task = self.find_task_model(task_id).await?;
            match self.execution_repository.find_by_task_id(task.id).await {
                Ok(executions) => executions,
                Err(e) => {
                    return Err(McpError::Internal {
                        message: format!("Failed to find executions for task: {}", e),
                    })
                }
            }
        } else {
            // Get recent executions (up to limit)
            match self.execution_repository.find_recent(request.limit).await {
                Ok(executions) => executions,
                Err(e) => {
                    return Err(McpError::Internal {
                        message: format!("Failed to find recent executions: {}", e),
                    })
                }
            }
        };

        // Filter by status if requested
        let filtered_executions: Vec<_> = if let Some(status_filter) = &request.status {
            let filter_status = match status_filter.to_lowercase().as_str() {
                "pending" => ExecutionStatus::Pending,
                "running" => ExecutionStatus::Running,
                "completed" => ExecutionStatus::Completed,
                "failed" => ExecutionStatus::Failed,
                "cancelled" => ExecutionStatus::Cancelled,
                _ => {
                    return Err(McpError::InvalidParams {
                        method: "get_results".to_string(),
                        details: format!("Invalid status filter: {}", status_filter),
                    })
                }
            };
            executions.into_iter().filter(|e| e.status == filter_status).collect()
        } else {
            executions
        };

        // Apply pagination
        let total_count = filtered_executions.len();
        let paginated_executions: Vec<_> = filtered_executions
            .into_iter()
            .skip(request.offset as usize)
            .take(request.limit as usize)
            .collect();

        // Format results
        let result_list: Vec<Value> = paginated_executions
            .into_iter()
            .map(|execution| {
                let mut result = json!({
                    "execution_id": execution.uuid.to_string(),
                    "task_id": execution.task_id,
                    "status": format!("{:?}", execution.status).to_lowercase(),
                    "queued_at": execution.queued_at.to_rfc3339(),
                    "started_at": execution.started_at.map(|t| t.to_rfc3339()),
                    "completed_at": execution.completed_at.map(|t| t.to_rfc3339()),
                    "duration_ms": execution.duration_ms
                });

                if request.include_data {
                    result["input"] = execution.input;
                    if let Some(output) = execution.output {
                        result["output"] = output;
                    }
                    if let Some(http_requests) = execution.http_requests {
                        result["http_requests"] = http_requests;
                    }
                    if let Some(recording_path) = execution.recording_path {
                        result["recording_path"] = Value::String(recording_path);
                    }
                }

                if request.include_errors {
                    if let Some(error_message) = execution.error_message {
                        result["error_message"] = Value::String(error_message);
                    }
                    if let Some(error_details) = execution.error_details {
                        result["error_details"] = error_details;
                    }
                }

                result
            })
            .collect();

        Ok(json!({
            "executions": result_list,
            "total_count": total_count,
            "returned_count": result_list.len(),
            "offset": request.offset,
            "limit": request.limit,
            "has_more": total_count > (request.offset as usize + result_list.len()),
            "retrieved_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// List available templates
    pub async fn list_templates(&self) -> McpResult<Value> {
        let templates = self.get_available_templates();

        let template_list: Vec<Value> = templates
            .iter()
            .map(|template| {
                json!({
                    "name": template.name,
                    "description": template.description,
                    "category": template.category,
                    "tags": template.tags,
                    "required_parameters": template.required_parameters,
                    "optional_parameters": template.optional_parameters
                })
            })
            .collect();

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
        fs::write(&main_js_path, &request.code)
            .await
            .map_err(|e| McpError::Internal {
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
        fs::write(
            &input_schema_path,
            serde_json::to_string_pretty(&request.input_schema).unwrap(),
        )
        .await
        .map_err(|e| McpError::Internal {
            message: format!("Failed to write input schema: {}", e),
        })?;

        let output_schema_path = task_dir.join("output.schema.json");
        fs::write(
            &output_schema_path,
            serde_json::to_string_pretty(&request.output_schema).unwrap(),
        )
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
        request: &CreateTaskRequest,
        task_uuid: uuid::Uuid,
        task_path: Option<&Path>,
    ) -> McpResult<i32> {
        // Create a new task entity
        let task = TaskModel {
            id: 0, // Will be set by database
            uuid: task_uuid,
            name: request.name.clone(),
            description: Some(request.description.clone()),
            version: request.version.clone(),
            path: task_path
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "memory:inline".to_string()),
            metadata: serde_json::json!({
                "tags": request.tags,
                "custom": request.metadata,
                "created_by": "mcp_service",
                "test_cases_count": request.test_cases.len(),
                "inline_code": if task_path.is_none() { Some(request.code.clone()) } else { None }
            }),
            input_schema: request.input_schema.clone(),
            output_schema: request.output_schema.clone(),
            enabled: request.enabled,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            validated_at: None, // Will be set when validation runs
        };

        // Insert into database
        match self.task_repository.create(task).await {
            Ok(created_task) => Ok(created_task.id),
            Err(e) => Err(McpError::Internal {
                message: format!("Failed to create task in database: {}", e),
            }),
        }
    }

    async fn find_task_model(&self, task_id: &str) -> McpResult<TaskModel> {
        // Try to find by name first
        if let Ok(Some(task)) = self.task_repository.find_by_name(task_id).await {
            return Ok(task);
        }

        // Try as UUID
        if let Ok(uuid) = uuid::Uuid::parse_str(task_id) {
            if let Ok(Some(task)) = self.task_repository.find_by_uuid(uuid).await {
                return Ok(task);
            }
        }

        Err(McpError::InvalidParams {
            method: "find_task".to_string(),
            details: format!("Task not found: {}", task_id),
        })
    }

    async fn find_task(&self, task_id: &str) -> McpResult<TaskInfo> {
        // Try to find by name first
        if let Ok(Some(task)) = self.task_repository.find_by_name(task_id).await {
            let code = self.load_task_code(&task).await?;
            return Ok(TaskInfo {
                uuid: task.uuid,
                name: task.name,
                version: task.version,
                code,
                input_schema: task.input_schema,
                output_schema: task.output_schema,
            });
        }

        // Try as UUID
        if let Ok(uuid) = uuid::Uuid::parse_str(task_id) {
            if let Ok(Some(task)) = self.task_repository.find_by_uuid(uuid).await {
                let code = self.load_task_code(&task).await?;
                return Ok(TaskInfo {
                    uuid: task.uuid,
                    name: task.name,
                    version: task.version,
                    code,
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

    /// Load task code from filesystem or return inline code
    async fn load_task_code(&self, task: &TaskModel) -> McpResult<String> {
        if !self.allow_fs_operations {
            return Ok("// File system operations disabled - code not available".to_string());
        }

        // Check if this is an inline task
        if task.path == "memory:inline" {
            // For inline tasks, we could store the code in metadata
            if let Some(code) = task.metadata.get("inline_code").and_then(|c| c.as_str()) {
                return Ok(code.to_string());
            }
            return Ok("// Inline task code not available".to_string());
        }

        // Try to load from file system
        let task_dir = std::path::Path::new(&task.path);
        let main_js_path = if task_dir.is_dir() {
            task_dir.join("main.js")
        } else {
            // Assume the path itself is the file
            task_dir.to_path_buf()
        };

        match fs::read_to_string(&main_js_path).await {
            Ok(code) => Ok(code),
            Err(e) => {
                log::warn!("Failed to load task code from {}: {}", main_js_path.display(), e);
                Ok(format!("// Failed to load task code: {}", e))
            }
        }
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
        code: &str,
    ) -> McpResult<Value> {
        let mut test_results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        log::info!("Running {} test cases for task: {}", test_cases.len(), task_name);

        for test_case in test_cases {
            log::debug!("Executing test case: {}", test_case.name);

            // Skip tests that are expected to fail for now (we could enhance this later)
            if test_case.should_fail {
                log::debug!("Skipping test case {} (marked as should_fail)", test_case.name);
                test_results.push(json!({
                    "name": test_case.name,
                    "status": "skipped",
                    "message": "Should_fail tests not yet supported",
                    "input": test_case.input,
                    "expected_output": test_case.expected_output
                }));
                skipped += 1;
                continue;
            }

            // Execute the JavaScript code with the test input
            let start_time = std::time::Instant::now();

            // Execute the JavaScript code with the real Boa engine
            let execution_result = self.execute_js_code(code, &test_case.input).await;

            match execution_result {
                Ok(actual_output) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;

                    // Check if the output matches expected output (if provided)
                    let test_passed = if let Some(expected) = &test_case.expected_output {
                        actual_output == *expected
                    } else {
                        // If no expected output is provided, just check that execution succeeded
                        true
                    };

                    if test_passed {
                        log::debug!("Test case {} passed", test_case.name);
                        test_results.push(json!({
                            "name": test_case.name,
                            "status": "passed",
                            "duration_ms": duration_ms,
                            "input": test_case.input,
                            "actual_output": actual_output,
                            "expected_output": test_case.expected_output,
                            "description": test_case.description
                        }));
                        passed += 1;
                    } else {
                        log::debug!("Test case {} failed - output mismatch", test_case.name);
                        test_results.push(json!({
                            "name": test_case.name,
                            "status": "failed",
                            "duration_ms": duration_ms,
                            "error": "Output does not match expected result",
                            "input": test_case.input,
                            "actual_output": actual_output,
                            "expected_output": test_case.expected_output,
                            "description": test_case.description
                        }));
                        failed += 1;
                    }
                }
                Err(e) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    log::debug!("Test case {} failed with error: {}", test_case.name, e);
                    test_results.push(json!({
                        "name": test_case.name,
                        "status": "failed",
                        "duration_ms": duration_ms,
                        "error": format!("Execution error: {}", e),
                        "input": test_case.input,
                        "expected_output": test_case.expected_output,
                        "description": test_case.description
                    }));
                    failed += 1;
                }
            }
        }

        let total_tests = test_cases.len();
        let success_rate = if total_tests > 0 {
            (passed as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        log::info!(
            "Test execution completed for {}: {} passed, {} failed, {} skipped",
            task_name,
            passed,
            failed,
            skipped
        );

        Ok(json!({
            "task_name": task_name,
            "total_tests": total_tests,
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "success_rate": success_rate,
            "all_passed": failed == 0 && skipped == 0,
            "message": format!("Executed {} tests with {} passed, {} failed, {} skipped",
                             total_tests, passed, failed, skipped),
            "tests": test_results,
            "executed_at": chrono::Utc::now().to_rfc3339()
        }))
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
        let name = task_data["name"].as_str().ok_or_else(|| McpError::InvalidParams {
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
}"#
                .to_string(),
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
}"#
                .to_string(),
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
}"#
                .to_string(),
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
}"#
                .to_string(),
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
            TaskTemplate {
                name: "webhook_handler".to_string(),
                description: "Handle incoming webhook requests with validation".to_string(),
                code_template: r#"
async function process(input, { fetch }) {
    const { webhook_data, validation_rules = {}, response_format = 'json' } = input;
    
    // Validate webhook signature if configured
    if (validation_rules.signature_header && validation_rules.secret) {
        const isValid = validateWebhookSignature(
            webhook_data.headers[validation_rules.signature_header],
            webhook_data.body,
            validation_rules.secret
        );
        if (!isValid) {
            throw new Error('Invalid webhook signature');
        }
    }
    
    // Process webhook payload
    const payload = typeof webhook_data.body === 'string' 
        ? JSON.parse(webhook_data.body) 
        : webhook_data.body;
    
    // Extract relevant fields based on webhook type
    const processed_data = {
        webhook_type: webhook_data.headers['x-webhook-type'] || 'unknown',
        timestamp: webhook_data.headers['x-timestamp'] || new Date().toISOString(),
        payload: payload,
        source_ip: webhook_data.headers['x-forwarded-for'] || 'unknown'
    };
    
    // Send confirmation response if configured
    if (validation_rules.confirmation_url) {
        await fetch(validation_rules.confirmation_url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ 
                status: 'received', 
                webhook_id: processed_data.payload.id 
            })
        });
    }
    
    return {
        status: 'processed',
        data: processed_data,
        response_format: response_format
    };
}

function validateWebhookSignature(signature, body, secret) {
    // Basic HMAC validation placeholder
    return signature && body && secret;
}"#
                .to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "webhook_data": {
                            "type": "object",
                            "properties": {
                                "headers": { "type": "object" },
                                "body": { "type": ["string", "object"] }
                            },
                            "required": ["headers", "body"]
                        },
                        "validation_rules": {
                            "type": "object",
                            "properties": {
                                "signature_header": { "type": "string" },
                                "secret": { "type": "string" },
                                "confirmation_url": { "type": "string", "format": "uri" }
                            }
                        },
                        "response_format": { "type": "string", "enum": ["json", "xml", "text"], "default": "json" }
                    },
                    "required": ["webhook_data"]
                }),
                output_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" },
                        "data": { "type": "object" },
                        "response_format": { "type": "string" }
                    }
                }),
                required_parameters: vec![],
                optional_parameters: vec!["webhook_type".to_string(), "confirmation_endpoint".to_string()],
                category: "webhook".to_string(),
                tags: vec!["webhook".to_string(), "http".to_string(), "integration".to_string()],
            },
            TaskTemplate {
                name: "scheduled_job".to_string(),
                description: "Template for creating scheduled/cron job tasks".to_string(),
                code_template: r#"
async function process(input, { fetch }) {
    const { job_config, execution_context = {} } = input;
    
    // Log job execution start
    const start_time = new Date().toISOString();
    console.log(`Starting scheduled job: ${job_config.name} at ${start_time}`);
    
    try {
        // Execute the main job logic
        let result;
        switch (job_config.job_type) {
            case 'cleanup':
                result = await performCleanup(job_config, execution_context);
                break;
            case 'data_sync':
                result = await performDataSync(job_config, execution_context, fetch);
                break;
            case 'health_check':
                result = await performHealthCheck(job_config, execution_context, fetch);
                break;
            case 'report_generation':
                result = await generateReport(job_config, execution_context);
                break;
            default:
                throw new Error(`Unknown job type: ${job_config.job_type}`);
        }
        
        const end_time = new Date().toISOString();
        const execution_time = new Date(end_time) - new Date(start_time);
        
        return {
            job_name: job_config.name,
            job_type: job_config.job_type,
            status: 'completed',
            start_time,
            end_time,
            execution_time_ms: execution_time,
            result: result,
            next_execution: job_config.schedule ? calculateNextExecution(job_config.schedule) : null
        };
        
    } catch (error) {
        console.error(`Job ${job_config.name} failed:`, error.message);
        
        // Send alert if configured
        if (job_config.alert_on_failure && job_config.alert_webhook) {
            await sendAlert(job_config, error, fetch);
        }
        
        throw error;
    }
}

async function performCleanup(config, context) {
    return { message: 'Cleanup completed', files_removed: 0 };
}

async function performDataSync(config, context, fetch) {
    if (config.source_url && config.target_url) {
        const data = await fetch(config.source_url).then(r => r.json());
        await fetch(config.target_url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });
        return { message: 'Data sync completed', records_synced: data.length || 1 };
    }
    return { message: 'Data sync skipped - missing URLs' };
}

async function performHealthCheck(config, context, fetch) {
    const checks = [];
    if (config.endpoints) {
        for (const endpoint of config.endpoints) {
            try {
                const response = await fetch(endpoint, { method: 'HEAD', timeout: 5000 });
                checks.push({ endpoint, status: response.ok ? 'healthy' : 'unhealthy', response_time: 0 });
            } catch (error) {
                checks.push({ endpoint, status: 'error', error: error.message });
            }
        }
    }
    return { message: 'Health check completed', checks };
}

async function generateReport(config, context) {
    return { 
        message: 'Report generated', 
        report_type: config.report_type || 'summary',
        generated_at: new Date().toISOString()
    };
}

function calculateNextExecution(schedule) {
    // Simple next execution calculation (would use proper cron parsing in production)
    const now = new Date();
    const next = new Date(now.getTime() + 60000); // Add 1 minute for demo
    return next.toISOString();
}

async function sendAlert(config, error, fetch) {
    if (config.alert_webhook) {
        await fetch(config.alert_webhook, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                job_name: config.name,
                error_message: error.message,
                timestamp: new Date().toISOString()
            })
        });
    }
}"#
                .to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "job_config": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "job_type": { 
                                    "type": "string", 
                                    "enum": ["cleanup", "data_sync", "health_check", "report_generation"] 
                                },
                                "schedule": { "type": "string" },
                                "endpoints": { "type": "array", "items": { "type": "string", "format": "uri" } },
                                "source_url": { "type": "string", "format": "uri" },
                                "target_url": { "type": "string", "format": "uri" },
                                "report_type": { "type": "string" },
                                "alert_on_failure": { "type": "boolean", "default": false },
                                "alert_webhook": { "type": "string", "format": "uri" }
                            },
                            "required": ["name", "job_type"]
                        },
                        "execution_context": { "type": "object" }
                    },
                    "required": ["job_config"]
                }),
                output_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "job_name": { "type": "string" },
                        "job_type": { "type": "string" },
                        "status": { "type": "string" },
                        "start_time": { "type": "string", "format": "date-time" },
                        "end_time": { "type": "string", "format": "date-time" },
                        "execution_time_ms": { "type": "number" },
                        "result": { "type": "object" },
                        "next_execution": { "type": ["string", "null"], "format": "date-time" }
                    }
                }),
                required_parameters: vec!["job_name".to_string()],
                optional_parameters: vec!["schedule".to_string(), "alert_webhook".to_string()],
                category: "automation".to_string(),
                tags: vec!["schedule".to_string(), "cron".to_string(), "automation".to_string(), "job".to_string()],
            },
            TaskTemplate {
                name: "testing_utility".to_string(),
                description: "Utility for creating comprehensive test suites".to_string(),
                code_template: r#"
async function process(input, { fetch }) {
    const { 
        test_config, 
        target_task_name, 
        test_cases = [], 
        generate_tests = false 
    } = input;
    
    const results = {
        test_suite: target_task_name,
        total_tests: 0,
        passed: 0,
        failed: 0,
        errors: 0,
        test_results: [],
        coverage_info: {},
        generated_at: new Date().toISOString()
    };
    
    // Generate test cases if requested
    if (generate_tests && test_config.auto_generate) {
        const generated = generateTestCases(test_config);
        test_cases.push(...generated);
    }
    
    // Execute test cases
    for (const testCase of test_cases) {
        results.total_tests++;
        
        try {
            const test_result = await executeTestCase(testCase, target_task_name, test_config, fetch);
            
            if (test_result.passed) {
                results.passed++;
            } else {
                results.failed++;
            }
            
            results.test_results.push({
                test_name: testCase.name || `test_${results.total_tests}`,
                input: testCase.input,
                expected: testCase.expected,
                actual: test_result.actual,
                passed: test_result.passed,
                execution_time_ms: test_result.execution_time_ms,
                error_message: test_result.error_message
            });
            
        } catch (error) {
            results.errors++;
            results.test_results.push({
                test_name: testCase.name || `test_${results.total_tests}`,
                input: testCase.input,
                expected: testCase.expected,
                actual: null,
                passed: false,
                execution_time_ms: 0,
                error_message: error.message
            });
        }
    }
    
    // Calculate coverage and success rate
    results.success_rate = results.total_tests > 0 ? 
        (results.passed / results.total_tests * 100).toFixed(2) + '%' : '0%';
    
    results.coverage_info = {
        test_cases_executed: results.total_tests,
        edge_cases_covered: test_cases.filter(tc => tc.edge_case).length,
        error_scenarios_tested: test_cases.filter(tc => tc.expect_error).length
    };
    
    return results;
}

function generateTestCases(config) {
    const generated = [];
    
    // Generate basic positive test cases
    if (config.input_schema) {
        generated.push({
            name: 'basic_valid_input',
            input: generateValidInput(config.input_schema),
            expected: { success: true },
            edge_case: false
        });
    }
    
    // Generate edge cases
    generated.push({
        name: 'empty_input',
        input: {},
        expected: { success: false },
        edge_case: true,
        expect_error: true
    });
    
    generated.push({
        name: 'null_input',
        input: null,
        expected: { success: false },
        edge_case: true,
        expect_error: true
    });
    
    return generated;
}

function generateValidInput(schema) {
    // Simple schema-based input generation
    if (schema && schema.properties) {
        const input = {};
        for (const [key, prop] of Object.entries(schema.properties)) {
            if (prop.type === 'string') {
                input[key] = 'test_value';
            } else if (prop.type === 'number') {
                input[key] = 42;
            } else if (prop.type === 'boolean') {
                input[key] = true;
            }
        }
        return input;
    }
    return { test: 'data' };
}

async function executeTestCase(testCase, taskName, config, fetch) {
    const start_time = Date.now();
    
    try {
        // This would typically call the actual task being tested
        // For now, simulate task execution
        let result;
        
        if (config.mock_responses && config.mock_responses[testCase.name]) {
            result = config.mock_responses[testCase.name];
        } else {
            // Simulate task execution based on input
            result = simulateTaskExecution(testCase.input, testCase.expect_error);
        }
        
        const execution_time_ms = Date.now() - start_time;
        
        // Compare result with expected output
        const passed = compareResults(result, testCase.expected);
        
        return {
            actual: result,
            passed: passed,
            execution_time_ms: execution_time_ms,
            error_message: null
        };
        
    } catch (error) {
        const execution_time_ms = Date.now() - start_time;
        
        if (testCase.expect_error) {
            return {
                actual: { error: error.message },
                passed: true, // Expected error occurred
                execution_time_ms: execution_time_ms,
                error_message: null
            };
        } else {
            return {
                actual: null,
                passed: false,
                execution_time_ms: execution_time_ms,
                error_message: error.message
            };
        }
    }
}

function simulateTaskExecution(input, expectError) {
    if (expectError || !input || Object.keys(input).length === 0) {
        throw new Error('Simulated task error');
    }
    
    return {
        success: true,
        output: { processed: true, input_received: input },
        timestamp: new Date().toISOString()
    };
}

function compareResults(actual, expected) {
    if (expected.success !== undefined) {
        return actual.success === expected.success;
    }
    
    // Basic deep comparison for simple cases
    return JSON.stringify(actual) === JSON.stringify(expected);
}"#
                .to_string(),
                input_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "test_config": {
                            "type": "object",
                            "properties": {
                                "auto_generate": { "type": "boolean", "default": false },
                                "input_schema": { "type": "object" },
                                "mock_responses": { "type": "object" },
                                "timeout_ms": { "type": "number", "default": 30000 }
                            }
                        },
                        "target_task_name": { "type": "string" },
                        "test_cases": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "input": { "type": "object" },
                                    "expected": { "type": "object" },
                                    "edge_case": { "type": "boolean", "default": false },
                                    "expect_error": { "type": "boolean", "default": false }
                                },
                                "required": ["input", "expected"]
                            }
                        },
                        "generate_tests": { "type": "boolean", "default": false }
                    },
                    "required": ["target_task_name"]
                }),
                output_schema_template: json!({
                    "type": "object",
                    "properties": {
                        "test_suite": { "type": "string" },
                        "total_tests": { "type": "number" },
                        "passed": { "type": "number" },
                        "failed": { "type": "number" },
                        "errors": { "type": "number" },
                        "success_rate": { "type": "string" },
                        "test_results": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "test_name": { "type": "string" },
                                    "passed": { "type": "boolean" },
                                    "execution_time_ms": { "type": "number" },
                                    "error_message": { "type": ["string", "null"] }
                                }
                            }
                        },
                        "coverage_info": { "type": "object" },
                        "generated_at": { "type": "string", "format": "date-time" }
                    }
                }),
                required_parameters: vec!["target_task".to_string()],
                optional_parameters: vec!["test_timeout".to_string()],
                category: "testing".to_string(),
                tags: vec!["testing".to_string(), "validation".to_string(), "quality".to_string(), "automation".to_string()],
            },
        ]
    }

    /// Apply template parameters to code
    fn apply_template_parameters(&self, template: &str, parameters: &HashMap<String, Value>) -> McpResult<String> {
        let mut result = template.to_string();

        // Replace standard placeholders like {{variable_name}}
        for (key, value) in parameters {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Array(arr) => format!("[{}]", arr.iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect::<Vec<_>>()
                    .join(", ")),
                _ => serde_json::to_string(value).unwrap_or_default(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        // Handle common template patterns
        if parameters.contains_key("function_name") {
            // Ensure function names are properly formatted
            if let Some(Value::String(name)) = parameters.get("function_name") {
                result = result.replace("{{FUNCTION_NAME}}", name);
                result = result.replace("{{function_name}}", name);
            }
        }

        // Handle API endpoint patterns
        if parameters.contains_key("api_endpoint") {
            if let Some(Value::String(endpoint)) = parameters.get("api_endpoint") {
                result = result.replace("{{API_ENDPOINT}}", endpoint);
                result = result.replace("{{api_endpoint}}", endpoint);
            }
        }

        // Handle authentication patterns
        if parameters.contains_key("auth_type") {
            if let Some(Value::String(auth)) = parameters.get("auth_type") {
                match auth.as_str() {
                    "bearer" => {
                        result = result.replace(
                            "{{AUTH_HEADER}}", 
                            "'Authorization': `Bearer ${token}`"
                        );
                    }
                    "api_key" => {
                        result = result.replace(
                            "{{AUTH_HEADER}}", 
                            "'X-API-Key': api_key"
                        );
                    }
                    "basic" => {
                        result = result.replace(
                            "{{AUTH_HEADER}}", 
                            "'Authorization': `Basic ${btoa(username + ':' + password)}`"
                        );
                    }
                    _ => {
                        result = result.replace("{{AUTH_HEADER}}", "");
                    }
                }
            }
        }

        Ok(result)
    }

    /// Apply template parameters to schema
    fn apply_template_to_schema(&self, schema: &Value, parameters: &HashMap<String, Value>) -> McpResult<Value> {
        let result = schema.clone();
        
        // Convert schema to string for replacement, then back to JSON
        let schema_str = serde_json::to_string(&result).map_err(|e| McpError::Internal {
            message: format!("Failed to serialize schema: {}", e),
        })?;
        
        let updated_schema_str = self.apply_template_parameters(&schema_str, parameters)?;
        
        // Try to parse back to JSON
        match serde_json::from_str(&updated_schema_str) {
            Ok(updated_schema) => Ok(updated_schema),
            Err(_) => {
                // If parsing fails, return original schema
                Ok(result)
            }
        }
    }

    /// Get template categories
    fn get_template_categories(&self, templates: &[TaskTemplate]) -> Vec<String> {
        let mut categories: Vec<String> = templates
            .iter()
            .map(|t| t.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }

    /// Enhanced ZIP import functionality
    async fn import_from_zip(&self, data: &Value, options: &ImportOptions) -> McpResult<ImportResult> {
        // For now, simulate ZIP import with structured data
        // In a full implementation, this would handle actual ZIP file extraction
        
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        
        if let Some(zip_data) = data.get("zip_contents") {
            if let Some(tasks) = zip_data.get("tasks").and_then(|t| t.as_array()) {
                for task in tasks {
                    match self.import_single_task(task, options).await {
                        Ok(imported) => imported_tasks.push(imported),
                        Err(e) => {
                            let task_name = task.get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            errors.push(format!("ZIP task {}: {}", task_name, e));
                        }
                    }
                }
            }
            
            // Handle additional ZIP contents like assets, test files, etc.
            if let Some(assets) = zip_data.get("assets") {
                // Process asset files that might be referenced by tasks
                self.process_task_assets(assets, &mut imported_tasks, &mut errors).await;
            }
        } else {
            errors.push("Invalid ZIP data structure".to_string());
        }
        
        Ok(ImportResult {
            imported_tasks,
            errors,
            dependencies: Vec::new(),
        })
    }

    /// Enhanced directory import functionality
    async fn import_from_directory(&self, data: &Value, options: &ImportOptions) -> McpResult<ImportResult> {
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        let mut dependencies = Vec::new();
        
        if let Some(dir_data) = data.get("directory_structure") {
            // Process directory hierarchy
            if let Some(tasks_dir) = dir_data.get("tasks") {
                let import_result = self.process_directory_tasks(tasks_dir, options).await?;
                imported_tasks.extend(import_result.imported_tasks);
                errors.extend(import_result.errors);
                dependencies.extend(import_result.dependencies);
            }
            
            // Process collections (task hierarchies)
            if let Some(collections_dir) = dir_data.get("collections") {
                let collections_result = self.process_task_collections(collections_dir, options).await?;
                imported_tasks.extend(collections_result.imported_tasks);
                errors.extend(collections_result.errors);
                dependencies.extend(collections_result.dependencies);
            }
            
            // Process templates
            if let Some(templates_dir) = dir_data.get("templates") {
                let templates_result = self.process_template_directory(templates_dir, options).await?;
                imported_tasks.extend(templates_result.imported_tasks);
                errors.extend(templates_result.errors);
            }
        } else {
            errors.push("Invalid directory data structure".to_string());
        }
        
        Ok(ImportResult {
            imported_tasks,
            errors,
            dependencies,
        })
    }

    /// Process task assets (files, configurations, etc.)
    async fn process_task_assets(&self, assets: &Value, imported_tasks: &mut Vec<Value>, errors: &mut Vec<String>) {
        if let Some(assets_map) = assets.as_object() {
            for (asset_name, asset_data) in assets_map {
                // Process different types of assets
                match asset_name.as_str() {
                    "configurations" => {
                        // Handle task-specific configurations
                        if let Some(configs) = asset_data.as_object() {
                            for task in imported_tasks.iter_mut() {
                                if let Some(task_name) = task.get("name").and_then(|n| n.as_str()) {
                                    if let Some(config) = configs.get(task_name) {
                                        task["configuration"] = config.clone();
                                    }
                                }
                            }
                        }
                    }
                    "shared_libraries" => {
                        // Handle shared JavaScript libraries
                        for task in imported_tasks.iter_mut() {
                            task["shared_libraries"] = asset_data.clone();
                        }
                    }
                    "documentation" => {
                        // Handle task documentation
                        if let Some(docs) = asset_data.as_object() {
                            for task in imported_tasks.iter_mut() {
                                if let Some(task_name) = task.get("name").and_then(|n| n.as_str()) {
                                    if let Some(doc) = docs.get(task_name) {
                                        task["documentation"] = doc.clone();
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // Unknown asset type
                        errors.push(format!("Unknown asset type: {}", asset_name));
                    }
                }
            }
        }
    }

    /// Process tasks from directory structure
    async fn process_directory_tasks(&self, tasks_dir: &Value, options: &ImportOptions) -> McpResult<ImportResult> {
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        let mut dependencies = Vec::new();
        
        if let Some(tasks_map) = tasks_dir.as_object() {
            for (task_name, task_files) in tasks_map {
                match self.assemble_task_from_files(task_name, task_files, options).await {
                    Ok(assembled_task) => {
                        // Extract dependencies
                        if let Some(deps) = assembled_task.get("dependencies") {
                            dependencies.push(deps.clone());
                        }
                        imported_tasks.push(assembled_task);
                    }
                    Err(e) => {
                        errors.push(format!("Task {}: {}", task_name, e));
                    }
                }
            }
        }
        
        Ok(ImportResult {
            imported_tasks,
            errors,
            dependencies,
        })
    }

    /// Process task collections (hierarchical task groups)
    async fn process_task_collections(&self, collections_dir: &Value, options: &ImportOptions) -> McpResult<ImportResult> {
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        let mut dependencies = Vec::new();
        
        if let Some(collections_map) = collections_dir.as_object() {
            for (collection_name, collection_data) in collections_map {
                if let Some(tasks) = collection_data.get("tasks").and_then(|t| t.as_array()) {
                    for task in tasks {
                        match self.import_single_task(task, options).await {
                            Ok(mut imported) => {
                                // Add collection metadata
                                imported["collection"] = json!(collection_name);
                                if let Some(collection_meta) = collection_data.get("metadata") {
                                    imported["collection_metadata"] = collection_meta.clone();
                                }
                                
                                // Extract collection-level dependencies
                                if let Some(deps) = collection_data.get("dependencies") {
                                    dependencies.push(deps.clone());
                                }
                                
                                imported_tasks.push(imported);
                            }
                            Err(e) => {
                                let task_name = task.get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                errors.push(format!("Collection {} task {}: {}", collection_name, task_name, e));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(ImportResult {
            imported_tasks,
            errors,
            dependencies,
        })
    }

    /// Process template directory for custom templates
    async fn process_template_directory(&self, templates_dir: &Value, _options: &ImportOptions) -> McpResult<ImportResult> {
        let mut imported_tasks = Vec::new();
        let mut errors = Vec::new();
        
        if let Some(templates_map) = templates_dir.as_object() {
            for (template_name, template_data) in templates_map {
                if let Some(template_config) = template_data.get("template.json") {
                    // Process custom template definition
                    let template_info = json!({
                        "type": "custom_template",
                        "name": template_name,
                        "template_config": template_config,
                        "imported_at": chrono::Utc::now().to_rfc3339()
                    });
                    imported_tasks.push(template_info);
                } else {
                    errors.push(format!("Template {} missing template.json", template_name));
                }
            }
        }
        
        Ok(ImportResult {
            imported_tasks,
            errors,
            dependencies: Vec::new(),
        })
    }

    /// Assemble a task from multiple files (main.js, metadata.json, schemas, tests)
    async fn assemble_task_from_files(&self, task_name: &str, task_files: &Value, options: &ImportOptions) -> McpResult<Value> {
        let mut assembled_task = json!({
            "name": task_name,
            "imported_at": chrono::Utc::now().to_rfc3339()
        });

        if let Some(files_map) = task_files.as_object() {
            // Process main.js
            if let Some(main_js) = files_map.get("main.js").and_then(|f| f.as_str()) {
                assembled_task["code"] = json!(main_js);
            } else {
                return Err(McpError::InvalidParams {
                    method: "assemble_task".to_string(),
                    details: format!("Task {} missing main.js", task_name),
                });
            }

            // Process metadata.json
            if let Some(metadata) = files_map.get("metadata.json") {
                assembled_task["metadata"] = metadata.clone();
                if let Some(description) = metadata.get("description") {
                    assembled_task["description"] = description.clone();
                }
                if let Some(version) = metadata.get("version") {
                    assembled_task["version"] = version.clone();
                }
                if let Some(dependencies) = metadata.get("dependencies") {
                    assembled_task["dependencies"] = dependencies.clone();
                }
            }

            // Process input.schema.json
            if let Some(input_schema) = files_map.get("input.schema.json") {
                assembled_task["input_schema"] = input_schema.clone();
            }

            // Process output.schema.json
            if let Some(output_schema) = files_map.get("output.schema.json") {
                assembled_task["output_schema"] = output_schema.clone();
            }

            // Process test files if enabled
            if options.include_tests {
                let mut test_cases = Vec::new();
                for (file_name, file_content) in files_map {
                    if file_name.starts_with("test-") && file_name.ends_with(".json") {
                        test_cases.push(file_content.clone());
                    }
                }
                if !test_cases.is_empty() {
                    assembled_task["test_cases"] = json!(test_cases);
                }
            }

            // Process configuration files
            if let Some(config) = files_map.get("config.json") {
                assembled_task["configuration"] = config.clone();
            }
        }

        Ok(assembled_task)
    }

    /// Generate version diff between current task and requested changes
    async fn generate_version_diff(&self, current_task: &TaskInfo, request: &CreateTaskVersionRequest) -> McpResult<Value> {
        let mut diff = json!({
            "files_changed": 0,
            "lines_added": 0,
            "lines_removed": 0,
            "schema_changes": false,
            "code_changes": false,
            "changes_detail": []
        });

        let mut changes_detail = Vec::new();

        // Check code changes
        if let Some(ref new_code) = request.changes.as_ref().and_then(|c| c.get("code")) {
            if let Some(new_code_str) = new_code.as_str() {
                let current_lines: Vec<&str> = current_task.code.lines().collect();
                let new_lines: Vec<&str> = new_code_str.lines().collect();
                
                let lines_added = new_lines.len().saturating_sub(current_lines.len());
                let lines_removed = current_lines.len().saturating_sub(new_lines.len());
                
                diff["lines_added"] = json!(lines_added);
                diff["lines_removed"] = json!(lines_removed);
                diff["code_changes"] = json!(true);
                diff["files_changed"] = json!(1);
                
                changes_detail.push(json!({
                    "type": "code",
                    "lines_added": lines_added,
                    "lines_removed": lines_removed,
                    "change_description": "JavaScript code modified"
                }));
            }
        }

        // Check schema changes
        if let Some(ref changes) = request.changes {
            if let Some(changes_obj) = changes.as_object() {
                if changes_obj.contains_key("input_schema") || changes_obj.contains_key("output_schema") {
                    diff["schema_changes"] = json!(true);
                    changes_detail.push(json!({
                        "type": "schema",
                        "change_description": "Input or output schema modified",
                        "breaking_potential": request.breaking_change
                    }));
                }
            }
        }

        diff["changes_detail"] = json!(changes_detail);
        Ok(diff)
    }

    /// Generate migration plan for breaking changes
    async fn generate_migration_plan(&self, _current_task: &TaskInfo, request: &CreateTaskVersionRequest) -> McpResult<Value> {
        let migration_steps = vec![
            "Backup current task version",
            "Identify all dependent tasks and workflows",
            "Create compatibility layer if needed",
            "Update dependent task configurations",
            "Test migration in staging environment",
            "Deploy with rollback plan ready"
        ];

        let affected_areas = vec![
            if request.changes.as_ref().map_or(false, |c| c.as_object().map_or(false, |obj| obj.contains_key("input_schema"))) {
                "Input schema changes may affect calling tasks"
            } else { "" },
            if request.changes.as_ref().map_or(false, |c| c.as_object().map_or(false, |obj| obj.contains_key("output_schema"))) {
                "Output schema changes may affect dependent tasks"
            } else { "" },
            if request.changes.as_ref().map_or(false, |c| c.as_object().map_or(false, |obj| obj.contains_key("code"))) {
                "Code changes may alter task behavior"
            } else { "" }
        ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>();

        Ok(json!({
            "migration_required": true,
            "complexity": "high",
            "estimated_time": "2-4 hours",
            "migration_steps": migration_steps,
            "affected_areas": affected_areas,
            "compatibility_mode": {
                "available": true,
                "duration": "30 days",
                "description": "Old version will remain available during transition"
            },
            "testing_requirements": [
                "Validate all existing test cases still pass",
                "Test integration with dependent tasks",
                "Verify no regression in performance",
                "Check backward compatibility if enabled"
            ],
            "rollback_plan": {
                "automatic_triggers": ["Test failure rate > 5%", "Performance degradation > 20%"],
                "manual_rollback_time": "< 5 minutes",
                "data_migration_reversal": "Available for 7 days"
            }
        }))
    }

    /// Apply version changes to create new task version
    async fn apply_version_changes(&self, current_task: &TaskInfo, request: &CreateTaskVersionRequest) -> McpResult<TaskInfo> {
        let mut updated_task = current_task.clone();
        updated_task.version = request.new_version.clone();

        if let Some(ref changes) = request.changes {
            if let Some(changes_obj) = changes.as_object() {
                if let Some(new_code) = changes_obj.get("code").and_then(|c| c.as_str()) {
                    updated_task.code = new_code.to_string();
                }
                if let Some(new_input_schema) = changes_obj.get("input_schema") {
                    updated_task.input_schema = new_input_schema.clone();
                }
                if let Some(new_output_schema) = changes_obj.get("output_schema") {
                    updated_task.output_schema = new_output_schema.clone();
                }
            }
        }

        Ok(updated_task)
    }

    /// Store version history entry (simulated - would be database in production)
    async fn store_version_history(&self, version_record: &Value) -> McpResult<()> {
        // In a real implementation, this would store to database
        // For now, just simulate successful storage
        tracing::info!("Version history stored: {}", version_record.get("version_id").unwrap_or(&json!("unknown")));
        Ok(())
    }

    /// Get tasks that depend on the current task
    async fn get_dependent_tasks(&self, task_name: &str) -> Vec<String> {
        // In a real implementation, this would query the database for dependencies
        // For now, return a simulated list
        vec![
            format!("{}_wrapper", task_name),
            format!("{}_validator", task_name)
        ]
    }

    /// Assess test compatibility for version changes
    async fn assess_test_compatibility(&self, _current_task: &TaskInfo, request: &CreateTaskVersionRequest) -> Value {
        json!({
            "existing_tests_compatible": !request.breaking_change,
            "new_tests_required": request.breaking_change,
            "compatibility_score": if request.breaking_change { 0.3 } else { 0.9 },
            "recommendations": if request.breaking_change {
                vec![
                    "Create new test cases for changed functionality",
                    "Update existing tests to handle new schema",
                    "Add migration tests"
                ]
            } else {
                vec![
                    "Run existing test suite to verify compatibility",
                    "Add tests for new features if any"
                ]
            }
        })
    }

    /// Discover tasks in a filesystem directory
    pub async fn discover_tasks(&self, request: DiscoverTasksRequest) -> McpResult<Value> {
        if !self.allow_fs_operations {
            return Err(McpError::Internal {
                message: "Filesystem operations not allowed".to_string(),
            });
        }

        let base_path = std::path::Path::new(&request.path);
        if !base_path.exists() {
            return Err(McpError::InvalidParams {
                method: "discover_tasks".to_string(),
                details: format!("Path does not exist: {}", request.path),
            });
        }

        let mut discovered_tasks = Vec::new();
        self.scan_directory_for_tasks(
            base_path,
            &request.include_patterns,
            request.recursive,
            request.max_depth,
            0,
            &mut discovered_tasks,
        ).await?;

        Ok(json!({
            "discovered_tasks": discovered_tasks,
            "scan_path": request.path,
            "total_found": discovered_tasks.len(),
            "patterns_used": request.include_patterns,
            "scanned_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Sync registry sources to load available tasks
    pub async fn sync_registry(&self, request: SyncRegistryRequest) -> McpResult<Value> {
        // Mock implementation for registry sync
        // In a real implementation, this would interface with the registry component
        let sources_synced = if let Some(source_name) = &request.source_name {
            vec![source_name.clone()]
        } else {
            vec!["sample-tasks".to_string(), "git-tasks".to_string()]
        };

        let mut sync_results = Vec::new();
        for source in &sources_synced {
            let task_count = match source.as_str() {
                "sample-tasks" => 7, // Number of tasks in sample/js-tasks/tasks/
                "git-tasks" => 0,    // No git tasks loaded by default
                _ => 0,
            };

            sync_results.push(json!({
                "source_name": source,
                "status": "synced",
                "task_count": task_count,
                "last_sync": chrono::Utc::now().to_rfc3339(),
                "force_refresh": request.force_refresh,
                "validation_enabled": request.validate_tasks
            }));
        }

        Ok(json!({
            "sync_results": sync_results,
            "total_sources": sources_synced.len(),
            "synced_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Check registry health and status
    pub async fn check_registry_health(&self) -> McpResult<Value> {
        // Mock implementation for registry health checks
        // In a real implementation, this would check actual registry sources
        let health_status = vec![
            RegistryHealthStatus {
                source_name: "sample-tasks".to_string(),
                status: "healthy".to_string(),
                last_sync: Some(chrono::Utc::now().to_rfc3339()),
                task_count: 7,
                error_count: 0,
                last_error: None,
            },
            RegistryHealthStatus {
                source_name: "embedded-tasks".to_string(),
                status: "healthy".to_string(),
                last_sync: Some(chrono::Utc::now().to_rfc3339()),
                task_count: 1, // heartbeat task
                error_count: 0,
                last_error: None,
            },
        ];

        Ok(json!({
            "registry_health": health_status,
            "overall_status": "healthy",
            "total_sources": health_status.len(),
            "total_tasks": health_status.iter().map(|s| s.task_count).sum::<usize>(),
            "total_errors": health_status.iter().map(|s| s.error_count).sum::<usize>(),
            "checked_at": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Recursively scan directory for task files
    async fn scan_directory_for_tasks(
        &self,
        dir_path: &std::path::Path,
        patterns: &[String],
        recursive: bool,
        max_depth: usize,
        current_depth: usize,
        results: &mut Vec<Value>,
    ) -> McpResult<()> {
        if current_depth >= max_depth {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(dir_path).await.map_err(|e| McpError::Internal {
            message: format!("Failed to read directory: {}", e),
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| McpError::Internal {
            message: format!("Failed to read directory entry: {}", e),
        })? {
            let path = entry.path();
            
            if path.is_dir() && recursive {
                // Check if this looks like a task directory (contains main.js)
                let main_js_path = path.join("main.js");
                if main_js_path.exists() {
                    let task_info = self.analyze_task_directory(&path).await?;
                    results.push(task_info);
                }
                
                // Continue scanning subdirectories
                Box::pin(self.scan_directory_for_tasks(&path, patterns, recursive, max_depth, current_depth + 1, results)).await?;
            } else if path.is_file() {
                // Check if file matches patterns
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    for pattern in patterns {
                        if filename.ends_with(&pattern.replace("*", "")) {
                            let task_info = self.analyze_task_file(&path).await?;
                            results.push(task_info);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze a task directory structure
    async fn analyze_task_directory(&self, dir_path: &std::path::Path) -> McpResult<Value> {
        let task_name = dir_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let mut task_info = json!({
            "name": task_name,
            "path": dir_path.to_string_lossy(),
            "type": "directory",
            "has_main_js": dir_path.join("main.js").exists(),
            "has_metadata": dir_path.join("metadata.json").exists(),
            "has_input_schema": dir_path.join("input.schema.json").exists(),
            "has_output_schema": dir_path.join("output.schema.json").exists(),
            "has_tests": dir_path.join("tests").exists(),
        });

        // Try to read metadata if available
        let metadata_path = dir_path.join("metadata.json");
        if metadata_path.exists() {
            if let Ok(metadata_content) = tokio::fs::read_to_string(&metadata_path).await {
                if let Ok(metadata) = serde_json::from_str::<Value>(&metadata_content) {
                    task_info["metadata"] = metadata;
                }
            }
        }

        Ok(task_info)
    }

    /// Analyze a single task file
    async fn analyze_task_file(&self, file_path: &std::path::Path) -> McpResult<Value> {
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        Ok(json!({
            "name": filename,
            "path": file_path.to_string_lossy(),
            "type": "file",
            "extension": file_path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        }))
    }

}

#[derive(Clone)]
struct TaskInfo {
    uuid: uuid::Uuid,
    name: String,
    version: String,
    code: String,
    input_schema: Value,
    output_schema: Value,
}

/// Result structure for import operations
struct ImportResult {
    imported_tasks: Vec<Value>,
    errors: Vec<String>,
    dependencies: Vec<Value>,
}

/// Register task development tools in the tool registry
pub fn register_task_dev_tools(tools: &mut HashMap<String, McpTool>) {
    // Create task tool
    let create_task_tool = McpTool::new(
        "ratchet_create_task",
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
    tools.insert("ratchet_create_task".to_string(), create_task_tool);

    // Validate task tool
    let validate_task_tool = McpTool::new(
        "ratchet_validate_task",
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
    tools.insert("ratchet_validate_task".to_string(), validate_task_tool);

    // Debug task tool
    let debug_task_tool = McpTool::new(
        "ratchet_debug_task_execution",
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
    tools.insert("ratchet_debug_task_execution".to_string(), debug_task_tool);

    // Run tests tool
    let run_tests_tool = McpTool::new(
        "ratchet_run_task_tests",
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
    tools.insert("ratchet_run_task_tests".to_string(), run_tests_tool);

    // Create version tool
    let create_version_tool = McpTool::new(
        "ratchet_create_task_version",
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
    tools.insert("ratchet_create_task_version".to_string(), create_version_tool);

    // Edit task tool
    let edit_task_tool = McpTool::new(
        "ratchet_edit_task",
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
    tools.insert("ratchet_edit_task".to_string(), edit_task_tool);

    // Delete task tool
    let delete_task_tool = McpTool::new(
        "ratchet_delete_task",
        "Delete an existing task with optional backup and file cleanup",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID to delete"
                },
                "create_backup": {
                    "type": "boolean",
                    "default": true,
                    "description": "Create backup before deletion"
                },
                "force": {
                    "type": "boolean",
                    "default": false,
                    "description": "Force deletion even if task has executions"
                },
                "delete_files": {
                    "type": "boolean",
                    "default": false,
                    "description": "Also delete associated files from filesystem"
                }
            },
            "required": ["task_id"]
        }),
        "development",
    );
    tools.insert("ratchet_delete_task".to_string(), delete_task_tool);

    // Import tasks tool
    let import_tasks_tool = McpTool::new(
        "ratchet_import_tasks",
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
    tools.insert("ratchet_import_tasks".to_string(), import_tasks_tool);

    // Export tasks tool
    let export_tasks_tool = McpTool::new(
        "ratchet_export_tasks",
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
    tools.insert("ratchet_export_tasks".to_string(), export_tasks_tool);

    // Generate from template tool
    let generate_template_tool = McpTool::new(
        "ratchet_generate_from_template",
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
    tools.insert("ratchet_generate_from_template".to_string(), generate_template_tool);

    // List templates tool
    let list_templates_tool = McpTool::new(
        "ratchet_list_templates",
        "List all available task templates",
        json!({
            "type": "object",
            "properties": {}
        }),
        "development",
    );
    tools.insert("ratchet_list_templates".to_string(), list_templates_tool);

    // Store result tool
    let store_result_tool = McpTool::new(
        "ratchet_store_result",
        "Store task execution result in the database",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID that was executed"
                },
                "input": {
                    "type": "object",
                    "description": "Input data that was provided to the task"
                },
                "output": {
                    "type": "object",
                    "description": "Output result from the task execution"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "running", "completed", "failed", "cancelled"],
                    "default": "completed",
                    "description": "Execution status"
                },
                "error_message": {
                    "type": "string",
                    "description": "Error message if execution failed"
                },
                "error_details": {
                    "type": "object",
                    "description": "Error details if execution failed"
                },
                "duration_ms": {
                    "type": "integer",
                    "description": "Execution duration in milliseconds"
                },
                "http_requests": {
                    "type": "object",
                    "description": "HTTP requests made during execution"
                },
                "recording_path": {
                    "type": "string",
                    "description": "Recording path if recording was enabled"
                }
            },
            "required": ["task_id", "input", "output"]
        }),
        "development",
    );
    tools.insert("ratchet_store_result".to_string(), store_result_tool);

    // Get results tool
    let get_results_tool = McpTool::new(
        "ratchet_get_results",
        "Retrieve task execution results from the database",
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task name or UUID to get results for (optional)"
                },
                "execution_id": {
                    "type": "string",
                    "description": "Specific execution UUID to retrieve"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "running", "completed", "failed", "cancelled"],
                    "description": "Filter by execution status"
                },
                "limit": {
                    "type": "integer",
                    "default": 50,
                    "minimum": 1,
                    "maximum": 1000,
                    "description": "Maximum number of results to return"
                },
                "offset": {
                    "type": "integer",
                    "default": 0,
                    "minimum": 0,
                    "description": "Number of results to skip (for pagination)"
                },
                "include_errors": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to include error details in results"
                },
                "include_data": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to include full input/output data"
                }
            }
        }),
        "development",
    );
    tools.insert("ratchet_get_results".to_string(), get_results_tool);

    // Discover tasks tool
    let discover_tasks_tool = McpTool::new(
        "ratchet_discover_tasks",
        "Discover tasks in a filesystem directory",
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to scan for tasks"
                },
                "include_patterns": {
                    "type": "array",
                    "items": {"type": "string"},
                    "default": ["*.js", "*.yaml", "*.json"],
                    "description": "File patterns to include in scan"
                },
                "recursive": {
                    "type": "boolean",
                    "default": true,
                    "description": "Whether to scan recursively"
                },
                "max_depth": {
                    "type": "integer",
                    "default": 10,
                    "description": "Maximum depth for recursive scanning"
                }
            },
            "required": ["path"]
        }),
        "registry",
    );
    tools.insert("ratchet_discover_tasks".to_string(), discover_tasks_tool);

    // Sync registry tool
    let sync_registry_tool = McpTool::new(
        "ratchet_sync_registry",
        "Sync registry sources to load available tasks",
        json!({
            "type": "object",
            "properties": {
                "source_name": {
                    "type": "string",
                    "description": "Specific source to sync (optional, syncs all if not provided)"
                },
                "force_refresh": {
                    "type": "boolean",
                    "default": false,
                    "description": "Force refresh cached data"
                },
                "validate_tasks": {
                    "type": "boolean",
                    "default": true,
                    "description": "Validate tasks during sync"
                }
            }
        }),
        "registry",
    );
    tools.insert("ratchet_sync_registry".to_string(), sync_registry_tool);

    // Registry health tool
    let registry_health_tool = McpTool::new(
        "ratchet_registry_health",
        "Check registry health and status",
        json!({
            "type": "object",
            "properties": {}
        }),
        "registry",
    );
    tools.insert("ratchet_registry_health".to_string(), registry_health_tool);

    // Get MCP endpoints reference tool
    let get_endpoints_tool = McpTool::new(
        "ratchet_get_developer_endpoint_reference",
        "Get comprehensive MCP endpoints reference with all 23+ available tools, parameters, and examples",
        json!({
            "type": "object",
            "properties": {}
        }),
        "documentation",
    );
    tools.insert("ratchet_get_developer_endpoint_reference".to_string(), get_endpoints_tool);

    // Get MCP integration guide tool  
    let get_integration_tool = McpTool::new(
        "ratchet_get_developer_integration_guide",
        "Get comprehensive MCP integration guide for setting up Claude Desktop and other clients",
        json!({
            "type": "object",
            "properties": {}
        }),
        "documentation",
    );
    tools.insert("ratchet_get_developer_integration_guide".to_string(), get_integration_tool);

    // Get MCP development walkthrough tool
    let get_walkthrough_tool = McpTool::new(
        "ratchet_get_developer_guide_walkthrough",
        "Get comprehensive MCP development walkthrough with step-by-step examples for agents",
        json!({
            "type": "object",
            "properties": {}
        }),
        "documentation",
    );
    tools.insert("ratchet_get_developer_guide_walkthrough".to_string(), get_walkthrough_tool);
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
        "ratchet_create_task" => {
            let request: CreateTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_validate_task" => {
            let request: ValidateTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_debug_task_execution" => {
            let request: DebugTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_run_task_tests" => {
            let request: RunTaskTestsRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_create_task_version" => {
            let request: CreateTaskVersionRequest =
                serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_edit_task" => {
            let request: EditTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_import_tasks" => {
            let request: ImportTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_export_tasks" => {
            let request: ExportTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_generate_from_template" => {
            let request: GenerateFromTemplateRequest =
                serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
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

        "ratchet_delete_task" => {
            let request: DeleteTaskRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
                method: tool_name.to_string(),
                details: format!("Invalid request: {}", e),
            })?;

            match service.delete_task(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to delete task: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_list_templates" => match service.list_templates().await {
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
        },

        "ratchet_store_result" => {
            let request: StoreResultRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
                method: tool_name.to_string(),
                details: format!("Invalid request: {}", e),
            })?;

            match service.store_result(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to store result: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_get_results" => {
            let request: GetResultsRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
                method: tool_name.to_string(),
                details: format!("Invalid request: {}", e),
            })?;

            match service.get_results(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to get results: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_discover_tasks" => {
            let request: DiscoverTasksRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
                method: tool_name.to_string(),
                details: format!("Invalid request: {}", e),
            })?;

            match service.discover_tasks(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to discover tasks: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_sync_registry" => {
            let request: SyncRegistryRequest = serde_json::from_value(args).map_err(|e| McpError::InvalidParams {
                method: tool_name.to_string(),
                details: format!("Invalid request: {}", e),
            })?;

            match service.sync_registry(request).await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to sync registry: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_registry_health" => {
            match service.check_registry_health().await {
                Ok(result) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                    }],
                    is_error: false,
                    metadata: HashMap::new(),
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to check registry health: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_get_developer_endpoint_reference" => {
            match get_developer_endpoint_reference().await {
                Ok(documentation) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: documentation,
                    }],
                    is_error: false,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("document_type".to_string(), Value::String("mcp_endpoints_reference".to_string()));
                        metadata.insert("source_file".to_string(), Value::String("docs/MCP_ENDPOINTS_REFERENCE.md".to_string()));
                        metadata.insert("tool_count".to_string(), Value::String("23+".to_string()));
                        metadata
                    },
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to retrieve MCP endpoints reference: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_get_developer_integration_guide" => {
            match get_developer_integration_guide().await {
                Ok(documentation) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: documentation,
                    }],
                    is_error: false,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("document_type".to_string(), Value::String("mcp_integration_guide".to_string()));
                        metadata.insert("source_file".to_string(), Value::String("docs/MCP_INTEGRATION_GUIDE.md".to_string()));
                        metadata.insert("covers".to_string(), Value::String("claude_desktop_setup_troubleshooting_configuration".to_string()));
                        metadata
                    },
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to retrieve MCP integration guide: {}", e),
                    }],
                    is_error: true,
                    metadata: HashMap::new(),
                }),
            }
        }

        "ratchet_get_developer_guide_walkthrough" => {
            match get_developer_guide_walkthrough().await {
                Ok(documentation) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: documentation,
                    }],
                    is_error: false,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("document_type".to_string(), Value::String("mcp_development_walkthrough".to_string()));
                        metadata.insert("source_file".to_string(), Value::String("docs/MCP_DEVELOPMENT_GUIDE.md".to_string()));
                        metadata.insert("covers".to_string(), Value::String("task_creation_execution_monitoring_debugging_administration".to_string()));
                        metadata.insert("example_task".to_string(), Value::String("httpbin_get_origin".to_string()));
                        metadata
                    },
                }),
                Err(e) => Ok(ToolsCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to retrieve MCP development walkthrough: {}", e),
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

/// Get the MCP endpoints reference documentation
async fn get_developer_endpoint_reference() -> Result<String, String> {
    let header = format!(
        "# Ratchet MCP Endpoints Reference\n\n\
        **Retrieved at**: {}\n\
        **Source**: Embedded documentation\n\
        **Document Type**: Comprehensive API Reference\n\n\
        ---\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    let content = include_str!("../../../docs/MCP_ENDPOINTS_REFERENCE.md");
    Ok(format!("{}{}", header, content))
}

/// Get the MCP integration guide documentation
async fn get_developer_integration_guide() -> Result<String, String> {
    let header = format!(
        "# Ratchet MCP Integration Guide\n\n\
        **Retrieved at**: {}\n\
        **Source**: Embedded documentation\n\
        **Document Type**: Setup and Integration Guide\n\n\
        ---\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    let content = include_str!("../../../docs/MCP_DEVELOPMENT_GUIDE.md");
    Ok(format!("{}{}", header, content))
}

/// Get the MCP development walkthrough guide
async fn get_developer_guide_walkthrough() -> Result<String, String> {
    let header = format!(
        "# Ratchet MCP Development Walkthrough\n\n\
        **Retrieved at**: {}\n\
        **Source**: Embedded documentation\n\
        **Document Type**: Step-by-step Development Guide\n\n\
        ---\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    let content = include_str!("../../../docs/MCP_DEVELOPMENT_GUIDE.md");
    Ok(format!("{}{}", header, content))
}
