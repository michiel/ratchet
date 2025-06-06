//! Resilience patterns for Ratchet
//!
//! This crate provides resilience patterns including retry policies,
//! circuit breakers, and graceful shutdown coordination.

pub mod backoff;
pub mod circuit_breaker;
pub mod retry;
pub mod shutdown;

// Re-export commonly used types
pub use backoff::{BackoffCalculator, BackoffStrategy, DecorrelatedJitterCalculator};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerBuilder, CircuitBreakerConfig, CircuitState,
};
pub use retry::{RetryError, RetryExecutor, RetryPolicy, Retryable};
pub use shutdown::{
    GracefulTask, ProcessShutdownManager, ShutdownAwareTask, ShutdownCoordinator, ShutdownError,
    ShutdownSignal,
};
