use axum::http::{HeaderName, Method};
use tower_http::cors::{Any, CorsLayer};

/// Create CORS layer for REST API
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .expose_headers([
            HeaderName::from_static("x-total-count"),
            HeaderName::from_static("content-range"),
        ])
}