use axum::{
    extract::ConnectInfo,
    http::{Request, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::rest::models::common::ApiError;

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
#[derive(Debug)]
struct ClientInfo {
    bucket: TokenBucket,
    first_seen: Instant,
    last_seen: Instant,
    total_requests: u64,
    blocked_requests: u64,
}

impl ClientInfo {
    fn new(config: &RateLimitConfig) -> Self {
        let refill_rate = config.requests_per_minute as f64 / 60.0;
        
        Self {
            bucket: TokenBucket::new(config.burst_size, refill_rate),
            first_seen: Instant::now(),
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
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let rate_limiter = Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start cleanup task
        rate_limiter.start_cleanup_task();
        
        rate_limiter
    }

    /// Check if request should be allowed
    pub async fn check_rate_limit(&self, client_id: &str) -> RateLimitResult {
        let mut clients = self.clients.write().await;
        
        let client_info = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientInfo::new(&self.config));

        client_info.update_activity();

        if client_info.bucket.try_consume(1.0) {
            debug!("Rate limit OK for client: {}", client_id);
            RateLimitResult::Allowed {
                remaining: client_info.bucket.remaining_tokens() as u32,
                reset_at: client_info.last_seen + self.config.window_size,
            }
        } else {
            client_info.record_blocked();
            let retry_after = client_info.bucket.time_until_available();
            
            warn!(
                "Rate limit exceeded for client: {} (total: {}, blocked: {})",
                client_id, client_info.total_requests, client_info.blocked_requests
            );
            
            RateLimitResult::RateLimited { retry_after }
        }
    }

    /// Get rate limit statistics for a client
    pub async fn get_client_stats(&self, client_id: &str) -> Option<ClientStats> {
        let clients = self.clients.read().await;
        clients.get(client_id).map(|info| ClientStats {
            total_requests: info.total_requests,
            blocked_requests: info.blocked_requests,
            first_seen: info.first_seen,
            last_seen: info.last_seen,
            current_tokens: info.bucket.tokens as u32,
        })
    }

    /// Get global rate limiter statistics
    pub async fn get_global_stats(&self) -> GlobalStats {
        let clients = self.clients.read().await;
        
        let total_clients = clients.len();
        let total_requests: u64 = clients.values().map(|c| c.total_requests).sum();
        let total_blocked: u64 = clients.values().map(|c| c.blocked_requests).sum();
        
        GlobalStats {
            total_clients,
            total_requests,
            total_blocked,
            block_rate: if total_requests > 0 {
                total_blocked as f64 / total_requests as f64
            } else {
                0.0
            },
        }
    }

    fn start_cleanup_task(&self) {
        let clients = self.clients.clone();
        let cleanup_interval = self.config.cleanup_interval;
        let window_size = self.config.window_size;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                
                let mut clients_guard = clients.write().await;
                let now = Instant::now();
                
                // Remove clients that haven't been seen for twice the window size
                let cleanup_threshold = window_size * 2;
                let initial_count = clients_guard.len();
                
                clients_guard.retain(|_, client| {
                    now.duration_since(client.last_seen) < cleanup_threshold
                });
                
                let removed_count = initial_count - clients_guard.len();
                if removed_count > 0 {
                    debug!("Cleaned up {} inactive rate limit entries", removed_count);
                }
            }
        });
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            clients: self.clients.clone(),
        }
    }
}

/// Rate limit check result
#[derive(Debug)]
pub enum RateLimitResult {
    Allowed { remaining: u32, reset_at: Instant },
    RateLimited { retry_after: Duration },
}

/// Client statistics
#[derive(Debug, Clone)]
pub struct ClientStats {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub current_tokens: u32,
}

/// Global rate limiter statistics
#[derive(Debug, Clone)]
pub struct GlobalStats {
    pub total_clients: usize,
    pub total_requests: u64,
    pub total_blocked: u64,
    pub block_rate: f64,
}

/// Extract client identifier from request
fn extract_client_id(headers: &HeaderMap, connect_info: Option<ConnectInfo<SocketAddr>>) -> String {
    // Try to get client ID from various sources
    
    // 1. API Key header
    if let Some(api_key) = headers.get("X-API-Key").and_then(|h| h.to_str().ok()) {
        return format!("api_key:{}", api_key);
    }
    
    // 2. Authorization header
    if let Some(auth) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return format!("bearer:{}", token);
        }
    }
    
    // 3. X-Forwarded-For header (for proxy setups)
    if let Some(forwarded) = headers.get("X-Forwarded-For").and_then(|h| h.to_str().ok()) {
        if let Some(ip) = forwarded.split(',').next() {
            return format!("ip:{}", ip.trim());
        }
    }
    
    // 4. X-Real-IP header
    if let Some(real_ip) = headers.get("X-Real-IP").and_then(|h| h.to_str().ok()) {
        return format!("ip:{}", real_ip);
    }
    
    // 5. Connection info (direct IP)
    if let Some(ConnectInfo(addr)) = connect_info {
        return format!("ip:{}", addr.ip());
    }
    
    // 6. Fallback to a default identifier
    "unknown".to_string()
}

