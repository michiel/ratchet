use anyhow::Result;
use boa_engine::{Context as BoaContext, Source};
use jsonschema::{JSONSchema, Draft};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

pub mod js_task;
pub mod http;
pub mod task;
pub mod test;

/// A module for executing JavaScript tasks
pub mod js_executor {
    use super::*;

    /// Errors that can occur during JavaScript execution
    #[derive(Error, Debug)]
    pub enum JsExecutionError {
        #[error("Failed to read JavaScript file: {0}")]
        FileReadError(#[from] std::io::Error),

        #[error("Failed to compile JavaScript: {0}")]
        CompileError(String),

        #[error("Failed to execute JavaScript: {0}")]
        ExecutionError(String),

        #[error("Schema validation error: {0}")]
        SchemaValidationError(String),

        #[error("Invalid input schema: {0}")]
        InvalidInputSchema(String),

        #[error("Invalid output schema: {0}")]
        InvalidOutputSchema(String),

        #[error("Invalid output format: {0}")]
        InvalidOutputFormat(String),
    }

    /// Validate JSON data against a schema
    pub fn validate_json(data: &JsonValue, schema: &JsonValue) -> Result<(), JsExecutionError> {
        let compiled_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(schema)
            .map_err(|e| JsExecutionError::SchemaValidationError(e.to_string()))?;

        compiled_schema
            .validate(data)
            .map_err(|errs| {
                let error_msgs: Vec<String> = errs.map(|e| e.to_string()).collect();
                JsExecutionError::SchemaValidationError(error_msgs.join(", "))
            })?;

        Ok(())
    }

    /// Parse a JSON schema from a file
    pub fn parse_schema(schema_path: &Path) -> Result<JsonValue, JsExecutionError> {
        let schema_str = fs::read_to_string(schema_path)
            .map_err(JsExecutionError::FileReadError)?;
        
        serde_json::from_str(&schema_str)
            .map_err(|e| JsExecutionError::InvalidInputSchema(e.to_string()))
    }

