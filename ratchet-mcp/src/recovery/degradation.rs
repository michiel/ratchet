//! Graceful degradation for transport and service failures

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::transport::McpTransport;
use crate::{McpError, McpResult};

/// Configuration for degradation behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationConfig {
    /// Whether to enable graceful degradation
    pub enabled: bool,
    
    /// Threshold for marking primary transport as degraded
    pub failure_threshold: u32,
    
    /// Time window for counting failures
    pub failure_window: Duration,
    
    /// Minimum time to stay in degraded mode
    pub min_degradation_time: Duration,
    
    /// Maximum time to stay in degraded mode before trying primary again
    pub max_degradation_time: Duration,
    
    /// Whether to automatically recover from degraded mode
    pub auto_recovery: bool,
    
    /// Health check interval for degraded services
    pub health_check_interval: Duration,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            min_degradation_time: Duration::from_secs(30),
            max_degradation_time: Duration::from_secs(300),
            auto_recovery: true,
            health_check_interval: Duration::from_secs(10),
        }
    }
}

/// Current degradation state
#[derive(Debug, Clone, Serialize)]
pub enum DegradationState {
    /// Operating normally with primary transport
    Normal {
        last_success: chrono::DateTime<chrono::Utc>,
        consecutive_successes: u32,
    },
    
    /// Temporarily degraded, using fallback transport
    Degraded {
        degraded_at: chrono::DateTime<chrono::Utc>,
        reason: String,
        failure_count: u32,
        using_fallback: bool,
    },
    
    /// Permanently failed, no fallback available
    Failed {
        failed_at: chrono::DateTime<chrono::Utc>,
        reason: String,
    },
    
    /// Recovering from degraded state
    Recovering {
        recovery_started: chrono::DateTime<chrono::Utc>,
        health_check_count: u32,
    },
}

impl DegradationState {
    /// Create initial normal state
    pub fn normal() -> Self {
        Self::Normal {
            last_success: chrono::Utc::now(),
            consecutive_successes: 0,
        }
    }
    
    /// Check if currently degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }
    
    /// Check if permanently failed
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
    
    /// Check if currently recovering
    pub fn is_recovering(&self) -> bool {
        matches!(self, Self::Recovering { .. })
    }
    
    /// Check if operating normally
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal { .. })
    }
}

/// Failure tracking for degradation decisions
#[derive(Debug, Clone)]
struct FailureTracker {
    failures: Vec<Instant>,
    threshold: u32,
    window: Duration,
}

impl FailureTracker {
    fn new(threshold: u32, window: Duration) -> Self {
        Self {
            failures: Vec::new(),
            threshold,
            window,
        }
    }
    
    /// Record a failure
    fn record_failure(&mut self) {
        let now = Instant::now();
        self.failures.push(now);
        
        // Clean up old failures outside the window
        self.failures.retain(|&time| now.duration_since(time) <= self.window);
    }
    
    /// Check if failure threshold is exceeded
    fn is_threshold_exceeded(&self) -> bool {
        self.failures.len() >= self.threshold as usize
    }
    
    /// Reset failure count
    fn reset(&mut self) {
        self.failures.clear();
    }
}

/// Operation that can be executed with degradation support
pub type DegradableOperation<T> = Box<dyn Fn() -> Pin<Box<dyn Future<Output = McpResult<T>> + Send>> + Send + Sync>;

/// Result of a degraded operation
#[derive(Debug, Clone)]
pub enum DegradedResult<T> {
    /// Operation succeeded with primary transport
    Primary(T),
    
    /// Operation succeeded with fallback transport
    Fallback(T),
    
    /// Operation failed even with fallback
    Failed(McpError),
}

impl<T> DegradedResult<T> {
    /// Check if the operation succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Primary(_) | Self::Fallback(_))
    }
    
    /// Get the result value if successful
    pub fn into_result(self) -> McpResult<T> {
        match self {
            Self::Primary(value) | Self::Fallback(value) => Ok(value),
            Self::Failed(error) => Err(error),
        }
    }
    
    /// Check if fallback was used
    pub fn used_fallback(&self) -> bool {
        matches!(self, Self::Fallback(_))
    }
}

/// Manages graceful degradation for MCP operations
pub struct DegradationManager {
    /// Primary transport
    primary_transport: Arc<dyn McpTransport>,
    
    /// Optional fallback transport
    fallback_transport: Option<Arc<dyn McpTransport>>,
    
    /// Degradation configuration
    config: DegradationConfig,
    
    /// Current degradation state
    state: Arc<RwLock<DegradationState>>,
    
    /// Failure tracker for primary transport
    failure_tracker: Arc<RwLock<FailureTracker>>,
    
