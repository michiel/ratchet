//! Rate limiting implementation for MCP operations

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{McpError, McpResult};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,

    /// Time window duration
    pub window_duration: Duration,

    /// Whether to use sliding window (vs fixed window)
    pub sliding_window: bool,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            sliding_window: true,
        }
    }

    /// Create rate limit for requests per minute
    pub fn per_minute(max_requests: u32) -> Self {
        Self::new(max_requests, Duration::from_secs(60))
    }

    /// Create rate limit for requests per second
    pub fn per_second(max_requests: u32) -> Self {
        Self::new(max_requests, Duration::from_secs(1))
    }

    /// Create rate limit for requests per hour
    pub fn per_hour(max_requests: u32) -> Self {
        Self::new(max_requests, Duration::from_secs(3600))
    }
}

/// Request record for tracking
#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp: Instant,
    count: u32,
}

/// Rate limiter state for a specific key
#[derive(Debug)]
struct RateLimiterState {
    requests: Vec<RequestRecord>,
    window_start: Instant,
    total_requests: u32,
}

impl RateLimiterState {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            window_start: Instant::now(),
            total_requests: 0,
        }
    }
}

/// Token bucket rate limiter
pub struct RateLimiter {
    /// Rate limiting configuration
    config: RateLimitConfig,

    /// State per client/key
    states: Arc<RwLock<HashMap<String, RateLimiterState>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed for the given key
    pub async fn check_rate_limit(&self, key: &str) -> McpResult<()> {
        self.check_rate_limit_with_count(key, 1).await
    }

    /// Check rate limit with a specific request count
    pub async fn check_rate_limit_with_count(&self, key: &str, count: u32) -> McpResult<()> {
        let now = Instant::now();
        let mut states = self.states.write().await;

        let state = states.entry(key.to_string()).or_insert_with(RateLimiterState::new);

        if self.config.sliding_window {
            self.check_sliding_window(state, now, count)
        } else {
            self.check_fixed_window(state, now, count)
        }
    }

    /// Check sliding window rate limit
    fn check_sliding_window(&self, state: &mut RateLimiterState, now: Instant, count: u32) -> McpResult<()> {
        // Remove old requests outside the window
        let window_start = now - self.config.window_duration;
        state.requests.retain(|record| record.timestamp >= window_start);

        // Count total requests in the window
        let current_requests: u32 = state.requests.iter().map(|r| r.count).sum();

        // Check if adding this request would exceed the limit
        if current_requests + count > self.config.max_requests {
            let retry_after = self.calculate_retry_after(state, now);
            return Err(McpError::RateLimitExceeded {
                message: format!(
                    "Rate limit exceeded: {} requests in {}s (max: {})",
                    current_requests + count,
                    self.config.window_duration.as_secs(),
                    self.config.max_requests
                ),
                retry_after: Some(retry_after.as_secs()),
            });
        }

        // Add the new request
        state.requests.push(RequestRecord { timestamp: now, count });

        Ok(())
    }

    /// Check fixed window rate limit
    fn check_fixed_window(&self, state: &mut RateLimiterState, now: Instant, count: u32) -> McpResult<()> {
        // Check if we need to reset the window
        if now.duration_since(state.window_start) >= self.config.window_duration {
            state.window_start = now;
            state.total_requests = 0;
        }

        // Check if adding this request would exceed the limit
        if state.total_requests + count > self.config.max_requests {
            let retry_after = self.calculate_fixed_window_retry_after(state, now);
            return Err(McpError::RateLimitExceeded {
                message: format!(
                    "Rate limit exceeded: {} requests in current window (max: {})",
                    state.total_requests + count,
                    self.config.max_requests
                ),
                retry_after: Some(retry_after.as_secs()),
            });
        }

        // Add the request count
        state.total_requests += count;

        Ok(())
    }

    /// Calculate when the client can retry (sliding window)
    fn calculate_retry_after(&self, state: &RateLimiterState, now: Instant) -> Duration {
        if let Some(oldest) = state.requests.first() {
            let window_end = oldest.timestamp + self.config.window_duration;
            if window_end > now {
                window_end - now
            } else {
                Duration::from_secs(1) // Minimum retry delay
            }
        } else {
            Duration::from_secs(1)
        }
    }

