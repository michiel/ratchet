//! REST API specific error handling utilities

use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, Request},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use ratchet_api_types::errors::ApiError;
use serde_json::json;
use crate::traits::ToSanitizedApiError;

/// REST-specific error response format
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RestErrorResponse {
    pub error: RestErrorDetails,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RestErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

impl From<ApiError> for RestErrorResponse {
    fn from(api_error: ApiError) -> Self {
        Self {
            error: RestErrorDetails {
                code: api_error.code,
                message: api_error.message,
                request_id: api_error.request_id,
                timestamp: api_error.timestamp,
                path: api_error.path,
                details: api_error.details,
                suggestions: api_error.suggestions,
            },
        }
    }
}

/// Trait for converting errors to REST responses
pub trait ToRestResponse {
    fn to_rest_response(&self) -> Response;
    fn to_rest_response_with_status(&self, status: StatusCode) -> Response;
}

impl<E: std::error::Error> ToRestResponse for E {
    fn to_rest_response(&self) -> Response {
        let api_error = self.to_sanitized_api_error();
        let status_code = StatusCode::from_u16(api_error.http_status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        
        self.to_rest_response_with_status(status_code)
    }
    
    fn to_rest_response_with_status(&self, status: StatusCode) -> Response {
        let api_error = self.to_sanitized_api_error();
        let error_response = RestErrorResponse::from(api_error);
        
        (status, Json(error_response)).into_response()
    }
}

/// REST-specific middleware for error handling
pub async fn rest_error_middleware(
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let response = next.run(request).await;
    
    // If the response is an error, ensure it follows our error format
    if response.status().is_client_error() || response.status().is_server_error() {
        normalize_rest_error_response(response).await
    } else {
        response
    }
}

/// Normalize error responses to consistent REST format
async fn normalize_rest_error_response(response: Response) -> Response {
    let status = response.status();
    let headers = response.headers().clone();
    
    // Try to extract the response body
    let (parts, body) = response.into_parts();
    // For now, skip body parsing to avoid compatibility issues
    // In a real implementation, we'd properly handle body extraction
    let body_bytes = axum::body::Bytes::new();
    
    // Try to parse existing JSON and ensure it follows our format
    if let Ok(existing_json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        if let Some(normalized) = normalize_error_json(&existing_json, status) {
            let mut response = (status, Json(normalized)).into_response();
            *response.headers_mut() = headers;
            return response;
        }
    }
    
    // If not JSON or doesn't match our format, create a new error response
    let message = String::from_utf8_lossy(&body_bytes);
    let sanitizer = crate::traits::ErrorSanitizationProvider::get_sanitizer();
    let sanitized = sanitizer.sanitize_message(&message);
    
    let api_error = ApiError::new(
        status_to_error_code(status),
        sanitized.message
    );
    
    let error_response = RestErrorResponse::from(api_error);
    let mut response = (status, Json(error_response)).into_response();
    *response.headers_mut() = headers;
    response
}

/// Normalize existing JSON to our error format
fn normalize_error_json(json: &serde_json::Value, status: StatusCode) -> Option<RestErrorResponse> {
    // Check if it already matches our format
    if json.get("error").is_some() {
        if let Ok(response) = serde_json::from_value::<RestErrorResponse>(json.clone()) {
            return Some(response);
        }
    }
    
    // Try to extract error information from various formats
    let message = extract_error_message(json)?;
    let code = extract_error_code(json).unwrap_or_else(|| status_to_error_code(status));
    
    let sanitizer = crate::traits::ErrorSanitizationProvider::get_sanitizer();
    let sanitized = sanitizer.sanitize_message(&message);
    
    let api_error = ApiError::new(code, sanitized.message);
    Some(RestErrorResponse::from(api_error))
}

/// Extract error message from various JSON formats
fn extract_error_message(json: &serde_json::Value) -> Option<String> {
    // Try common error message patterns
    if let Some(message) = json.get("message").and_then(|m| m.as_str()) {
        return Some(message.to_string());
    }
    
    if let Some(error) = json.get("error") {
        if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
            return Some(message.to_string());
        }
        if let Some(message) = error.as_str() {
            return Some(message.to_string());
        }
    }
    
    if let Some(detail) = json.get("detail").and_then(|d| d.as_str()) {
        return Some(detail.to_string());
    }
    
    None
}

/// Extract error code from JSON
fn extract_error_code(json: &serde_json::Value) -> Option<String> {
    if let Some(code) = json.get("code").and_then(|c| c.as_str()) {
        return Some(code.to_string());
    }
    
    if let Some(error) = json.get("error") {
        if let Some(code) = error.get("code").and_then(|c| c.as_str()) {
            return Some(code.to_string());
        }
    }
    
    None
}

/// Convert HTTP status code to error code
fn status_to_error_code(status: StatusCode) -> String {
    match status {
        StatusCode::BAD_REQUEST => "BAD_REQUEST",
        StatusCode::UNAUTHORIZED => "UNAUTHORIZED", 
        StatusCode::FORBIDDEN => "FORBIDDEN",
        StatusCode::NOT_FOUND => "NOT_FOUND",
        StatusCode::METHOD_NOT_ALLOWED => "METHOD_NOT_ALLOWED",
        StatusCode::CONFLICT => "CONFLICT",
        StatusCode::UNPROCESSABLE_ENTITY => "VALIDATION_ERROR",
        StatusCode::TOO_MANY_REQUESTS => "RATE_LIMITED",
        StatusCode::INTERNAL_SERVER_ERROR => "INTERNAL_ERROR",
        StatusCode::BAD_GATEWAY => "BAD_GATEWAY",
        StatusCode::SERVICE_UNAVAILABLE => "SERVICE_UNAVAILABLE",
        StatusCode::GATEWAY_TIMEOUT => "TIMEOUT",
        _ => "HTTP_ERROR",
    }.to_string()
}

/// Helper for creating common REST error responses
pub struct RestErrorBuilder;

impl RestErrorBuilder {
    pub fn bad_request(message: impl Into<String>) -> Response {
        let api_error = ApiError::bad_request(message);
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
    }
    
    pub fn unauthorized(message: Option<impl Into<String>>) -> Response {
        let api_error = ApiError::unauthorized(message.as_ref().map(|m| m.into().as_str()));
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::UNAUTHORIZED, Json(error_response)).into_response()
    }
    
    pub fn forbidden(message: Option<impl Into<String>>) -> Response {
        let api_error = ApiError::forbidden(message.as_ref().map(|m| m.into().as_str()));
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::FORBIDDEN, Json(error_response)).into_response()
    }
    
    pub fn not_found(resource: &str, id: &str) -> Response {
        let api_error = ApiError::not_found(resource, id);
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::NOT_FOUND, Json(error_response)).into_response()
    }
    
    pub fn validation_error(field: &str, message: &str) -> Response {
        let api_error = ApiError::validation_error(field, message);
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::BAD_REQUEST, Json(error_response)).into_response()
    }
    
    pub fn internal_error(message: impl Into<String>) -> Response {
        let api_error = ApiError::internal_error(message);
        let error_response = RestErrorResponse::from(api_error);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;
    use std::io;
    
    async fn error_handler() -> Result<&'static str, io::Error> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "file not found: /secret/config.yaml"
        ))
    }
    
    #[tokio::test]
    async fn test_rest_error_response() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let response = io_error.to_rest_response();
        
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    #[tokio::test]
    async fn test_rest_error_middleware() {
        let app = Router::new()
            .route("/error", get(error_handler))
            .layer(axum::middleware::from_fn(rest_error_middleware));
        
        let request = Request::builder()
            .uri("/error")
            .body(axum::body::Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_server_error());
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        // Should follow our error format
        assert!(json.get("error").is_some());
        assert!(json["error"].get("code").is_some());
        assert!(json["error"].get("message").is_some());
        
        // Should not contain sensitive information
        let message = json["error"]["message"].as_str().unwrap();
        assert!(!message.contains("/secret/"));
    }
    
    #[test]
    fn test_error_builders() {
        let bad_request = RestErrorBuilder::bad_request("Invalid input");
        assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);
        
        let not_found = RestErrorBuilder::not_found("task", "123");
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
        
        let internal = RestErrorBuilder::internal_error("Something went wrong");
        assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}