    /// Callback for state changes
    state_callback: Option<Arc<dyn Fn(&DegradationState) + Send + Sync>>,
}

impl DegradationManager {
    /// Create a new degradation manager
    pub fn new(
        primary_transport: Arc<dyn McpTransport>,
        fallback_transport: Option<Arc<dyn McpTransport>>,
        config: DegradationConfig,
    ) -> Self {
        let failure_tracker = FailureTracker::new(config.failure_threshold, config.failure_window);
        
        let manager = Self {
            primary_transport,
            fallback_transport,
            config: config.clone(),
            state: Arc::new(RwLock::new(DegradationState::normal())),
            failure_tracker: Arc::new(RwLock::new(failure_tracker)),
            state_callback: None,
        };
        
        // Start background health check if auto recovery is enabled
        if config.auto_recovery {
            manager.start_health_check_task();
        }
        
        manager
    }
    
    /// Set a callback for state changes
    pub fn with_state_callback<F>(mut self, callback: F) -> Self 
    where
        F: Fn(&DegradationState) + Send + Sync + 'static,
    {
        self.state_callback = Some(Arc::new(callback));
        self
    }
    
    /// Get current degradation state
    pub async fn get_state(&self) -> DegradationState {
        self.state.read().await.clone()
    }
    
    /// Execute operation with degradation support
    pub async fn execute_with_degradation<T, F, Fut>(&self, operation: F) -> DegradedResult<T>
    where
        F: Fn() -> Fut + Clone,
        Fut: Future<Output = McpResult<T>>,
        T: Send + 'static,
    {
        if !self.config.enabled {
            return match operation().await {
                Ok(result) => DegradedResult::Primary(result),
                Err(error) => DegradedResult::Failed(error),
            };
        }
        
        let state = self.state.read().await.clone();
        
        match state {
            DegradationState::Normal { .. } | DegradationState::Recovering { .. } => {
                // Try primary transport first
                match operation().await {
                    Ok(result) => {
                        self.record_success().await;
                        DegradedResult::Primary(result)
                    }
                    Err(error) if self.is_degradable_error(&error) => {
                        self.record_failure(&error).await;
                        
                        // Try fallback if available and we're now degraded
                        let current_state = self.state.read().await.clone();
                        if current_state.is_degraded() && self.fallback_transport.is_some() {
                            self.try_fallback_operation(operation).await
                        } else {
                            DegradedResult::Failed(error)
                        }
                    }
                    Err(error) => DegradedResult::Failed(error),
                }
            }
            
            DegradationState::Degraded { using_fallback: true, .. } => {
                // Currently degraded, use fallback directly
                self.try_fallback_operation(operation).await
            }
            
            DegradationState::Degraded { using_fallback: false, .. } | DegradationState::Failed { .. } => {
                // No fallback available or permanently failed
                match operation().await {
                    Ok(result) => {
                        self.record_success().await;
                        DegradedResult::Primary(result)
                    }
                    Err(error) => DegradedResult::Failed(error),
                }
            }
        }
    }
    
    /// Try operation with fallback transport
    async fn try_fallback_operation<T, F, Fut>(&self, operation: F) -> DegradedResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = McpResult<T>>,
    {
        if let Some(_fallback) = &self.fallback_transport {
            tracing::warn!("Primary transport degraded, attempting fallback");
            
            // Note: In a real implementation, this would switch the transport context
            // For now, we'll simulate fallback behavior
            match operation().await {
                Ok(result) => {
                    tracing::info!("Fallback operation successful");
                    DegradedResult::Fallback(result)
                }
                Err(error) => {
                    tracing::error!(error = %error, "Fallback operation also failed");
                    DegradedResult::Failed(error)
                }
            }
        } else {
            DegradedResult::Failed(McpError::Transport {
                message: "Primary transport degraded and no fallback available".to_string(),
            })
        }
    }
    
