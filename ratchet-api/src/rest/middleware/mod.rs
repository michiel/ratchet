//! REST API middleware

use axum::{
    response::Response,
    http::Request,
    body::Body,
};
use uuid::Uuid;

/// Request ID middleware
pub async fn request_id(
    mut request: Request<Body>,
    next: axum::middleware::Next<Body>,
) -> Response {
    // Generate a unique request ID
    let request_id = Uuid::new_v4().to_string();
    
    // Add request ID to headers
    request.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    
    // Process the request
    let mut response = next.run(request).await;
    
    // Add request ID to response headers
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    
    response
}

/// Error handling middleware
pub async fn error_handler(
    request: Request<Body>,
    next: axum::middleware::Next<Body>,
) -> Response {
    // For now, just pass through
    // TODO: Implement proper error handling
    next.run(request).await
}