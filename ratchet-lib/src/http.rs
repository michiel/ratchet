use anyhow::Result;
use boa_engine::{
    Context, JsValue, JsError, Source,
    JsNativeError, JsResult
};
use reqwest::{self, blocking::Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde_json::{Value as JsonValue, json};
use std::str::FromStr;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::RefCell;
use lazy_static::lazy_static;
use tracing::{debug, info, warn};

// Thread-local storage for mock data during tests
thread_local! {
    static MOCK_HTTP_DATA: RefCell<Option<JsonValue>> = RefCell::new(None);
}

// Global flag to indicate if we're in mock mode
lazy_static! {
    static ref MOCK_ENABLED: AtomicBool = AtomicBool::new(false);
}

/// Set mock data for HTTP calls
pub fn set_mock_http_data(mock_data: Option<JsonValue>) {
    let has_mock = mock_data.is_some();
    if has_mock {
        debug!("Enabling HTTP mock mode");
    } else {
        debug!("Disabling HTTP mock mode");
    }
    MOCK_HTTP_DATA.with(|cell| {
        *cell.borrow_mut() = mock_data;
    });
    MOCK_ENABLED.store(has_mock, Ordering::SeqCst);
}

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
#[allow(dead_code)]
fn js_error_to_http_error(err: JsError) -> HttpError {
    HttpError::JsError(err.to_string())
}

/// Perform an HTTP request similar to the JavaScript fetch API
pub fn call_http(
    url: &str,
    params: Option<&JsonValue>,
    body: Option<&JsonValue>,
) -> Result<JsonValue, HttpError> {
    info!("Making HTTP request to: {}", url);
    debug!("Request params: {:?}", params);
    debug!("Request body: {:?}", body);
    
    // Check if we're in mock mode and return mock data if available
    if MOCK_ENABLED.load(Ordering::SeqCst) {
        debug!("Mock mode enabled, checking for mock response");
        let mut mock_response = None;
        
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
        
        // Get the mock data from thread-local storage
        MOCK_HTTP_DATA.with(|cell| {
            if let Some(mock_data) = &*cell.borrow() {
                // Check if we have HTTP mock data
                if let Some(http_mock) = mock_data.get("http") {
                    // Check if URL and method match
                    let mock_url = http_mock.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let mock_method = http_mock.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
                    
                    // If the URL and method match, use the mock response
                    if (mock_url.is_empty() || url.contains(mock_url)) && 
                       (mock_method.eq_ignore_ascii_case(method_str)) {
                        if let Some(response) = http_mock.get("response") {
                            debug!("Found matching mock response for {} {}", method_str, url);
                            // Create a response object from the mock data
                            mock_response = Some(json!({
                                "ok": true,
                                "status": 200,
                                "statusText": "OK",
                                "headers": {},
                                "body": response
                            }));
                        }
                    }
                }
            }
        });
        
        if let Some(response) = mock_response {
            debug!("Returning mock HTTP response");
            return Ok(response);
        } else {
            debug!("No matching mock response found, proceeding with real HTTP call");
        }
    }
    
    // If no mock data or mock doesn't match, perform a real HTTP request
    debug!("Creating HTTP client with 30s timeout");
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

    debug!("Building {} request to {}", method_str, url);
    // Build the request
    let mut request = client.request(method, url);

    // Add headers if provided
    if let Some(params) = params {
        if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
            debug!("Adding {} custom headers", headers.len());
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
        // Check if the Content-Type header indicates form data
        let is_form_data = if let Some(params) = params {
            if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
                headers.get("Content-Type")
                    .and_then(|ct| ct.as_str())
                    .map(|ct| ct.contains("application/x-www-form-urlencoded"))
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        };
        
        if is_form_data {
            // Send as form data if body is a string
            if let Some(body_str) = body.as_str() {
                debug!("Adding form-encoded body to request");
                request = request.body(body_str.to_string());
            } else {
                debug!("Adding JSON body to request (form data expected but body is not string)");
                request = request.json(body);
            }
        } else {
            debug!("Adding JSON body to request");
            request = request.json(body);
        }
    }

    // Send the request and get the response
    debug!("Sending HTTP request");
    let response = request.send()?;
    
    // Get status info
    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown Status");
    
    info!("HTTP response received: {} {}", status_code, status_text);

    // Try to parse the response as JSON, fall back to text if it fails
    debug!("Parsing response body");
    let response_body = match response.json::<JsonValue>() {
        Ok(json_data) => {
            debug!("Successfully parsed response as JSON");
            json_data
        },
        Err(_) => {
            warn!("Failed to parse response as JSON, falling back to text");
            // Fall back to text - we need to send a new request since json() consumes the response
            let text_response = client.request(Method::from_str(method_str).unwrap(), url).send()?;
            let text = text_response.text()?;
            debug!("Response parsed as text: {} bytes", text.len());
            json!(text)
        }
    };

    // Construct a response object similar to JavaScript's Response
    debug!("Constructing response object");
    let result = json!({
        "ok": status.is_success(),
        "status": status_code,
        "statusText": status_text,
        "headers": {},  // We could parse headers here if needed
        "body": response_body
    });

    debug!("HTTP call completed successfully");
    Ok(result)
}

/// Native function to handle fetch calls from JavaScript
#[allow(dead_code)]
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
    "#))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use axum::{
        routing::{get, post},
        Router,
        http::StatusCode,
        response::Json,
        extract::State,
    };
    use tower::ServiceBuilder;
    use tower_http::trace::TraceLayer;

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

    // Define the shared state for our test server
    #[derive(Clone)]
    struct AppState {
        requests: Arc<Mutex<Vec<String>>>,
    }

    // Helper function to start a mock HTTP server for testing
    fn start_mock_server(port: u16) -> (thread::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
        // Create a shared vector to record requests
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = AppState {
            requests: requests.clone(),
        };

        // Define the Echo handler for POST requests
        async fn echo_handler(
            State(state): State<AppState>,
            Json(payload): Json<JsonValue>,
        ) -> Json<JsonValue> {
            // Record the request
            if let Ok(mut req_list) = state.requests.lock() {
                req_list.push(payload.to_string());
            }
            
            // Return the payload as-is
            Json(payload)
        }

        // Define the JSON handler for GET requests
        async fn json_handler() -> Json<JsonValue> {
            Json(json!({
                "message": "Hello, World!",
                "status": "success"
            }))
        }

        // Define the Text handler for GET requests
        async fn text_handler() -> (StatusCode, &'static str) {
            (StatusCode::OK, "Plain text response")
        }

        // Create the router with our routes
        let app = Router::new()
            .route("/", post(echo_handler))
            .route("/json", get(json_handler))
            .route("/text", get(text_handler))
            .with_state(state)
            .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        
        // Start the server in a separate thread
        let server = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        });

        // Give the server a moment to start
        thread::sleep(Duration::from_millis(100));

        (server, requests)
    }

    #[test]
    fn test_call_http_get_json() {
        let (_server, _requests) = start_mock_server(3030);

        // Test a GET request to JSON endpoint
        let result = call_http(
            "http://localhost:3030/json", 
            Some(&json!({"method": "GET"})),
            None
        ).unwrap();

        assert!(result.get("ok").unwrap().as_bool().unwrap());
        assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
        assert!(result.get("body").is_some());
        
        let body = result.get("body").unwrap();
        assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Hello, World!");
        assert_eq!(body.get("status").unwrap().as_str().unwrap(), "success");
    }

    #[test]
    fn test_call_http_post() {
        let (_server, requests) = start_mock_server(3031);
        
        // Test a POST request with a JSON body
        let test_body = json!({
            "name": "Test User",
            "email": "test@example.com"
        });

        let result = call_http(
            "http://localhost:3031/", 
            Some(&json!({"method": "POST"})),
            Some(&test_body)
        ).unwrap();

        assert!(result.get("ok").unwrap().as_bool().unwrap());
        assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
        
        // Verify the request was recorded correctly
        let recorded_requests = requests.lock().unwrap();
        assert!(!recorded_requests.is_empty());
        
        // The request should match our test body
        let request_json: JsonValue = serde_json::from_str(&recorded_requests[0]).unwrap();
        assert_eq!(request_json.get("name").unwrap().as_str().unwrap(), "Test User");
        assert_eq!(request_json.get("email").unwrap().as_str().unwrap(), "test@example.com");
    }

    #[test]
    fn test_js_fetch_integration() {
        let (_server, _requests) = start_mock_server(3032);
        
        // Create a JavaScript context
        let mut context = Context::default();
        
        // Register the fetch API
        register_fetch(&mut context).unwrap();
        
        // Initialize fetch variables
        context.eval(Source::from_bytes("var __fetch_url = null; var __fetch_params = null; var __fetch_body = null;"))
            .unwrap();
        
        // Create a JavaScript fetch call
        context.eval(Source::from_bytes(r#"
            function testFetch() {
                return fetch("http://localhost:3032/json", { method: "GET" });
            }
            
            // Call the function to set up the fetch parameters
            testFetch();
        "#)).unwrap();
        
        // Verify that the fetch variables were set correctly
        let url = context.eval(Source::from_bytes("__fetch_url")).unwrap();
        assert_eq!(url.to_string(&mut context).unwrap().to_std_string().unwrap(), "http://localhost:3032/json");
        
        let params = context.eval(Source::from_bytes("__fetch_params")).unwrap();
        assert!(!params.is_null());
        
        // Now we would need to extract the parameters and make the actual HTTP call,
        // which is what happens in the execute_task function in lib.rs
        
        // Extract URL
        let url_js = context.eval(Source::from_bytes("__fetch_url")).unwrap();
        let url = url_js.to_string(&mut context).unwrap().to_std_string().unwrap();
        
        // Extract params
        let params_js = context.eval(Source::from_bytes("__fetch_params")).unwrap();
        let params_str = params_js.to_string(&mut context).unwrap().to_std_string().unwrap();
        let params: Option<JsonValue> = Some(serde_json::from_str(&params_str).unwrap());
        
        // Make the HTTP call
        let result = call_http(&url, params.as_ref(), None).unwrap();
        
        // Verify the result
        assert!(result.get("ok").unwrap().as_bool().unwrap());
        assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
        
        let body = result.get("body").unwrap();
        assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Hello, World!");
    }
}