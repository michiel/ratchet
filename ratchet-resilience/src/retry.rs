//! Retry policy and executor

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

use crate::backoff::BackoffCalculator;
use crate::circuit_breaker::CircuitBreaker;

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay between retries
    #[serde(with = "humantime_serde")]
    pub initial_delay: Duration,

    /// Maximum delay between retries
    #[serde(with = "humantime_serde")]
    pub max_delay: Duration,

    /// Backoff strategy
    pub backoff_strategy: crate::backoff::BackoffStrategy,

    /// Whether to add jitter to retry delays
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_strategy: crate::backoff::BackoffStrategy::Exponential { base: 2.0 },
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a conservative retry policy for critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            backoff_strategy: crate::backoff::BackoffStrategy::Exponential { base: 1.5 },
            jitter: true,
        }
    }

    /// Create an aggressive retry policy for fast operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_strategy: crate::backoff::BackoffStrategy::Exponential { base: 1.2 },
            jitter: true,
        }
    }

    /// Create a linear retry policy
    pub fn linear(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            max_delay: delay * max_attempts,
            backoff_strategy: crate::backoff::BackoffStrategy::Linear,
            jitter: false,
        }
    }

    /// Calculate delay for a specific attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let calculator = BackoffCalculator::new(
            self.backoff_strategy.clone(),
            self.initial_delay,
            self.max_delay,
            self.jitter,
        );

        calculator.calculate_delay(attempt)
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
    /// Create a new retry executor with the given policy
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// Create with default policy
    pub fn with_default_policy() -> Self {
        Self::new(RetryPolicy::default())
    }

    /// Execute a function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Retryable + std::fmt::Display + Clone,
    {
        self.execute_with_context(|_attempt| f()).await
    }

    /// Execute a function with retry logic and attempt context
    pub async fn execute_with_context<F, Fut, T, E>(&self, mut f: F) -> Result<T, RetryError<E>>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Retryable + std::fmt::Display + Clone,
    {
        let mut attempt = 1;

        loop {
            debug!(
                "Executing attempt {} of {}",
                attempt, self.policy.max_attempts
            );

            match f(attempt).await {
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

                    if error.is_transient() && delay < Duration::from_millis(10) {
                        debug!("Transient error, retrying immediately");
                    } else {
                        warn!(
                            "Attempt {} failed: {}. Retrying in {:?}",
                            attempt, error, delay
                        );
                        sleep(delay).await;
                    }

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

/// Retry error types
#[derive(Debug, thiserror::Error)]
pub enum RetryError<E> {
    /// Maximum retry attempts exceeded
    #[error("Maximum retry attempts ({attempts}) exceeded. Last error: {last_error}")]
    MaxAttemptsExceeded { attempts: u32, last_error: E },

    /// Non-retryable error encountered
    #[error("Non-retryable error: {0}")]
    NonRetryableError(E),

    /// Circuit breaker is open
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
}

impl<E> RetryError<E> {
    /// Get the underlying error if present
    pub fn into_inner(self) -> Option<E> {
        match self {
            RetryError::MaxAttemptsExceeded { last_error, .. } => Some(last_error),
            RetryError::NonRetryableError(error) => Some(error),
            RetryError::CircuitBreakerOpen => None,
        }
    }

    /// Check if this represents a circuit breaker open error
    pub fn is_circuit_breaker_open(&self) -> bool {
        matches!(self, RetryError::CircuitBreakerOpen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestError {
        retryable: bool,
        message: String,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Retryable for TestError {
        fn is_retryable(&self) -> bool {
            self.retryable
        }
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_strategy: crate::backoff::BackoffStrategy::Fixed,
            jitter: false,
        };

        let executor = RetryExecutor::new(policy);

        let result = executor
            .execute(|| {
                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                async move {
                    if count < 2 {
                        Err(TestError {
                            retryable: true,
                            message: "Temporary failure".to_string(),
                        })
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
            backoff_strategy: crate::backoff::BackoffStrategy::Fixed,
            jitter: false,
        };

        let executor = RetryExecutor::new(policy);

        let result: Result<(), RetryError<TestError>> = executor
            .execute(|| async {
                Err(TestError {
                    retryable: true,
                    message: "Always fails".to_string(),
                })
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RetryError::MaxAttemptsExceeded { .. }
        ));
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let executor = RetryExecutor::with_default_policy();

        let result: Result<(), RetryError<TestError>> = executor
            .execute(|| async {
                Err(TestError {
                    retryable: false,
                    message: "Non-retryable".to_string(),
                })
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RetryError::NonRetryableError(_)
        ));
    }

    #[tokio::test]
    async fn test_execute_with_context() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let executor = RetryExecutor::new(RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_strategy: crate::backoff::BackoffStrategy::Fixed,
            jitter: false,
        });

        let result = executor
            .execute_with_context(|attempt| {
                attempts_clone.store(attempt, Ordering::Relaxed);
                async move {
                    if attempt < 3 {
                        Err(TestError {
                            retryable: true,
                            message: format!("Attempt {}", attempt),
                        })
                    } else {
                        Ok(attempt)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }
}