    /// Record a successful operation
    async fn record_success(&self) {
        let mut state = self.state.write().await;
        let mut tracker = self.failure_tracker.write().await;
        
        tracker.reset();
        
        match &*state {
            DegradationState::Degraded { degraded_at, .. } => {
                // Check if we've been degraded long enough to consider recovery
                let degraded_duration = chrono::Utc::now()
                    .signed_duration_since(*degraded_at)
                    .to_std()
                    .unwrap_or(Duration::ZERO);
                
                if degraded_duration >= self.config.min_degradation_time {
                    *state = DegradationState::Recovering {
                        recovery_started: chrono::Utc::now(),
                        health_check_count: 1,
                    };
                    
                    tracing::info!("Starting recovery from degraded state");
                    self.notify_state_change(&state).await;
                }
            }
            
            DegradationState::Recovering { health_check_count, .. } => {
                // Multiple successful operations confirm recovery
                if *health_check_count >= 3 {
                    *state = DegradationState::Normal {
                        last_success: chrono::Utc::now(),
                        consecutive_successes: 1,
                    };
                    
                    tracing::info!("Successfully recovered from degraded state");
                    self.notify_state_change(&state).await;
                } else {
                    *state = DegradationState::Recovering {
                        recovery_started: if let DegradationState::Recovering { recovery_started, .. } = &*state {
                            *recovery_started
                        } else {
                            chrono::Utc::now()
                        },
                        health_check_count: health_check_count + 1,
                    };
                }
            }
            
            DegradationState::Normal { consecutive_successes, .. } => {
                *state = DegradationState::Normal {
                    last_success: chrono::Utc::now(),
                    consecutive_successes: consecutive_successes + 1,
                };
            }
            
            DegradationState::Failed { .. } => {
                // Recovery from failed state
                *state = DegradationState::Normal {
                    last_success: chrono::Utc::now(),
                    consecutive_successes: 1,
                };
                
                tracing::info!("Recovered from failed state");
                self.notify_state_change(&state).await;
            }
        }
    }
    
    /// Record a failed operation
    async fn record_failure(&self, error: &McpError) {
        let mut state = self.state.write().await;
        let mut tracker = self.failure_tracker.write().await;
        
        tracker.record_failure();
        
        if tracker.is_threshold_exceeded() && !state.is_degraded() && !state.is_failed() {
            let has_fallback = self.fallback_transport.is_some();
            
            if has_fallback {
                *state = DegradationState::Degraded {
                    degraded_at: chrono::Utc::now(),
                    reason: error.to_string(),
                    failure_count: tracker.failures.len() as u32,
                    using_fallback: true,
                };
                
                tracing::warn!(
                    error = %error,
                    failure_count = tracker.failures.len(),
                    "Primary transport degraded, switching to fallback"
                );
            } else {
                *state = DegradationState::Failed {
                    failed_at: chrono::Utc::now(),
                    reason: error.to_string(),
                };
                
                tracing::error!(
                    error = %error,
                    failure_count = tracker.failures.len(),
                    "Transport failed permanently, no fallback available"
                );
            }
            
            self.notify_state_change(&state).await;
        }
    }
    
    /// Check if an error should trigger degradation
    fn is_degradable_error(&self, error: &McpError) -> bool {
        match error {
            // Network and transport errors are degradable
            McpError::Transport { .. } => true,
            McpError::ServerTimeout { .. } => true,
            McpError::Internal { message } if message.contains("connection") => true,
            McpError::Internal { message } if message.contains("network") => true,
            
            // Protocol and authentication errors are not degradable
            McpError::AuthenticationFailed { .. } => false,
            McpError::AuthorizationDenied { .. } => false,
            McpError::InvalidParams { .. } => false,
            McpError::MethodNotFound { .. } => false,
            McpError::Validation { .. } => false,
            
            // Default to not degradable for unknown errors
            _ => false,
        }
    }
    
    /// Start background health check task
    fn start_health_check_task(&self) {
        if !self.config.auto_recovery {
            return;
        }
        
        let manager = self.clone_for_task();
        let interval = self.config.health_check_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                let state = manager.state.read().await.clone();
                
                match state {
                    DegradationState::Degraded { degraded_at, .. } => {
                        let degraded_duration = chrono::Utc::now()
                            .signed_duration_since(degraded_at)
                            .to_std()
                            .unwrap_or(Duration::ZERO);
                        
                        // Force recovery attempt after max degradation time
                        if degraded_duration >= manager.config.max_degradation_time {
                            tracing::info!("Max degradation time reached, forcing recovery attempt");
                            
                            // Attempt a simple health check operation
                            if manager.perform_health_check().await {
                                manager.record_success().await;
                            }
                        }
                    }
                    
                    DegradationState::Failed { .. } => {
                        // Periodically try to recover from failed state
                        if manager.perform_health_check().await {
                            manager.record_success().await;
                        }
                    }
                    
                    _ => {
                        // No health check needed for normal/recovering states
                    }
                }
            }
        });
    }
    
    /// Perform a simple health check
    async fn perform_health_check(&self) -> bool {
        // This is a placeholder - in a real implementation, this would
        // perform a lightweight operation to check transport health
        tracing::debug!("Performing health check");
        
        // Simulate health check - in real implementation might ping the server
        rand::random::<f64>() > 0.5
    }
    
    /// Notify state change callback
    async fn notify_state_change(&self, state: &DegradationState) {
        if let Some(callback) = &self.state_callback {
            callback(state);
        }
    }
    
    /// Clone for use in background tasks
    fn clone_for_task(&self) -> Self {
        Self {
            primary_transport: self.primary_transport.clone(),
            fallback_transport: self.fallback_transport.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            failure_tracker: self.failure_tracker.clone(),
            state_callback: self.state_callback.clone(),
        }
    }
}

