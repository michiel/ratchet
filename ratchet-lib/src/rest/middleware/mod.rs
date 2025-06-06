pub mod cors;
pub mod error_handler;
pub mod pagination;
pub mod request_id;
// pub mod validation;
pub mod rate_limit;

pub use cors::cors_layer;
pub use error_handler::{handle_error, RestError};
pub use pagination::{add_pagination_headers, WithPaginationHeaders};
pub use request_id::{request_id_middleware, RequestId, RequestIdExt};
// pub use validation::{ValidatedJson, ValidationRejection, rules};
pub use rate_limit::{
    create_rate_limit_layer, rate_limit_middleware, rate_limit_middleware_with_state,
    RateLimitConfig, RateLimiter,
};
