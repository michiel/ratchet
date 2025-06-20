//! Authentication extractors for Axum

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{errors::WebError, middleware::AuthContext};

/// Auth context extractor for Axum handlers
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get auth context from request extensions
        // This should be set by the auth middleware
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or_else(|| WebError::internal("Authentication context not found. Ensure auth middleware is enabled."))
    }
}
