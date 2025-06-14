use axum::{
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use ratchet_api_types::pagination::PaginationMeta;

/// Middleware to add pagination headers to responses
pub async fn pagination_response_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    // Check if the response contains pagination metadata in extensions
    if let Some(pagination_meta) = response.extensions().get::<PaginationMeta>().cloned() {
        let headers = response.headers_mut();
        
        // Add pagination headers for Refine.dev compatibility
        for (key, value) in pagination_meta.to_headers() {
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.insert(
                    axum::http::HeaderName::from_bytes(key.as_bytes()).unwrap_or_else(|_| {
                        axum::http::HeaderName::from_static("x-custom-header")
                    }),
                    header_value,
                );
            }
        }
    }

    response
}

/// Create pagination response layer
pub fn pagination_response_layer() -> tower::layer::util::Identity {
    // For now, return identity layer - full implementation needs proper type handling
    tower::layer::util::Identity::new()
}

/// Helper to add pagination metadata to response extensions
pub fn add_pagination_headers(response: &mut Response, meta: PaginationMeta) {
    response.extensions_mut().insert(meta);
}