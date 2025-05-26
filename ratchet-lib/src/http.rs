use anyhow::Result;
use boa_engine::{
    Context, JsValue, JsError, Source,
    JsNativeError, JsResult
};
use reqwest::{self, blocking::Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use serde_json::{Value as JsonValue, json};
use std::str::FromStr;
use std::time::Duration;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use chrono::Utc;

/// HTTP Manager for handling HTTP requests with mock support
#[derive(Debug, Clone)]
pub struct HttpManager {
    offline: bool,
    mocks: HashMap<String, JsonValue>,
}

impl Default for HttpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpManager {
    /// Create a new HttpManager in online mode
    pub fn new() -> Self {
        Self {
            offline: false,
            mocks: HashMap::new(),
        }
    }
    
    /// Set offline mode
    pub fn set_offline(&mut self) {
        self.offline = true;
        debug!("HttpManager set to offline mode");
    }
    
    /// Set online mode
    pub fn set_online(&mut self) {
        self.offline = false;
        debug!("HttpManager set to online mode");
    }
    
    /// Add HTTP mocks to the manager
    /// Key should be in format "METHOD:URL" (e.g., "GET:http://example.com")
    pub fn add_mocks(&mut self, mocks: HashMap<String, JsonValue>) {
        self.mocks.extend(mocks);
        debug!("Added {} HTTP mocks", self.mocks.len());
    }
    
    /// Add a single HTTP mock
    pub fn add_mock(&mut self, method: &str, url: &str, response: JsonValue) {
        let key = format!("{}:{}", method.to_uppercase(), url);
        self.mocks.insert(key, response);
        debug!("Added HTTP mock for {} {}", method, url);
    }
    
    /// Clear all mocks
    pub fn clear_mocks(&mut self) {
        self.mocks.clear();
        debug!("Cleared all HTTP mocks");
    }
    
    /// Perform an HTTP request similar to the JavaScript fetch API
    pub fn call_http(
        &self,
        url: &str,
        params: Option<&JsonValue>,
        body: Option<&JsonValue>,
    ) -> Result<JsonValue, HttpError> {
        let start_time = Utc::now();
        
        info!("Making HTTP request to: {}", url);
        debug!("Request params: {:?}", params);
        debug!("Request body: {:?}", body);
        
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
        
        // Check if we're in offline mode and return mock data if available
        if self.offline {
            debug!("Offline mode enabled, checking for mock response");
            let mock_key = format!("{}:{}", method_str.to_uppercase(), url);
            
            if let Some(mock_response) = self.mocks.get(&mock_key) {
                debug!("Found matching mock response for {} {}", method_str, url);
                // Create a response object from the mock data
                let response = json!({
                    "ok": true,
                    "status": 200,
                    "statusText": "OK",
                    "headers": {},
                    "body": mock_response
                });
                return Ok(response);
            } else {
                // Check for partial URL matches
                for (key, response) in &self.mocks {
                    if let Some((mock_method, mock_url)) = key.split_once(':') {
                        if mock_method.eq_ignore_ascii_case(method_str) && 
                           (url.contains(mock_url) || mock_url.contains(url)) {
                            debug!("Found partial matching mock response for {} {}", method_str, url);
                            let response = json!({
                                "ok": true,
                                "status": 200,
                                "statusText": "OK",
                                "headers": {},
                                "body": response
                            });
                            return Ok(response);
                        }
                    }
                }
                
                debug!("No matching mock response found for {} {}", method_str, url);
                return Err(HttpError::InvalidUrl(
                    "No mock response available in offline mode".to_string()
                ));
            }
        }
        
        // If no mock data or mock doesn't match, perform a real HTTP request
        debug!("Creating HTTP client with 30s timeout");
        // Create a client with default settings
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        // Extract headers for recording
        let request_headers: Option<HashMap<String, String>> = if let Some(params) = params {
            if let Some(headers) = params.get("headers").and_then(|h| h.as_object()) {
                Some(headers.iter().filter_map(|(k, v)| {
                    v.as_str().map(|s| (k.clone(), s.to_string()))
                }).collect())
            } else {
                None
            }
        } else {
            None
        };
        
        // Convert body to string for recording
        let request_body_str = body.map(|b| {
            if let Some(s) = b.as_str() {
                s.to_string()
            } else {
                serde_json::to_string(b).unwrap_or_default()
            }
        });
        
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
        
        // Collect response headers for recording
        let response_headers: HashMap<String, String> = response.headers()
            .iter()
            .filter_map(|(name, value)| {
                value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
            })
            .collect();

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

        // Record the HTTP request if recording is enabled
        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;
        
        if crate::recording::is_recording() {
            let response_body_str = serde_json::to_string(&response_body).unwrap_or_default();
            if let Err(e) = crate::recording::record_http_request(
                url,
                method_str,
                request_headers.as_ref(),
                request_body_str.as_deref(),
                status_code,
                Some(&response_headers),
                &response_body_str,
                start_time,
                duration_ms,
            ) {
                warn!("Failed to record HTTP request: {}", e);
            }
        }
        
        // Construct a response object similar to JavaScript's Response
        debug!("Constructing response object");
        let result = json!({
            "ok": status.is_success(),
            "status": status_code,
            "statusText": status_text,
            "headers": response_headers,
            "body": response_body
        });

        debug!("HTTP call completed successfully");
        Ok(result)
    }
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

/// Create a default HttpManager instance for backward compatibility
pub fn create_http_manager() -> HttpManager {
    HttpManager::new()
}

/// Perform an HTTP request using a default HttpManager (for backward compatibility)
pub fn call_http(
    url: &str,
    params: Option<&JsonValue>,
    body: Option<&JsonValue>,
) -> Result<JsonValue, HttpError> {
    let manager = HttpManager::new();
    manager.call_http(url, params, body)
}

/// Deprecated: Set mock data for HTTP calls (for backward compatibility)
/// Use HttpManager with set_offline() and add_mocks() instead
#[deprecated(since = "0.1.1", note = "Use HttpManager with set_offline() and add_mocks() instead")]
pub fn set_mock_http_data(_mock_data: Option<JsonValue>) {
    // This function is kept for backward compatibility but does nothing
    // The new approach uses HttpManager instances with offline mode and mocks
    warn!("set_mock_http_data is deprecated. Use HttpManager with set_offline() and add_mocks() instead");
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
    use std::collections::HashMap;
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
    
    #[test]
    fn test_http_manager_offline_mode() {
        let mut manager = HttpManager::new();
        manager.set_offline();
        
        // Add a mock for GET request
        manager.add_mock("GET", "http://example.com/api", json!({
            "message": "Mock response",
            "id": 123
        }));
        
        // Test that the mock is returned in offline mode
        let result = manager.call_http(
            "http://example.com/api",
            Some(&json!({"method": "GET"})),
            None
        ).unwrap();
        
        assert!(result.get("ok").unwrap().as_bool().unwrap());
        assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
        
        let body = result.get("body").unwrap();
        assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Mock response");
        assert_eq!(body.get("id").unwrap().as_u64().unwrap(), 123);
    }
    
    #[test]
    fn test_http_manager_offline_mode_no_mock() {
        let mut manager = HttpManager::new();
        manager.set_offline();
        
        // Test that an error is returned when no mock is available
        let result = manager.call_http(
            "http://example.com/api",
            Some(&json!({"method": "GET"})),
            None
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_http_manager_add_mocks() {
        let mut manager = HttpManager::new();
        manager.set_offline();
        
        let mut mocks = HashMap::new();
        mocks.insert("GET:http://api1.com".to_string(), json!({"data": "response1"}));
        mocks.insert("POST:http://api2.com".to_string(), json!({"data": "response2"}));
        
        manager.add_mocks(mocks);
        
        // Test first mock
        let result1 = manager.call_http(
            "http://api1.com",
            Some(&json!({"method": "GET"})),
            None
        ).unwrap();
        
        let body1 = result1.get("body").unwrap();
        assert_eq!(body1.get("data").unwrap().as_str().unwrap(), "response1");
        
        // Test second mock
        let result2 = manager.call_http(
            "http://api2.com",
            Some(&json!({"method": "POST"})),
            None
        ).unwrap();
        
        let body2 = result2.get("body").unwrap();
        assert_eq!(body2.get("data").unwrap().as_str().unwrap(), "response2");
    }
}