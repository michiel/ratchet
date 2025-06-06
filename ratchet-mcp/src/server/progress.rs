//! Progress notification handling for streaming task execution

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::protocol::messages::{McpMethod, McpNotification, TaskProgressNotification};
use crate::transport::connection::TransportConnection;

/// Progress update information
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub execution_id: String,
    pub task_id: String,
    pub progress: f32,
    pub step: Option<String>,
    pub step_number: Option<u32>,
    pub total_steps: Option<u32>,
    pub message: Option<String>,
    pub data: Option<Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Progress notification manager that handles streaming updates for long-running tasks
pub struct ProgressNotificationManager {
    /// Active progress subscriptions
    subscriptions: Arc<RwLock<HashMap<String, Vec<ProgressSubscription>>>>,

    /// Notification sender channel
    notification_sender: mpsc::UnboundedSender<ProgressNotification>,
}

/// Progress subscription details
#[derive(Clone)]
struct ProgressSubscription {
    /// Subscription ID
    id: String,

    /// Client connection
    connection: Arc<dyn TransportConnection>,

    /// Optional filter criteria
    filter: Option<ProgressFilter>,

    /// Subscription timestamp
    created_at: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Debug for ProgressSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressSubscription")
            .field("id", &self.id)
            .field("filter", &self.filter)
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Filter criteria for progress notifications
#[derive(Debug, Clone)]
pub struct ProgressFilter {
    /// Minimum progress change to trigger notification (0.0-1.0)
    pub min_progress_delta: Option<f32>,

    /// Maximum notification frequency (milliseconds)
    pub max_frequency_ms: Option<u64>,

    /// Include only specific steps
    pub step_filter: Option<Vec<String>>,

    /// Include data in notifications
    pub include_data: bool,
}

/// Progress notification message
#[derive(Debug, Clone)]
struct ProgressNotification {
    execution_id: String,
    update: ProgressUpdate,
}

impl ProgressNotificationManager {
    /// Create a new progress notification manager
    pub fn new() -> Self {
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();

        let subscriptions = Arc::new(RwLock::new(HashMap::new()));

        // Start the notification processing task
        let subscriptions_clone = subscriptions.clone();
        tokio::spawn(async move {
            Self::process_notifications(subscriptions_clone, notification_receiver).await;
        });

        Self {
            subscriptions,
            notification_sender,
        }
    }

    /// Subscribe to progress updates for a specific execution
    pub async fn subscribe_to_execution(
        &self,
        execution_id: String,
        connection: Arc<dyn TransportConnection>,
        filter: Option<ProgressFilter>,
    ) -> String {
        let subscription_id = Uuid::new_v4().to_string();

        let subscription = ProgressSubscription {
            id: subscription_id.clone(),
            connection,
            filter,
            created_at: chrono::Utc::now(),
        };

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(execution_id.clone())
            .or_insert_with(Vec::new)
            .push(subscription);

        tracing::debug!(
            "Created progress subscription {} for execution {}",
            subscription_id,
            execution_id
        );

        subscription_id
    }

    /// Unsubscribe from progress updates
    pub async fn unsubscribe(&self, execution_id: &str, subscription_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(subs) = subscriptions.get_mut(execution_id) {
            subs.retain(|s| s.id != subscription_id);

            // Clean up empty subscription lists
            if subs.is_empty() {
                subscriptions.remove(execution_id);
            }
        }

        tracing::debug!(
            "Removed progress subscription {} for execution {}",
            subscription_id,
            execution_id
        );
    }

    /// Send a progress update for an execution
    pub async fn send_progress_update(&self, update: ProgressUpdate) -> Result<(), String> {
        let notification = ProgressNotification {
            execution_id: update.execution_id.clone(),
            update,
        };

        self.notification_sender
            .send(notification)
            .map_err(|e| format!("Failed to send progress notification: {}", e))?;

        Ok(())
    }

    /// Get number of active subscriptions for an execution
    pub async fn get_subscription_count(&self, execution_id: &str) -> usize {
        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .get(execution_id)
            .map(|subs| subs.len())
            .unwrap_or(0)
    }

    /// Clean up subscriptions for completed executions
    pub async fn cleanup_execution(&self, execution_id: &str) {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(subs) = subscriptions.remove(execution_id) {
            tracing::debug!(
                "Cleaned up {} progress subscriptions for execution {}",
                subs.len(),
                execution_id
            );
        }
    }

