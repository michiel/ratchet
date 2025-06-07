//! Simplified JavaScript execution without Task dependencies

use crate::{conversion::{convert_js_result_to_json, prepare_input_argument}, error_handling::{parse_js_error, register_error_types}, JsExecutionError};
use boa_engine::{Context as BoaContext, Source, Script, property::PropertyKey, JsString};
use ratchet_core::validation::{parse_schema, validate_json};
use serde_json::Value as JsonValue;
use std::path::Path;
use tracing::{debug, info};

/// Call a JavaScript function with input data
pub async fn call_js_function(
    context: &mut BoaContext,
    script: &Script,
    input_data: &JsonValue,
    http_manager: &impl ratchet_http::HttpClient,
) -> Result<JsonValue, JsExecutionError> {
    // Prepare input argument
    let input_arg = prepare_input_argument(context, input_data)?;

    // Execute the script first to define functions
    script.evaluate(context).map_err(|e| {
        let parsed_error = parse_js_error(&e.to_string());
        JsExecutionError::TypedJsError(parsed_error)
    })?;

    // Get the main function from the global context
    let main_function = context.global_object().get(PropertyKey::from(JsString::from("main")), context).map_err(|e| {
        JsExecutionError::RuntimeError(format!("Failed to get main function: {}", e))
    })?;

    // Check for HTTP fetch calls
    let result = if let Some((url, params, body)) = crate::http_integration::check_fetch_call(context)? {
        debug!("Detected HTTP fetch call to: {}", url);
        let js_result = crate::http_integration::handle_fetch_processing(
            context,
            &main_function,
            &input_arg,
            http_manager,
            url,
            params,
            body,
        )
        .await?;
        convert_js_result_to_json(context, js_result)?
    } else {
        // Call the main function normally
        let result = main_function
            .as_callable()
            .ok_or_else(|| JsExecutionError::RuntimeError("main is not a function".to_string()))?
            .call(&boa_engine::JsValue::undefined(), &[input_arg], context)
            .map_err(|e| {
                let parsed_error = parse_js_error(&e.to_string());
                JsExecutionError::TypedJsError(parsed_error)
            })?;

        convert_js_result_to_json(context, result)?
    };

    Ok(result)
}

/// Call a JavaScript function with input data and execution context
pub async fn call_js_function_with_context(
    context: &mut BoaContext,
    script: &Script,
    input_data: &JsonValue,
    http_manager: &impl ratchet_http::HttpClient,
    execution_context: &crate::ExecutionContext,
) -> Result<JsonValue, JsExecutionError> {
    // Prepare input and context arguments
    let input_arg = prepare_input_argument(context, input_data)?;
    let context_arg = prepare_input_argument(context, &serde_json::json!({
        "executionId": execution_context.execution_id,
        "taskId": execution_context.task_id,
        "taskVersion": execution_context.task_version,
        "jobId": execution_context.job_id
    }))?;

    // Execute the script first to define functions
    script.evaluate(context).map_err(|e| {
        let parsed_error = parse_js_error(&e.to_string());
        JsExecutionError::TypedJsError(parsed_error)
    })?;

    // Get the main function from the global context
    let main_function = context.global_object().get(PropertyKey::from(JsString::from("main")), context).map_err(|e| {
        JsExecutionError::RuntimeError(format!("Failed to get main function: {}", e))
    })?;

    // Check for HTTP fetch calls
    let result = if let Some((url, params, body)) = crate::http_integration::check_fetch_call(context)? {
        debug!("Detected HTTP fetch call to: {}", url);
        let js_result = crate::http_integration::handle_fetch_processing_with_context(
            context,
            &main_function,
            &input_arg,
            &context_arg,
            http_manager,
            url,
            params,
            body,
        )
        .await?;
        convert_js_result_to_json(context, js_result)?
    } else {
        // Call the main function with both input and context
        let result = main_function
            .as_callable()
            .ok_or_else(|| JsExecutionError::RuntimeError("main is not a function".to_string()))?
            .call(&boa_engine::JsValue::undefined(), &[input_arg, context_arg], context)
            .map_err(|e| {
                let parsed_error = parse_js_error(&e.to_string());
                JsExecutionError::TypedJsError(parsed_error)
            })?;

        convert_js_result_to_json(context, result)?
    };

    Ok(result)
}

/// Execute JavaScript file with input data
pub async fn execute_js_file(
    js_file_path: &Path,
    input_data: JsonValue,
    input_schema_path: &Path,
    output_schema_path: &Path,
    http_manager: &impl ratchet_http::HttpClient,
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

    execute_js_with_content(
        &js_code,
        input_data,
        Some(&input_schema),
        Some(&output_schema),
        http_manager,
        None,
    )
    .await
}

/// Execute JavaScript code with content directly (without file system)
pub async fn execute_js_with_content(
    js_code: &str,
    input_data: JsonValue,
    input_schema: Option<&JsonValue>,
    output_schema: Option<&JsonValue>,
    http_manager: &impl ratchet_http::HttpClient,
    execution_context: Option<&crate::ExecutionContext>,
) -> Result<JsonValue, JsExecutionError> {
    info!("Executing JavaScript code directly");
    debug!(
        "Input data: {}",
        serde_json::to_string(&input_data).unwrap_or_else(|_| "<invalid json>".to_string())
    );

    // Validate input against schema if provided
    if let Some(schema) = input_schema {
        debug!("Validating input against schema");
        validate_json(&input_data, schema)?;
    }

    debug!("Creating JavaScript execution context");
    // Create a new Boa context for JavaScript execution
    let mut context = BoaContext::default();

    debug!("Registering error types");
    // Register custom error types
    register_error_types(&mut context)?;

    debug!("Registering fetch API");
    // Register the fetch API
    #[cfg(feature = "http")]
    crate::fetch::register_fetch(&mut context).map_err(|e| {
        JsExecutionError::ExecutionError(format!("Failed to register fetch API: {}", e))
    })?;

    debug!("Compiling JavaScript code");
    // Parse and compile the JavaScript code
    let source = Source::from_bytes(js_code);
    let script = Script::parse(source, None, &mut context).map_err(|e| {
        JsExecutionError::CompilationError(format!("Compilation failed: {}", e))
    })?;

    debug!("Calling JavaScript function");
    // Call the JavaScript function with the input data and execution context
    let result = if let Some(exec_ctx) = execution_context {
        call_js_function_with_context(&mut context, &script, &input_data, http_manager, exec_ctx).await?
    } else {
        call_js_function(&mut context, &script, &input_data, http_manager).await?
    };

    // Validate output against schema if provided
    if let Some(schema) = output_schema {
        debug!("Validating output against schema");
        validate_json(&result, schema)?;
    }

    info!("JavaScript code execution completed successfully");
    debug!(
        "Output data: {}",
        serde_json::to_string(&result).unwrap_or_else(|_| "<invalid json>".to_string())
    );

    Ok(result)
}