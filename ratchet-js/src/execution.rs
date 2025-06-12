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
    call_js_function_with_code(context, script, None, input_data, http_manager).await
}

/// Internal function to call JavaScript with access to original code for fallback parsing
async fn call_js_function_with_code(
    context: &mut BoaContext,
    script: &Script,
    js_code: Option<&str>,
    input_data: &JsonValue,
    http_manager: &impl ratchet_http::HttpClient,
) -> Result<JsonValue, JsExecutionError> {
    // Prepare input argument
    let input_arg = prepare_input_argument(context, input_data)?;

    // Execute the script first
    let script_result = script.evaluate(context).map_err(|e| {
        let parsed_error = parse_js_error(&e.to_string());
        JsExecutionError::TypedJsError(parsed_error)
    })?;

    // Try to get the main function from the global context first
    let main_function_result = context.global_object().get(PropertyKey::from(JsString::from("main")), context);
    
    let result = if let Ok(main_fn) = main_function_result {
        // Check if main function exists and is callable
        if main_fn.is_callable() {
            debug!("Using named main function");
            // Call the main function first to allow it to set fetch variables
            let initial_result = main_fn
                .as_callable()
                .ok_or_else(|| JsExecutionError::RuntimeError("main is not a function".to_string()))?
                .call(&boa_engine::JsValue::undefined(), &[input_arg.clone()], context)
                .map_err(|e| {
                    let parsed_error = parse_js_error(&e.to_string());
                    JsExecutionError::TypedJsError(parsed_error)
                })?;

            // After function execution, check for HTTP fetch calls
            if let Some((url, params, body)) = crate::http_integration::check_fetch_call(context)? {
                debug!("Detected HTTP fetch call to: {} after function execution", url);
                let js_result = crate::http_integration::handle_fetch_processing(
                    context,
                    &main_fn,
                    &input_arg,
                    http_manager,
                    url,
                    params,
                    body,
                )
                .await?;
                convert_js_result_to_json(context, js_result)?
            } else {
                // No fetch calls detected, use the initial result
                debug!("No fetch calls detected, using initial result");
                convert_js_result_to_json(context, initial_result)?
            }
        } else {
            // main exists but is not callable, fall through to anonymous function handling
            debug!("main exists but is not callable, trying anonymous function handling");
            handle_anonymous_function_with_http(context, script_result, input_arg, js_code, http_manager).await?
        }
    } else {
        // Failed to get main function or main doesn't exist, try anonymous function handling
        debug!("Failed to get main function or main doesn't exist, trying anonymous function handling");
        handle_anonymous_function_with_http(context, script_result, input_arg, js_code, http_manager).await?
    };

    Ok(result)
}

/// Handle anonymous function execution with HTTP support
async fn handle_anonymous_function_with_http(
    context: &mut BoaContext,
    script_result: boa_engine::JsValue,
    input_arg: boa_engine::JsValue,
    js_code: Option<&str>,
    http_manager: &impl ratchet_http::HttpClient,
) -> Result<JsonValue, JsExecutionError> {
    debug!("Handling anonymous function with HTTP support. Script result type: {:?}, is_callable: {}, is_undefined: {}", 
           script_result.type_of(), script_result.is_callable(), script_result.is_undefined());
    
    if script_result.is_callable() {
        debug!("Using anonymous function result");
        // Call the function first to allow it to set fetch variables
        let initial_result = script_result
            .as_callable()
            .ok_or_else(|| JsExecutionError::RuntimeError("Script result is not callable".to_string()))?
            .call(&boa_engine::JsValue::undefined(), &[input_arg.clone()], context)
            .map_err(|e| {
                let parsed_error = parse_js_error(&e.to_string());
                JsExecutionError::TypedJsError(parsed_error)
            })?;

        // After function execution, check for HTTP fetch calls
        if let Some((url, params, body)) = crate::http_integration::check_fetch_call(context)? {
            debug!("Detected HTTP fetch call to: {} after anonymous function execution", url);
            let js_result = crate::http_integration::handle_fetch_processing(
                context,
                &script_result,
                &input_arg,
                http_manager,
                url,
                params,
                body,
            )
            .await?;
            convert_js_result_to_json(context, js_result)
        } else {
            // No fetch calls detected, use the initial result
            debug!("No fetch calls detected in anonymous function, using initial result");
            convert_js_result_to_json(context, initial_result)
        }
    } else if !script_result.is_undefined() && !script_result.is_null() {
        debug!("Using script result directly as value");
        convert_js_result_to_json(context, script_result)
    } else {
        // The script didn't return a function or value, try function expression handling
        handle_function_expression_with_http(context, input_arg, js_code, http_manager).await
    }
}

