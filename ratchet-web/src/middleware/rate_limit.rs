use axum::{extract::ConnectInfo, http::Request, middleware::Next, response::Response};
use chrono::{DateTime, Utc};
use lru::LruCache;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::errors::WebError;
use crate::middleware::{AuditEvent, AuditEventType, AuditLogger, AuditSeverity, AuthContext, TracingAuditLogger};

/// User role-based rate limit quotas
#[derive(Debug, Clone)]
pub struct UserQuotas {
    /// Default quota for unauthenticated users (IP-based)
    pub anonymous: RateLimitQuota,
    /// Quota for authenticated regular users
    pub user: RateLimitQuota,
    /// Quota for admin users
    pub admin: RateLimitQuota,
    /// Quota for read-only users
    pub readonly: RateLimitQuota,
    /// Quota for service API keys
    pub service: RateLimitQuota,
}

impl Default for UserQuotas {
    fn default() -> Self {
        Self {
            anonymous: RateLimitQuota {
                requests_per_minute: 30,
                burst_size: 5,
                daily_limit: Some(1000),
            },
            user: RateLimitQuota {
                requests_per_minute: 120,
                burst_size: 20,
                daily_limit: Some(10000),
            },
            admin: RateLimitQuota {
                requests_per_minute: 300,
                burst_size: 50,
                daily_limit: Some(50000),
            },
            readonly: RateLimitQuota {
                requests_per_minute: 60,
                burst_size: 10,
                daily_limit: Some(5000),
            },
            service: RateLimitQuota {
                requests_per_minute: 600,
                burst_size: 100,
                daily_limit: None, // No daily limit for services
            },
        }
    }
}

/// Individual rate limit quota
#[derive(Debug, Clone)]
pub struct RateLimitQuota {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub daily_limit: Option<u32>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// User-based quotas
    pub quotas: UserQuotas,
    /// Window size for rate limiting
    pub window_size: Duration,
    /// Cleanup interval for old client data
    pub cleanup_interval: Duration,
    /// Maximum number of clients to track
    pub max_clients: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            quotas: UserQuotas::default(),
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_clients: 10000,
        }
    }
}

impl RateLimitConfig {
    /// Create a permissive configuration for development
    pub fn permissive() -> Self {
        let mut config = Self::default();
        config.quotas = UserQuotas {
            anonymous: RateLimitQuota {
                requests_per_minute: 300,
                burst_size: 50,
                daily_limit: Some(10000),
            },
            user: RateLimitQuota {
                requests_per_minute: 600,
                burst_size: 100,
                daily_limit: Some(50000),
            },
            admin: RateLimitQuota {
                requests_per_minute: 1200,
                burst_size: 200,
                daily_limit: None,
            },
            readonly: RateLimitQuota {
                requests_per_minute: 300,
                burst_size: 50,
                daily_limit: Some(25000),
            },
            service: RateLimitQuota {
                requests_per_minute: 1800,
                burst_size: 300,
                daily_limit: None,
            },
        };
        config
    }

    /// Create a strict configuration for production
    pub fn strict() -> Self {
        let mut config = Self::default();
        config.quotas = UserQuotas {
            anonymous: RateLimitQuota {
                requests_per_minute: 15,
                burst_size: 3,
                daily_limit: Some(500),
            },
            user: RateLimitQuota {
                requests_per_minute: 60,
                burst_size: 10,
                daily_limit: Some(5000),
            },
            admin: RateLimitQuota {
                requests_per_minute: 150,
                burst_size: 25,
                daily_limit: Some(25000),
            },
            readonly: RateLimitQuota {
                requests_per_minute: 30,
                burst_size: 5,
                daily_limit: Some(2500),
            },
            service: RateLimitQuota {
                requests_per_minute: 300,
                burst_size: 50,
                daily_limit: Some(100000),
            },
        };
        config
    }

    /// Disable rate limiting entirely
    pub fn disabled() -> Self {
        let mut config = Self::default();
        config.enabled = false;
        config
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens as f64,
            last_refill: Instant::now(),
            max_tokens: max_tokens as f64,
            refill_rate,
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let tokens_to_add = elapsed * self.refill_rate;
        self.tokens = (self.tokens + tokens_to_add).min(self.max_tokens);
        self.last_refill = now;
    }

