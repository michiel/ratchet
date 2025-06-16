pub mod auth;
pub mod cors;
pub mod error_handler;
pub mod pagination;
pub mod rate_limit;
pub mod request_id;

// Re-export layer creation functions
pub use auth::{
    auth_layer, auth_middleware, optional_auth_middleware, require_admin, require_auth,
    require_write, AuthConfig, AuthContext, JwtClaims, JwtManager,
};
pub use cors::cors_layer;
pub use error_handler::{error_handler_layer, handle_error, handle_not_found, internal_error};
pub use pagination::{pagination_response_layer, add_pagination_headers};
pub use rate_limit::{rate_limit_layer, RateLimitConfig};
pub use request_id::{request_id_layer, RequestId, RequestIdExt, REQUEST_ID_HEADER};