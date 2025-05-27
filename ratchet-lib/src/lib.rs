use anyhow::Result;
use boa_engine::{Context as BoaContext, Source};
use jsonschema::{Draft, JSONSchema};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, trace, warn};

pub mod generate;
pub mod http;
pub mod js_task;
pub mod recording;
pub mod task;
pub mod test;

/// A module for executing JavaScript tasks
pub mod js_executor {
    use super::*;

    /// JavaScript error types that can be thrown from JS code
    #[derive(Error, Debug, Clone)]
    pub enum JsErrorType {
        #[error("Authentication failed: {0}")]
        AuthenticationError(String),

        #[error("Authorization failed: {0}")]
        AuthorizationError(String),

        #[error("Network error: {0}")]
        NetworkError(String),

        #[error("HTTP error {status}: {message}")]
        HttpError { status: u16, message: String },

        #[error("Validation error: {0}")]
        ValidationError(String),

        #[error("Configuration error: {0}")]
        ConfigurationError(String),

        #[error("Rate limit exceeded: {0}")]
        RateLimitError(String),

        #[error("Service unavailable: {0}")]
        ServiceUnavailableError(String),

        #[error("Timeout error: {0}")]
        TimeoutError(String),

        #[error("Data error: {0}")]
        DataError(String),

        #[error("Unknown error: {0}")]
        UnknownError(String),
    }