    fn remaining_tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }

    // For cases where we need immutable access
    fn remaining_tokens_immutable(&self) -> f64 {
        let mut cloned = self.clone();
        cloned.refill();
        cloned.tokens
    }

    fn time_until_available(&mut self) -> Duration {
        self.refill();

        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Client type for quota selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientType {
    Anonymous,
    User(String),     // user_id
    Admin(String),    // user_id
    Readonly(String), // user_id
    Service(String),  // api_key
}

/// Daily usage tracking
#[derive(Debug, Clone)]
struct DailyUsage {
    date: DateTime<Utc>,
    requests: u32,
}

impl DailyUsage {
    fn new() -> Self {
        Self {
            date: Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
            requests: 0,
        }
    }

    fn is_today(&self) -> bool {
        let today = Utc::now().date_naive();
        self.date.date_naive() == today
    }

    fn reset_if_new_day(&mut self) {
        if !self.is_today() {
            self.date = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
            self.requests = 0;
        }
    }

    fn add_request(&mut self) {
        self.reset_if_new_day();
        self.requests += 1;
    }

    fn can_make_request(&mut self, daily_limit: Option<u32>) -> bool {
        self.reset_if_new_day();
        match daily_limit {
            Some(limit) => self.requests < limit,
            None => true,
        }
    }
}

/// Client tracking information
#[derive(Debug, Clone)]
struct ClientInfo {
    client_type: ClientType,
    bucket: TokenBucket,
    daily_usage: DailyUsage,
    last_seen: Instant,
    total_requests: u64,
    blocked_requests: u64,
}

impl ClientInfo {
    fn new(client_type: ClientType, quota: &RateLimitQuota) -> Self {
        let refill_rate = quota.requests_per_minute as f64 / 60.0;

        Self {
            client_type,
            bucket: TokenBucket::new(quota.burst_size, refill_rate),
            daily_usage: DailyUsage::new(),
            last_seen: Instant::now(),
            total_requests: 0,
            blocked_requests: 0,
        }
    }

    fn update_activity(&mut self, quota: &RateLimitQuota) -> bool {
        self.last_seen = Instant::now();

        // Check daily limit first
        if !self.daily_usage.can_make_request(quota.daily_limit) {
            return false;
        }

        self.total_requests += 1;
        self.daily_usage.add_request();
        true
    }

    fn record_blocked(&mut self) {
        self.blocked_requests += 1;
    }
}

/// Rate limiter implementation
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Arc<RwLock<LruCache<String, ClientInfo>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.max_clients).unwrap();
        Self {
            config,
            clients: Arc::new(RwLock::new(LruCache::new(cache_size))),
        }
    }

    async fn check_rate_limit(&self, client_type: ClientType, client_id: &str) -> Result<(), WebError> {
        if !self.config.enabled {
            return Ok(());
        }

        let quota = self.get_quota_for_client(&client_type);
        let mut clients = self.clients.write().await;

        let client_info =
            clients.get_or_insert_mut(client_id.to_string(), || ClientInfo::new(client_type.clone(), quota));

        // Check daily limit first
        if !client_info.update_activity(quota) {
            client_info.record_blocked();
            warn!(
                "Daily limit exceeded for client: {} (type: {:?})",
                client_id, client_type
            );
            return Err(WebError::RateLimit);
        }

        // Check rate limit (burst + per-minute)
        if client_info.bucket.try_consume(1.0) {
            debug!(
                "Rate limit check passed for client: {} (type: {:?})",
                client_id, client_type
            );
            Ok(())
        } else {
            client_info.record_blocked();
            let retry_after = client_info.bucket.time_until_available();

            warn!(
                "Rate limit exceeded for client: {} (type: {:?}), retry after: {:?}",
                client_id, client_type, retry_after
            );

            Err(WebError::RateLimit)
        }
    }

    fn get_quota_for_client(&self, client_type: &ClientType) -> &RateLimitQuota {
        match client_type {
            ClientType::Anonymous => &self.config.quotas.anonymous,
            ClientType::User(_) => &self.config.quotas.user,
            ClientType::Admin(_) => &self.config.quotas.admin,
            ClientType::Readonly(_) => &self.config.quotas.readonly,
            ClientType::Service(_) => &self.config.quotas.service,
        }
    }

    fn extract_client_info(
        &self,
        auth_context: Option<&AuthContext>,
        connect_info: Option<&ConnectInfo<SocketAddr>>,
    ) -> (ClientType, String) {
        if let Some(auth) = auth_context {
            if auth.is_authenticated {
                // Determine user type based on role (stored as string in AuthContext)
                let client_type = match auth.role.as_str() {
                    "admin" => ClientType::Admin(auth.user_id.clone()),
                    "readonly" => ClientType::Readonly(auth.user_id.clone()),
                    "service" => ClientType::Service(auth.user_id.clone()),
                    _ => ClientType::User(auth.user_id.clone()),
                };
                let client_id = format!("user:{}", auth.user_id);
                return (client_type, client_id);
            }
        }

        // Fall back to IP address for anonymous users
        let client_id = if let Some(ConnectInfo(addr)) = connect_info {
            format!("ip:{}", addr.ip())
        } else {
            "unknown".to_string()
        };

        (ClientType::Anonymous, client_id)
    }

    /// Get rate limit statistics for a client
    pub async fn get_client_stats(&self, client_id: &str) -> Option<ClientStats> {
        let clients = self.clients.read().await;
        clients.peek(client_id).map(|info| ClientStats {
            client_type: info.client_type.clone(),
            total_requests: info.total_requests,
            blocked_requests: info.blocked_requests,
            daily_requests: info.daily_usage.requests,
            remaining_tokens: info.bucket.remaining_tokens_immutable() as u32,
            last_seen: info.last_seen,
        })
    }
}

