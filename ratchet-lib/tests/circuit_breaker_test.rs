/// Circuit breaker integration tests
use ratchet_lib::execution::retry::{
    CircuitBreaker, CircuitState, RetryExecutor, RetryPolicy, RetryError,
    BackoffStrategy, ExecutionError, Retryable,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test error type for circuit breaker tests
#[derive(Debug, Clone, thiserror::Error)]
enum TestError {
    #[error("Transient error: {0}")]
    Transient(String),
    
    #[error("Permanent error: {0}")]
    Permanent(String),
}

impl Retryable for TestError {
    fn is_retryable(&self) -> bool {
        matches!(self, TestError::Transient(_))
    }
    
    fn is_transient(&self) -> bool {
        matches!(self, TestError::Transient(_))
    }
}

#[tokio::test]
async fn test_circuit_breaker_state_transitions() {
    let mut breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
    
    // Initial state should be closed
    assert_eq!(breaker.state(), CircuitState::Closed);
    assert!(!breaker.is_open());
    
    // Record failures to trigger open state
    for i in 0..3 {
        breaker.record_failure();
        if i < 2 {
            assert_eq!(breaker.state(), CircuitState::Closed, "Should remain closed until threshold");
        }
    }
    
    // Should now be open
    assert_eq!(breaker.state(), CircuitState::Open);
    assert!(breaker.is_open());
    
    // Wait for timeout to transition to half-open
    sleep(Duration::from_millis(150)).await;
    assert!(!breaker.is_open());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    
    // Success in half-open state
    breaker.record_success();
    assert_eq!(breaker.state(), CircuitState::HalfOpen, "Should remain half-open until success threshold");
    
    // Second success should close the circuit
    breaker.record_success();
    assert_eq!(breaker.state(), CircuitState::Closed);
    assert!(!breaker.is_open());
}

#[tokio::test]
async fn test_circuit_breaker_half_open_failure() {
    let mut breaker = CircuitBreaker::new(2, 2, Duration::from_millis(50));
    
    // Open the circuit
    breaker.record_failure();
    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);
    
    // Wait for half-open
    sleep(Duration::from_millis(100)).await;
    assert!(!breaker.is_open());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    
    // Failure in half-open should immediately reopen
    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);
    assert!(breaker.is_open());
}

#[tokio::test]
async fn test_retry_with_circuit_breaker_success() {
    let policy = RetryPolicy {
        max_attempts: 5,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        backoff_strategy: BackoffStrategy::Fixed,
        jitter: false,
    };
    
    let executor = RetryExecutor::new(policy);
    let mut breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
    
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    
    let result = executor
        .execute_with_circuit_breaker(
            || {
                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                async move {
                    if count < 2 {
                        Err(TestError::Transient("Temporary failure".to_string()))
                    } else {
                        Ok("Success".to_string())
                    }
                }
            },
            &mut breaker,
        )
        .await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(breaker.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_prevents_execution() {
    let policy = RetryPolicy::default();
    let executor = RetryExecutor::new(policy);
    let mut breaker = CircuitBreaker::new(1, 1, Duration::from_secs(1));
    
    // Force circuit open
    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);
    
    // Attempt execution with open circuit
    let result: Result<(), RetryError<TestError>> = executor
        .execute_with_circuit_breaker(
            || async { Ok(()) },
            &mut breaker,
        )
        .await;
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RetryError::CircuitBreakerOpen));
}

#[tokio::test]
async fn test_circuit_breaker_with_execution_error() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(50),
        backoff_strategy: BackoffStrategy::Exponential { base: 2.0 },
        jitter: false,
    };
    
    let executor = RetryExecutor::new(policy);
    let mut breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
    
    // Test with ExecutionError types
    let network_error: Result<(), RetryError<ExecutionError>> = executor
        .execute_with_circuit_breaker(
            || async {
                Err(ExecutionError::Network("Connection refused".to_string()))
            },
            &mut breaker,
        )
        .await;
    
    assert!(network_error.is_err());
    assert!(matches!(network_error.unwrap_err(), RetryError::MaxAttemptsExceeded { .. }));
}

