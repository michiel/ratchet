/// Unified error handling for both REST and GraphQL APIs
use async_graphql::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unified API error type that works for both REST and GraphQL
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// API path where error occurred
    pub path: Option<String>,
    /// Additional error details (development only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Suggestions for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: None,
            timestamp: Utc::now(),
            path: None,
            details: None,
            suggestions: None,
        }
    }
    
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
    
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = Some(suggestions);
        self
    }
    
    // Common error constructors
    
    pub fn not_found(resource: &str, id: &str) -> Self {
        Self::new(
            "NOT_FOUND",
            format!("{} with ID '{}' not found", resource, id)
        ).with_suggestions(vec![
            format!("Verify that the {} ID is correct", resource),
            format!("Check if the {} still exists", resource),
        ])
    }
    
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }
    
    pub fn validation_error(field: &str, reason: &str) -> Self {
        Self::new(
            "VALIDATION_ERROR",
            format!("Validation failed for field '{}': {}", field, reason)
        ).with_suggestions(vec![
            format!("Check the format of the '{}' field", field),
            "Refer to the API documentation for valid values".to_string(),
        ])
    }
    
    pub fn conflict(resource: &str, reason: &str) -> Self {
        Self::new(
            "CONFLICT",
            format!("{} operation failed: {}", resource, reason)
        )
    }
    
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
            .with_suggestions(vec![
                "This is likely a server issue. Please try again later".to_string(),
                "If the problem persists, contact support".to_string(),
            ])
    }
    
    pub fn unauthorized(reason: Option<&str>) -> Self {
        let message = reason.unwrap_or("Authentication required");
        Self::new("UNAUTHORIZED", message)
            .with_suggestions(vec![
                "Provide valid authentication credentials".to_string(),
                "Check if your API key or token is still valid".to_string(),
            ])
    }
    
    pub fn forbidden(reason: Option<&str>) -> Self {
        let message = reason.unwrap_or("Access denied");
        Self::new("FORBIDDEN", message)
            .with_suggestions(vec![
                "Check if you have permission to access this resource".to_string(),
                "Contact an administrator if you believe this is an error".to_string(),
            ])
    }
    
    pub fn rate_limited(retry_after: Option<u64>) -> Self {
        let message = if let Some(seconds) = retry_after {
            format!("Too many requests. Try again in {} seconds", seconds)
        } else {
            "Too many requests. Please slow down".to_string()
        };
        
        Self::new("RATE_LIMITED", message)
            .with_suggestions(vec![
                "Reduce the frequency of your requests".to_string(),
                "Implement exponential backoff in your client".to_string(),
            ])
    }
    
    pub fn service_unavailable(reason: Option<&str>) -> Self {
        let message = reason.unwrap_or("Service temporarily unavailable");
        Self::new("SERVICE_UNAVAILABLE", message)
            .with_suggestions(vec![
                "Try again in a few moments".to_string(),
                "Check the service status page for updates".to_string(),
            ])
    }
    
    pub fn timeout(operation: &str) -> Self {
        Self::new(
            "TIMEOUT",
            format!("{} operation timed out", operation)
        ).with_suggestions(vec![
            "Try again with a smaller request".to_string(),
            "Contact support if timeouts persist".to_string(),
        ])
    }
}

/// Convert ApiError to GraphQL Error
impl From<ApiError> for Error {
    fn from(api_error: ApiError) -> Self {
        let mut error = Error::new(api_error.message.clone());
        
        // Add error code as extension
        error = error.extend_with(|_, e| {
            e.set("code", api_error.code.clone());
            if let Some(request_id) = &api_error.request_id {
                e.set("requestId", request_id.clone());
            }
            if let Some(path) = &api_error.path {
                e.set("path", path.clone());
            }
            if let Some(details) = &api_error.details {
                // Convert serde_json::Value to string for GraphQL extensions
                e.set("details", details.to_string());
            }
            if let Some(suggestions) = &api_error.suggestions {
                e.set("suggestions", suggestions.clone());
            }
            e.set("timestamp", api_error.timestamp.to_rfc3339());
        });
        
        error
    }
}

/// Result type for unified error handling
pub type ApiResult<T> = Result<T, ApiError>;

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
    pub rejected_value: Option<serde_json::Value>,
}

/// Multiple validation errors
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
    pub message: String,
}

impl ValidationErrors {
    pub fn new(errors: Vec<ValidationError>) -> Self {
        let message = if errors.len() == 1 {
            format!("Validation failed for field '{}'", errors[0].field)
        } else {
            format!("Validation failed for {} fields", errors.len())
        };
        
        Self { errors, message }
    }
    
    pub fn single(field: &str, code: &str, message: &str) -> Self {
        Self::new(vec![ValidationError {
            field: field.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            rejected_value: None,
        }])
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(validation_errors: ValidationErrors) -> Self {
        ApiError::new("VALIDATION_ERROR", validation_errors.message)
            .with_details(serde_json::to_value(validation_errors.errors).unwrap())
    }
}

/// HTTP status code mapping for REST API
impl ApiError {
    pub fn http_status_code(&self) -> u16 {
        match self.code.as_str() {
            "NOT_FOUND" => 404,
            "BAD_REQUEST" | "VALIDATION_ERROR" => 400,
            "UNAUTHORIZED" => 401,
            "FORBIDDEN" => 403,
            "CONFLICT" => 409,
            "RATE_LIMITED" => 429,
            "TIMEOUT" => 408,
            "SERVICE_UNAVAILABLE" => 503,
            "INTERNAL_ERROR" => 500,
            _ => 500,
        }
    }
}