/// Rate limit statistics for monitoring
#[derive(Debug, Clone)]
pub struct ClientStats {
    pub client_type: ClientType,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub daily_requests: u32,
    pub remaining_tokens: u32,
    pub last_seen: Instant,
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, WebError> {
    // Extract rate limiter from request extensions
    let rate_limiter = request
        .extensions()
        .get::<Arc<RateLimiter>>()
        .ok_or_else(|| WebError::internal("Rate limiter not configured"))?;

    // Extract auth context if available
    let auth_context = request.extensions().get::<AuthContext>();

    let (client_type, client_id) = rate_limiter.extract_client_info(auth_context, connect_info.as_ref());

    // Check rate limit
    if let Err(err) = rate_limiter.check_rate_limit(client_type.clone(), &client_id).await {
        // Log security event for rate limit violations
        if let Some(audit_config) = request.extensions().get::<crate::middleware::AuditConfig>() {
            let logger = TracingAuditLogger::new(audit_config.clone());
            let mut event = AuditEvent::new(
                AuditEventType::RateLimitExceeded,
                AuditSeverity::Warning,
                format!("Rate limit exceeded for client {} (type: {:?})", client_id, client_type),
            );

            if let Some(auth) = auth_context {
                if auth.is_authenticated {
                    event = event.with_user(auth.user_id.clone(), Some(auth.session_id.clone()));
                }
            }

            logger.log_event(event);
        }

        return Err(err);
    }

    // If rate limit check passes, continue with the request
    Ok(next.run(request).await)
}

/// Create rate limiting layer with configuration
pub fn rate_limit_layer(_config: RateLimitConfig) {
    // Config will be passed directly to the middleware when applied
    // This is a placeholder function for the public API
}

/// Create rate limiting middleware with config
pub fn create_rate_limit_middleware(config: RateLimitConfig) -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::get, Router};
    use std::net::{IpAddr, Ipv4Addr};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "success"
    }

    #[tokio::test]
    async fn test_rate_limit_allows_requests_within_limit() {
        let config = RateLimitConfig::permissive();

        let app = {
            rate_limit_layer(config);
            Router::new().route("/test", get(test_handler)).layer(())
        };

        // First request should succeed
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let request = axum::http::Request::builder()
            .uri("/test")
            .extension(ConnectInfo(addr))
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(5, 1.0); // 5 tokens, 1 token per second

        // Consume all tokens
        for _ in 0..5 {
            assert!(bucket.try_consume(1.0));
        }

        // Should be empty now
        assert!(!bucket.try_consume(1.0));

        // Wait a bit and check that tokens are refilled
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(bucket.try_consume(1.0));
    }
}
