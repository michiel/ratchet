//! Circuit breaker pattern implementation

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally
    Closed,
    /// Circuit is open, requests are blocked
    Open,
    /// Circuit is half-open, limited requests allowed to test recovery
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    
    /// Number of successes in half-open state before closing
    pub success_threshold: u32,
    
    /// Time to wait before transitioning from open to half-open
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    
    /// Time window for counting failures (rolling window)
    #[serde(with = "humantime_serde", default = "default_window")]
    pub window: Duration,
    
    /// Minimum number of requests in window before evaluating
    #[serde(default = "default_min_requests")]
    pub min_requests: u32,
}

fn default_window() -> Duration {
    Duration::from_secs(60)
}

fn default_min_requests() -> u32 {
    5
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            window: default_window(),
            min_requests: default_min_requests(),
        }
    }
}

/// Circuit breaker metrics
#[derive(Debug, Clone, Default)]
pub struct CircuitMetrics {
    /// Total number of requests
    pub total_requests: u64,
    /// Number of successful requests
    pub total_successes: u64,
    /// Number of failed requests
    pub total_failures: u64,
    /// Number of requests rejected due to open circuit
    pub total_rejected: u64,
    /// Current consecutive failures
    pub consecutive_failures: u32,
    /// Current consecutive successes (in half-open state)
    pub consecutive_successes: u32,
    /// Last failure time
    pub last_failure_time: Option<Instant>,
    /// Last success time
    pub last_success_time: Option<Instant>,
    /// Last state change time
    pub last_state_change: Option<Instant>,
}

/// Thread-safe circuit breaker implementation
#[derive(Clone)]
pub struct CircuitBreaker {
    config: Arc<CircuitBreakerConfig>,
    state: Arc<Mutex<CircuitBreakerState>>,
}