    /// Process notifications and send them to subscribers
    async fn process_notifications(
        subscriptions: Arc<RwLock<HashMap<String, Vec<ProgressSubscription>>>>,
        mut receiver: mpsc::UnboundedReceiver<ProgressNotification>,
    ) {
        while let Some(notification) = receiver.recv().await {
            let subscriptions_guard = subscriptions.read().await;

            if let Some(subs) = subscriptions_guard.get(&notification.execution_id) {
                for subscription in subs {
                    // Apply filter if present
                    if let Some(filter) = &subscription.filter {
                        if !Self::should_send_notification(&notification.update, filter) {
                            continue;
                        }
                    }

                    // Create MCP notification
                    let task_progress = TaskProgressNotification {
                        execution_id: notification.update.execution_id.clone(),
                        task_id: notification.update.task_id.clone(),
                        progress: notification.update.progress,
                        step: notification.update.step.clone(),
                        step_number: notification.update.step_number,
                        total_steps: notification.update.total_steps,
                        message: notification.update.message.clone(),
                        data: if subscription
                            .filter
                            .as_ref()
                            .map(|f| f.include_data)
                            .unwrap_or(true)
                        {
                            notification.update.data.clone()
                        } else {
                            None
                        },
                        timestamp: notification.update.timestamp.to_rfc3339(),
                    };

                    let mcp_notification = McpNotification {
                        jsonrpc: "2.0".to_string(),
                        method: McpMethod::NotificationsTaskProgress(task_progress),
                    };

                    // Send notification to client
                    if let Err(e) = subscription
                        .connection
                        .send_notification(mcp_notification)
                        .await
                    {
                        tracing::warn!(
                            "Failed to send progress notification to subscription {}: {}",
                            subscription.id,
                            e
                        );
                    }
                }
            }
        }
    }

