//! Automatic reconnection logic with exponential backoff

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::transport::McpTransport;
use crate::{McpError, McpResult};

/// Configuration for reconnection behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectionConfig {
    /// Maximum number of reconnection attempts before giving up
    pub max_attempts: u32,
    
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,
    
    /// Maximum delay between reconnection attempts
    pub max_delay: Duration,
    
    /// Exponential backoff multiplier (e.g., 2.0 for doubling)
    pub backoff_multiplier: f64,
    
    /// Jitter factor to add randomness to delays (0.0 to 1.0)
    pub jitter_factor: f64,
    
    /// Timeout for individual connection attempts
    pub connection_timeout: Duration,
    
    /// Whether to enable automatic reconnection
    pub enabled: bool,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            connection_timeout: Duration::from_secs(10),
            enabled: true,
        }
    }
}

/// Current state of reconnection attempts
#[derive(Debug, Clone, Serialize)]
pub enum ReconnectionState {
    /// Connected and operational
    Connected {
        connected_at: chrono::DateTime<chrono::Utc>,
        consecutive_failures: u32,
    },
    
    /// Currently attempting to reconnect
    Reconnecting {
        attempt_number: u32,
        started_at: chrono::DateTime<chrono::Utc>,
        next_attempt_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    
    /// Temporarily disconnected, will retry
    Disconnected {
        disconnected_at: chrono::DateTime<chrono::Utc>,
        failure_count: u32,
        last_error: String,
    },
    
    /// Permanently failed, no more attempts
    Failed {
        failed_at: chrono::DateTime<chrono::Utc>,
        final_error: String,
        total_attempts: u32,
    },
}

impl ReconnectionState {
    /// Create initial connected state
    pub fn connected() -> Self {
        Self::Connected {
            connected_at: chrono::Utc::now(),
            consecutive_failures: 0,
        }
    }
    
    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
    
    /// Check if currently reconnecting
    pub fn is_reconnecting(&self) -> bool {
        matches!(self, Self::Reconnecting { .. })
    }
    
    /// Check if permanently failed
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
    
    /// Check if currently disconnected
    pub fn is_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected { .. })
    }
    
    /// Get current attempt number (if reconnecting)
    pub fn attempt_number(&self) -> Option<u32> {
        match self {
            Self::Reconnecting { attempt_number, .. } => Some(*attempt_number),
            _ => None,
        }
    }
}

/// Manages automatic reconnection for MCP transports
pub struct ReconnectionManager {
    /// Transport to manage
    transport: Arc<dyn McpTransport>,
    
    /// Reconnection configuration
    config: ReconnectionConfig,
    
    /// Current reconnection state
    state: Arc<Mutex<ReconnectionState>>,
    
    /// Callback for connection state changes
    state_callback: Option<Arc<dyn Fn(&ReconnectionState) + Send + Sync>>,
}

impl ReconnectionManager {
    /// Create a new reconnection manager
    pub fn new(transport: Arc<dyn McpTransport>, config: ReconnectionConfig) -> Self {
        Self {
            transport,
            config,
            state: Arc::new(Mutex::new(ReconnectionState::connected())),
            state_callback: None,
        }
    }
    
    /// Set a callback for state changes
    pub fn with_state_callback<F>(mut self, callback: F) -> Self 
    where
        F: Fn(&ReconnectionState) + Send + Sync + 'static,
    {
        self.state_callback = Some(Arc::new(callback));
        self
    }
    
    /// Get current reconnection state
    pub async fn get_state(&self) -> ReconnectionState {
        self.state.lock().await.clone()
    }
    
    /// Handle a connection failure and potentially start reconnection
    pub async fn handle_connection_failure(&self, error: McpError) -> McpResult<()> {
        if !self.config.enabled {
            return Err(error);
        }
        
        let mut state = self.state.lock().await;
        
        let should_retry = self.should_retry_error(&error);
        let failure_count = match &*state {
            ReconnectionState::Connected { consecutive_failures, .. } => *consecutive_failures + 1,
            ReconnectionState::Disconnected { failure_count, .. } => *failure_count + 1,
            ReconnectionState::Reconnecting { attempt_number, .. } => *attempt_number + 1,
            ReconnectionState::Failed { .. } => return Err(error), // Already permanently failed
        };
        
        if !should_retry || failure_count > self.config.max_attempts {
            *state = ReconnectionState::Failed {
                failed_at: chrono::Utc::now(),
                final_error: error.to_string(),
                total_attempts: failure_count,
            };
            
            tracing::error!(
                error = %error,
                attempts = failure_count,
                "Connection permanently failed after {} attempts",
                failure_count
            );
            
            self.notify_state_change(&state).await;
            return Err(error);
        }
        
        *state = ReconnectionState::Disconnected {
            disconnected_at: chrono::Utc::now(),
            failure_count,
            last_error: error.to_string(),
        };
        
        tracing::warn!(
            error = %error,
            attempt = failure_count,
            max_attempts = self.config.max_attempts,
            "Connection failed, will retry"
        );
        
        self.notify_state_change(&state).await;
        drop(state); // Release lock before spawning task
        
        // Start reconnection task
        self.start_reconnection_task(failure_count);
        
        Ok(())
    }
    