#[tokio::test]
async fn test_circuit_breaker_recovery_pattern() {
    let mut breaker = CircuitBreaker::new(2, 3, Duration::from_millis(100));
    
    // Simulate failure pattern
    breaker.record_failure();
    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);
    
    // Wait for recovery
    sleep(Duration::from_millis(150)).await;
    assert!(!breaker.is_open());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    
    // Gradual recovery
    for i in 0..3 {
        breaker.record_success();
        if i < 2 {
            assert_eq!(breaker.state(), CircuitState::HalfOpen, "Should stay half-open during recovery");
        }
    }
    
    // Fully recovered
    assert_eq!(breaker.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_multiple_circuit_breakers() {
    let policy = RetryPolicy::default();
    let executor = RetryExecutor::new(policy);
    
    // Create breakers for different services
    let mut db_breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
    let mut api_breaker = CircuitBreaker::new(2, 1, Duration::from_millis(50));
    
    // Simulate DB failures
    for _ in 0..3 {
        let _: Result<(), RetryError<ExecutionError>> = executor
            .execute_with_circuit_breaker(
                || async {
                    Err(ExecutionError::DatabaseConnection("Connection lost".to_string()))
                },
                &mut db_breaker,
            )
            .await;
    }
    
    assert_eq!(db_breaker.state(), CircuitState::Open);
    assert_eq!(api_breaker.state(), CircuitState::Closed);
    
    // API should still work
    let api_result: Result<&str, RetryError<TestError>> = executor
        .execute_with_circuit_breaker(
            || async { Ok("API response") },
            &mut api_breaker,
        )
        .await;
    
    assert!(api_result.is_ok());
}

#[tokio::test]
async fn test_circuit_breaker_metrics() {
    let mut breaker = CircuitBreaker::new(5, 3, Duration::from_millis(200));
    
    // Record failures without interspersed successes (successes reset failure count)
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_failure();
    
    // Should still be closed (4 failures < 5 threshold)
    assert_eq!(breaker.state(), CircuitState::Closed);
    
    // One more failure should open it
    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);
    
    // Wait for timeout to transition to half-open
    sleep(Duration::from_millis(250)).await;
    assert!(!breaker.is_open());
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
    
    // Success path - need 3 successes to close
    for _ in 0..3 {
        breaker.record_success();
    }
    assert_eq!(breaker.state(), CircuitState::Closed);
    
    // Verify that success in closed state resets failure count
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_success(); // This should reset failure count
    breaker.record_failure();
    
    // Should still be closed since success reset the count
    assert_eq!(breaker.state(), CircuitState::Closed);
}

#[tokio::test]
async fn test_circuit_breaker_concurrent_access() {
    let breaker = Arc::new(tokio::sync::Mutex::new(
        CircuitBreaker::new(10, 5, Duration::from_millis(100))
    ));
    
    let mut handles = vec![];
    
    // Simulate concurrent failures
    for i in 0..15 {
        let breaker_clone = breaker.clone();
        let handle = tokio::spawn(async move {
            let mut breaker = breaker_clone.lock().await;
            breaker.record_failure();
            (i, breaker.state())
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    let mut results = vec![];
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }
    
    // Final state should be open
    let final_breaker = breaker.lock().await;
    assert_eq!(final_breaker.state(), CircuitState::Open);
}

#[tokio::test]
async fn test_circuit_breaker_timeout_accuracy() {
    let timeout = Duration::from_millis(50);
    let mut breaker = CircuitBreaker::new(1, 1, timeout);
    
    // Open the circuit
    breaker.record_failure();
    assert!(breaker.is_open());
    
    // Check just before timeout
    sleep(Duration::from_millis(40)).await;
    assert!(breaker.is_open(), "Should still be open before timeout");
    
    // Check after timeout
    sleep(Duration::from_millis(20)).await;
    assert!(!breaker.is_open(), "Should be half-open after timeout");
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
}