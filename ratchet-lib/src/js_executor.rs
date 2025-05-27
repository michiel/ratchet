use crate::errors::{JsErrorType, JsExecutionError};
use crate::validation::{validate_json, parse_schema};
use boa_engine::{Context as BoaContext, Source};
use regex;
use serde_json::Value as JsonValue;
use tracing::{debug, info, trace, warn};

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
/// Prepare input data for JavaScript execution
fn prepare_input_argument(
    context: &mut BoaContext<'_>,
    input_data: &JsonValue,
) -> Result<boa_engine::JsValue, JsExecutionError> {
    trace!("Converting input data to JavaScript format");
    let input_js_str = serde_json::to_string(input_data)
        .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;

    trace!("Parsing input JSON string into JavaScript object");
    context
        .eval(Source::from_bytes(&format!(
            "JSON.parse('{}')",
            input_js_str.replace("'", "\\'")
        )))
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to parse input JSON: {}", e))
        })
}

/// Execute JavaScript function and handle errors
fn execute_javascript_function(
    context: &mut BoaContext<'_>,
    func: &boa_engine::JsValue,
    input_arg: &boa_engine::JsValue,
) -> Result<boa_engine::JsValue, JsExecutionError> {
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
    trace!("Calling JavaScript function with input data");
    func_obj
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
        })
}

/// Check if fetch API was called and extract parameters
fn check_fetch_call(
    context: &mut BoaContext<'_>,
) -> Result<Option<(String, Option<JsonValue>, Option<JsonValue>)>, JsExecutionError> {
    debug!("Checking for fetch API calls");
    let fetch_marker = context
        .eval(Source::from_bytes(
            "typeof __fetch_url === 'string' && __fetch_url !== null",
        ))
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

    if !fetch_marker.as_boolean().unwrap_or(false) {
        return Ok(None);
    }

    debug!("Detected fetch API call, extracting parameters");
    
    // Get URL
    let url_js = context
        .eval(Source::from_bytes("__fetch_url"))
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;
    
    let url = url_js
        .to_string(context)
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
        .to_std_string_escaped();

    // Get parameters
    let params_js = context
        .eval(Source::from_bytes("__fetch_params"))
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

    let params = if !params_js.is_null() && !params_js.is_undefined() {
        let params_str = params_js
            .to_string(context)
            .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
            .to_std_string_escaped();

        Some(serde_json::from_str(&params_str)
            .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?)
    } else {
        None
    };

    // Get body
    let body_js = context
        .eval(Source::from_bytes("__fetch_body"))
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

    let body = if !body_js.is_null() && !body_js.is_undefined() {
        let body_str = body_js
            .to_string(context)
            .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?
            .to_std_string_escaped();

        // Try to parse as JSON first, if that fails treat as a string
        Some(match serde_json::from_str::<JsonValue>(&body_str) {
            Ok(json_val) => json_val,
            Err(_) => JsonValue::String(body_str),
        })
    } else {
        None
    };

    Ok(Some((url, params, body)))
}

/// Handle HTTP fetch processing and inject result back into context
async fn handle_fetch_processing(
    context: &mut BoaContext<'_>,
    func: &boa_engine::JsValue,
    input_arg: &boa_engine::JsValue,
    http_manager: &crate::http::HttpManager,
    url: String,
    params: Option<JsonValue>,
    body: Option<JsonValue>,
) -> Result<boa_engine::JsValue, JsExecutionError> {
    debug!("Making HTTP call to: {}", url);
    
    // Perform the HTTP call
    let http_result = http_manager
        .call_http(&url, params.as_ref(), body.as_ref())
        .await
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
    
    // Get the function as an object
    let func_obj = func.as_object().ok_or_else(|| {
        JsExecutionError::ExecutionError("Failed to convert to object".to_string())
    })?;

    // Re-call the JavaScript function now that fetch will return the real result
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

    debug!("Clearing fetch state variables");
    
    // Clear the fetch state
    context
        .eval(Source::from_bytes(
            "__fetch_url = null; __fetch_params = null; __fetch_body = null; __http_result = null;",
        ))
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

    Ok(result)
}

/// Convert JavaScript result to JSON
fn convert_js_result_to_json(
    context: &mut BoaContext<'_>,
    result: boa_engine::JsValue,
) -> Result<JsonValue, JsExecutionError> {
    debug!("Converting JavaScript result back to JSON");
    
    // Set temporary variable to hold the result so we can stringify it
    context
        .global_object()
        .set("__temp_result", result, true, context)
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to set temporary result: {}", e))
        })?;

    // Convert to JSON string
    let result_json_str = context
        .eval(Source::from_bytes("JSON.stringify(__temp_result)"))
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to stringify result: {}", e))
        })?;

    // Convert to Rust string
    let result_str = result_json_str
        .to_string(context)
        .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?;

    let json_str = result_str.to_std_string().unwrap();

    // Parse the JSON string into a JsonValue
    serde_json::from_str(&json_str)
        .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))
}

/// Execute a JavaScript function with HTTP fetch support
pub async fn call_js_function(
    context: &mut BoaContext<'_>,
    func: &boa_engine::JsValue,
    input_data: &JsonValue,
    http_manager: &crate::http::HttpManager,
) -> Result<JsonValue, JsExecutionError> {
    // Prepare input argument
    let input_arg = prepare_input_argument(context, input_data)?;

    // Execute the JavaScript function first time
    let result = execute_javascript_function(context, func, &input_arg)?;

    // Check if a fetch call was made
    if let Some((url, params, body)) = check_fetch_call(context)? {
        // Handle HTTP fetch processing and re-execute function
        let result = handle_fetch_processing(
            context, func, &input_arg, http_manager, url, params, body
        ).await?;
        
        debug!("HTTP call completed successfully");
        convert_js_result_to_json(context, result)
    } else {
        // No fetch call, convert result directly
        convert_js_result_to_json(context, result)
    }
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
    js_file_path: &std::path::Path,
    input_schema_path: &std::path::Path,
    output_schema_path: &std::path::Path,
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
    let js_code = std::fs::read_to_string(js_file_path).map_err(JsExecutionError::FileReadError)?;

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