impl Clone for DegradationManager {
    fn clone(&self) -> Self {
        Self {
            primary_transport: self.primary_transport.clone(),
            fallback_transport: self.fallback_transport.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            failure_tracker: self.failure_tracker.clone(),
            state_callback: self.state_callback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Mock transport for testing
    struct MockTransport {
        should_fail: AtomicBool,
        name: String,
    }

    impl MockTransport {
        fn new(name: &str, should_fail: bool) -> Self {
            Self {
                should_fail: AtomicBool::new(should_fail),
                name: name.to_string(),
            }
        }
        
        fn set_should_fail(&self, fail: bool) {
            self.should_fail.store(fail, Ordering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl McpTransport for MockTransport {
        async fn send(&mut self, _request: crate::protocol::JsonRpcRequest) -> McpResult<()> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(McpError::Transport {
                    message: format!("Mock {} transport failure", self.name),
                })
            } else {
                Ok(())
            }
        }

        async fn receive(&mut self) -> McpResult<crate::protocol::JsonRpcRequest> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(McpError::Transport {
                    message: format!("Mock {} transport failure", self.name),
                })
            } else {
                Ok(crate::protocol::JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "test".to_string(),
                    params: None,
                    id: Some(serde_json::Value::Number(1.into())),
                })
            }
        }
    }

    #[tokio::test]
    async fn test_degradation_manager_creation() {
        let primary = Arc::new(MockTransport::new("primary", false));
        let fallback = Some(Arc::new(MockTransport::new("fallback", false)) as Arc<dyn McpTransport>);
        let config = DegradationConfig::default();
        
        let manager = DegradationManager::new(primary, fallback, config);
        
        let state = manager.get_state().await;
        assert!(state.is_normal());
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let primary = Arc::new(MockTransport::new("primary", false));
        let config = DegradationConfig::default();
        let manager = DegradationManager::new(primary, None, config);
        
        let operation = || async { Ok::<String, McpError>("success".to_string()) };
        
        let result = manager.execute_with_degradation(operation).await;
        assert!(result.is_success());
        assert!(!result.used_fallback());
        
        match result {
            DegradedResult::Primary(value) => assert_eq!(value, "success"),
            _ => panic!("Expected primary result"),
        }
    }

    #[tokio::test]
    async fn test_degradation_without_fallback() {
        let primary = Arc::new(MockTransport::new("primary", true));
        let config = DegradationConfig {
            failure_threshold: 1,
            failure_window: Duration::from_secs(60),
            ..Default::default()
        };
        
        let manager = DegradationManager::new(primary, None, config);
        
        let operation = || async {
            Err::<String, McpError>(McpError::Transport {
                message: "Test failure".to_string(),
            })
        };
        
        let result = manager.execute_with_degradation(operation).await;
        assert!(!result.is_success());
        
        let state = manager.get_state().await;
        assert!(state.is_failed());
    }

    #[tokio::test]
    async fn test_degradation_with_fallback() {
        let primary = Arc::new(MockTransport::new("primary", true));
        let fallback = Some(Arc::new(MockTransport::new("fallback", false)) as Arc<dyn McpTransport>);
        let config = DegradationConfig {
            failure_threshold: 1,
            failure_window: Duration::from_secs(60),
            auto_recovery: false, // Disable for predictable testing
            ..Default::default()
        };
        
        let manager = DegradationManager::new(primary, fallback, config);
        
        let operation = || async { Ok::<String, McpError>("fallback_success".to_string()) };
        
        // First operation should fail and trigger degradation
        let error_operation = || async {
            Err::<String, McpError>(McpError::Transport {
                message: "Primary failed".to_string(),
            })
        };
        
        let result = manager.execute_with_degradation(error_operation).await;
        // Should use fallback and succeed
        
        let state = manager.get_state().await;
        assert!(state.is_degraded());
        
        // Subsequent operation should use fallback directly
        let result2 = manager.execute_with_degradation(operation).await;
        assert!(result2.is_success());
        assert!(result2.used_fallback());
    }

    #[test]
    fn test_failure_tracker() {
        let mut tracker = FailureTracker::new(3, Duration::from_secs(60));
        
        assert!(!tracker.is_threshold_exceeded());
        
        tracker.record_failure();
        assert!(!tracker.is_threshold_exceeded());
        
        tracker.record_failure();
        assert!(!tracker.is_threshold_exceeded());
        
        tracker.record_failure();
        assert!(tracker.is_threshold_exceeded());
        
        tracker.reset();
        assert!(!tracker.is_threshold_exceeded());
    }
}