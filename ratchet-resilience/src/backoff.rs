//! Backoff strategies for retry policies

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Backoff strategy for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,

    /// Linear increase: delay = initial_delay * attempt
    Linear,

    /// Exponential increase: delay = initial_delay * base^(attempt-1)
    Exponential {
        /// Base for exponential calculation (e.g., 2.0 for doubling)
        base: f64,
    },

    /// Fibonacci sequence backoff
    Fibonacci,

    /// Custom delay sequence
    Custom {
        /// Delays in milliseconds for each attempt
        delays_ms: Vec<u64>,
    },
}

/// Backoff delay calculator
pub struct BackoffCalculator {
    strategy: BackoffStrategy,
    initial_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl BackoffCalculator {
    /// Create a new backoff calculator
    pub fn new(
        strategy: BackoffStrategy,
        initial_delay: Duration,
        max_delay: Duration,
        jitter: bool,
    ) -> Self {
        Self {
            strategy,
            initial_delay,
            max_delay,
            jitter,
        }
    }

    /// Calculate delay for a specific attempt (1-indexed)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.calculate_base_delay(attempt);
        let capped_delay = base_delay.min(self.max_delay);

        if self.jitter {
            self.add_jitter(capped_delay)
        } else {
            capped_delay
        }
    }

    fn calculate_base_delay(&self, attempt: u32) -> Duration {
        match &self.strategy {
            BackoffStrategy::Fixed => self.initial_delay,

            BackoffStrategy::Linear => self.initial_delay * attempt,

            BackoffStrategy::Exponential { base } => {
                if attempt == 0 {
                    return Duration::ZERO;
                }
                let multiplier = base.powi(attempt as i32 - 1);
                Duration::from_nanos((self.initial_delay.as_nanos() as f64 * multiplier) as u64)
            }

            BackoffStrategy::Fibonacci => {
                let fib_number = fibonacci(attempt);
                Duration::from_nanos(
                    (self.initial_delay.as_nanos() as f64 * fib_number as f64) as u64,
                )
            }

            BackoffStrategy::Custom { delays_ms } => {
                let index = (attempt as usize).saturating_sub(1);
                if index < delays_ms.len() {
                    Duration::from_millis(delays_ms[index])
                } else {
                    // Use last delay for attempts beyond the custom sequence
                    delays_ms
                        .last()
                        .map(|&ms| Duration::from_millis(ms))
                        .unwrap_or(self.max_delay)
                }
            }
        }
    }

    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();

        // Add ±20% jitter
        let jitter_factor = rng.gen_range(0.8..1.2);
        Duration::from_nanos((delay.as_nanos() as f64 * jitter_factor) as u64)
    }
}

/// Calculate the nth Fibonacci number (1-indexed)
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        2 => 1,
        _ => {
            let mut a = 1;
            let mut b = 1;
            for _ in 2..n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

/// Decorrelated jitter backoff calculator
///
/// This implements the "decorrelated jitter" algorithm which provides
/// better distributed retry times across multiple clients.
pub struct DecorrelatedJitterCalculator {
    base_delay: Duration,
    max_delay: Duration,
    last_delay: Option<Duration>,
}

impl DecorrelatedJitterCalculator {
    /// Create a new decorrelated jitter calculator
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            last_delay: None,
        }
    }

    /// Calculate the next delay using decorrelated jitter
    pub fn next_delay(&mut self) -> Duration {
        let mut rng = rand::thread_rng();

        let delay = match self.last_delay {
            None => self.base_delay,
            Some(last) => {
                let min_delay = self.base_delay;
                let max_delay = (last * 3).min(self.max_delay);

                if min_delay >= max_delay {
                    max_delay
                } else {
                    let range = max_delay.as_nanos() - min_delay.as_nanos();
                    let jitter = rng.gen_range(0..=range);
                    Duration::from_nanos(
                        (min_delay.as_nanos() + jitter).min(u64::MAX as u128) as u64
                    )
                }
            }
        };

        self.last_delay = Some(delay);
        delay
    }

    /// Reset the calculator to its initial state
    pub fn reset(&mut self) {
        self.last_delay = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_backoff() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Fixed,
            Duration::from_millis(100),
            Duration::from_secs(1),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(10), Duration::from_millis(100));
    }

    #[test]
    fn test_linear_backoff() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Linear,
            Duration::from_millis(100),
            Duration::from_secs(1),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(calc.calculate_delay(5), Duration::from_millis(500));
        assert_eq!(calc.calculate_delay(20), Duration::from_secs(1)); // Capped at max
    }

    #[test]
    fn test_exponential_backoff() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Exponential { base: 2.0 },
            Duration::from_millis(100),
            Duration::from_secs(10),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(calc.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(calc.calculate_delay(4), Duration::from_millis(800));
    }

    #[test]
    fn test_fibonacci_backoff() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Fibonacci,
            Duration::from_millis(100),
            Duration::from_secs(10),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100)); // 1 * 100
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(100)); // 1 * 100
        assert_eq!(calc.calculate_delay(3), Duration::from_millis(200)); // 2 * 100
        assert_eq!(calc.calculate_delay(4), Duration::from_millis(300)); // 3 * 100
        assert_eq!(calc.calculate_delay(5), Duration::from_millis(500)); // 5 * 100
    }

    #[test]
    fn test_custom_backoff() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Custom {
                delays_ms: vec![100, 200, 500, 1000],
            },
            Duration::from_millis(50), // Ignored for custom
            Duration::from_secs(10),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(calc.calculate_delay(3), Duration::from_millis(500));
        assert_eq!(calc.calculate_delay(4), Duration::from_millis(1000));
        assert_eq!(calc.calculate_delay(5), Duration::from_millis(1000)); // Uses last value
    }

    #[test]
    fn test_max_delay_cap() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Exponential { base: 2.0 },
            Duration::from_millis(100),
            Duration::from_millis(500),
            false,
        );

        assert_eq!(calc.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(calc.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(calc.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(calc.calculate_delay(4), Duration::from_millis(500)); // Capped
        assert_eq!(calc.calculate_delay(10), Duration::from_millis(500)); // Still capped
    }

    #[test]
    fn test_jitter() {
        let calc = BackoffCalculator::new(
            BackoffStrategy::Fixed,
            Duration::from_millis(1000),
            Duration::from_secs(10),
            true,
        );

        // With jitter, delays should vary but be close to base
        let delay = calc.calculate_delay(1);
        assert!(delay >= Duration::from_millis(800));
        assert!(delay <= Duration::from_millis(1200));
    }

    #[test]
    fn test_decorrelated_jitter() {
        let mut calc =
            DecorrelatedJitterCalculator::new(Duration::from_millis(100), Duration::from_secs(10));

        // First delay should be base delay
        let delay1 = calc.next_delay();
        assert_eq!(delay1, Duration::from_millis(100));

        // Subsequent delays should be random but bounded
        for _ in 0..10 {
            let delay = calc.next_delay();
            assert!(delay >= Duration::from_millis(100));
            assert!(delay <= Duration::from_secs(10));
        }
    }
}
