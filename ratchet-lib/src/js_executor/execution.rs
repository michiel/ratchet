use crate::errors::JsExecutionError;
use crate::js_executor::conversion::{convert_js_result_to_json, prepare_input_argument};
use crate::js_executor::error_handling::{parse_js_error, register_error_types};
use crate::js_executor::http_integration::{
    check_fetch_call, handle_fetch_processing, handle_fetch_processing_with_context,
};
use boa_engine::{Context as BoaContext, Source};
use ratchet_core::validation::{parse_schema, validate_json};
use serde_json::Value as JsonValue;
use tracing::{debug, info, trace, warn};

/// Execute JavaScript function and handle errors
fn execute_javascript_function(
    context: &mut BoaContext,
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

/// Execute JavaScript function with context and handle errors
fn execute_javascript_function_with_context(
    context: &mut BoaContext,
    func: &boa_engine::JsValue,
    input_arg: &boa_engine::JsValue,
    context_arg: &boa_engine::JsValue,
) -> Result<boa_engine::JsValue, JsExecutionError> {
    // Check if func is callable
    if !func.is_callable() {
        warn!("JavaScript code did not return a callable function");
        return Err(JsExecutionError::ExecutionError(
            "The evaluated JavaScript code did not return a callable function".to_string(),
        ));
    }

    // Get the function as an object and invoke it with the input and context
    let func_obj = func.as_object().ok_or_else(|| {
        JsExecutionError::ExecutionError("Failed to convert to object".to_string())
    })?;

    // Call the function with input and context parameters
    trace!("Calling JavaScript function with input data and context");
    func_obj
        .call(func, &[input_arg.clone(), context_arg.clone()], context)
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

/// Execute a JavaScript function with HTTP fetch support
pub async fn call_js_function(
    context: &mut BoaContext,
    func: &boa_engine::JsValue,
    input_data: &JsonValue,
    http_manager: &crate::http::HttpManager,
) -> Result<JsonValue, JsExecutionError> {
    call_js_function_with_context(context, func, input_data, http_manager, None).await
}

/// Execute a JavaScript function with HTTP fetch support and execution context
pub async fn call_js_function_with_context(
    context: &mut BoaContext,
    func: &boa_engine::JsValue,
    input_data: &JsonValue,
    http_manager: &crate::http::HttpManager,
    execution_context: Option<crate::execution::ipc::ExecutionContext>,
) -> Result<JsonValue, JsExecutionError> {
    // Prepare input argument
    let input_arg = prepare_input_argument(context, input_data)?;

    // Prepare context argument if provided
    let context_arg = if let Some(exec_ctx) = execution_context {
        // Build context object as JSON first, then parse it
        let mut ctx_json = serde_json::json!({
            "executionId": exec_ctx.execution_id,
            "taskId": exec_ctx.task_id,
            "taskVersion": exec_ctx.task_version
        });

        if let Some(job_id) = exec_ctx.job_id {
            ctx_json["jobId"] = serde_json::Value::String(job_id);
        }

        // Convert JSON to JavaScript object
        let ctx_arg = prepare_input_argument(context, &ctx_json)?;
        Some(ctx_arg)
    } else {
        None
    };

    // Execute the JavaScript function with context
    let result = if let Some(ref ctx_arg) = context_arg {
        execute_javascript_function_with_context(context, func, &input_arg, ctx_arg)?
    } else {
        execute_javascript_function(context, func, &input_arg)?
    };

    // Check if a fetch call was made
    if let Some((url, params, body)) = check_fetch_call(context)? {
        // Handle HTTP fetch processing and re-execute function
        let result = if let Some(ref ctx_arg) = context_arg {
            handle_fetch_processing_with_context(
                context,
                func,
                &input_arg,
                ctx_arg,
                http_manager,
                url,
                params,
                body,
            )
            .await?
        } else {
            handle_fetch_processing(context, func, &input_arg, http_manager, url, params, body)
                .await?
        };

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
            context
                .eval(Source::from_bytes(
                    "var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;",
                ))
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

/// Execute a task with the given input and execution context
pub async fn execute_task_with_context(
    task: &mut crate::task::Task,
    input_data: JsonValue,
    http_manager: &crate::http::HttpManager,
    execution_context: Option<crate::execution::ipc::ExecutionContext>,
) -> Result<JsonValue, JsExecutionError> {
    info!(
        "Executing task with context: {} ({})",
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
            context
                .eval(Source::from_bytes(
                    "var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;",
                ))
                .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

            debug!("Compiling JavaScript code");
            // Evaluate the JavaScript code from memory
            let func = context
                .eval(Source::from_bytes(&js_content.as_ref()))
                .map_err(|e| JsExecutionError::CompileError(e.to_string()))?;

            debug!("Calling JavaScript function with context");
            // Call the JavaScript function with the input data and execution context
            let result = call_js_function_with_context(
                &mut context,
                &func,
                &input_data,
                http_manager,
                execution_context,
            )
            .await?;

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
                "Task execution with context completed successfully: {} ({})",
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
