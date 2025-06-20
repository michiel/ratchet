pub mod audit;
pub mod auth;
pub mod cors;
pub mod error_handler;
pub mod pagination;
pub mod rate_limit;
pub mod request_id;
pub mod security;
pub mod session;

// Re-export layer creation functions
pub use audit::{
    audit_layer, audit_middleware, AuditConfig, AuditEvent, AuditEventType, AuditLogger, AuditSeverity,
    TracingAuditLogger,
};
pub use auth::{
    auth_layer, auth_middleware, optional_auth_middleware, require_admin, require_auth, require_write, AuthConfig,
    AuthContext, JwtClaims, JwtManager,
};
pub use cors::cors_layer;
pub use error_handler::{error_handler_layer, handle_error, handle_not_found, internal_error};
pub use pagination::{add_pagination_headers, pagination_response_layer};
pub use rate_limit::{
    create_rate_limit_middleware, rate_limit_layer, rate_limit_middleware, ClientStats, RateLimitConfig,
    RateLimitQuota, RateLimiter, UserQuotas,
};
pub use request_id::{request_id_layer, RequestId, RequestIdExt, REQUEST_ID_HEADER};
pub use security::{security_headers_layer, security_headers_middleware, SecurityConfig, TlsConfig, TlsProtocol};
pub use session::{
    create_session_manager, session_layer, session_middleware, SessionConfig, SessionError, SessionInfo,
    SessionManager, SessionStats,
};
