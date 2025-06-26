//! Enhanced error recovery and graceful degradation for MCP operations

pub mod reconnection;
pub mod degradation; 
pub mod batch_error;

pub use reconnection::{ReconnectionManager, ReconnectionConfig, ReconnectionState};
pub use degradation::{DegradationManager, DegradationConfig, DegradationState};
pub use batch_error::{BatchErrorHandler, BatchResult, PartialFailurePolicy, RetryPolicy};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for error recovery behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// Reconnection settings
    pub reconnection: ReconnectionConfig,
    
    /// Degradation settings
    pub degradation: DegradationConfig,
    
    /// Retry policy for operations
    pub retry_policy: RetryPolicy,
    
    /// Whether to enable enhanced error recovery
    pub enabled: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            reconnection: ReconnectionConfig::default(),
            degradation: DegradationConfig::default(),
            retry_policy: RetryPolicy::default(),
            enabled: true,
        }
    }
}

/// Centralized error recovery coordinator
pub struct ErrorRecoveryCoordinator {
    config: ErrorRecoveryConfig,
    reconnection_manager: Option<ReconnectionManager>,
    degradation_manager: Option<DegradationManager>,
    batch_error_handler: BatchErrorHandler,
}

impl ErrorRecoveryCoordinator {
    /// Create a new error recovery coordinator
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        let batch_error_handler = BatchErrorHandler::new(
            PartialFailurePolicy::default(),
            config.retry_policy.clone(),
        );
        
        Self {
            config,
            reconnection_manager: None,
            degradation_manager: None,
            batch_error_handler,
        }
    }
    
    /// Set the reconnection manager
    pub fn with_reconnection_manager(mut self, manager: ReconnectionManager) -> Self {
        self.reconnection_manager = Some(manager);
        self
    }
    
    /// Set the degradation manager
    pub fn with_degradation_manager(mut self, manager: DegradationManager) -> Self {
        self.degradation_manager = Some(manager);
        self
    }
    
    /// Get the reconnection manager
    pub fn reconnection_manager(&self) -> Option<&ReconnectionManager> {
        self.reconnection_manager.as_ref()
    }
    
    /// Get the degradation manager
    pub fn degradation_manager(&self) -> Option<&DegradationManager> {
        self.degradation_manager.as_ref()
    }
    
    /// Get the batch error handler
    pub fn batch_error_handler(&self) -> &BatchErrorHandler {
        &self.batch_error_handler
    }
    
    /// Check if error recovery is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}