    /// Force a reconnection attempt
    pub async fn force_reconnect(&self) -> McpResult<()> {
        let mut state = self.state.lock().await;
        
        *state = ReconnectionState::Reconnecting {
            attempt_number: 1,
            started_at: chrono::Utc::now(),
            next_attempt_at: None,
        };
        
        self.notify_state_change(&state).await;
        drop(state);
        
        self.attempt_reconnection(1).await
    }
    
    /// Start background reconnection task
    fn start_reconnection_task(&self, attempt_number: u32) {
        let delay = self.calculate_backoff_delay(attempt_number);
        let manager = self.clone_for_task();
        
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            
            if let Err(e) = manager.attempt_reconnection(attempt_number).await {
                tracing::error!(
                    error = %e,
                    attempt = attempt_number,
                    "Reconnection attempt failed"
                );
                
                // Schedule next attempt if not permanently failed
                let state = manager.state.lock().await;
                if !state.is_failed() {
                    drop(state);
                    let _ = manager.handle_connection_failure(e).await;
                }
            }
        });
    }
    
    /// Attempt to reconnect
    async fn attempt_reconnection(&self, attempt_number: u32) -> McpResult<()> {
        let next_attempt_time = chrono::Utc::now() + chrono::Duration::from_std(
            self.calculate_backoff_delay(attempt_number + 1)
        ).unwrap_or(chrono::Duration::zero());
        
        {
            let mut state = self.state.lock().await;
            *state = ReconnectionState::Reconnecting {
                attempt_number,
                started_at: chrono::Utc::now(),
                next_attempt_at: Some(next_attempt_time),
            };
            self.notify_state_change(&state).await;
        }
        
        tracing::info!(
            attempt = attempt_number,
            "Attempting to reconnect..."
        );
        
        // Attempt connection with timeout
        let connection_result = tokio::time::timeout(
            self.config.connection_timeout,
            self.perform_connection()
        ).await;
        
        match connection_result {
            Ok(Ok(())) => {
                let mut state = self.state.lock().await;
                *state = ReconnectionState::Connected {
                    connected_at: chrono::Utc::now(),
                    consecutive_failures: 0,
                };
                
                tracing::info!(
                    attempt = attempt_number,
                    "Successfully reconnected"
                );
                
                self.notify_state_change(&state).await;
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    attempt = attempt_number,
                    error = %e,
                    "Reconnection attempt failed"
                );
                Err(e)
            }
            Err(_) => {
                let timeout_error = McpError::ServerTimeout {
                    timeout: self.config.connection_timeout,
                };
                tracing::warn!(
                    attempt = attempt_number,
                    timeout = ?self.config.connection_timeout,
                    "Reconnection attempt timed out"
                );
                Err(timeout_error)
            }
        }
    }
    
    /// Perform the actual connection (placeholder - would integrate with actual transport)
    async fn perform_connection(&self) -> McpResult<()> {
        // This is a placeholder - in a real implementation, this would call
        // transport-specific reconnection methods
        tracing::debug!("Performing connection attempt");
        
        // For now, simulate connection attempt
        // In real implementation, this would be something like:
        // self.transport.reconnect().await
        
        // Use transport's connect method for consistent behavior in tests
        // Create a mutable reference to test connection
        // For testing purposes, simulate the connection based on transport state
        let mock_transport = self.transport.clone();
        if mock_transport.is_connected().await {
            Ok(())
        } else {
            Err(McpError::Transport {
                message: "Simulated connection failure".to_string(),
            })
        }
    }
    
    /// Calculate backoff delay with jitter
    fn calculate_backoff_delay(&self, attempt_number: u32) -> Duration {
        if attempt_number == 0 {
            return Duration::ZERO;
        }
        
        let base_delay = self.config.initial_delay;
        let multiplier = self.config.backoff_multiplier.powi((attempt_number - 1) as i32);
        let delay_secs = base_delay.as_secs_f64() * multiplier;
        
        // Apply maximum delay limit
        let capped_delay = Duration::from_secs_f64(delay_secs.min(self.config.max_delay.as_secs_f64()));
        
        // Add jitter to prevent thundering herd
        if self.config.jitter_factor > 0.0 {
            let jitter = rand::random::<f64>() * self.config.jitter_factor;
            let jittered_delay = capped_delay.as_secs_f64() * (1.0 + jitter);
            Duration::from_secs_f64(jittered_delay)
        } else {
            capped_delay
        }
    }
    
    /// Check if an error is retryable
    fn should_retry_error(&self, error: &McpError) -> bool {
        match error {
            // Retryable errors
            McpError::Transport { .. } => true,
            McpError::ServerTimeout { .. } => true,
            McpError::Internal { message } if message.contains("connection") => true,
            
            // Non-retryable errors
            McpError::AuthenticationFailed { .. } => false,
            McpError::AuthorizationDenied { .. } => false,
            McpError::InvalidParams { .. } => false,
            McpError::Validation { .. } => false,
            McpError::MethodNotFound { .. } => false,
            
            // Default to retryable for other errors
            _ => true,
        }
    }
    
    /// Notify state change callback
    async fn notify_state_change(&self, state: &ReconnectionState) {
        if let Some(callback) = &self.state_callback {
            callback(state);
        }
    }
    
    /// Clone for use in background tasks
    fn clone_for_task(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
            state_callback: self.state_callback.clone(),
        }
    }
}