    /// Calculate when the client can retry (fixed window)
    fn calculate_fixed_window_retry_after(&self, state: &RateLimiterState, now: Instant) -> Duration {
        let window_end = state.window_start + self.config.window_duration;
        if window_end > now {
            window_end - now
        } else {
            Duration::from_secs(1)
        }
    }

    /// Get current rate limit status for a key
    pub async fn get_status(&self, key: &str) -> RateLimitStatus {
        let now = Instant::now();
        let states = self.states.read().await;

        if let Some(state) = states.get(key) {
            if self.config.sliding_window {
                let window_start = now - self.config.window_duration;
                let current_requests: u32 = state
                    .requests
                    .iter()
                    .filter(|r| r.timestamp >= window_start)
                    .map(|r| r.count)
                    .sum();

                RateLimitStatus {
                    current_requests,
                    max_requests: self.config.max_requests,
                    window_duration: self.config.window_duration,
                    remaining_requests: self.config.max_requests.saturating_sub(current_requests),
                    reset_time: self.calculate_sliding_window_reset(state, now),
                }
            } else {
                let remaining_time = self
                    .config
                    .window_duration
                    .saturating_sub(now.duration_since(state.window_start));

                RateLimitStatus {
                    current_requests: state.total_requests,
                    max_requests: self.config.max_requests,
                    window_duration: self.config.window_duration,
                    remaining_requests: self.config.max_requests.saturating_sub(state.total_requests),
                    reset_time: remaining_time,
                }
            }
        } else {
            RateLimitStatus {
                current_requests: 0,
                max_requests: self.config.max_requests,
                window_duration: self.config.window_duration,
                remaining_requests: self.config.max_requests,
                reset_time: self.config.window_duration,
            }
        }
    }

    /// Calculate reset time for sliding window
    fn calculate_sliding_window_reset(&self, state: &RateLimiterState, now: Instant) -> Duration {
        if let Some(oldest) = state.requests.first() {
            let expires = oldest.timestamp + self.config.window_duration;
            if expires > now {
                expires - now
            } else {
                Duration::from_secs(0)
            }
        } else {
            Duration::from_secs(0)
        }
    }

    /// Clean up old state entries
    pub async fn cleanup(&self, max_age: Duration) {
        let now = Instant::now();
        let mut states = self.states.write().await;

        states.retain(|_, state| {
            if self.config.sliding_window {
                // For sliding window, keep if there are recent requests
                let window_start = now - self.config.window_duration;
                state.requests.iter().any(|r| r.timestamp >= window_start)
            } else {
                // For fixed window, keep if window is still active
                now.duration_since(state.window_start) < max_age
            }
        });
    }
}

/// Rate limit status information
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    /// Current number of requests in the window
    pub current_requests: u32,

    /// Maximum allowed requests in the window
    pub max_requests: u32,

    /// Window duration
    pub window_duration: Duration,

    /// Remaining requests before hitting the limit
    pub remaining_requests: u32,

    /// Time until the window resets/oldest request expires
    pub reset_time: Duration,
}

impl RateLimitStatus {
    /// Check if the rate limit is currently exceeded
    pub fn is_exceeded(&self) -> bool {
        self.current_requests >= self.max_requests
    }

    /// Get the percentage of the limit used
    pub fn usage_percentage(&self) -> f64 {
        if self.max_requests == 0 {
            0.0
        } else {
            (self.current_requests as f64 / self.max_requests as f64) * 100.0
        }
    }
}

/// Multi-tier rate limiter for different operation types
pub struct MultiTierRateLimiter {
    limiters: HashMap<String, RateLimiter>,
}

impl MultiTierRateLimiter {
    /// Create a new multi-tier rate limiter
    pub fn new() -> Self {
        Self {
            limiters: HashMap::new(),
        }
    }

    /// Add a rate limiter for a specific operation type
    pub fn add_limiter(&mut self, operation: impl Into<String>, config: RateLimitConfig) {
        self.limiters.insert(operation.into(), RateLimiter::new(config));
    }

    /// Check rate limit for a specific operation and client
    pub async fn check_rate_limit(&self, operation: &str, client_id: &str) -> McpResult<()> {
        if let Some(limiter) = self.limiters.get(operation) {
            limiter.check_rate_limit(client_id).await
        } else {
            // No rate limit configured for this operation
            Ok(())
        }
    }

