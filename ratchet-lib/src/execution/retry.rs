use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_strategy: BackoffStrategy,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_strategy: BackoffStrategy::Exponential { base: 2.0 },
            jitter: true,
        }
    }
}

/// Backoff strategy for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase: delay = initial_delay * attempt
    Linear,
    /// Exponential increase: delay = initial_delay * base^attempt
    Exponential { base: f64 },
}

impl RetryPolicy {
    /// Create a conservative retry policy for critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::Exponential { base: 1.5 },
            jitter: true,
        }
    }

    /// Create an aggressive retry policy for fast operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_strategy: BackoffStrategy::Exponential { base: 1.2 },
            jitter: true,
        }
    }

    /// Create a linear retry policy
    pub fn linear(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            max_delay: delay * max_attempts,
            backoff_strategy: BackoffStrategy::Linear,
            jitter: false,
        }
    }

    /// Calculate delay for a specific attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = match self.backoff_strategy {
            BackoffStrategy::Fixed => self.initial_delay,
            BackoffStrategy::Linear => self.initial_delay * attempt,
            BackoffStrategy::Exponential { base } => {
                let multiplier = base.powi(attempt as i32 - 1);
                Duration::from_nanos((self.initial_delay.as_nanos() as f64 * multiplier) as u64)
            }
        };

        let delay = base_delay.min(self.max_delay);

        if self.jitter {
            self.add_jitter(delay)
        } else {
            delay
        }
    }

    fn add_jitter(&self, delay: Duration) -> Duration {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.8..1.2);
        Duration::from_nanos((delay.as_nanos() as f64 * jitter_factor) as u64)
    }
}

/// Trait for errors that can be retried
pub trait Retryable {
    /// Whether this error is retryable
    fn is_retryable(&self) -> bool;

    /// Whether this is a transient error that should be retried immediately
    fn is_transient(&self) -> bool {
        false
    }

    /// Custom retry delay for this error type
    fn retry_delay(&self) -> Option<Duration> {
        None
    }
}

/// Retry executor
pub struct RetryExecutor {
    policy: RetryPolicy,
}

impl RetryExecutor {
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// Execute a function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Retryable + std::fmt::Display + Clone,
    {
        let mut attempt = 1;

        loop {
            debug!(
                "Executing attempt {} of {}",
                attempt, self.policy.max_attempts
            );

            match f().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("Operation succeeded after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    if attempt >= self.policy.max_attempts {
                        warn!("Operation failed after {} attempts: {}", attempt, error);
                        return Err(RetryError::MaxAttemptsExceeded {
                            attempts: attempt,
                            last_error: error,
                        });
                    }

                    if !error.is_retryable() {
                        warn!("Operation failed with non-retryable error: {}", error);
                        return Err(RetryError::NonRetryableError(error));
                    }

                    // Calculate delay
                    let delay = error
                        .retry_delay()
                        .unwrap_or_else(|| self.policy.delay_for_attempt(attempt));

                    warn!(
                        "Attempt {} failed: {}. Retrying in {:?}",
                        attempt, error, delay
                    );

                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Execute with a circuit breaker pattern
    pub async fn execute_with_circuit_breaker<F, Fut, T, E>(
        &self,
        f: F,
        circuit_breaker: &mut CircuitBreaker,
    ) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut + Clone,
        Fut: Future<Output = Result<T, E>>,
        E: Retryable + std::fmt::Display + Clone,
    {
        if circuit_breaker.is_open() {
            return Err(RetryError::CircuitBreakerOpen);
        }

        match self.execute(f).await {
            Ok(result) => {
                circuit_breaker.record_success();
                Ok(result)
            }
            Err(retry_error) => {
                circuit_breaker.record_failure();
                Err(retry_error)
            }
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Simple circuit breaker implementation
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    success_count: u32,
    success_threshold: u32,
    last_failure_time: Option<std::time::Instant>,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            success_threshold,
            last_failure_time: None,
            timeout,
        }
    }

    pub fn is_open(&mut self) -> bool {
        match self.state {
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.timeout {
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => false,
            CircuitState::Closed => false,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.success_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError<E> {
    #[error("Maximum retry attempts ({attempts}) exceeded. Last error: {last_error}")]
    MaxAttemptsExceeded { attempts: u32, last_error: E },

    #[error("Non-retryable error: {0}")]
    NonRetryableError(E),

    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
}

/// Common error types that implement Retryable
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecutionError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Resource busy: {0}")]
    ResourceBusy(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Retryable for ExecutionError {
    fn is_retryable(&self) -> bool {
        match self {
            ExecutionError::Network(_) => true,
            ExecutionError::DatabaseConnection(_) => true,
            ExecutionError::Timeout(_) => true,
            ExecutionError::ResourceBusy(_) => true,
            ExecutionError::PermissionDenied(_) => false,
            ExecutionError::InvalidInput(_) => false,
            ExecutionError::Internal(_) => true,
        }
    }

    fn is_transient(&self) -> bool {
        match self {
            ExecutionError::Network(_) => true,
            ExecutionError::DatabaseConnection(_) => true,
            ExecutionError::Timeout(_) => true,
            ExecutionError::ResourceBusy(_) => true,
            _ => false,
        }
    }

    fn retry_delay(&self) -> Option<Duration> {
        match self {
            ExecutionError::ResourceBusy(_) => Some(Duration::from_secs(1)),
            ExecutionError::DatabaseConnection(_) => Some(Duration::from_millis(500)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_strategy: BackoffStrategy::Fixed,
            jitter: false,
        };

        let executor = RetryExecutor::new(policy);

        let result = executor
            .execute(|| {
                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                async move {
                    if count < 2 {
                        Err(ExecutionError::Network("Temporary failure".to_string()))
                    } else {
                        Ok("Success".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let policy = RetryPolicy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_strategy: BackoffStrategy::Fixed,
            jitter: false,
        };

        let executor = RetryExecutor::new(policy);

        let result: Result<(), RetryError<ExecutionError>> = executor
            .execute(|| async { Err(ExecutionError::Network("Always fails".to_string())) })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RetryError::MaxAttemptsExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let policy = RetryPolicy::default();
        let executor = RetryExecutor::new(policy);

        let result: Result<(), RetryError<ExecutionError>> = executor
            .execute(|| async { Err(ExecutionError::InvalidInput("Bad input".to_string())) })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RetryError::NonRetryableError(_)
        ));
    }

    #[test]
    fn test_backoff_strategies() {
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_strategy: BackoffStrategy::Exponential { base: 2.0 },
            jitter: false,
        };

        let delay1 = policy.delay_for_attempt(1);
        let delay2 = policy.delay_for_attempt(2);
        let delay3 = policy.delay_for_attempt(3);

        assert_eq!(delay1, Duration::from_millis(100));
        assert_eq!(delay2, Duration::from_millis(200));
        assert_eq!(delay3, Duration::from_millis(400));
    }

    #[test]
    fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(!breaker.is_open());

        // Record failures to open circuit
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(breaker.is_open());

        // After timeout, should transition to half-open
        std::thread::sleep(Duration::from_millis(150));
        assert!(!breaker.is_open());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record successes to close circuit
        breaker.record_success();
        breaker.record_success();

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(!breaker.is_open());
    }
}
