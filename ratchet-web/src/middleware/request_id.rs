use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, Request, request::Parts},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::Instrument;
use uuid::Uuid;

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Request ID extension that can be extracted in handlers
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Middleware to add request ID to all requests
pub async fn request_id_middleware(
    headers: HeaderMap,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Try to get request ID from incoming headers, otherwise generate one
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| RequestId::from_string(s.to_string()))
        .unwrap_or_default();

    // Store request ID in request extensions for handlers to access
    request
        .extensions_mut()
        .insert(Arc::new(request_id.clone()));

    // Add request ID to tracing span
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    async move {
        let mut response = next.run(request).await;

        // Add request ID to response headers
        if let Ok(header_value) = HeaderValue::from_str(&request_id.0) {
            response
                .headers_mut()
                .insert(REQUEST_ID_HEADER, header_value);
        }

        response
    }
    .instrument(span)
    .await
}

/// Extension trait for extracting request ID from axum requests
pub trait RequestIdExt {
    fn request_id(&self) -> Option<RequestId>;
    fn request_id_or_generate(&self) -> RequestId;
}

impl RequestIdExt for Request<axum::body::Body> {
    fn request_id(&self) -> Option<RequestId> {
        self.extensions()
            .get::<Arc<RequestId>>()
            .map(|id| id.as_ref().clone())
    }

    fn request_id_or_generate(&self) -> RequestId {
        self.request_id().unwrap_or_default()
    }
}

/// Axum extractor for request ID
impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Arc<RequestId>>()
            .map(|id| id.as_ref().clone())
            .unwrap_or_default())
    }
}

/// Create request ID middleware layer
pub fn request_id_layer() -> tower::layer::util::Identity {
    // For now, return identity layer - full implementation needs proper type handling
    tower::layer::util::Identity::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::{middleware, routing::get, Router};
    use tower::ServiceExt;

    async fn test_handler(request_id: RequestId) -> impl IntoResponse {
        (StatusCode::OK, format!("Request ID: {}", request_id))
    }

    #[tokio::test]
    async fn test_request_id_middleware_generates_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    }

    #[tokio::test]
    async fn test_request_id_middleware_preserves_existing_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let existing_id = "test-request-id-123";
        let request = Request::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, existing_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let response_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        assert_eq!(response_id.to_str().unwrap(), existing_id);
    }
}