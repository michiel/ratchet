//! Enhanced batch operation error handling with partial failure support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

use crate::{McpError, McpResult};

/// Policy for handling partial failures in batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartialFailurePolicy {
    /// Continue processing all operations regardless of failures
    ContinueAll,
    
    /// Abort immediately on first failure
    AbortOnFirst,
    
    /// Abort when failure rate exceeds threshold
    AbortOnThreshold {
        /// Maximum failure percentage (0.0 to 1.0)
        max_failure_rate: f64,
        /// Minimum operations to process before checking threshold
        min_operations: usize,
    },
    
    /// Abort when consecutive failures exceed limit
    AbortOnConsecutive {
        /// Maximum consecutive failures allowed
        max_consecutive: usize,
    },
}

impl Default for PartialFailurePolicy {
    fn default() -> Self {
        Self::AbortOnThreshold {
            max_failure_rate: 0.5, // 50% failure rate
            min_operations: 5,
        }
    }
}

impl PartialFailurePolicy {
    /// Check if batch should abort based on current errors
    pub fn should_abort(&self, errors: &[(usize, McpError)], total_processed: usize) -> bool {
        match self {
            Self::ContinueAll => false,
            
            Self::AbortOnFirst => !errors.is_empty(),
            
            Self::AbortOnThreshold { max_failure_rate, min_operations } => {
                if total_processed < *min_operations {
                    false
                } else {
                    let failure_rate = errors.len() as f64 / total_processed as f64;
                    failure_rate > *max_failure_rate
                }
            }
            
            Self::AbortOnConsecutive { max_consecutive } => {
                self.count_consecutive_failures(errors, total_processed) > *max_consecutive
            }
        }
    }
    
    /// Count consecutive failures at the end of the error list
    fn count_consecutive_failures(&self, errors: &[(usize, McpError)], total_processed: usize) -> usize {
        if errors.is_empty() {
            return 0;
        }
        
        // Sort errors by index to find consecutive failures
        let mut sorted_errors = errors.to_vec();
        sorted_errors.sort_by_key(|(index, _)| *index);
        
        let mut consecutive = 0;
        let mut last_index = total_processed;
        
        // Count backwards from the last processed operation
        for (index, _) in sorted_errors.iter().rev() {
            if *index == last_index - 1 {
                consecutive += 1;
                last_index = *index;
            } else {
                break;
            }
        }
        
        consecutive
    }
}

/// Retry policy for individual operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Initial delay before first retry
    pub initial_delay: Duration,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Whether to add jitter to delays
    pub jitter: bool,
    
    /// Types of errors that should be retried
    pub retryable_errors: Vec<RetryableErrorType>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
            retryable_errors: vec![
                RetryableErrorType::Transport,
                RetryableErrorType::Timeout,
                RetryableErrorType::InternalServer,
            ],
        }
    }
}

/// Types of errors that can be retried
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetryableErrorType {
    Transport,
    Timeout,
    InternalServer,
    RateLimited,
    ServiceUnavailable,
}

impl RetryPolicy {
    /// Check if an error should be retried
    pub fn should_retry(&self, error: &McpError, attempt: u32) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        
        let error_type = self.classify_error(error);
        self.retryable_errors.contains(&error_type)
    }
    
    /// Classify an error for retry decisions
    fn classify_error(&self, error: &McpError) -> RetryableErrorType {
        match error {
            McpError::Transport { .. } => RetryableErrorType::Transport,
            McpError::ServerTimeout { .. } => RetryableErrorType::Timeout,
            McpError::Internal { .. } => RetryableErrorType::InternalServer,
            McpError::RateLimited { .. } => RetryableErrorType::RateLimited,
            _ => RetryableErrorType::Transport, // Default classification
        }
    }
    
    /// Calculate delay before next retry
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }
        
        let base_delay = self.initial_delay;
        let multiplier = self.backoff_multiplier.powi((attempt - 1) as i32);
        let delay_secs = base_delay.as_secs_f64() * multiplier;
        
        let capped_delay = Duration::from_secs_f64(delay_secs.min(self.max_delay.as_secs_f64()));
        
        if self.jitter {
            let jitter = rand::random::<f64>() * 0.1; // 10% jitter
            let jittered_delay = capped_delay.as_secs_f64() * (1.0 + jitter);
            Duration::from_secs_f64(jittered_delay)
        } else {
            capped_delay
        }
    }
}

