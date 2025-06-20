use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
// use tower::ServiceExt; // Not needed for basic error handling
use tracing::error;

use crate::errors::WebError;

/// Error handling middleware that catches and converts errors to appropriate HTTP responses
pub async fn error_handler_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    let response = next.run(request).await;

    // If the response status indicates an error, we could add additional logging here
    if response.status().is_server_error() {
        error!("Server error occurred: {}", response.status());
    }

    response
}

/// Global error handler for unhandled errors
pub async fn handle_error(err: Box<dyn std::error::Error + Send + Sync>) -> impl IntoResponse {
    error!("Unhandled error: {}", err);

    // Convert to WebError for consistent response format
    let web_error = WebError::internal(err.to_string());
    web_error.into_response()
}

/// Handle 404 errors
pub async fn handle_not_found() -> impl IntoResponse {
    WebError::not_found("The requested resource was not found").into_response()
}

/// Handle method not allowed
pub async fn handle_method_not_allowed() -> impl IntoResponse {
    (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed")
}

/// Create error handling layer
pub fn error_handler_layer() -> tower::layer::util::Identity {
    // For now, return identity layer since axum handles most error conversion automatically
    // Real implementations might use tower::ServiceBuilder to add error handling
    tower::layer::util::Identity::new()
}

/// Convenience function to convert any error to WebError
pub fn internal_error<E: std::fmt::Display>(err: E) -> WebError {
    WebError::internal(err.to_string())
}

/// Convert validation errors to WebError
pub fn validation_error(field: Option<String>, message: String) -> WebError {
    WebError::validation_single(field, message, "VALIDATION_FAILED".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    async fn error_handler() -> Result<&'static str, WebError> {
        Err(WebError::internal("Test error"))
    }

    #[tokio::test]
    async fn test_error_conversion() {
        let app = Router::new()
            .route("/error", get(error_handler))
            .layer(error_handler_layer());

        let request = axum::http::Request::builder()
            .uri("/error")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