    /// Errors that can occur during JavaScript execution
    #[derive(Error, Debug)]
    pub enum JsExecutionError {
        #[error("Failed to read JavaScript file: {0}")]
        FileReadError(#[from] std::io::Error),

        #[error("Failed to compile JavaScript: {0}")]
        CompileError(String),

        #[error("Failed to execute JavaScript: {0}")]
        ExecutionError(String),

        #[error("JavaScript threw typed error: {0}")]
        TypedJsError(#[from] JsErrorType),

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

        compiled_schema.validate(data).map_err(|errs| {
            let error_msgs: Vec<String> = errs.map(|e| e.to_string()).collect();
            JsExecutionError::SchemaValidationError(error_msgs.join(", "))
        })?;

        Ok(())
    }

    /// Parse a JSON schema from a file
    pub fn parse_schema(schema_path: &Path) -> Result<JsonValue, JsExecutionError> {
        let schema_str =
            fs::read_to_string(schema_path).map_err(JsExecutionError::FileReadError)?;

        serde_json::from_str(&schema_str)
            .map_err(|e| JsExecutionError::InvalidInputSchema(e.to_string()))
    }

    /// Configuration for JavaScript error types
    #[derive(Debug, Clone)]
    pub struct JsErrorConfig {
        pub name: &'static str,
        pub default_message: &'static str,
        pub has_status: bool,
    }

    /// Predefined JavaScript error types with their configurations
    pub const JS_ERROR_CONFIGS: &[JsErrorConfig] = &[
        JsErrorConfig {
            name: "AuthenticationError",
            default_message: "Authentication failed",
            has_status: false,
        },
        JsErrorConfig {
            name: "AuthorizationError", 
            default_message: "Authorization failed",
            has_status: false,
        },
        JsErrorConfig {
            name: "NetworkError",
            default_message: "Network error",
            has_status: false,
        },
        JsErrorConfig {
            name: "HttpError",
            default_message: "HTTP error",
            has_status: true,
        },
        JsErrorConfig {
            name: "ValidationError",
            default_message: "Validation error", 
            has_status: false,
        },
        JsErrorConfig {
            name: "ConfigurationError",
            default_message: "Configuration error",
            has_status: false,
        },
        JsErrorConfig {
            name: "RateLimitError",
            default_message: "Rate limit exceeded",
            has_status: false,
        },
        JsErrorConfig {
            name: "ServiceUnavailableError",
            default_message: "Service unavailable",
            has_status: false,
        },
        JsErrorConfig {
            name: "TimeoutError",
            default_message: "Timeout error",
            has_status: false,
        },
        JsErrorConfig {
            name: "DataError",
            default_message: "Data error",
            has_status: false,
        },
    ];

    /// Generate JavaScript error class definition for a single error type
    pub fn generate_error_class(error_config: &JsErrorConfig) -> String {
        if error_config.has_status {
            // Special case for HttpError which takes status and message
            format!(r#"
            // {name}
            function {name}(status, message) {{
                this.name = "{name}";
                this.status = status;
                this.message = message || "{default_message}";
                this.stack = (new Error()).stack;
            }}
            {name}.prototype = Object.create(Error.prototype);
            {name}.prototype.constructor = {name};"#,
                name = error_config.name,
                default_message = error_config.default_message
            )
        } else {
            // Standard error type with just message
            format!(r#"
            // {name}
            function {name}(message) {{
                this.name = "{name}";
                this.message = message || "{default_message}";
                this.stack = (new Error()).stack;
            }}
            {name}.prototype = Object.create(Error.prototype);
            {name}.prototype.constructor = {name};"#,
                name = error_config.name,
                default_message = error_config.default_message
            )
        }
    }

    /// Generate all JavaScript error class definitions
    pub fn generate_all_error_classes() -> String {
        JS_ERROR_CONFIGS
            .iter()
            .map(generate_error_class)
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// Register custom error types in the JavaScript context
    pub fn register_error_types(context: &mut BoaContext<'_>) -> Result<(), JsExecutionError> {
        let error_classes = generate_all_error_classes();

        context
            .eval(Source::from_bytes(&error_classes))
            .map_err(|e| JsExecutionError::CompileError(format!("Failed to register error types: {}", e)))?;
        
        Ok(())
    }

    /// Parse JavaScript error and convert to JsErrorType
    pub fn parse_js_error(error_message: &str) -> JsErrorType {
        // Try to extract error type and message from the error string
        if let Some(captures) = regex::Regex::new(r"(\w+Error): (.+)")
            .unwrap()
            .captures(error_message) 
        {
            let error_type = &captures[1];
            let message = captures[2].to_string();
            
            match error_type {
                "AuthenticationError" => JsErrorType::AuthenticationError(message),
                "AuthorizationError" => JsErrorType::AuthorizationError(message),
                "NetworkError" => JsErrorType::NetworkError(message),
                "HttpError" => {
                    // Try to extract status code from message
                    if let Some(status_captures) = regex::Regex::new(r"(\d+)")
                        .unwrap()
                        .captures(&message) 
                    {
                        if let Ok(status) = status_captures[1].parse::<u16>() {
                            return JsErrorType::HttpError { status, message };
                        }
                    }
                    JsErrorType::HttpError { status: 0, message }
                },
                "ValidationError" => JsErrorType::ValidationError(message),
                "ConfigurationError" => JsErrorType::ConfigurationError(message),
                "RateLimitError" => JsErrorType::RateLimitError(message),
                "ServiceUnavailableError" => JsErrorType::ServiceUnavailableError(message),
                "TimeoutError" => JsErrorType::TimeoutError(message),
                "DataError" => JsErrorType::DataError(message),
                _ => JsErrorType::UnknownError(message),
            }
        } else {
            JsErrorType::UnknownError(error_message.to_string())
        }
    }

    /// Call a JavaScript function with the given input
    pub async fn call_js_function(
        context: &mut BoaContext<'_>,
        func: &boa_engine::JsValue,
        input_data: &JsonValue,
        http_manager: &crate::http::HttpManager,
    ) -> Result<JsonValue, JsExecutionError> {
        trace!("Converting input data to JavaScript format");
        // Convert input_data to JsValue
        let input_js_str = serde_json::to_string(input_data)
            .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;

        trace!("Parsing input JSON string into JavaScript object");
        // Parse the JSON string into a JavaScript object by evaluating it directly
        let input_arg = context
            .eval(Source::from_bytes(&format!(
                "JSON.parse('{}')",
                input_js_str.replace("'", "\\'")
            )))
            .map_err(|e| {
                JsExecutionError::ExecutionError(format!("Failed to parse input JSON: {}", e))
            })?;

        // Check if func is callable
        if !func.is_callable() {
            warn!("JavaScript code did not return a callable function");
            return Err(JsExecutionError::ExecutionError(
                "The evaluated JavaScript code did not return a callable function".to_string(),
            ));
        }

        // Get the function as an object and invoke it with the input
        let func_obj = func.as_object().ok_or_else(|| {
            JsExecutionError::ExecutionError("Failed to convert to object".to_string())
        })?;

        // Call the function with itself as the 'this' value
        trace!("Calling JavaScript function with input data",);
        let result = func_obj
            .call(func, &[input_arg.clone()], context)
            .map_err(|e| {
                let error_message = e.to_string();
                // Try to parse as a typed JS error first
                if error_message.contains("Error:") {
                    let parsed_error = parse_js_error(&error_message);
                    JsExecutionError::TypedJsError(parsed_error)
                } else {
                    JsExecutionError::ExecutionError(error_message)
                }
            })?;

        // Check if we need to process a fetch call
        debug!("Checking for fetch API calls");
        let fetch_marker = context
            .eval(Source::from_bytes(
                "typeof __fetch_url === 'string' && __fetch_url !== null",
            ))
            .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

        if fetch_marker.as_boolean().unwrap_or(false) {
            debug!("Detected fetch API call, processing HTTP request");
            // Get the fetch parameters
            let url_js = context
                .eval(Source::from_bytes("__fetch_url"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

            let params_js = context
                .eval(Source::from_bytes("__fetch_params"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

            let body_js = context
                .eval(Source::from_bytes("__fetch_body"))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

            // Convert to Rust values
            let url = url_js
                .to_string(context)
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                .to_std_string_escaped();

            // Parse params if provided
            let params = if !params_js.is_null() && !params_js.is_undefined() {
                let params_str = params_js
                    .to_string(context)
                    .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                    .to_std_string_escaped();

                serde_json::from_str(&params_str)
                    .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?
            } else {
                None
            };

            // Parse body if provided
            let body = if !body_js.is_null() && !body_js.is_undefined() {
                let body_str = body_js
                    .to_string(context)
                    .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
                    .to_std_string_escaped();

                // Try to parse as JSON first, if that fails treat as a string
                match serde_json::from_str::<JsonValue>(&body_str) {
                    Ok(json_val) => Some(json_val),
                    Err(_) => {
                        // If it's not valid JSON, treat it as a string
                        Some(JsonValue::String(body_str))
                    }
                }
            } else {
                None
            };

            debug!("Making HTTP call to: {}", url);
            // Perform the HTTP call using the provided HttpManager
            let http_result = http_manager.call_http(&url, params.as_ref(), body.as_ref()).await
                .map_err(|e| JsExecutionError::ExecutionError(format!("HTTP error: {}", e)))?;

            debug!("Injecting HTTP result back into JavaScript context");
            // Store the HTTP result in a global variable
            context
                .global_object()
                .set("__http_result", 
                     context.eval(Source::from_bytes(&format!("({})", 
                        serde_json::to_string(&http_result)
                            .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to serialize HTTP result: {}", e)))?
                     )))
                     .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to parse HTTP result JSON: {}", e)))?, 
                     true, 
                     context)
                .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to set HTTP result: {}", e)))?;
            
            // Replace the fetch function to return the stored result and throw appropriate errors
            context
                .eval(Source::from_bytes(r#"
                    fetch = function(url, params, body) {
                        var response = __http_result;
                        
                        // Check if response is OK, throw appropriate errors if not
                        if (!response.ok) {
                            var status = response.status || 0;
                            var statusText = response.statusText || "Unknown Status";
                            
                            // Map status codes to appropriate error types
                            if (status === 401) {
                                throw new AuthenticationError("HTTP " + status + ": " + statusText);
                            } else if (status === 403) {
                                throw new AuthorizationError("HTTP " + status + ": " + statusText);
                            } else if (status === 429) {
                                throw new RateLimitError("HTTP " + status + ": " + statusText);
                            } else if (status >= 500 && status < 600) {
                                throw new ServiceUnavailableError("HTTP " + status + ": " + statusText);
                            } else if (status >= 400 && status < 500) {
                                throw new HttpError(status, "HTTP " + status + ": " + statusText);
                            } else {
                                throw new NetworkError("HTTP " + status + ": " + statusText);
                            }
                        }
                        
                        return response;
                    };
                "#))
                .map_err(|e| JsExecutionError::ExecutionError(format!("Failed to replace fetch function: {}", e)))?;

            debug!("Re-calling JavaScript function with updated fetch");
            // Re-call the JavaScript function now that fetch will return the real result
            let result = func_obj
                .call(func, &[input_arg], context)
                .map_err(|e| {
                    let error_message = e.to_string();
                    // Try to parse as a typed JS error first
                    if error_message.contains("Error:") {
                        let parsed_error = parse_js_error(&error_message);
                        JsExecutionError::TypedJsError(parsed_error)
                    } else {
                        JsExecutionError::ExecutionError(error_message)
                    }
                })?;

            debug!("Clearing fetch state variables");
            // Clear the fetch state
            context
                .eval(Source::from_bytes(
                    "__fetch_url = null; __fetch_params = null; __fetch_body = null; __http_result = null;",
                ))
                .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
            
            debug!("Converting result from second function call");
            // Process the result from the second call the same way as normal
            context
                .global_object()
                .set("__temp_result", result, true, context)
                .map_err(|e| {
                    JsExecutionError::ExecutionError(format!("Failed to set temporary result: {}", e))
                })?;

            let result_json_str = context
                .eval(Source::from_bytes("JSON.stringify(__temp_result)"))
                .map_err(|e| {
                    JsExecutionError::ExecutionError(format!("Failed to stringify result: {}", e))
                })?;

            let result_str = result_json_str
                .to_string(context)
                .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?
                .to_std_string_escaped();

            let result_json: JsonValue = serde_json::from_str(&result_str)
                .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;

            debug!("HTTP call completed successfully");
            return Ok(result_json);
        }

        debug!("Converting JavaScript result back to JSON");
        // Convert result back to JsonValue by first converting to JSON string
        // We need to create a temporary variable to hold the result so we can stringify it
        context
            .global_object()
            .set("__temp_result", result, true, context)
            .map_err(|e| {
                JsExecutionError::ExecutionError(format!("Failed to set temporary result: {}", e))
            })?;

        let result_json_str = context
            .eval(Source::from_bytes("JSON.stringify(__temp_result)"))
            .map_err(|e| {
                JsExecutionError::ExecutionError(format!("Failed to stringify result: {}", e))
            })?;

        let result_str = result_json_str
            .to_string(context)
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
        http_manager: &crate::http::HttpManager,
    ) -> Result<JsonValue, JsExecutionError> {
        info!(
            "Executing task: {} ({})",
            task.metadata.label, task.metadata.uuid
        );
        debug!(
            "Input data: {}",
            serde_json::to_string(&input_data).unwrap_or_else(|_| "<invalid json>".to_string())
        );

        match &task.task_type {
            crate::task::TaskType::JsTask { .. } => {
                debug!("Loading JavaScript content for execution");
                // Load content if not already loaded
                task.ensure_content_loaded().map_err(|e| {
                    JsExecutionError::FileReadError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to load JavaScript content: {}", e),
                    ))
                })?;

                let js_content = task.get_js_content().map_err(|e| {
                    JsExecutionError::FileReadError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to get JavaScript content: {}", e),
                    ))
                })?;

                let input_schema_path = task.path.join("input.schema.json");
                let output_schema_path = task.path.join("output.schema.json");

                debug!("Parsing input and output schemas");
                // Parse input and output schemas
                let input_schema = parse_schema(&input_schema_path)?;
                let output_schema = parse_schema(&output_schema_path)?;

                debug!("Validating input data against schema");
                // Validate input against schema
                validate_json(&input_data, &input_schema)?;
                
                // Record input if recording is active
                if crate::recording::is_recording() {
                    crate::recording::record_input(&input_data).map_err(|e| {
                        JsExecutionError::ExecutionError(format!("Failed to record input: {}", e))
                    })?;
                }

                debug!("Creating JavaScript execution context");
                // Create a new Boa context for JavaScript execution
                let mut context = BoaContext::default();

                debug!("Registering fetch API");
                // Register the fetch API
                crate::http::register_fetch(&mut context).map_err(|e| {
                    JsExecutionError::ExecutionError(format!("Failed to register fetch API: {}", e))
                })?;

                debug!("Registering error types");
                // Register custom error types
                register_error_types(&mut context).map_err(|e| {
                    JsExecutionError::ExecutionError(format!("Failed to register error types: {}", e))
                })?;

                // Initialize fetch variables
                debug!("Initializing fetch variables");
                context.eval(Source::from_bytes("var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;"))
                    .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

                debug!("Compiling JavaScript code");
                // Evaluate the JavaScript code from memory
                let func = context
                    .eval(Source::from_bytes(&js_content.as_ref()))
                    .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

                debug!("Calling JavaScript function");
                // Call the JavaScript function with the input data
                let result = call_js_function(&mut context, &func, &input_data, http_manager).await?;

                debug!("Validating output against schema");
                // Validate output against schema
                validate_json(&result, &output_schema)?;
                
                // Record output if recording is active
                if crate::recording::is_recording() {
                    crate::recording::record_output(&result).map_err(|e| {
                        JsExecutionError::ExecutionError(format!("Failed to record output: {}", e))
                    })?;
                }

                info!(
                    "Task execution completed successfully: {} ({})",
                    task.metadata.label, task.metadata.uuid
                );
                debug!(
                    "Output data: {}",
                    serde_json::to_string(&result).unwrap_or_else(|_| "<invalid json>".to_string())
                );

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
        http_manager: &crate::http::HttpManager,
    ) -> Result<JsonValue, JsExecutionError> {
        info!("Executing JavaScript file: {:?}", js_file_path);
        debug!(
            "Input data: {}",
            serde_json::to_string(&input_data).unwrap_or_else(|_| "<invalid json>".to_string())
        );

        debug!("Parsing input and output schemas");
        // Parse input and output schemas
        let input_schema = parse_schema(input_schema_path)?;
        let output_schema = parse_schema(output_schema_path)?;

        debug!("Validating input against schema");
        // Validate input against schema
        validate_json(&input_data, &input_schema)?;

        debug!("Reading JavaScript file: {:?}", js_file_path);
        // Read and execute the JavaScript file
        let js_code = fs::read_to_string(js_file_path).map_err(JsExecutionError::FileReadError)?;

        debug!("Creating JavaScript execution context");
        // Create a new Boa context for JavaScript execution
        let mut context = BoaContext::default();

        debug!("Registering fetch API");
        // Register the fetch API
        crate::http::register_fetch(&mut context).map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to register fetch API: {}", e))
        })?;

        debug!("Registering error types");
        // Register custom error types
        register_error_types(&mut context).map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to register error types: {}", e))
        })?;

        // Initialize fetch variables
        debug!("Initializing fetch variables");
        context
            .eval(Source::from_bytes(
                "var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;",
            ))
            .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

        debug!("Compiling JavaScript code");
        // Evaluate the JavaScript file
        let func = context
            .eval(Source::from_bytes(&js_code))
            .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

        debug!("Calling JavaScript function");
        // Call the JavaScript function with the input data
        let result = call_js_function(&mut context, &func, &input_data, http_manager).await?;

        debug!("Validating output against schema");
        // Validate output against schema
        validate_json(&result, &output_schema)?;

        info!(
            "JavaScript file execution completed successfully: {:?}",
            js_file_path
        );
        debug!(
            "Output data: {}",
            serde_json::to_string(&result).unwrap_or_else(|_| "<invalid json>".to_string())
        );

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
    use crate::task::{Task, TaskMetadata, TaskType};
    use js_executor::*;
    use serde_json::json;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio_test::block_on;
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
        fs::write(
            &js_file,
            r#"
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
"#,
        )?;

        let input_schema = temp_dir.path().join("input.schema.json");
        fs::write(
            &input_schema,
            r#"{
    "type": "object",
    "properties": {
        "num1": { "type": "number" },
        "num2": { "type": "number" }
    },
    "required": ["num1", "num2"]
}"#,
        )?;

        let output_schema = temp_dir.path().join("output.schema.json");
        fs::write(
            &output_schema,
            r#"{
    "type": "object",
    "properties": {
        "sum": { "type": "number" }
    },
    "required": ["sum"]
}"#,
        )?;

        let bad_input_schema = temp_dir.path().join("bad_input.schema.json");
        fs::write(
            &bad_input_schema,
            r#"{
    "type": "object",
    "properties": {
        "num1": { "type": "string" },
        "num2": { "type": "string" }
    },
    "required": ["num1", "num2"]
}"#,
        )?;

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