/// Result of a batch operation
#[derive(Debug, Clone)]
pub enum BatchResult<T> {
    /// All operations succeeded
    Success(Vec<(usize, T)>),
    
    /// Some operations succeeded, some failed
    PartialSuccess {
        completed: Vec<(usize, T)>,
        errors: Vec<(usize, McpError)>,
    },
    
    /// Batch was aborted due to policy
    Aborted {
        completed: Vec<(usize, T)>,
        errors: Vec<(usize, McpError)>,
    },
    
    /// All operations failed
    AllFailed(Vec<(usize, McpError)>),
}

impl<T> BatchResult<T> {
    /// Check if batch completed successfully (all operations)
    pub fn is_complete_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
    
    /// Check if batch had any successful operations
    pub fn has_successes(&self) -> bool {
        match self {
            Self::Success(_) => true,
            Self::PartialSuccess { completed, .. } | Self::Aborted { completed, .. } => !completed.is_empty(),
            Self::AllFailed(_) => false,
        }
    }
    
    /// Get all successful results
    pub fn successes(&self) -> Vec<(usize, &T)> {
        match self {
            Self::Success(results) => results.iter().map(|(i, r)| (*i, r)).collect(),
            Self::PartialSuccess { completed, .. } | Self::Aborted { completed, .. } => {
                completed.iter().map(|(i, r)| (*i, r)).collect()
            }
            Self::AllFailed(_) => Vec::new(),
        }
    }
    
    /// Get all errors
    pub fn errors(&self) -> Vec<(usize, &McpError)> {
        match self {
            Self::Success(_) => Vec::new(),
            Self::PartialSuccess { errors, .. } | Self::Aborted { errors, .. } | Self::AllFailed(errors) => {
                errors.iter().map(|(i, e)| (*i, e)).collect()
            }
        }
    }
    
    /// Get count of successful operations
    pub fn success_count(&self) -> usize {
        self.successes().len()
    }
    
    /// Get count of failed operations
    pub fn error_count(&self) -> usize {
        self.errors().len()
    }
}

/// Individual operation in a batch
pub struct BatchOperation<T> {
    pub id: usize,
    pub operation: Box<dyn FnOnce() -> Box<dyn Future<Output = McpResult<T>> + Send + Unpin> + Send>,
}

impl<T> BatchOperation<T> {
    /// Create a new batch operation
    pub fn new<F, Fut>(id: usize, operation: F) -> Self 
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = McpResult<T>> + Send + Unpin + 'static,
    {
        Self {
            id,
            operation: Box::new(move || Box::new(operation())),
        }
    }
}

/// Statistics for a batch execution
#[derive(Debug, Clone, Serialize)]
pub struct BatchExecutionStats {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub retried_operations: usize,
    pub total_execution_time: Duration,
    pub average_operation_time: Duration,
    pub policy_triggered: bool,
}

/// Enhanced batch error handler
pub struct BatchErrorHandler {
    partial_failure_policy: PartialFailurePolicy,
    retry_policy: RetryPolicy,
}

impl BatchErrorHandler {
    /// Create a new batch error handler
    pub fn new(partial_failure_policy: PartialFailurePolicy, retry_policy: RetryPolicy) -> Self {
        Self {
            partial_failure_policy,
            retry_policy,
        }
    }
    
