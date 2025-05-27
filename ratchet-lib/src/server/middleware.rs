use axum::{
    middleware::Next,
    response::Response,
    body::Body,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

/// Request logging middleware
pub async fn logging_middleware(request: axum::http::Request<Body>, next: Next<Body>) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    info!("Request: {} {}", method, uri);
    
    let response = next.run(request).await;
    
    info!("Response: {} - {}", method, response.status());
    
    response
}

/// Create CORS layer for API
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Create trace layer for request tracing
pub fn trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
}