                let http_manager = crate::http::HttpManager::new();
                let result = execute_js_file(
                    &files.js_file,
                    &files.input_schema,
                    &files.output_schema,
                    input_data,
                    &http_manager,
                )
                .await
                .unwrap();

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
                let http_manager = crate::http::HttpManager::new();
                let result = execute_task(&mut task, input_data, &http_manager).await.unwrap();

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
                let http_manager = crate::http::HttpManager::new();
                let result = execute_task(&mut task, input_data, &http_manager).await.unwrap();

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

                let http_manager = crate::http::HttpManager::new();
                let result = execute_js_file(
                    &files.js_file,
                    &files.input_schema,
                    &files.output_schema,
                    input_data,
                    &http_manager,
                )
                .await;

                assert!(result.is_err());
                match result {
                    Err(JsExecutionError::SchemaValidationError(_)) => {}
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

                let http_manager = crate::http::HttpManager::new();
                let result = execute_js_file(
                    &files.js_file,
                    &files.bad_input_schema,
                    &files.output_schema,
                    input_data,
                    &http_manager,
                )
                .await;

                assert!(result.is_err());
            } else {
                // Skip test if files can't be created
                println!("Skipping test_invalid_schema due to file setup issues");
            }
        });
    }

    #[test]
    fn test_generate_error_class_standard() {
        let error_config = js_executor::JsErrorConfig {
            name: "TestError",
            default_message: "Test error message",
            has_status: false,
        };
        
        let generated = js_executor::generate_error_class(&error_config);
        
        assert!(generated.contains("function TestError(message)"));
        assert!(generated.contains("this.name = \"TestError\""));
        assert!(generated.contains("message || \"Test error message\""));
        assert!(generated.contains("TestError.prototype = Object.create(Error.prototype)"));
        assert!(generated.contains("TestError.prototype.constructor = TestError"));
    }

    #[test]
    fn test_generate_error_class_with_status() {
        let error_config = js_executor::JsErrorConfig {
            name: "HttpError",
            default_message: "HTTP error",
            has_status: true,
        };
        
        let generated = js_executor::generate_error_class(&error_config);
        
        assert!(generated.contains("function HttpError(status, message)"));
        assert!(generated.contains("this.name = \"HttpError\""));
        assert!(generated.contains("this.status = status"));
        assert!(generated.contains("message || \"HTTP error\""));
        assert!(generated.contains("HttpError.prototype = Object.create(Error.prototype)"));
        assert!(generated.contains("HttpError.prototype.constructor = HttpError"));
    }

    #[test] 
    fn test_generate_all_error_classes() {
        let all_classes = js_executor::generate_all_error_classes();
        
        // Check that all expected error types are included
        assert!(all_classes.contains("function AuthenticationError(message)"));
        assert!(all_classes.contains("function AuthorizationError(message)"));
        assert!(all_classes.contains("function NetworkError(message)"));
        assert!(all_classes.contains("function HttpError(status, message)"));
        assert!(all_classes.contains("function ValidationError(message)"));
        assert!(all_classes.contains("function ConfigurationError(message)"));
        assert!(all_classes.contains("function RateLimitError(message)"));
        assert!(all_classes.contains("function ServiceUnavailableError(message)"));
        assert!(all_classes.contains("function TimeoutError(message)"));
        assert!(all_classes.contains("function DataError(message)"));
        
        // Check that prototype setup is included for each
        assert!(all_classes.contains("AuthenticationError.prototype = Object.create(Error.prototype)"));
        assert!(all_classes.contains("HttpError.prototype = Object.create(Error.prototype)"));
    }

    #[test]
    fn test_js_error_configs_constants() {
        // Verify the error configs array is properly configured
        assert_eq!(js_executor::JS_ERROR_CONFIGS.len(), 10);
        
        // Find HttpError and verify it has status
        let http_error = js_executor::JS_ERROR_CONFIGS
            .iter()
            .find(|e| e.name == "HttpError")
            .expect("HttpError should be defined");
        assert!(http_error.has_status);
        
        // Find a standard error and verify it doesn't have status
        let auth_error = js_executor::JS_ERROR_CONFIGS
            .iter()
            .find(|e| e.name == "AuthenticationError")
            .expect("AuthenticationError should be defined");
        assert!(!auth_error.has_status);
    }

    #[test]
    fn test_register_error_types_integration() {
        let mut context = BoaContext::default();
        
        // Should successfully register all error types
        let result = js_executor::register_error_types(&mut context);
        assert!(result.is_ok());
        
        // Verify that error types are accessible in the context
        let test_code = r#"
            typeof AuthenticationError === 'function' &&
            typeof HttpError === 'function' &&
            typeof ValidationError === 'function'
        "#;
        
        let result = context.eval(Source::from_bytes(test_code)).unwrap();
        assert!(result.as_boolean().unwrap_or(false));
    }
}

