//! Middleware implementations for authentication, security, and request processing

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "auth")]
pub mod api_key;

pub mod security;

// Re-export commonly used middleware
#[cfg(feature = "auth")]
pub use auth::{AuthUser, JwtAuth, OptionalJwtAuth, Claims, LoginRequest, LoginResponse};

#[cfg(feature = "auth")]
pub use api_key::{ApiKeyAuth, OptionalApiKeyAuth, Auth, ApiKeyUser};

pub use security::{SecurityHeaders, ContentValidator};