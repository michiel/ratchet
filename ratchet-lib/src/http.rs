use anyhow::Result;
use boa_engine::{
    Context, JsValue, JsError, Source,
    JsNativeError, JsResult
};
use reqwest::{self, blocking::Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde_json::{Value as JsonValue, json};
use std::str::FromStr;
use std::time::Duration;

/// Error type for HTTP operations
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid header name: {0}")]
    InvalidHeaderName(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("JavaScript error: {0}")]
    JsError(String),
}

/// Convert a JS error to an HttpError
fn js_error_to_http_error(err: JsError) -> HttpError {
    HttpError::JsError(err.to_string())
}

/// Perform an HTTP request similar to the JavaScript fetch API
pub fn call_http(
    url: &str,
    params: Option<&JsonValue>,
    body: Option<&JsonValue>,
) -> Result<JsonValue, HttpError> {
    // Create a client with default settings
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Extract method from params or default to GET
    let method_str = if let Some(params) = params {
        if let Some(method_str) = params.get("method").and_then(|m| m.as_str()) {
            method_str
        } else {
            "GET"
        }
    } else {
        "GET"
    };
    
    let method = Method::from_str(method_str)
        .map_err(|_| HttpError::InvalidMethod(method_str.to_string()))?;

    // Build the request
    let mut request = client.request(method, url);

    // Add headers if provided
    if let Some(params) = params {
        if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
            let mut header_map = HeaderMap::new();
            for (key, value) in headers {
                if let Some(value_str) = value.as_str() {
                    let header_name = HeaderName::from_str(key)
                        .map_err(|_| HttpError::InvalidHeaderName(key.to_string()))?;
                    
                    if let Ok(header_value) = HeaderValue::from_str(value_str) {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
            request = request.headers(header_map);
        }
    }

    // Add body if provided
    if let Some(body) = body {
        request = request.json(body);
    }

    // Send the request and get the response
    let response = request.send()?;
    
    // Get status info
    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown Status");

    // Try to parse the response as JSON, fall back to text if it fails
    let response_body = match response.json::<JsonValue>() {
        Ok(json_data) => json_data,
        Err(_) => {
            // Fall back to text - we need to send a new request since json() consumes the response
            let text_response = client.request(Method::from_str(method_str).unwrap(), url).send()?;
            let text = text_response.text()?;
            json!(text)
        }
    };

    // Construct a response object similar to JavaScript's Response
    let result = json!({
        "ok": status.is_success(),
        "status": status_code,
        "statusText": status_text,
        "headers": {},  // We could parse headers here if needed
        "body": response_body
    });

    Ok(result)
}

/// Native function to handle fetch calls from JavaScript
fn fetch_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    // Extract arguments
    let url = if args.is_empty() {
        return Err(JsNativeError::error()
            .with_message("URL parameter is required")
            .into());
    } else {
        if !args[0].is_string() {
            return Err(JsNativeError::error()
                .with_message("URL must be a string")
                .into());
        }
        args[0].to_string(context)?.to_std_string_escaped()
    };

    // Convert params to JsonValue if provided
    let params_json = if args.len() > 1 && !args[1].is_null() && !args[1].is_undefined() {
        // Set a temporary variable in the global context
        context.global_object().set("__temp_params", args[1].clone(), true, context)?;
        
        // Stringify it using JSON.stringify
        let params_json_str = context.eval(Source::from_bytes("JSON.stringify(__temp_params)"))?;
        let params_str = params_json_str.to_string(context)?.to_std_string_escaped();
        
        // Parse the JSON string into a JsonValue
        match serde_json::from_str::<JsonValue>(&params_str) {
            Ok(json_val) => Some(json_val),
            Err(_) => None
        }
    } else {
        None
    };

    // Convert body to JsonValue if provided
    let body_json = if args.len() > 2 && !args[2].is_null() && !args[2].is_undefined() {
        // Set a temporary variable in the global context
        context.global_object().set("__temp_body", args[2].clone(), true, context)?;
        
        // Stringify it using JSON.stringify
        let body_json_str = context.eval(Source::from_bytes("JSON.stringify(__temp_body)"))?;
        let body_str = body_json_str.to_string(context)?.to_std_string_escaped();
        
        // Parse the JSON string into a JsonValue
        match serde_json::from_str::<JsonValue>(&body_str) {
            Ok(json_val) => Some(json_val),
            Err(_) => None
        }
    } else {
        None
    };

    // Make the HTTP call
    let result = match call_http(&url, params_json.as_ref(), body_json.as_ref()) {
        Ok(result) => result,
        Err(e) => {
            return Err(JsNativeError::error()
                .with_message(format!("HTTP error: {}", e))
                .into());
        }
    };

    // Convert the result back to a JS value
    let result_str = match serde_json::to_string(&result) {
        Ok(s) => s,
        Err(e) => {
            return Err(JsNativeError::error()
                .with_message(format!("Failed to serialize result: {}", e))
                .into());
        }
    };

    // Parse the JSON string into a JS value
    context.eval(Source::from_bytes(&format!("JSON.parse('{}')", result_str.replace('\'', "\\'")))) 
}

/// Register the fetch function in the JavaScript context
pub fn register_fetch(context: &mut Context) -> Result<(), JsError> {
    // Create a direct JavaScript implementation
    // that will handle the fetch API by calling into Rust
    context.eval(Source::from_bytes(r#"
        // Define the fetch API
        function fetch(url, params, body) {
            if (typeof url !== 'string') {
                throw new Error('URL must be a string');
            }
            
            // Convert params and body to strings so they can be parsed in Rust
            let paramsStr = params ? JSON.stringify(params) : null;
            let bodyStr = body ? JSON.stringify(body) : null;
            
            // This will be processed by the Rust function call_http directly
            // We'll capture these values in the execute_js_file function
            __fetch_url = url;
            __fetch_params = paramsStr;
            __fetch_body = bodyStr;
            
            // Return a dummy response - we'll replace this with the actual HTTP call in Rust
            return { 
                _internal_fetch_call: true,
                url: url,
                params: params,
                body: body
            };
        }
    "#))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_fetch() {
        let mut context = Context::default();
        let result = register_fetch(&mut context);
        assert!(result.is_ok());

        // Verify that fetch is now defined in the global scope
        let is_fetch_defined = context
            .eval(Source::from_bytes("typeof fetch === 'function'"))
            .unwrap();
        assert!(is_fetch_defined.as_boolean().unwrap());
    }

    // Note: More tests would typically include mocking HTTP requests
    // to avoid external dependencies during testing
}