struct CircuitBreakerState {
    state: CircuitState,
    metrics: CircuitMetrics,
    window_requests: Vec<(Instant, bool)>, // (timestamp, success)
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config: Arc::new(config),
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                metrics: CircuitMetrics::default(),
                window_requests: Vec::new(),
            })),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Check if the circuit breaker is open (requests should be blocked)
    pub fn is_open(&self) -> bool {
        let mut state = self.state.lock();
        self.update_state(&mut state);
        
        match state.state {
            CircuitState::Open => true,
            CircuitState::HalfOpen => false,
            CircuitState::Closed => false,
        }
    }

    /// Get the current state
    pub fn state(&self) -> CircuitState {
        let mut state = self.state.lock();
        self.update_state(&mut state);
        state.state
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        let mut state = self.state.lock();
        self.update_state(&mut state);
        
        let now = Instant::now();
        state.metrics.total_requests += 1;
        state.metrics.total_successes += 1;
        state.metrics.last_success_time = Some(now);
        state.window_requests.push((now, true));
        
        match state.state {
            CircuitState::HalfOpen => {
                state.metrics.consecutive_successes += 1;
                state.metrics.consecutive_failures = 0;
                
                if state.metrics.consecutive_successes >= self.config.success_threshold {
                    self.transition_to_closed(&mut state);
                }
            }
            CircuitState::Closed => {
                state.metrics.consecutive_failures = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
        
        self.clean_window(&mut state);
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        let mut state = self.state.lock();
        self.update_state(&mut state);
        
        let now = Instant::now();
        state.metrics.total_requests += 1;
        state.metrics.total_failures += 1;
        state.metrics.last_failure_time = Some(now);
        state.metrics.consecutive_failures += 1;
        state.window_requests.push((now, false));
        
        match state.state {
            CircuitState::Closed => {
                self.clean_window(&mut state);
                if self.should_open(&state) {
                    self.transition_to_open(&mut state);
                }
            }
            CircuitState::HalfOpen => {
                state.metrics.consecutive_successes = 0;
                self.transition_to_open(&mut state);
            }
            CircuitState::Open => {
                // Already open, no action needed
            }
        }
    }

    /// Record a rejected request (due to open circuit)
    pub fn record_rejection(&self) {
        let mut state = self.state.lock();
        state.metrics.total_rejected += 1;
    }

    /// Get current metrics
    pub fn metrics(&self) -> CircuitMetrics {
        let state = self.state.lock();
        state.metrics.clone()
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut state = self.state.lock();
        state.state = CircuitState::Closed;
        state.metrics = CircuitMetrics::default();
        state.window_requests.clear();
        state.metrics.last_state_change = Some(Instant::now());
    }

    // Internal methods

    fn update_state(&self, state: &mut CircuitBreakerState) {
        if state.state == CircuitState::Open {
            if let Some(last_failure) = state.metrics.last_failure_time {
                if last_failure.elapsed() >= self.config.timeout {
                    self.transition_to_half_open(state);
                }
            }
        }
    }

    fn should_open(&self, state: &CircuitBreakerState) -> bool {
        let window_failures = state.window_requests.iter()
            .filter(|(_, success)| !success)
            .count() as u32;
        
        let window_total = state.window_requests.len() as u32;
        
        window_total >= self.config.min_requests && 
        window_failures >= self.config.failure_threshold
    }

    fn clean_window(&self, state: &mut CircuitBreakerState) {
        let cutoff = Instant::now() - self.config.window;
        state.window_requests.retain(|(timestamp, _)| *timestamp > cutoff);
    }

    fn transition_to_open(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Open;
        state.metrics.last_state_change = Some(Instant::now());
        log::warn!("Circuit breaker opened after {} consecutive failures", 
                  state.metrics.consecutive_failures);
    }

    fn transition_to_closed(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::Closed;
        state.metrics.consecutive_failures = 0;
        state.metrics.consecutive_successes = 0;
        state.metrics.last_state_change = Some(Instant::now());
        log::info!("Circuit breaker closed after successful recovery");
    }

    fn transition_to_half_open(&self, state: &mut CircuitBreakerState) {
        state.state = CircuitState::HalfOpen;
        state.metrics.consecutive_successes = 0;
        state.metrics.last_state_change = Some(Instant::now());
        log::info!("Circuit breaker transitioned to half-open state");
    }
}

/// Builder for circuit breaker configuration
pub struct CircuitBreakerBuilder {
    config: CircuitBreakerConfig,
}

impl CircuitBreakerBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: CircuitBreakerConfig::default(),
        }
    }

    /// Set failure threshold
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    /// Set success threshold for recovery
    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    /// Set timeout before attempting recovery
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set time window for failure counting
    pub fn window(mut self, window: Duration) -> Self {
        self.config.window = window;
        self
    }

    /// Set minimum requests before evaluation
    pub fn min_requests(mut self, min: u32) -> Self {
        self.config.min_requests = min;
        self
    }

    /// Build the circuit breaker
    pub fn build(self) -> CircuitBreaker {
        CircuitBreaker::new(self.config)
    }
}

impl Default for CircuitBreakerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_breaker_basic_flow() {
        let breaker = CircuitBreakerBuilder::new()
            .failure_threshold(3)
            .success_threshold(2)
            .timeout(Duration::from_millis(100))
            .min_requests(1)
            .build();

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(!breaker.is_open());

        // Record failures to open circuit
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(breaker.is_open());

        // Wait for timeout to transition to half-open
        thread::sleep(Duration::from_millis(150));
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        assert!(!breaker.is_open());

        // Record successes to close circuit
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let breaker = CircuitBreakerBuilder::new()
            .failure_threshold(2)
            .success_threshold(2)
            .timeout(Duration::from_millis(50))
            .min_requests(1)
            .build();

        // Open the circuit
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for half-open
        thread::sleep(Duration::from_millis(100));
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Failure in half-open should reopen immediately
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_metrics() {
        let breaker = CircuitBreaker::with_defaults();

        breaker.record_success();
        breaker.record_success();
        breaker.record_failure();
        breaker.record_rejection();

        let metrics = breaker.metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.total_successes, 2);
        assert_eq!(metrics.total_failures, 1);
        assert_eq!(metrics.total_rejected, 1);
    }

    #[test]
    fn test_circuit_breaker_window() {
        let breaker = CircuitBreakerBuilder::new()
            .failure_threshold(3)
            .window(Duration::from_millis(200))
            .min_requests(3)
            .build();

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        
        // Wait for window to expire
        thread::sleep(Duration::from_millis(250));
        
        // Old failures should not count
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        
        // But new failures within window should
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }
}