    /// Get status for all operations for a client
    pub async fn get_all_status(&self, client_id: &str) -> HashMap<String, RateLimitStatus> {
        let mut statuses = HashMap::new();

        for (operation, limiter) in &self.limiters {
            let status = limiter.get_status(client_id).await;
            statuses.insert(operation.clone(), status);
        }

        statuses
    }
}

impl Default for MultiTierRateLimiter {
    fn default() -> Self {
        let mut limiter = Self::new();

        // Default rate limits for different operations
        limiter.add_limiter("execute_task", RateLimitConfig::per_minute(10));
        limiter.add_limiter("get_logs", RateLimitConfig::per_minute(100));
        limiter.add_limiter("get_traces", RateLimitConfig::per_minute(50));
        limiter.add_limiter("list_tools", RateLimitConfig::per_minute(200));

        limiter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_sliding_window_rate_limiter() {
        let config = RateLimitConfig::new(3, Duration::from_millis(100));
        let limiter = RateLimiter::new(config);

        // First 3 requests should pass
        assert!(limiter.check_rate_limit("client1").await.is_ok());
        assert!(limiter.check_rate_limit("client1").await.is_ok());
        assert!(limiter.check_rate_limit("client1").await.is_ok());

        // 4th request should fail
        assert!(limiter.check_rate_limit("client1").await.is_err());

        // Wait for window to slide
        sleep(Duration::from_millis(110)).await;

        // Should allow requests again
        assert!(limiter.check_rate_limit("client1").await.is_ok());
    }

    #[tokio::test]
    async fn test_fixed_window_rate_limiter() {
        let mut config = RateLimitConfig::new(2, Duration::from_millis(100));
        config.sliding_window = false;
        let limiter = RateLimiter::new(config);

        // First 2 requests should pass
        assert!(limiter.check_rate_limit("client1").await.is_ok());
        assert!(limiter.check_rate_limit("client1").await.is_ok());

        // 3rd request should fail
        assert!(limiter.check_rate_limit("client1").await.is_err());

        // Wait for window to reset
        sleep(Duration::from_millis(110)).await;

        // Should allow requests again
        assert!(limiter.check_rate_limit("client1").await.is_ok());
    }

    #[tokio::test]
    async fn test_per_client_isolation() {
        let config = RateLimitConfig::per_minute(1);
        let limiter = RateLimiter::new(config);

        // Client1 uses their quota
        assert!(limiter.check_rate_limit("client1").await.is_ok());
        assert!(limiter.check_rate_limit("client1").await.is_err());

        // Client2 should still have quota
        assert!(limiter.check_rate_limit("client2").await.is_ok());
        assert!(limiter.check_rate_limit("client2").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_status() {
        let config = RateLimitConfig::new(5, Duration::from_millis(1000));
        let limiter = RateLimiter::new(config);

        // Initial status
        let status = limiter.get_status("client1").await;
        assert_eq!(status.current_requests, 0);
        assert_eq!(status.remaining_requests, 5);

        // After some requests
        limiter.check_rate_limit("client1").await.unwrap();
        limiter.check_rate_limit("client1").await.unwrap();

        let status = limiter.get_status("client1").await;
        assert_eq!(status.current_requests, 2);
        assert_eq!(status.remaining_requests, 3);
        assert!(!status.is_exceeded());
    }

    #[tokio::test]
    async fn test_multi_tier_rate_limiter() {
        let mut limiter = MultiTierRateLimiter::new();
        limiter.add_limiter("test_op", RateLimitConfig::new(2, Duration::from_millis(100)));

        // Test operation-specific rate limiting
        assert!(limiter.check_rate_limit("test_op", "client1").await.is_ok());
        assert!(limiter.check_rate_limit("test_op", "client1").await.is_ok());
        assert!(limiter.check_rate_limit("test_op", "client1").await.is_err());

        // Non-configured operation should pass
        assert!(limiter.check_rate_limit("other_op", "client1").await.is_ok());
    }

    #[test]
    fn test_rate_limit_config_helpers() {
        let per_minute = RateLimitConfig::per_minute(60);
        assert_eq!(per_minute.max_requests, 60);
        assert_eq!(per_minute.window_duration, Duration::from_secs(60));

        let per_second = RateLimitConfig::per_second(10);
        assert_eq!(per_second.max_requests, 10);
        assert_eq!(per_second.window_duration, Duration::from_secs(1));
    }
}
