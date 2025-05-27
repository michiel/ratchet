use super::*;
use crate::types::HttpMethod;
use boa_engine::{Context, Source};
use serde_json::json;
// use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::HashMap;
// use std::time::Duration;
// Temporarily disabled due to axum version compatibility issues
// use axum::{
//     routing::{get, post},
//     Router,
//     http::StatusCode,
//     response::Json,
//     extract::State,
// };
// use tower::ServiceBuilder;
// use tower_http::trace::TraceLayer;

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
#[allow(dead_code)]
struct AppState {
    requests: Arc<Mutex<Vec<String>>>,
}

// Helper function to start a mock HTTP server for testing
// Temporarily disabled due to axum compatibility issues
#[allow(dead_code)]
fn start_mock_server(_port: u16) -> (thread::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
    // Disabled implementation
    let requests = Arc::new(Mutex::new(Vec::new()));
    let handle = thread::spawn(|| {});
    (handle, requests)
}

/*
#[allow(dead_code)]
fn _start_mock_server_disabled(port: u16) -> (thread::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
    // Create a shared vector to record requests
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = AppState {
        requests: requests.clone(),
    };

    // Define the Echo handler for POST requests
    async fn echo_handler(
        State(state): State<AppState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        // Record the request
        if let Ok(mut req_list) = state.requests.lock() {
            req_list.push(payload.to_string());
        }
        
        // Return the payload as-is
        Json(payload)
    }

    // Define the JSON handler for GET requests
    async fn json_handler() -> Json<serde_json::Value> {
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
*/

// Temporarily disabled due to axum compatibility issues
#[ignore]
#[tokio::test]
async fn test_call_http_get_json() {
    let (_server, _requests) = start_mock_server(3030);

    // Test a GET request to JSON endpoint
    let result = call_http(
        "http://localhost:3030/json", 
        Some(&json!({"method": "GET"})),
        None
    ).await.unwrap();

    assert!(result.get("ok").unwrap().as_bool().unwrap());
    assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
    assert!(result.get("body").is_some());
    
    let body = result.get("body").unwrap();
    assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Hello, World!");
    assert_eq!(body.get("status").unwrap().as_str().unwrap(), "success");
}

// Temporarily disabled due to axum compatibility issues  
#[ignore]
#[tokio::test]
async fn test_call_http_post() {
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
    ).await.unwrap();

    assert!(result.get("ok").unwrap().as_bool().unwrap());
    assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
    
    // Verify the request was recorded correctly
    let recorded_requests = requests.lock().unwrap();
    assert!(!recorded_requests.is_empty());
    
    // The request should match our test body
    let request_json: serde_json::Value = serde_json::from_str(&recorded_requests[0]).unwrap();
    assert_eq!(request_json.get("name").unwrap().as_str().unwrap(), "Test User");
    assert_eq!(request_json.get("email").unwrap().as_str().unwrap(), "test@example.com");
}

// Temporarily disabled due to axum compatibility issues
#[ignore]  
#[tokio::test]
async fn test_js_fetch_integration() {
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
    let params: Option<serde_json::Value> = Some(serde_json::from_str(&params_str).unwrap());
    
    // Make the HTTP call
    let result = call_http(&url, params.as_ref(), None).await.unwrap();
    
    // Verify the result
    assert!(result.get("ok").unwrap().as_bool().unwrap());
    assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
    
    let body = result.get("body").unwrap();
    assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Hello, World!");
}

#[tokio::test]
async fn test_http_manager_offline_mode() {
    let mut manager = HttpManager::new();
    manager.set_offline();
    
    // Add a mock for GET request
    manager.add_mock(HttpMethod::Get, "http://example.com/api", json!({
        "message": "Mock response",
        "id": 123
    }));
    
    // Test that the mock is returned in offline mode
    let result = manager.call_http(
        "http://example.com/api",
        Some(&json!({"method": "GET"})),
        None
    ).await.unwrap();
    
    assert!(result.get("ok").unwrap().as_bool().unwrap());
    assert_eq!(result.get("status").unwrap().as_u64().unwrap(), 200);
    
    let body = result.get("body").unwrap();
    assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Mock response");
    assert_eq!(body.get("id").unwrap().as_u64().unwrap(), 123);
}

#[tokio::test]
async fn test_http_manager_offline_mode_no_mock() {
    let mut manager = HttpManager::new();
    manager.set_offline();
    
    // Test that an error is returned when no mock is available
    let result = manager.call_http(
        "http://example.com/api",
        Some(&json!({"method": "GET"})),
        None
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_manager_add_mocks() {
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
    ).await.unwrap();
    
    let body1 = result1.get("body").unwrap();
    assert_eq!(body1.get("data").unwrap().as_str().unwrap(), "response1");
    
    // Test second mock
    let result2 = manager.call_http(
        "http://api2.com",
        Some(&json!({"method": "POST"})),
        None
    ).await.unwrap();
    
    let body2 = result2.get("body").unwrap();
    assert_eq!(body2.get("data").unwrap().as_str().unwrap(), "response2");
}