    /// Check if a notification should be sent based on filter criteria
    fn should_send_notification(update: &ProgressUpdate, filter: &ProgressFilter) -> bool {
        // Check step filter
        if let Some(step_filter) = &filter.step_filter {
            if let Some(step) = &update.step {
                if !step_filter.contains(step) {
                    return false;
                }
            }
        }

        // TODO: Implement progress delta and frequency filtering
        // This would require tracking last notification state per subscription

        true
    }
}

impl Default for ProgressNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress tracking helper for tasks
pub struct TaskProgressTracker {
    execution_id: String,
    task_id: String,
    notification_manager: Arc<ProgressNotificationManager>,
    last_progress: f32,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl TaskProgressTracker {
    /// Create a new progress tracker for a task execution
    pub fn new(
        execution_id: String,
        task_id: String,
        notification_manager: Arc<ProgressNotificationManager>,
    ) -> Self {
        Self {
            execution_id,
            task_id,
            notification_manager,
            last_progress: 0.0,
            start_time: chrono::Utc::now(),
        }
    }

    /// Update progress
    pub async fn update_progress(
        &mut self,
        progress: f32,
        step: Option<String>,
        message: Option<String>,
    ) -> Result<(), String> {
        self.update_progress_detailed(progress, step, None, None, message, None)
            .await
    }

    /// Update progress with detailed information
    pub async fn update_progress_detailed(
        &mut self,
        progress: f32,
        step: Option<String>,
        step_number: Option<u32>,
        total_steps: Option<u32>,
        message: Option<String>,
        data: Option<Value>,
    ) -> Result<(), String> {
        // Clamp progress to valid range
        let progress = progress.max(0.0).min(1.0);

        let update = ProgressUpdate {
            execution_id: self.execution_id.clone(),
            task_id: self.task_id.clone(),
            progress,
            step,
            step_number,
            total_steps,
            message,
            data,
            timestamp: chrono::Utc::now(),
        };

        self.notification_manager
            .send_progress_update(update)
            .await?;
        self.last_progress = progress;

        Ok(())
    }

    /// Mark task as completed
    pub async fn complete(&mut self, message: Option<String>) -> Result<(), String> {
        self.update_progress(1.0, Some("completed".to_string()), message)
            .await
    }

    /// Mark task as failed
    pub async fn fail(&mut self, error_message: String) -> Result<(), String> {
        self.update_progress(
            self.last_progress,
            Some("failed".to_string()),
            Some(error_message),
        )
        .await
    }

    /// Get current progress
    pub fn get_progress(&self) -> f32 {
        self.last_progress
    }

    /// Get elapsed time since start
    pub fn get_elapsed_time(&self) -> chrono::Duration {
        chrono::Utc::now().signed_duration_since(self.start_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockTransportConnection {
        notifications: Arc<RwLock<Vec<McpNotification>>>,
    }

    impl MockTransportConnection {
        fn new() -> Self {
            Self {
                notifications: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn get_notifications(&self) -> Vec<McpNotification> {
            self.notifications.read().await.clone()
        }
    }

    #[async_trait]
    impl TransportConnection for MockTransportConnection {
        async fn send_notification(
            &self,
            notification: McpNotification,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.notifications.write().await.push(notification);
            Ok(())
        }

        async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_progress_notification_manager() {
        let manager = ProgressNotificationManager::new();
        let connection = Arc::new(MockTransportConnection::new());

        let execution_id = "test-execution-123".to_string();

        // Subscribe to progress updates
        let _subscription_id = manager
            .subscribe_to_execution(execution_id.clone(), connection.clone(), None)
            .await;

        // Send a progress update
        let update = ProgressUpdate {
            execution_id: execution_id.clone(),
            task_id: "test-task".to_string(),
            progress: 0.5,
            step: Some("processing".to_string()),
            step_number: Some(2),
            total_steps: Some(4),
            message: Some("Halfway done".to_string()),
            data: Some(serde_json::json!({"processed": 50})),
            timestamp: chrono::Utc::now(),
        };

        manager.send_progress_update(update).await.unwrap();

        // Give some time for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check that notification was sent
        let notifications = connection.get_notifications().await;
        assert_eq!(notifications.len(), 1);

        // Verify subscription count
        assert_eq!(manager.get_subscription_count(&execution_id).await, 1);

        // Cleanup
        manager.cleanup_execution(&execution_id).await;
        assert_eq!(manager.get_subscription_count(&execution_id).await, 0);
    }

    #[tokio::test]
    async fn test_task_progress_tracker() {
        let manager = Arc::new(ProgressNotificationManager::new());
        let mut tracker = TaskProgressTracker::new(
            "test-execution".to_string(),
            "test-task".to_string(),
            manager.clone(),
        );

        // Test progress updates
        assert!(tracker
            .update_progress(
                0.25,
                Some("step1".to_string()),
                Some("Starting".to_string())
            )
            .await
            .is_ok());
        assert_eq!(tracker.get_progress(), 0.25);

        assert!(tracker
            .update_progress(
                0.75,
                Some("step2".to_string()),
                Some("Almost done".to_string())
            )
            .await
            .is_ok());
        assert_eq!(tracker.get_progress(), 0.75);

        assert!(tracker
            .complete(Some("Finished successfully".to_string()))
            .await
            .is_ok());
        assert_eq!(tracker.get_progress(), 1.0);

        // Test elapsed time
        assert!(tracker.get_elapsed_time().num_milliseconds() >= 0);
    }

    #[tokio::test]
    async fn test_progress_filter() {
        let manager = ProgressNotificationManager::new();
        let connection = Arc::new(MockTransportConnection::new());

        let execution_id = "test-execution-filtered".to_string();

        // Subscribe with step filter
        let filter = ProgressFilter {
            min_progress_delta: None,
            max_frequency_ms: None,
            step_filter: Some(vec!["important".to_string()]),
            include_data: false,
        };

        let _subscription_id = manager
            .subscribe_to_execution(execution_id.clone(), connection.clone(), Some(filter))
            .await;

        // Send updates - one matching filter, one not
        let update1 = ProgressUpdate {
            execution_id: execution_id.clone(),
            task_id: "test-task".to_string(),
            progress: 0.3,
            step: Some("unimportant".to_string()),
            step_number: None,
            total_steps: None,
            message: Some("Skipped step".to_string()),
            data: None,
            timestamp: chrono::Utc::now(),
        };

        let update2 = ProgressUpdate {
            execution_id: execution_id.clone(),
            task_id: "test-task".to_string(),
            progress: 0.6,
            step: Some("important".to_string()),
            step_number: None,
            total_steps: None,
            message: Some("Important step".to_string()),
            data: Some(serde_json::json!({"key": "value"})),
            timestamp: chrono::Utc::now(),
        };

        manager.send_progress_update(update1).await.unwrap();
        manager.send_progress_update(update2).await.unwrap();

        // Give some time for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check that only the important step notification was sent
        let notifications = connection.get_notifications().await;
        assert_eq!(notifications.len(), 1);

        // Verify the notification doesn't include data (due to filter)
        if let McpMethod::NotificationsTaskProgress(task_progress) = &notifications[0].method {
            assert_eq!(task_progress.step, Some("important".to_string()));
            assert!(task_progress.data.is_none());
        } else {
            panic!("Expected TaskProgress notification");
        }
    }
}
