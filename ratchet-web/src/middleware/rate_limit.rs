use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use lru::LruCache;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::errors::WebError;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_size: Duration,
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl RateLimitConfig {
    pub fn permissive() -> Self {
        Self {
            requests_per_minute: 1000,
            burst_size: 100,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        }
    }

    pub fn strict() -> Self {
        Self {
            requests_per_minute: 30,
            burst_size: 5,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        }
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

/// Client tracking information
#[derive(Debug, Clone)]
struct ClientInfo {
    bucket: TokenBucket,
    last_seen: Instant,
    total_requests: u64,
    blocked_requests: u64,
}

impl ClientInfo {
    fn new(config: &RateLimitConfig) -> Self {
        let refill_rate = config.requests_per_minute as f64 / 60.0;

        Self {
            bucket: TokenBucket::new(config.burst_size, refill_rate),
            last_seen: Instant::now(),
            total_requests: 0,
            blocked_requests: 0,
        }
    }

    fn update_activity(&mut self) {
        self.last_seen = Instant::now();
        self.total_requests += 1;
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
        let cache_size = NonZeroUsize::new(10000).unwrap(); // Max 10k tracked clients
        Self {
            config,
            clients: Arc::new(RwLock::new(LruCache::new(cache_size))),
        }
    }

    async fn check_rate_limit(&self, client_id: &str) -> Result<(), WebError> {
        let mut clients = self.clients.write().await;
        
        let client_info = clients
            .get_or_insert_mut(client_id.to_string(), || ClientInfo::new(&self.config));

        client_info.update_activity();

        if client_info.bucket.try_consume(1.0) {
            debug!("Rate limit check passed for client: {}", client_id);
            Ok(())
        } else {
            client_info.record_blocked();
            let retry_after = client_info.bucket.time_until_available();
            
            warn!(
                "Rate limit exceeded for client: {}, retry after: {:?}",
                client_id, retry_after
            );

            Err(WebError::RateLimit)
        }
    }

    fn extract_client_id(&self, headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> String {
        // Try to get client ID from headers first (e.g., API key, user ID)
        if let Some(api_key) = headers.get("X-API-Key").and_then(|h| h.to_str().ok()) {
            return format!("api:{}", api_key);
        }

        if let Some(user_id) = headers.get("X-User-ID").and_then(|h| h.to_str().ok()) {
            return format!("user:{}", user_id);
        }

        // Fall back to IP address
        if let Some(ConnectInfo(addr)) = connect_info {
            return format!("ip:{}", addr.ip());
        }

        // Last resort - use a default identifier
        "unknown".to_string()
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware<B>(
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, WebError> {
    // Extract rate limiter from request extensions (it should be added by the layer)
    let rate_limiter = request
        .extensions()
        .get::<Arc<RateLimiter>>()
        .ok_or_else(|| WebError::internal("Rate limiter not configured"))?;

    let client_id = rate_limiter.extract_client_id(&headers, connect_info.as_ref());

    // Check rate limit
    rate_limiter.check_rate_limit(&client_id).await?;

    // If rate limit check passes, continue with the request
    Ok(next.run(request).await)
}

/// Create rate limiting layer with configuration
pub fn rate_limit_layer(_config: RateLimitConfig) -> tower::layer::util::Identity {
    // For now, return identity layer - full implementation requires more complex setup
    // Real implementation would need proper state management
    tower::layer::util::Identity::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use std::net::{IpAddr, Ipv4Addr};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "success"
    }

    #[tokio::test]
    async fn test_rate_limit_allows_requests_within_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        };

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(rate_limit_layer(config));

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