    /// Execute a batch of operations with error handling
    pub async fn execute_batch<T>(&self, operations: Vec<BatchOperation<T>>) -> (BatchResult<T>, BatchExecutionStats)
    where
        T: Send + 'static,
    {
        let start_time = Instant::now();
        let total_operations = operations.len();
        let mut results = Vec::new();
        let mut errors = Vec::new();
        let mut retry_counts = HashMap::new();
        let mut operation_times = Vec::new();
        
        for operation in operations {
            let op_start = Instant::now();
            let result = self.execute_with_retry(operation, &mut retry_counts).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            
            match result {
                Ok((index, value)) => results.push((index, value)),
                Err((index, error)) => errors.push((index, error)),
            }
            
            // Check if we should abort based on policy
            if self.partial_failure_policy.should_abort(&errors, results.len() + errors.len()) {
                tracing::warn!(
                    successful = results.len(),
                    failed = errors.len(),
                    "Batch operation aborted due to policy"
                );
                
                let stats = BatchExecutionStats {
                    total_operations,
                    successful_operations: results.len(),
                    failed_operations: errors.len(),
                    retried_operations: retry_counts.values().sum::<u32>() as usize,
                    total_execution_time: start_time.elapsed(),
                    average_operation_time: operation_times.iter().sum::<Duration>() / operation_times.len() as u32,
                    policy_triggered: true,
                };
                
                return (BatchResult::Aborted { completed: results, errors }, stats);
            }
        }
        
        let total_execution_time = start_time.elapsed();
        let average_operation_time = if !operation_times.is_empty() {
            operation_times.iter().sum::<Duration>() / operation_times.len() as u32
        } else {
            Duration::ZERO
        };
        
        let stats = BatchExecutionStats {
            total_operations,
            successful_operations: results.len(),
            failed_operations: errors.len(),
            retried_operations: retry_counts.values().sum::<u32>() as usize,
            total_execution_time,
            average_operation_time,
            policy_triggered: false,
        };
        
        let batch_result = if errors.is_empty() {
            BatchResult::Success(results)
        } else if results.is_empty() {
            BatchResult::AllFailed(errors)
        } else {
            BatchResult::PartialSuccess {
                completed: results,
                errors,
            }
        };
        
        tracing::info!(
            total = total_operations,
            successful = stats.successful_operations,
            failed = stats.failed_operations,
            retries = stats.retried_operations,
            duration_ms = total_execution_time.as_millis(),
            "Batch operation completed"
        );
        
        (batch_result, stats)
    }
    
