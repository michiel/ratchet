use ratchet_http::call_http;
use boa_engine::{
    property::PropertyKey, Context, JsError, JsNativeError, JsResult, JsString, JsValue, Source,
};
use serde_json::Value as JsonValue;

/// Native function to handle fetch calls from JavaScript
#[allow(dead_code)]
async fn fetch_native(
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
        context.global_object().set(
            PropertyKey::from(JsString::from("__temp_params")),
            args[1].clone(),
            true,
            context,
        )?;

        // Stringify it using JSON.stringify
        let params_json_str = context.eval(Source::from_bytes("JSON.stringify(__temp_params)"))?;
        let params_str = params_json_str.to_string(context)?.to_std_string_escaped();

        // Parse the JSON string into a JsonValue
        match serde_json::from_str::<JsonValue>(&params_str) {
            Ok(json_val) => Some(json_val),
            Err(_) => None,
        }
    } else {
        None
    };

    // Convert body to JsonValue if provided
    let body_json = if args.len() > 2 && !args[2].is_null() && !args[2].is_undefined() {
        // Set a temporary variable in the global context
        context.global_object().set(
            PropertyKey::from(JsString::from("__temp_body")),
            args[2].clone(),
            true,
            context,
        )?;

        // Stringify it using JSON.stringify
        let body_json_str = context.eval(Source::from_bytes("JSON.stringify(__temp_body)"))?;
        let body_str = body_json_str.to_string(context)?.to_std_string_escaped();

        // Parse the JSON string into a JsonValue
        match serde_json::from_str::<JsonValue>(&body_str) {
            Ok(json_val) => Some(json_val),
            Err(_) => None,
        }
    } else {
        None
    };

    // Make the HTTP call
    let result = match call_http(&url, params_json.as_ref(), body_json.as_ref()).await {
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
    context.eval(Source::from_bytes(&format!(
        "JSON.parse('{}')",
        result_str.replace('\'', "\\'")
    )))
}

/// Register the fetch function in the JavaScript context
pub fn register_fetch(context: &mut Context) -> Result<(), JsError> {
    // Create a direct JavaScript implementation
    // that will handle the fetch API by calling into Rust
    context.eval(Source::from_bytes(
        r#"
        // Define the fetch API
        function fetch(url, params, body) {
            if (typeof url !== 'string') {
                throw new Error('URL must be a string');
            }
            
            // Convert params and body to strings so they can be parsed in Rust
            let paramsStr = params ? JSON.stringify(params) : null;
            let bodyStr = body ? (typeof body === 'string' ? body : JSON.stringify(body)) : null;
            
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
    "#,
    ))?;

    Ok(())
}