/// Handle function expression execution with HTTP support
async fn handle_function_expression_with_http(
    context: &mut BoaContext,
    input_arg: boa_engine::JsValue,
    js_code: Option<&str>,
    http_manager: &impl ratchet_http::HttpClient,
) -> Result<JsonValue, JsExecutionError> {
    debug!("Handling function expression with HTTP support");
    
    if let Some(code) = js_code {
        let wrapped_code = format!("({})", code.trim());
        
        // Try to parse and execute the wrapped code
        let wrapped_source = Source::from_bytes(&wrapped_code);
        match Script::parse(wrapped_source, None, context) {
            Ok(wrapped_script) => {
                let wrapped_result = wrapped_script.evaluate(context).map_err(|e| {
                    let parsed_error = parse_js_error(&e.to_string());
                    JsExecutionError::TypedJsError(parsed_error)
                })?;
                
                if wrapped_result.is_callable() {
                    debug!("Successfully extracted function from expression");
                    // Call the function first to allow it to set fetch variables
                    let initial_result = wrapped_result
                        .as_callable()
                        .ok_or_else(|| JsExecutionError::RuntimeError("Wrapped result is not callable".to_string()))?
                        .call(&boa_engine::JsValue::undefined(), &[input_arg.clone()], context)
                        .map_err(|e| {
                            let parsed_error = parse_js_error(&e.to_string());
                            JsExecutionError::TypedJsError(parsed_error)
                        })?;

                    // After function execution, check for HTTP fetch calls
                    if let Some((url, params, body)) = crate::http_integration::check_fetch_call(context)? {
                        debug!("Detected HTTP fetch call to: {} after function expression execution", url);
                        let js_result = crate::http_integration::handle_fetch_processing(
                            context,
                            &wrapped_result,
                            &input_arg,
                            http_manager,
                            url,
                            params,
                            body,
                        )
                        .await?;
                        convert_js_result_to_json(context, js_result)
                    } else {
                        // No fetch calls detected, use the initial result
                        debug!("No fetch calls detected in function expression, using initial result");
                        convert_js_result_to_json(context, initial_result)
                    }
                } else {
                    debug!("Wrapped result is not callable");
                    convert_js_result_to_json(context, wrapped_result)
                }
            }
            Err(e) => {
                debug!("Failed to parse wrapped function expression: {}", e);
                Err(JsExecutionError::ExecutionError(format!(
                    "Failed to parse function expression: {}",
                    e
                )))
            }
        }
    } else {
        debug!("No JavaScript code available for function expression handling");
        Err(JsExecutionError::ExecutionError(
            "Script returned undefined and no code available for re-parsing".to_string(),
        ))
    }
}

/// Handle anonymous function execution when no main function is found (legacy sync version)
fn handle_anonymous_function(
    context: &mut BoaContext,
    script_result: boa_engine::JsValue,
    input_arg: boa_engine::JsValue,
    js_code: Option<&str>,
) -> Result<JsonValue, JsExecutionError> {
    debug!("Handling anonymous function. Script result type: {:?}, is_callable: {}, is_undefined: {}", 
           script_result.type_of(), script_result.is_callable(), script_result.is_undefined());
    
    if script_result.is_callable() {
        debug!("Using anonymous function result");
        // The script itself is a function, call it with input
        let result = script_result
            .as_callable()
            .ok_or_else(|| JsExecutionError::RuntimeError("Script result is not callable".to_string()))?
            .call(&boa_engine::JsValue::undefined(), &[input_arg], context)
            .map_err(|e| {
                let parsed_error = parse_js_error(&e.to_string());
                JsExecutionError::TypedJsError(parsed_error)
            })?;

        convert_js_result_to_json(context, result)
    } else if !script_result.is_undefined() && !script_result.is_null() {
        debug!("Using script result directly as value");
        convert_js_result_to_json(context, script_result)
    } else {
        // The script didn't return a function or value, which means it might be a function expression
        // that wasn't returned. Let's try to re-execute it as a function expression.
        debug!("Script returned undefined, trying to handle as function expression");
        
        // For function expressions like (function(input) { ... }), we need to wrap them to return the function
        if let Some(code) = js_code {
            let wrapped_code = format!("({})", code.trim());
            
            // Try to parse and execute the wrapped code
            let wrapped_source = Source::from_bytes(&wrapped_code);
            match Script::parse(wrapped_source, None, context) {
                Ok(wrapped_script) => {
                    let wrapped_result = wrapped_script.evaluate(context).map_err(|e| {
                        let parsed_error = parse_js_error(&e.to_string());
                        JsExecutionError::TypedJsError(parsed_error)
                    })?;
                    
                    if wrapped_result.is_callable() {
                        debug!("Successfully extracted function from expression");
                        let result = wrapped_result
                            .as_callable()
                            .ok_or_else(|| JsExecutionError::RuntimeError("Wrapped result is not callable".to_string()))?
                            .call(&boa_engine::JsValue::undefined(), &[input_arg], context)
                            .map_err(|e| {
                                let parsed_error = parse_js_error(&e.to_string());
                                JsExecutionError::TypedJsError(parsed_error)
                            })?;

                        convert_js_result_to_json(context, result)
                    } else {
                        Err(JsExecutionError::RuntimeError(
                            "Wrapped script does not return a callable function".to_string()
                        ))
                    }
                }
                Err(_) => {
                    Err(JsExecutionError::RuntimeError(
                        "No main function found and script does not return a callable function or value. For anonymous functions, use format: (function(input) { ... }) or define a function main(input) { ... }".to_string()
                    ))
                }
            }
        } else {
            Err(JsExecutionError::RuntimeError(
                "No main function found and script does not return a callable function or value".to_string()
            ))
        }
    }
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
        call_js_function_with_code(&mut context, &script, Some(js_code), &input_data, http_manager).await?
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