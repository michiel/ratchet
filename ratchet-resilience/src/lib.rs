//! Resilience patterns for Ratchet
//! 
//! This crate provides resilience patterns including retry policies,
//! circuit breakers, and graceful shutdown coordination.

pub mod retry;
pub mod circuit_breaker;
pub mod shutdown;
pub mod backoff;

// Re-export commonly used types
pub use retry::{RetryPolicy, RetryExecutor, Retryable, RetryError};
pub use circuit_breaker::{CircuitBreaker, CircuitState, CircuitBreakerConfig, CircuitBreakerBuilder};
pub use shutdown::{ShutdownCoordinator, ShutdownSignal, ShutdownError, GracefulTask, ShutdownAwareTask, ProcessShutdownManager};
pub use backoff::{BackoffStrategy, BackoffCalculator, DecorrelatedJitterCalculator};