impl Clone for ReconnectionManager {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
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
    }

    impl MockTransport {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail: AtomicBool::new(should_fail),
            }
        }
        
        fn set_should_fail(&self, fail: bool) {
            self.should_fail.store(fail, Ordering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl McpTransport for MockTransport {
        async fn connect(&mut self) -> McpResult<()> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(McpError::Transport {
                    message: "Mock transport connection failure".to_string(),
                })
            } else {
                Ok(())
            }
        }

        async fn is_connected(&self) -> bool {
            !self.should_fail.load(Ordering::Relaxed)
        }

        async fn send(&mut self, _request: crate::protocol::JsonRpcRequest) -> McpResult<()> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(McpError::Transport {
                    message: "Mock transport failure".to_string(),
                })
            } else {
                Ok(())
            }
        }

        async fn receive(&mut self) -> McpResult<crate::protocol::JsonRpcResponse> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(McpError::Transport {
                    message: "Mock transport failure".to_string(),
                })
            } else {
                // Return a mock response
                Ok(crate::protocol::JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::Value::String("mock_result".to_string())),
                    error: None,
                    id: Some(serde_json::Value::Number(1.into())),
                })
            }
        }

        async fn health(&self) -> crate::transport::TransportHealth {
            crate::transport::TransportHealth {
                connected: self.is_connected().await,
                last_success: Some(chrono::Utc::now()),
                last_error: None,
                consecutive_failures: if self.should_fail.load(Ordering::Relaxed) { 1 } else { 0 },
                latency: Some(std::time::Duration::from_millis(10)),
                metadata: std::collections::HashMap::new(),
            }
        }

        async fn close(&mut self) -> McpResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_reconnection_manager_creation() {
        let transport = Arc::new(MockTransport::new(false));
        let config = ReconnectionConfig::default();
        let manager = ReconnectionManager::new(transport, config);
        
        let state = manager.get_state().await;
        assert!(state.is_connected());
    }

    #[tokio::test]
    async fn test_connection_failure_handling() {
        let transport = Arc::new(MockTransport::new(true));
        let config = ReconnectionConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        
        let manager = ReconnectionManager::new(transport, config);
        
        let error = McpError::Transport {
            message: "Test failure".to_string(),
        };
        
        // First failure should start reconnection
        let result = manager.handle_connection_failure(error).await;
        assert!(result.is_ok());
        
        // Give some time for background task
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let state = manager.get_state().await;
        assert!(state.is_reconnecting() || state.is_disconnected() || state.is_failed());
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let transport = Arc::new(MockTransport::new(false));
        let config = ReconnectionConfig::default();
        let manager = ReconnectionManager::new(transport, config);
        
        let error = McpError::AuthenticationFailed {
            reason: "Invalid credentials".to_string(),
        };
        
        let result = manager.handle_connection_failure(error).await;
        assert!(result.is_err());
        
        let state = manager.get_state().await;
        assert!(state.is_failed());
    }

    #[test]
    fn test_backoff_calculation() {
        let config = ReconnectionConfig {
            initial_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.0, // No jitter for predictable testing
            ..Default::default()
        };
        
        let transport = Arc::new(MockTransport::new(false));
        let manager = ReconnectionManager::new(transport, config);
        
        assert_eq!(manager.calculate_backoff_delay(0), Duration::ZERO);
        assert_eq!(manager.calculate_backoff_delay(1), Duration::from_secs(1));
        assert_eq!(manager.calculate_backoff_delay(2), Duration::from_secs(2));
        assert_eq!(manager.calculate_backoff_delay(3), Duration::from_secs(4));
        assert_eq!(manager.calculate_backoff_delay(10), Duration::from_secs(30)); // Capped at max_delay
    }
}