/// Rate limiting middleware
pub async fn rate_limit_middleware<B>(
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, Response> {
    // Extract rate limiter from app state
    let rate_limiter = request
        .extensions()
        .get::<RateLimiter>()
        .ok_or_else(|| {
            tracing::error!("Rate limiter not found in request extensions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal_error("Rate limiter not configured")),
            ).into_response()
        })?
        .clone();

    let client_id = extract_client_id(&headers, connect_info);
    
    match rate_limiter.check_rate_limit(&client_id).await {
        RateLimitResult::Allowed { remaining, reset_at } => {
            let mut response = next.run(request).await;
            
            // Add rate limit headers
            let headers = response.headers_mut();
            
            if let Ok(remaining_value) = HeaderValue::from_str(&remaining.to_string()) {
                headers.insert("X-RateLimit-Remaining", remaining_value);
            }
            
            if let Ok(reset_value) = HeaderValue::from_str(&reset_at.elapsed().as_secs().to_string()) {
                headers.insert("X-RateLimit-Reset", reset_value);
            }
            
            Ok(response)
        }
        RateLimitResult::RateLimited { retry_after } => {
            let error_response = ApiError::new("Rate limit exceeded")
                .with_code("RATE_LIMIT_EXCEEDED");
            
            let response = (StatusCode::TOO_MANY_REQUESTS, Json(error_response)).into_response();
            
            let mut final_response = response;
            let headers = final_response.headers_mut();
            
            if let Ok(retry_value) = HeaderValue::from_str(&retry_after.as_secs().to_string()) {
                headers.insert("Retry-After", retry_value);
            }
            
            if let Ok(limit_value) = HeaderValue::from_str(&rate_limiter.config.requests_per_minute.to_string()) {
                headers.insert("X-RateLimit-Limit", limit_value);
            }
            
            headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
            
            Err(final_response)
        }
    }
}

/// Create rate limiter middleware layer
pub fn create_rate_limit_layer(config: RateLimitConfig) -> RateLimiter {
    RateLimiter::new(config)
}

/// Create middleware that injects RateLimiter into request extensions and handles rate limiting
pub async fn rate_limit_middleware_with_state<B>(
    rate_limiter: RateLimiter,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, Response> {
    let client_id = extract_client_id(&headers, connect_info);
    
    match rate_limiter.check_rate_limit(&client_id).await {
        RateLimitResult::Allowed { remaining, reset_at } => {
            let mut response = next.run(request).await;
            
            // Add rate limit headers
            let headers = response.headers_mut();
            
            if let Ok(remaining_value) = HeaderValue::from_str(&remaining.to_string()) {
                headers.insert("X-RateLimit-Remaining", remaining_value);
            }
            
            if let Ok(reset_value) = HeaderValue::from_str(&reset_at.elapsed().as_secs().to_string()) {
                headers.insert("X-RateLimit-Reset", reset_value);
            }
            
            Ok(response)
        }
        RateLimitResult::RateLimited { retry_after } => {
            // Rate limited - return 429 with retry information
            let error_response = ApiError::new("Rate limit exceeded. You have exceeded the rate limit. Please try again later.")
                .with_code("RATE_LIMIT_EXCEEDED");

            let mut final_response = (StatusCode::TOO_MANY_REQUESTS, Json(error_response)).into_response();
            
            let headers = final_response.headers_mut();
            if let Ok(retry_value) = HeaderValue::from_str(&retry_after.as_secs().to_string()) {
                headers.insert("Retry-After", retry_value);
            }
            
            if let Ok(limit_value) = HeaderValue::from_str(&rate_limiter.config.requests_per_minute.to_string()) {
                headers.insert("X-RateLimit-Limit", limit_value);
            }
            
            headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
            
            Err(final_response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(5, 1.0); // 5 tokens, 1 token per second
        
        // Should start with full bucket
        assert!(bucket.try_consume(1.0));
        assert!(bucket.try_consume(1.0));
        assert!(bucket.try_consume(1.0));
        assert!(bucket.try_consume(1.0));
        assert!(bucket.try_consume(1.0));
        
        // Should be empty now
        assert!(!bucket.try_consume(1.0));
        
        // Wait and try again (in real test, would need actual time passage)
        // For unit test, we can't easily test time-based refilling
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        };
        
        let limiter = RateLimiter::new(config);
        let client_id = "test_client";
        
        // First few requests should be allowed
        for i in 0..5 {
            let result = limiter.check_rate_limit(client_id).await;
            assert!(matches!(result, RateLimitResult::Allowed { .. }), "Request {} failed", i);
        }
        
        // Next request should be rate limited (burst exhausted)
        let result = limiter.check_rate_limit(client_id).await;
        assert!(matches!(result, RateLimitResult::RateLimited { .. }));
        
        // Stats should reflect the requests
        let stats = limiter.get_client_stats(client_id).await.unwrap();
        assert_eq!(stats.total_requests, 6);
        assert_eq!(stats.blocked_requests, 1);
    }

    #[tokio::test]
    async fn test_global_stats() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);
        
        // Make requests from different clients
        limiter.check_rate_limit("client1").await;
        limiter.check_rate_limit("client2").await;
        limiter.check_rate_limit("client1").await;
        
        let stats = limiter.get_global_stats().await;
        assert_eq!(stats.total_clients, 2);
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.total_blocked, 0);
    }

    #[test]
    fn test_client_id_extraction() {
        let mut headers = HeaderMap::new();
        
        // Test API key extraction
        headers.insert("X-API-Key", HeaderValue::from_static("test-key"));
        let client_id = extract_client_id(&headers, None);
        assert_eq!(client_id, "api_key:test-key");
        
        // Test Authorization header
        headers.clear();
        headers.insert("Authorization", HeaderValue::from_static("Bearer test-token"));
        let client_id = extract_client_id(&headers, None);
        assert_eq!(client_id, "bearer:test-token");
        
        // Test IP address
        headers.clear();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let client_id = extract_client_id(&headers, Some(ConnectInfo(addr)));
        assert_eq!(client_id, "ip:127.0.0.1");
    }
}