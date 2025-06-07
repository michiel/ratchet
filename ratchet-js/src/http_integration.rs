use crate::JsExecutionError;
use crate::error_handling::parse_js_error;
use boa_engine::{property::PropertyKey, Context as BoaContext, JsString, Source};
use serde_json::Value as JsonValue;
use tracing::debug;

/// Check if fetch API was called and extract parameters
pub fn check_fetch_call(
    context: &mut BoaContext,
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

        Some(
            serde_json::from_str(&params_str)
                .map_err(|e| JsExecutionError::InvalidOutputFormat(e.to_string()))?,
        )
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
pub async fn handle_fetch_processing(
    context: &mut BoaContext,
    func: &boa_engine::JsValue,
    input_arg: &boa_engine::JsValue,
    http_manager: &impl ratchet_http::HttpClient,
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
        .set(
            PropertyKey::from(JsString::from("__http_result")),
            context
                .eval(Source::from_bytes(&format!(
                    "({})",
                    serde_json::to_string(&http_result).map_err(|e| {
                        JsExecutionError::ExecutionError(format!(
                            "Failed to serialize HTTP result: {}",
                            e
                        ))
                    })?
                )))
                .map_err(|e| {
                    JsExecutionError::ExecutionError(format!(
                        "Failed to parse HTTP result JSON: {}",
                        e
                    ))
                })?,
            true,
            context,
        )
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to set HTTP result: {}", e))
        })?;

    // Replace the fetch function to return the stored result and throw appropriate errors
    context
        .eval(Source::from_bytes(
            r#"
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
        "#,
        ))
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to replace fetch function: {}", e))
        })?;

    debug!("Re-calling JavaScript function with updated fetch");

    // Re-call the JavaScript function now that fetch will return the real result
    let result = func
        .as_callable()
        .ok_or_else(|| JsExecutionError::ExecutionError("Function is not callable".to_string()))?
        .call(&boa_engine::JsValue::undefined(), &[input_arg.clone()], context)
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

/// Handle HTTP fetch processing with context and re-execute function
pub async fn handle_fetch_processing_with_context(
    context: &mut BoaContext,
    func: &boa_engine::JsValue,
    input_arg: &boa_engine::JsValue,
    context_arg: &boa_engine::JsValue,
    http_manager: &impl ratchet_http::HttpClient,
    url: String,
    params: Option<JsonValue>,
    body: Option<JsonValue>,
) -> Result<boa_engine::JsValue, JsExecutionError> {
    debug!("Processing HTTP fetch request for URL: {}", url);

    // Make the actual HTTP request
    let response_result = http_manager
        .call_http(&url, params.as_ref(), body.as_ref())
        .await
        .map_err(|e| JsExecutionError::ExecutionError(format!("HTTP request failed: {}", e)))?;

    debug!("HTTP request completed, setting result");

    // Set the HTTP response result in the JavaScript context
    crate::conversion::set_js_value(
        context,
        "__http_result",
        &serde_json::to_value(response_result).map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to serialize HTTP result: {}", e))
        })?,
    )
    .map_err(|e| {
        JsExecutionError::ExecutionError(format!("Failed to parse HTTP result JSON: {}", e))
    })?;

    // Replace the fetch function to return the stored result and throw appropriate errors
    context
        .eval(Source::from_bytes(
            r#"
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
        "#,
        ))
        .map_err(|e| {
            JsExecutionError::ExecutionError(format!("Failed to replace fetch function: {}", e))
        })?;

    debug!("Re-calling JavaScript function with updated fetch and context");

    // Re-call the JavaScript function with both input and context now that fetch will return the real result
    let result = func
        .as_callable()
        .ok_or_else(|| JsExecutionError::ExecutionError("Function is not callable".to_string()))?
        .call(&boa_engine::JsValue::undefined(), &[input_arg.clone(), context_arg.clone()], context)
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
