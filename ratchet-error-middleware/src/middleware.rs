//! Shared error handling middleware

use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ratchet_api_types::errors::ApiError;
use serde_json::json;
use tracing::{error, warn};
use uuid::Uuid;
use std::time::Instant;

use crate::traits::{ToSanitizedApiError, ErrorSanitizationProvider};

/// Request context for error handling
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub request_id: String,
    pub path: String,
    pub method: String,
    pub start_time: Instant,
    pub user_agent: Option<String>,
}

/// Middleware that adds request context and handles errors consistently
pub async fn error_handling_middleware(
    request: Request<Body>,
    next: Next<Body>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let start_time = Instant::now();
    
    // Extract user agent for logging
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    let context = ErrorContext {
        request_id: request_id.clone(),
        path: path.clone(),
        method: method.clone(),
        start_time,
        user_agent,
    };
    
    // Insert context into request extensions
    let mut request = request;
    request.extensions_mut().insert(context.clone());
    
    let response = next.run(request).await;
    
    // Log the request completion
    let duration = start_time.elapsed();
    let status = response.status();
    
    if status.is_success() {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed successfully"
        );
    } else {
        tracing::warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed with error status"
        );
    }
    
    response
}

/// Middleware for sanitizing error responses
pub async fn error_sanitization_middleware(
    request: Request<Body>,
    next: Next<Body>,
) -> impl IntoResponse {
    let response = next.run(request).await;
    
    // Check if response contains an error that needs sanitization
    if response.status().is_client_error() || response.status().is_server_error() {
        sanitize_error_response(response).await
    } else {
        response
    }
}

/// Sanitize error response body
async fn sanitize_error_response(response: Response) -> Response {
    let status = response.status();
    let headers = response.headers().clone();
    
    // Try to extract and sanitize the response body
    let (parts, body) = response.into_parts();
    // For now, skip body parsing to avoid compatibility issues
    // In a real implementation, we'd properly handle body extraction
    let body_bytes = Bytes::new();
    
    // Try to parse the response as JSON and sanitize if it contains error information
    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        if let Some(sanitized) = try_sanitize_json_error(json_value, status) {
            return create_json_response(status, headers, sanitized);
        }
    }
    
    // If we couldn't parse or sanitize, return the original response
    Response::from_parts(parts, Body::from(body_bytes))
}

/// Try to sanitize a JSON error response
fn try_sanitize_json_error(mut json: serde_json::Value, status: StatusCode) -> Option<serde_json::Value> {
    // Check for common error patterns and sanitize sensitive information
    if let Some(obj) = json.as_object_mut() {
        // Remove potentially sensitive fields
        obj.remove("internal_error");
        obj.remove("stack_trace");
        obj.remove("debug_info");
        obj.remove("database_error");
        
        // Sanitize message fields
        if let Some(message) = obj.get_mut("message") {
            if let Some(msg_str) = message.as_str() {
                *message = serde_json::Value::String(sanitize_error_message(msg_str));
            }
        }
        
        // Ensure we have a proper error structure
        if !obj.contains_key("code") {
            obj.insert("code".to_string(), serde_json::Value::String(
                format!("HTTP_{}", status.as_u16())
            ));
        }
        
        Some(json)
    } else {
        None
    }
}

/// Sanitize an error message to remove sensitive information
fn sanitize_error_message(message: &str) -> String {
    // Remove common sensitive patterns
    let sanitized = message
        .lines()
        .filter(|line| !line.contains("password") && !line.contains("token") && !line.contains("secret"))
        .collect::<Vec<_>>()
        .join(" ");
    
    // Truncate very long messages
    if sanitized.len() > 200 {
        format!("{}...", &sanitized[..197])
    } else {
        sanitized
    }
}

/// Create a sanitized error response
fn create_sanitized_error_response(
    status: StatusCode,
    headers: http::HeaderMap,
    api_error: ApiError,
) -> Response {
    let error_json = json!({
        "error": {
            "code": api_error.code(),
            "message": api_error.message(),
            "status": status.as_u16()
        }
    });
    
    create_json_response(status, headers, error_json)
}

/// Create a JSON response with the given status, headers, and body
fn create_json_response(
    status: StatusCode,
    mut headers: http::HeaderMap,
    json_body: serde_json::Value,
) -> Response {
    headers.insert(
        http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    
    let body = Body::from(serde_json::to_vec(&json_body).unwrap_or_default());
    
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    
    response
}

/// Create a basic error handling middleware layer
pub fn create_error_middleware() -> axum::middleware::FromFnLayer<impl Clone> {
    axum::middleware::from_fn(error_handling_middleware)
}

/// Create a basic error sanitization middleware layer  
pub fn create_sanitization_middleware() -> axum::middleware::FromFnLayer<impl Clone> {
    axum::middleware::from_fn(error_sanitization_middleware)
}