    /// Call a JavaScript function with the given input
    pub fn call_js_function(
        context: &mut BoaContext,
        func: &boa_engine::JsValue,
        input_data: &JsonValue
    ) -> Result<JsonValue, JsExecutionError> {
        debug!("Converting input data to JavaScript format");
        // Convert input_data to JsValue 
        let input_js_str = serde_json::to_string(input_data)
            .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;
        
        // Parse the JSON string into a JavaScript object by evaluating it directly
        let input_arg = context.eval(Source::from_bytes(&format!("JSON.parse('{}')", input_js_str.replace("'", "\\'"))))
            .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to parse input JSON: {}", e)))?;

        // Check if func is callable
        if !func.is_callable() {
            warn!("JavaScript code did not return a callable function");
            return Err(JsExecutionError::ExecutionError("The evaluated JavaScript code did not return a callable function".to_string()));
        }
        
        // Get the function as an object and invoke it with the input
        let func_obj = func.as_object()
            .ok_or_else(|| JsExecutionError::ExecutionError("Failed to convert to object".to_string()))?;
            
        // Call the function with itself as the 'this' value
        let result = func_obj
            .call(func, &[input_arg], context)
            .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
            
        // Check if we need to process a fetch call
        debug!("Checking for fetch API calls");
        let fetch_marker = context.eval(Source::from_bytes(
            "typeof __fetch_url === 'string' && __fetch_url !== null"
        )).map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
        
        if fetch_marker.as_boolean().unwrap_or(false) {
            debug!("Detected fetch API call, processing HTTP request");
            // Get the fetch parameters
            let url_js = context.eval(Source::from_bytes("__fetch_url"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
            
            let params_js = context.eval(Source::from_bytes("__fetch_params"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
                
            let body_js = context.eval(Source::from_bytes("__fetch_body"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
            
            // Convert to Rust values
            let url = url_js.to_string(context)
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                .to_std_string_escaped();
                
            // Parse params if provided
            let params = if !params_js.is_null() && !params_js.is_undefined() {
                let params_str = params_js.to_string(context)
                    .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                    .to_std_string_escaped();
                    
                serde_json::from_str(&params_str)
                    .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?
            } else {
                None
            };
            
            // Parse body if provided
            let body = if !body_js.is_null() && !body_js.is_undefined() {
                let body_str = body_js.to_string(context)
                    .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                    .to_std_string_escaped();
                    
                serde_json::from_str(&body_str)
                    .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?
            } else {
                None
            };
            
            debug!("Making HTTP call to: {}", url);
            // Perform the HTTP call
            let http_result = crate::http::call_http(&url, params.as_ref(), body.as_ref())
                .map_err(|e| JsExecutionError::ExecutionError(format!("HTTP error: {}", e)))?;
                
            debug!("Clearing fetch state variables");
            // Clear the fetch state
            context.eval(Source::from_bytes("__fetch_url = null; __fetch_params = null; __fetch_body = null;"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
                
            debug!("Returning HTTP result");
            // Return the HTTP result instead
            return Ok(http_result);
        }

        debug!("Converting JavaScript result back to JSON");
        // Convert result back to JsonValue by first converting to JSON string
        // We need to create a temporary variable to hold the result so we can stringify it
        context.global_object().set("__temp_result", result, true, context)
            .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to set temporary result: {}", e)))?;
        
        let result_json_str = context.eval(Source::from_bytes("JSON.stringify(__temp_result)"))
            .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to stringify result: {}", e)))?;
        
        let result_str = result_json_str.to_string(context)
            .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;
        
        // Now convert the JavaScript string representation to a Rust string
        let json_str = result_str.to_std_string().unwrap();
        
        // Parse the JSON string into a JsonValue
        let result_json: JsonValue = serde_json::from_str(&json_str)
            .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;

        Ok(result_json)
    }

    /// Execute a task with the given input
    pub async fn execute_task(
        task: &mut crate::task::Task,
        input_data: JsonValue,
    ) -> Result<JsonValue, JsExecutionError> {
        info!("Executing task: {} ({})", task.metadata.label, task.metadata.uuid);
        debug!("Input data: {}", serde_json::to_string(&input_data).unwrap_or_else(|_| "<invalid json>".to_string()));
        
        match &task.task_type {
            crate::task::TaskType::JsTask { .. } => {
                debug!("Loading JavaScript content for execution");
                // Load content if not already loaded
                task.ensure_content_loaded()
                    .map_err(|e| JsExecutionError::FileReadError(std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        format!("Failed to load JavaScript content: {}", e)
                    )))?;
                
                let js_content = task.get_js_content()
                    .map_err(|e| JsExecutionError::FileReadError(std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        format!("Failed to get JavaScript content: {}", e)
                    )))?;
                
                let input_schema_path = task.path.join("input.schema.json");
                let output_schema_path = task.path.join("output.schema.json");
                
                debug!("Parsing input and output schemas");
                // Parse input and output schemas
                let input_schema = parse_schema(&input_schema_path)?;
                let output_schema = parse_schema(&output_schema_path)?;

                debug!("Validating input data against schema");
                // Validate input against schema
                validate_json(&input_data, &input_schema)?;
                
                debug!("Creating JavaScript execution context");
                // Create a new Boa context for JavaScript execution
                let mut context = BoaContext::default();
                
                debug!("Registering fetch API");
                // Register the fetch API
                crate::http::register_fetch(&mut context)
                    .map_err(|e| JsExecutionError::ExecutionError(
                        format!("Failed to register fetch API: {}", e)
                    ))?;
                
                // Initialize fetch variables
                debug!("Initializing fetch variables");
                context.eval(Source::from_bytes("var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;"))
                    .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;
                
                debug!("Compiling JavaScript code");
                // Evaluate the JavaScript code from memory
                let func = context.eval(Source::from_bytes(&js_content.as_ref()))
                    .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;
                
                debug!("Calling JavaScript function");
                // Call the JavaScript function with the input data
                let result = call_js_function(&mut context, &func, &input_data)?;
                
                debug!("Validating output against schema");
                // Validate output against schema
                validate_json(&result, &output_schema)?;
                
                info!("Task execution completed successfully: {} ({})", task.metadata.label, task.metadata.uuid);
                debug!("Output data: {}", serde_json::to_string(&result).unwrap_or_else(|_| "<invalid json>".to_string()));
                
                Ok(result)
            }
        }
    }

    /// Execute a JavaScript file with the given input
    pub async fn execute_js_file(
        js_file_path: &Path,
        input_schema_path: &Path,
        output_schema_path: &Path,
        input_data: JsonValue,
    ) -> Result<JsonValue, JsExecutionError> {
        info!("Executing JavaScript file: {:?}", js_file_path);
        debug!("Input data: {}", serde_json::to_string(&input_data).unwrap_or_else(|_| "<invalid json>".to_string()));
        
        debug!("Parsing input and output schemas");
        // Parse input and output schemas
        let input_schema = parse_schema(input_schema_path)?;
        let output_schema = parse_schema(output_schema_path)?;

        debug!("Validating input against schema");
        // Validate input against schema
        validate_json(&input_data, &input_schema)?;

        debug!("Reading JavaScript file: {:?}", js_file_path);
        // Read and execute the JavaScript file
        let js_code = fs::read_to_string(js_file_path)
            .map_err(JsExecutionError::FileReadError)?;
        
        debug!("Creating JavaScript execution context");
        // Create a new Boa context for JavaScript execution
        let mut context = BoaContext::default();
        
        debug!("Registering fetch API");
        // Register the fetch API
        crate::http::register_fetch(&mut context)
            .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to register fetch API: {}", e)))?;
        
        // Initialize fetch variables
        debug!("Initializing fetch variables");
        context.eval(Source::from_bytes("var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;"))
            .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;
        
        debug!("Compiling JavaScript code");
        // Evaluate the JavaScript file
        let func = context.eval(Source::from_bytes(&js_code))
            .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;
        
        debug!("Calling JavaScript function");
        // Call the JavaScript function with the input data
        let result = call_js_function(&mut context, &func, &input_data)?;
        
        debug!("Validating output against schema");
        // Validate output against schema
        validate_json(&result, &output_schema)?;
        
        info!("JavaScript file execution completed successfully: {:?}", js_file_path);
        debug!("Output data: {}", serde_json::to_string(&result).unwrap_or_else(|_| "<invalid json>".to_string()));
        
        Ok(result)
    }

}

/// Legacy addition function (kept for compatibility)
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use js_executor::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio_test::block_on;
    use serde_json::json;
    use crate::task::{Task, TaskMetadata, TaskType};
    use uuid::Uuid;

    // We need to keep a reference to tempdir so it doesn't get dropped while we use the files
    struct TestFiles {
        _temp_dir: tempfile::TempDir, // Keep this field to prevent cleanup until TestFiles is dropped
        js_file: PathBuf,
        input_schema: PathBuf,
        output_schema: PathBuf,
        bad_input_schema: PathBuf,
    }

    fn setup_test_files() -> Result<TestFiles, std::io::Error> {
        let temp_dir = tempdir()?;
        
        let js_file = temp_dir.path().join("main.js");
        fs::write(&js_file, r#"
// Export a function for use
function processInput(input) {
  const num1 = input.num1;
  const num2 = input.num2;

  if (typeof num1 !== 'number' || typeof num2 !== 'number') {
    throw new Error('num1 and num2 must be numbers');
  }

  return {
    sum: num1 + num2
  };
}

// Return the function itself as the module's export
processInput
"#)?;

        let input_schema = temp_dir.path().join("input.schema.json");
        fs::write(&input_schema, r#"{
    "type": "object",
    "properties": {
        "num1": { "type": "number" },
        "num2": { "type": "number" }
    },
    "required": ["num1", "num2"]
}"#)?;

        let output_schema = temp_dir.path().join("output.schema.json");
        fs::write(&output_schema, r#"{
    "type": "object",
    "properties": {
        "sum": { "type": "number" }
    },
    "required": ["sum"]
}"#)?;

        let bad_input_schema = temp_dir.path().join("bad_input.schema.json");
        fs::write(&bad_input_schema, r#"{
    "type": "object",
    "properties": {
        "num1": { "type": "string" },
        "num2": { "type": "string" }
    },
    "required": ["num1", "num2"]
}"#)?;

        Ok(TestFiles {
            _temp_dir: temp_dir,
            js_file,
            input_schema,
            output_schema,
            bad_input_schema,
        })
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_valid_execution() {
        block_on(async {
            if let Ok(files) = setup_test_files() {
                let input_data = json!({
                    "num1": 5,
                    "num2": 7
                });

                let result = execute_js_file(
                    &files.js_file, 
                    &files.input_schema, 
                    &files.output_schema, 
                    input_data
                ).await.unwrap();

                // Check the result structure and value instead of exact equality
                assert!(result.is_object());
                assert!(result.get("sum").is_some());
                let sum = result["sum"].as_f64().unwrap();
                assert_eq!(sum, 12.0);
            } else {
                // Skip test if files can't be created
                println!("Skipping test_valid_execution due to file setup issues");
            }
        });
    }
    
    #[test]
    fn test_execute_task() {
        block_on(async {
            if let Ok(files) = setup_test_files() {
                // Create a test task
                let mut task = Task {
                    metadata: TaskMetadata {
                        uuid: Uuid::parse_str("bd6c6f98-4896-44cc-8c82-30328c3aefda").unwrap(),
                        version: "1.0.0".to_string(),
                        label: "Test Task".to_string(),
                        description: "Test task for unit testing".to_string(),
                    },
                    task_type: TaskType::JsTask { 
                        path: files.js_file.to_string_lossy().to_string(),
                        content: None,
                    },
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "num1": { "type": "number" },
                            "num2": { "type": "number" }
                        },
                        "required": ["num1", "num2"]
                    }),
                    output_schema: json!({
                        "type": "object",
                        "properties": {
                            "sum": { "type": "number" }
                        },
                        "required": ["sum"]
                    }),
                    path: files._temp_dir.path().to_path_buf(),
                    _temp_dir: None,
                };
                
                let input_data = json!({
                    "num1": 10,
                    "num2": 20
                });
                
                // Execute the task
                let result = execute_task(&mut task, input_data).await.unwrap();
                
                // Check the result
                assert!(result.is_object());
                assert!(result.get("sum").is_some());
                let sum = result["sum"].as_f64().unwrap();
                assert_eq!(sum, 30.0);
                
                // Content should now be loaded
                match &task.task_type {
                    TaskType::JsTask { content, .. } => {
                        assert!(content.is_some());
                    }
                }
                
                // Purge content and test executing again
                task.purge_content();
                
                let input_data = json!({
                    "num1": 30,
                    "num2": 40
                });
                
                // Execute the task again
                let result = execute_task(&mut task, input_data).await.unwrap();
                
                // Check the result
                let sum = result["sum"].as_f64().unwrap();
                assert_eq!(sum, 70.0);
            } else {
                // Skip test if files can't be created
                println!("Skipping test_execute_task due to file setup issues");
            }
        });
    }

    #[test]
    fn test_invalid_input_type() {
        block_on(async {
            if let Ok(files) = setup_test_files() {
                let input_data = json!({
                    "num1": "not a number",
                    "num2": 7
                });

                let result = execute_js_file(
                    &files.js_file, 
                    &files.input_schema, 
                    &files.output_schema, 
                    input_data
                ).await;

                assert!(result.is_err());
                match result {
                    Err(JsExecutionError::SchemaValidationError(_)) => {},
                    err => panic!("Expected SchemaValidationError, got {:?}", err),
                }
            } else {
                // Skip test if files can't be created
                println!("Skipping test_invalid_input_type due to file setup issues");
            }
        });
    }

    #[test]
    fn test_invalid_schema() {
        block_on(async {
            if let Ok(files) = setup_test_files() {
                let input_data = json!({
                    "num1": 5,
                    "num2": 7
                });

                let result = execute_js_file(
                    &files.js_file, 
                    &files.bad_input_schema, 
                    &files.output_schema, 
                    input_data
                ).await;

                assert!(result.is_err());
            } else {
                // Skip test if files can't be created
                println!("Skipping test_invalid_schema due to file setup issues");
            }
        });
    }

}