use axum::{
    extract::FromRequest,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
// use validator::{Validate, ValidationErrors};

use crate::rest::middleware::{RequestId, RestError};
use crate::rest::models::common::ApiError;

/// Validation middleware for request bodies
pub struct ValidatedJson<T>(pub T);

#[async_trait::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
{
    type Rejection = ValidationRejection;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| ValidationRejection::JsonError(err.to_string()))?;

        data.validate()
            .map_err(ValidationRejection::ValidationError)?;

        Ok(ValidatedJson(data))
    }
}

/// Request size limits
pub const MAX_JSON_BODY_SIZE: usize = 1024 * 1024; // 1MB
pub const MAX_FORM_DATA_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Validation rejection response
#[derive(Debug)]
pub enum ValidationRejection {
    JsonError(String),
    ValidationError(ValidationErrors),
    BodyTooLarge,
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            ValidationRejection::JsonError(msg) => (
                StatusCode::BAD_REQUEST,
                ApiError::bad_request(format!("Invalid JSON: {}", msg)),
            ),
            ValidationRejection::ValidationError(errors) => {
                let formatted_errors = format_validation_errors(&errors);
                let mut error = ApiError::new("Validation failed")
                    .with_code("VALIDATION_ERROR");
                
                // Add validation errors as debug info in development
                #[cfg(debug_assertions)]
                {
                    error = error.with_debug_info(formatted_errors);
                }
                
                (StatusCode::UNPROCESSABLE_ENTITY, error)
            },
            ValidationRejection::BodyTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiError::new("Request body too large")
                    .with_code("PAYLOAD_TOO_LARGE"),
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

impl fmt::Display for ValidationRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationRejection::JsonError(msg) => write!(f, "JSON error: {}", msg),
            ValidationRejection::ValidationError(errors) => {
                write!(f, "Validation error: {}", format_validation_errors(errors))
            }
            ValidationRejection::BodyTooLarge => write!(f, "Request body too large"),
        }
    }
}

impl std::error::Error for ValidationRejection {}

/// Format validation errors for user consumption
fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut formatted = Vec::new();

    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            let message = error
                .message
                .as_ref()
                .map(|msg| msg.to_string())
                .unwrap_or_else(|| format!("Invalid value for field '{}'", field));
            formatted.push(format!("{}: {}", field, message));
        }
    }

    formatted.join(", ")
}

/// Common validation rules
pub mod rules {
    use validator::ValidationError;

    /// Validate task priority (1-10)
    pub fn validate_priority(priority: &i32) -> Result<(), ValidationError> {
        if *priority < 1 || *priority > 10 {
            return Err(ValidationError::new("priority must be between 1 and 10"));
        }
        Ok(())
    }

    /// Validate delay seconds (0-86400, max 24 hours)
    pub fn validate_delay_seconds(seconds: &i32) -> Result<(), ValidationError> {
        if *seconds < 0 || *seconds > 86400 {
            return Err(ValidationError::new("delay must be between 0 and 86400 seconds"));
        }
        Ok(())
    }

    /// Validate cron expression format
    pub fn validate_cron_expression(cron: &str) -> Result<(), ValidationError> {
        // Simple validation - in production, use a proper cron parser
        let parts: Vec<&str> = cron.split_whitespace().collect();
        if parts.len() != 5 && parts.len() != 6 {
            return Err(ValidationError::new("cron expression must have 5 or 6 fields"));
        }
        Ok(())
    }

    /// Validate JSON object structure
    pub fn validate_json_object(value: &serde_json::Value) -> Result<(), ValidationError> {
        if !value.is_object() && !value.is_null() {
            return Err(ValidationError::new("value must be a JSON object or null"));
        }
        Ok(())
    }

    /// Validate URL format
    pub fn validate_url(url: &str) -> Result<(), ValidationError> {
        match url::Url::parse(url) {
            Ok(_) => Ok(()),
            Err(_) => Err(ValidationError::new("invalid URL format")),
        }
    }
}

/// Request validation models
#[derive(Debug, Deserialize, Validate)]
pub struct CreateJobRequest {
    #[validate(range(min = 1, message = "Task ID must be positive"))]
    pub task_id: i32,

    #[validate(custom = "rules::validate_priority")]
    pub priority: Option<i32>,

