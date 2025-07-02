//! Progress reporting for long-running operations

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, warn};

/// Progress update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Unique identifier for the operation
    pub operation_id: String,
    
    /// Current status message
    pub message: String,
    
    /// Progress level/severity
    pub level: ProgressLevel,
    
    /// Current progress (0-100 if percentage, or current count)
    pub current: usize,
    
    /// Total work units (100 for percentage, or total count)
    pub total: usize,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Timestamp when this update was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ProgressUpdate {
    /// Create a new progress update
    pub fn new(
        operation_id: String,
        message: String,
        level: ProgressLevel,
        current: usize,
        total: usize,
    ) -> Self {
        Self {
            operation_id,
            message,
            level,
            current,
            total,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create a progress update for operation start
    pub fn started(operation_id: String, message: String, total: usize) -> Self {
        Self::new(operation_id, message, ProgressLevel::Info, 0, total)
    }
    
    /// Create a progress update for ongoing progress
    pub fn progress(operation_id: String, message: String, current: usize) -> Self {
        Self {
            operation_id,
            message,
            level: ProgressLevel::Info,
            current,
            total: 0, // Will be set by the reporter if known
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create a progress update for operation completion
    pub fn completed(operation_id: String, message: String) -> Self {
        Self {
            operation_id,
            message,
            level: ProgressLevel::Success,
            current: 100,
            total: 100,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create a progress update for operation failure
    pub fn failed(operation_id: String, message: String, error: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), serde_json::Value::String(error));
        
        Self {
            operation_id,
            message,
            level: ProgressLevel::Error,
            current: 0,
            total: 100,
            metadata,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Add metadata to the progress update
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
    
    /// Get progress as a percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }
    
    /// Check if the operation is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.level, ProgressLevel::Success | ProgressLevel::Error)
    }
}

/// Progress level/severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressLevel {
    /// Informational progress update
    Info,
    /// Warning during operation
    Warning,
    /// Successful completion
    Success,
    /// Error/failure
    Error,
}

/// Progress operation tracking
#[derive(Debug, Clone)]
struct ProgressOperation {
    /// Total work units for this operation
    total: usize,
    /// Start time
    started_at: chrono::DateTime<chrono::Utc>,
    /// Latest update
    latest_update: Option<ProgressUpdate>,
}

/// Progress reporter for tracking and broadcasting operation progress
pub struct ProgressReporter {
    /// Active operations
    operations: Arc<RwLock<HashMap<String, ProgressOperation>>>,
    
    /// Broadcast channel for progress updates
    progress_tx: broadcast::Sender<ProgressUpdate>,
    
    /// Maximum number of subscribers
    max_subscribers: usize,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(1000);
        
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            progress_tx,
            max_subscribers: 100,
        }
    }
    
    /// Create a new progress reporter with custom capacity
    pub fn with_capacity(capacity: usize, max_subscribers: usize) -> Self {
        let (progress_tx, _) = broadcast::channel(capacity);
        
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            progress_tx,
            max_subscribers,
        }
    }
    
    /// Start tracking a new operation
    pub async fn start_operation(&self, operation_id: String, total: usize) {
        let operation = ProgressOperation {
            total,
            started_at: chrono::Utc::now(),
            latest_update: None,
        };
        
        let mut operations = self.operations.write().await;
        operations.insert(operation_id.clone(), operation);
        
        debug!("Started tracking operation: {}", operation_id);
    }
    
    /// Report progress for an operation
    pub async fn report_progress(&self, mut update: ProgressUpdate) {
        // Update total from operation if not set
        if update.total == 0 {
            let operations = self.operations.read().await;
            if let Some(operation) = operations.get(&update.operation_id) {
                update.total = operation.total;
            }
        }
        
        // Update the operation's latest update
        {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(&update.operation_id) {
                operation.latest_update = Some(update.clone());
            }
        }
        
        // Broadcast the update
        if let Err(e) = self.progress_tx.send(update.clone()) {
            warn!("Failed to broadcast progress update: {}", e);
        }
        
        debug!(
            "Progress update for {}: {} ({:.1}%)",
            update.operation_id,
            update.message,
            update.percentage()
        );
        
        // Clean up completed operations
        if update.is_complete() {
            self.complete_operation(&update.operation_id).await;
        }
    }
    
    /// Complete an operation and remove it from tracking
    pub async fn complete_operation(&self, operation_id: &str) {
        let mut operations = self.operations.write().await;
        if operations.remove(operation_id).is_some() {
            debug!("Completed operation: {}", operation_id);
        }
    }
    
    /// Subscribe to progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressUpdate> {
        if self.progress_tx.receiver_count() >= self.max_subscribers {
            warn!("Maximum number of progress subscribers reached");
        }
        
        self.progress_tx.subscribe()
    }
    
    /// Get the latest update for an operation
    pub async fn get_latest_update(&self, operation_id: &str) -> Option<ProgressUpdate> {
        let operations = self.operations.read().await;
        operations.get(operation_id)
            .and_then(|op| op.latest_update.clone())
    }
    
    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<String> {
        let operations = self.operations.read().await;
        operations.keys().cloned().collect()
    }
    
    /// Get operation statistics
    pub async fn get_operation_stats(&self, operation_id: &str) -> Option<OperationStats> {
        let operations = self.operations.read().await;
        operations.get(operation_id).map(|op| {
            let elapsed = chrono::Utc::now()
                .signed_duration_since(op.started_at)
                .to_std()
                .unwrap_or_default();
            
            let current = op.latest_update
                .as_ref()
                .map(|u| u.current)
                .unwrap_or(0);
            
            OperationStats {
                operation_id: operation_id.to_string(),
                total: op.total,
                current,
                elapsed,
                estimated_remaining: if current > 0 && current < op.total {
                    let rate = current as f64 / elapsed.as_secs_f64();
                    if rate > 0.0 {
                        let remaining_work = op.total - current;
                        Some(std::time::Duration::from_secs_f64(remaining_work as f64 / rate))
                    } else {
                        None
                    }
                } else {
                    None
                },
            }
        })
    }
    
    /// Clean up old completed operations (housekeeping)
    pub async fn cleanup_old_operations(&self, max_age: std::time::Duration) {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        
        let mut operations = self.operations.write().await;
        operations.retain(|_, operation| operation.started_at > cutoff);
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Operation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    /// Operation identifier
    pub operation_id: String,
    
    /// Total work units
    pub total: usize,
    
    /// Current progress
    pub current: usize,
    
    /// Elapsed time
    #[serde(with = "humantime_serde")]
    pub elapsed: std::time::Duration,
    
    /// Estimated remaining time
    #[serde(with = "humantime_serde", skip_serializing_if = "Option::is_none")]
    pub estimated_remaining: Option<std::time::Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_update_creation() {
        let update = ProgressUpdate::started("test-op".to_string(), "Starting test".to_string(), 100);
        
        assert_eq!(update.operation_id, "test-op");
        assert_eq!(update.message, "Starting test");
        assert_eq!(update.level, ProgressLevel::Info);
        assert_eq!(update.current, 0);
        assert_eq!(update.total, 100);
        assert_eq!(update.percentage(), 0.0);
        assert!(!update.is_complete());
    }

    #[test]
    fn test_progress_percentage() {
        let update = ProgressUpdate::new("test".to_string(), "Progress".to_string(), ProgressLevel::Info, 25, 100);
        assert_eq!(update.percentage(), 25.0);
        
        let update = ProgressUpdate::new("test".to_string(), "Progress".to_string(), ProgressLevel::Info, 0, 0);
        assert_eq!(update.percentage(), 0.0);
    }

    #[test]
    fn test_progress_completion() {
        let completed = ProgressUpdate::completed("test".to_string(), "Done".to_string());
        assert!(completed.is_complete());
        assert_eq!(completed.level, ProgressLevel::Success);
        
        let failed = ProgressUpdate::failed("test".to_string(), "Failed".to_string(), "Error".to_string());
        assert!(failed.is_complete());
        assert_eq!(failed.level, ProgressLevel::Error);
        
        let progress = ProgressUpdate::progress("test".to_string(), "Working".to_string(), 50);
        assert!(!progress.is_complete());
    }

    #[tokio::test]
    async fn test_progress_reporter() {
        let reporter = ProgressReporter::new();
        
        // Start an operation
        reporter.start_operation("test-op".to_string(), 100).await;
        
        // Check active operations
        let active = reporter.get_active_operations().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "test-op");
        
        // Report progress
        let update = ProgressUpdate::progress("test-op".to_string(), "Working".to_string(), 50);
        reporter.report_progress(update).await;
        
        // Get latest update
        let latest = reporter.get_latest_update("test-op").await;
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().current, 50);
        
        // Complete operation
        let completed = ProgressUpdate::completed("test-op".to_string(), "Done".to_string());
        reporter.report_progress(completed).await;
        
        // Check that operation is cleaned up
        let active = reporter.get_active_operations().await;
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_progress_subscription() {
        let reporter = ProgressReporter::new();
        let mut receiver = reporter.subscribe();
        
        // Report progress
        let update = ProgressUpdate::started("test".to_string(), "Test".to_string(), 100);
        reporter.report_progress(update.clone()).await;
        
        // Receive the update
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.operation_id, update.operation_id);
        assert_eq!(received.message, update.message);
    }
}