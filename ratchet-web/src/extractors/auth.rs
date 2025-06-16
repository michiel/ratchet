//! Authentication extractors for Axum

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};

use crate::{
    middleware::AuthContext,
    errors::WebError,
};

/// Auth context extractor for Axum handlers
#[async_trait]
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
            .ok_or_else(|| {
                WebError::internal("Authentication context not found. Ensure auth middleware is enabled.")
            })
    }
}