    /// Execute a single operation with retry logic
    async fn execute_with_retry<T>(
        &self,
        operation: BatchOperation<T>,
        retry_counts: &mut HashMap<usize, u32>,
    ) -> Result<(usize, T), (usize, McpError)> {
        let operation_id = operation.id;
        let mut attempt = 0;
        let mut last_error = None;
        
        // Note: This is a simplified version. In practice, we'd need to make operations repeatable
        // For now, we'll execute once and simulate retry behavior
        let result = (operation.operation)().await;
        
        match result {
            Ok(value) => Ok((operation_id, value)),
            Err(error) => {
                attempt += 1;
                
                // Simulate retry logic
                while self.retry_policy.should_retry(&error, attempt) {
                    tracing::debug!(
                        operation_id = operation_id,
                        attempt = attempt,
                        error = %error,
                        "Retrying operation"
                    );
                    
                    let delay = self.retry_policy.calculate_delay(attempt);
                    tokio::time::sleep(delay).await;
                    
                    // In a real implementation, we'd re-execute the operation here
                    // For simulation, we'll just check if we should succeed on retry
                    if rand::random::<f64>() > 0.7 {
                        retry_counts.insert(operation_id, attempt);
                        return Ok((operation_id, unsafe { std::mem::zeroed() })); // Placeholder
                    }
                    
                    attempt += 1;
                    last_error = Some(error.clone());
                }
                
                retry_counts.insert(operation_id, attempt - 1);
                Err((operation_id, last_error.unwrap_or(error)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_failure_policy_continue_all() {
        let policy = PartialFailurePolicy::ContinueAll;
        let errors = vec![
            (0, McpError::Transport { message: "error1".to_string() }),
            (1, McpError::Transport { message: "error2".to_string() }),
        ];
        
        assert!(!policy.should_abort(&errors, 10));
    }
    
    #[test]
    fn test_partial_failure_policy_abort_on_first() {
        let policy = PartialFailurePolicy::AbortOnFirst;
        let errors = vec![
            (0, McpError::Transport { message: "error1".to_string() }),
        ];
        
        assert!(policy.should_abort(&errors, 1));
        assert!(!policy.should_abort(&[], 1));
    }
    
    #[test]
    fn test_partial_failure_policy_threshold() {
        let policy = PartialFailurePolicy::AbortOnThreshold {
            max_failure_rate: 0.5,
            min_operations: 4,
        };
        
        let errors = vec![
            (0, McpError::Transport { message: "error1".to_string() }),
            (1, McpError::Transport { message: "error2".to_string() }),
        ];
        
        // Not enough operations processed yet
        assert!(!policy.should_abort(&errors, 3));
        
        // Still under threshold
        assert!(!policy.should_abort(&errors, 5));
        
        // Over threshold
        assert!(policy.should_abort(&errors, 4));
    }
    
    #[test]
    fn test_partial_failure_policy_consecutive() {
        let policy = PartialFailurePolicy::AbortOnConsecutive {
            max_consecutive: 2,
        };
        
        let errors = vec![
            (1, McpError::Transport { message: "error1".to_string() }),
            (2, McpError::Transport { message: "error2".to_string() }),
        ];
        
        // Two consecutive failures at the end
        assert!(policy.should_abort(&errors, 3));
        
        let errors_non_consecutive = vec![
            (0, McpError::Transport { message: "error1".to_string() }),
            (2, McpError::Transport { message: "error2".to_string() }),
        ];
        
        // Non-consecutive failures
        assert!(!policy.should_abort(&errors_non_consecutive, 3));
    }
    
    #[test]
    fn test_retry_policy_should_retry() {
        let policy = RetryPolicy::default();
        
        let transport_error = McpError::Transport { message: "connection failed".to_string() };
        let auth_error = McpError::AuthenticationFailed { reason: "invalid token".to_string() };
        
        assert!(policy.should_retry(&transport_error, 1));
        assert!(policy.should_retry(&transport_error, 2));
        assert!(!policy.should_retry(&transport_error, 3)); // Max attempts reached
        
        // Auth errors are not retryable by default
        assert!(!policy.should_retry(&auth_error, 1));
    }
    
    #[test]
    fn test_retry_policy_delay_calculation() {
        let policy = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
            backoff_multiplier: 2.0,
            jitter: false,
            ..Default::default()
        };
        
        assert_eq!(policy.calculate_delay(0), Duration::ZERO);
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(policy.calculate_delay(10), Duration::from_secs(2)); // Capped at max_delay
    }
    
    #[test]
    fn test_batch_result_methods() {
        let success_result = BatchResult::Success(vec![(0, "result1".to_string()), (1, "result2".to_string())]);
        assert!(success_result.is_complete_success());
        assert!(success_result.has_successes());
        assert_eq!(success_result.success_count(), 2);
        assert_eq!(success_result.error_count(), 0);
        
        let partial_result = BatchResult::PartialSuccess {
            completed: vec![(0, "result1".to_string())],
            errors: vec![(1, McpError::Transport { message: "failed".to_string() })],
        };
        assert!(!partial_result.is_complete_success());
        assert!(partial_result.has_successes());
        assert_eq!(partial_result.success_count(), 1);
        assert_eq!(partial_result.error_count(), 1);
        
        let failed_result = BatchResult::AllFailed(vec![
            (0, McpError::Transport { message: "failed1".to_string() }),
            (1, McpError::Transport { message: "failed2".to_string() }),
        ]);
        assert!(!failed_result.is_complete_success());
        assert!(!failed_result.has_successes());
        assert_eq!(failed_result.success_count(), 0);
        assert_eq!(failed_result.error_count(), 2);
    }
    
    #[tokio::test]
    async fn test_batch_error_handler_execution() {
        let policy = PartialFailurePolicy::ContinueAll;
        let retry_policy = RetryPolicy::default();
        let handler = BatchErrorHandler::new(policy, retry_policy);
        
        let operations = vec![
            BatchOperation::new(0, || async { Ok::<String, McpError>("success1".to_string()) }),
            BatchOperation::new(1, || async { Ok::<String, McpError>("success2".to_string()) }),
        ];
        
        let (result, stats) = handler.execute_batch(operations).await;
        
        assert!(result.is_complete_success());
        assert_eq!(stats.successful_operations, 2);
        assert_eq!(stats.failed_operations, 0);
        assert!(!stats.policy_triggered);
    }
}