    #[validate(custom = "rules::validate_json_object")]
    pub input_data: Option<serde_json::Value>,

    #[validate(custom = "rules::validate_delay_seconds")]
    pub delay_seconds: Option<i32>,

    #[validate(length(max = 500, message = "Description too long"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateJobRequest {
    #[validate(custom = "rules::validate_priority")]
    pub priority: Option<i32>,

    #[validate(custom = "rules::validate_delay_seconds")]
    pub delay_seconds: Option<i32>,

    #[validate(length(max = 500, message = "Description too long"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateScheduleRequest {
    #[validate(range(min = 1, message = "Task ID must be positive"))]
    pub task_id: i32,

    #[validate(custom = "rules::validate_cron_expression")]
    pub cron_expression: String,

    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,

    #[validate(length(max = 500, message = "Description too long"))]
    pub description: Option<String>,

    #[validate(custom = "rules::validate_json_object")]
    pub input_data: Option<serde_json::Value>,

    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateScheduleRequest {
    #[validate(custom = "rules::validate_cron_expression")]
    pub cron_expression: Option<String>,

    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,

    #[validate(length(max = 500, message = "Description too long"))]
    pub description: Option<String>,

    #[validate(custom = "rules::validate_json_object")]
    pub input_data: Option<serde_json::Value>,

    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct TaskFilters {
    #[validate(length(max = 100, message = "Name filter too long"))]
    pub name: Option<String>,

    pub enabled: Option<bool>,

    pub has_validation: Option<bool>,

    #[validate(length(max = 50, message = "Version filter too long"))]
    pub version: Option<String>,
}

/// Query parameter validation
#[derive(Debug, Deserialize, Validate)]
pub struct PaginationParams {
    #[validate(range(min = 0, max = 10000, message = "Offset must be between 0 and 10000"))]
    pub offset: Option<u64>,

    #[validate(range(min = 1, max = 1000, message = "Limit must be between 1 and 1000"))]
    pub limit: Option<u64>,

    #[validate(length(max = 50, message = "Order field name too long"))]
    pub order_by: Option<String>,

    pub order_desc: Option<bool>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(25),
            order_by: None,
            order_desc: Some(false),
        }
    }
}

/// Middleware to validate query parameters
pub async fn validate_query_params<T>() -> impl Fn(axum::extract::Query<T>) -> Result<T, ValidationRejection>
where
    T: for<'de> Deserialize<'de> + Validate,
{
    |axum::extract::Query(params): axum::extract::Query<T>| {
        params
            .validate()
            .map_err(ValidationRejection::ValidationError)?;
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::response::Response;
    use axum::routing::post;
    use axum::Router;
    use serde_json::json;
    use tower::ServiceExt;

    async fn test_handler(ValidatedJson(req): ValidatedJson<CreateJobRequest>) -> impl IntoResponse {
        Json(json!({ "task_id": req.task_id }))
    }

    #[tokio::test]
    async fn test_valid_request() {
        let app = Router::new().route("/test", post(test_handler));

        let valid_request = json!({
            "task_id": 1,
            "priority": 5,
            "delay_seconds": 60
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(valid_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_request() {
        let app = Router::new().route("/test", post(test_handler));

        let invalid_request = json!({
            "task_id": -1,  // Invalid: must be positive
            "priority": 15, // Invalid: must be 1-10
            "delay_seconds": 100000 // Invalid: max 86400
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(invalid_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_validation_rules() {
        assert!(rules::validate_priority(&5).is_ok());
        assert!(rules::validate_priority(&0).is_err());
        assert!(rules::validate_priority(&11).is_err());

        assert!(rules::validate_delay_seconds(&3600).is_ok());
        assert!(rules::validate_delay_seconds(&-1).is_err());
        assert!(rules::validate_delay_seconds(&100000).is_err());

        assert!(rules::validate_cron_expression("0 0 * * *").is_ok());
        assert!(rules::validate_cron_expression("invalid").is_err());

        let valid_json = json!({"key": "value"});
        assert!(rules::validate_json_object(&valid_json).is_ok());

        let invalid_json = json!("not an object");
        assert!(rules::validate_json_object(&invalid_json).is_err());
    }
}