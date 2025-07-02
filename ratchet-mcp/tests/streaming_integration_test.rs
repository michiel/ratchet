//! Integration tests for streaming progress updates in MCP server

use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use ratchet_mcp::protocol::messages::McpNotification;
use ratchet_mcp::server::{
    progress::{ProgressFilter, ProgressNotificationManager, ProgressUpdate},
    tools::{McpExecutionStatus, McpTaskExecutor, McpTaskInfo, RatchetToolRegistry},
};
use ratchet_mcp::transport::connection::TransportConnection;
use ratchet_mcp::McpResult;

/// Mock task executor for testing
struct MockStreamingTaskExecutor {
    progress_manager: Option<Arc<ProgressNotificationManager>>,
}

impl MockStreamingTaskExecutor {
    fn new() -> Self {
        Self { progress_manager: None }
    }

    fn with_progress_manager(mut self, manager: Arc<ProgressNotificationManager>) -> Self {
        self.progress_manager = Some(manager);
        self
    }
}

#[async_trait::async_trait]
impl McpTaskExecutor for MockStreamingTaskExecutor {
    async fn execute_task(&self, _task_path: &str, input: serde_json::Value) -> Result<serde_json::Value, String> {
        // Simulate a simple task
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(json!({"result": "simple_execution", "input": input}))
    }

    async fn execute_task_with_progress(
        &self,
        task_path: &str,
        input: serde_json::Value,
        progress_manager: Option<Arc<ProgressNotificationManager>>,
        connection: Option<Arc<dyn TransportConnection>>,
        filter: Option<ProgressFilter>,
    ) -> Result<(String, serde_json::Value), String> {
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Subscribe the connection to receive progress updates
        if let (Some(manager), Some(conn)) = (progress_manager.as_ref(), connection.as_ref()) {
            manager
                .subscribe_to_execution(execution_id.clone(), conn.clone(), filter)
                .await;
        }

        // Simulate a long-running task with progress updates
        if let Some(manager) = progress_manager {
            // Send initial progress
            let initial_update = ProgressUpdate {
                execution_id: execution_id.clone(),
                task_id: task_path.to_string(),
                progress: 0.0,
                step: Some("starting".to_string()),
                step_number: Some(1),
                total_steps: Some(4),
                message: Some("Task initialization".to_string()),
                data: None,
                timestamp: chrono::Utc::now(),
            };
            manager.send_progress_update(initial_update).await.unwrap();

            // Simulate work with progress updates
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_millis(50)).await;

                let progress_update = ProgressUpdate {
                    execution_id: execution_id.clone(),
                    task_id: task_path.to_string(),
                    progress: i as f32 / 4.0,
                    step: Some(format!("step_{}", i)),
                    step_number: Some(i + 1),
                    total_steps: Some(4),
                    message: Some(format!("Processing step {}", i)),
                    data: Some(json!({"step": i, "processed": i * 25})),
                    timestamp: chrono::Utc::now(),
                };
                manager.send_progress_update(progress_update).await.unwrap();
            }

            // Send completion
            let completion_update = ProgressUpdate {
                execution_id: execution_id.clone(),
                task_id: task_path.to_string(),
                progress: 1.0,
                step: Some("completed".to_string()),
                step_number: Some(4),
                total_steps: Some(4),
                message: Some("Task completed successfully".to_string()),
                data: Some(json!({"final_result": "success"})),
                timestamp: chrono::Utc::now(),
            };
            manager.send_progress_update(completion_update).await.unwrap();
        }

        let result = json!({
            "result": "streaming_execution",
            "input": input,
            "execution_id": execution_id
        });

        Ok((execution_id, result))
    }

    async fn list_tasks(&self, _filter: Option<&str>) -> Result<Vec<McpTaskInfo>, String> {
        Ok(vec![McpTaskInfo {
            id: "test-task-1".to_string(),
            name: "Test Task 1".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test task for streaming".to_string()),
            tags: vec!["test".to_string()],
            enabled: true,
            input_schema: Some(json!({"type": "object"})),
            output_schema: Some(json!({"type": "object"})),
        }])
    }

    async fn get_execution_logs(&self, _execution_id: &str, _level: &str, _limit: usize) -> Result<String, String> {
        Ok("Mock logs".to_string())
    }

    async fn get_execution_status(&self, execution_id: &str) -> Result<McpExecutionStatus, String> {
        Ok(McpExecutionStatus {
            execution_id: execution_id.to_string(),
            status: "completed".to_string(),
            task_id: 1,
            input: Some(json!({})),
            output: Some(json!({"result": "mock"})),
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now().to_rfc3339(),
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            duration_ms: Some(100),
            progress: Some(json!({"progress": 1.0})),
        })
    }
}

/// Mock transport connection for testing
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

    #[allow(dead_code)]
    async fn clear_notifications(&self) {
        self.notifications.write().await.clear();
    }
}

#[async_trait::async_trait]
impl TransportConnection for MockTransportConnection {
    async fn send_notification(
        &self,
        notification: McpNotification,
    ) -> McpResult<()> {
        self.notifications.write().await.push(notification);
        Ok(())
    }

    async fn close(&self) -> McpResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_progress_notification_system() {
    let progress_manager = Arc::new(ProgressNotificationManager::new());
    let connection = Arc::new(MockTransportConnection::new());

    let execution_id = "test-execution-123";

    // Subscribe to progress updates
    let subscription_id = progress_manager
        .subscribe_to_execution(execution_id.to_string(), connection.clone(), None)
        .await;

    // Send some progress updates
    let update1 = ProgressUpdate {
        execution_id: execution_id.to_string(),
        task_id: "test-task".to_string(),
        progress: 0.25,
        step: Some("initialization".to_string()),
        step_number: Some(1),
        total_steps: Some(4),
        message: Some("Starting task".to_string()),
        data: Some(json!({"started": true})),
        timestamp: chrono::Utc::now(),
    };

    let update2 = ProgressUpdate {
        execution_id: execution_id.to_string(),
        task_id: "test-task".to_string(),
        progress: 0.75,
        step: Some("processing".to_string()),
        step_number: Some(3),
        total_steps: Some(4),
        message: Some("Processing data".to_string()),
        data: Some(json!({"processed": 75})),
        timestamp: chrono::Utc::now(),
    };

    progress_manager.send_progress_update(update1).await.unwrap();
    progress_manager.send_progress_update(update2).await.unwrap();

    // Give some time for async processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check notifications were sent
    let notifications = connection.get_notifications().await;
    assert_eq!(notifications.len(), 2);

    // Verify notification content
    for notification in &notifications {
        if let ratchet_mcp::protocol::messages::McpMethod::NotificationsTaskProgress(task_progress) =
            &notification.method
        {
            assert_eq!(task_progress.execution_id, execution_id);
            assert_eq!(task_progress.task_id, "test-task");
            assert!(task_progress.progress >= 0.0 && task_progress.progress <= 1.0);
            assert!(task_progress.step.is_some());
            assert!(task_progress.step_number.is_some());
            assert_eq!(task_progress.total_steps, Some(4));
        } else {
            panic!("Expected TaskProgress notification");
        }
    }

    // Clean up
    progress_manager.unsubscribe(execution_id, &subscription_id).await;
    assert_eq!(progress_manager.get_subscription_count(execution_id).await, 0);
}

#[tokio::test]
async fn test_streaming_task_execution() {
    let progress_manager = Arc::new(ProgressNotificationManager::new());
    let connection = Arc::new(MockTransportConnection::new());
    let executor = Arc::new(MockStreamingTaskExecutor::new().with_progress_manager(progress_manager.clone()));

    // The actual implementation would generate an execution ID and subscribe before execution
    // For this test, we'll execute the task which will send progress notifications
    // The mock executor subscribes the connection internally during execution

    // Execute task with progress streaming
    let (execution_id, result) = executor
        .execute_task_with_progress(
            "test-streaming-task",
            json!({"input": "test"}),
            Some(progress_manager.clone()),
            Some(connection.clone()),
            None,
        )
        .await
        .unwrap();

    // Verify execution completed
    assert!(!execution_id.is_empty());
    assert_eq!(result["result"], "streaming_execution");

    // Give time for all progress notifications to be processed
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check that multiple progress notifications were sent
    let notifications = connection.get_notifications().await;
    assert!(notifications.len() >= 4); // Initial + 3 progress + completion

    // Verify progress sequence
    let mut last_progress = 0.0;
    for notification in &notifications {
        if let ratchet_mcp::protocol::messages::McpMethod::NotificationsTaskProgress(task_progress) =
            &notification.method
        {
            assert!(task_progress.progress >= last_progress);
            last_progress = task_progress.progress;
        }
    }

    // Should end with 100% progress
    assert_eq!(last_progress, 1.0);
}

#[tokio::test]
async fn test_progress_filtering() {
    let progress_manager = Arc::new(ProgressNotificationManager::new());
    let connection = Arc::new(MockTransportConnection::new());

    let execution_id = "filter-test-execution";

    // Subscribe with a step filter
    let filter = ProgressFilter {
        min_progress_delta: None,
        max_frequency_ms: None,
        step_filter: Some(vec!["important".to_string()]),
        include_data: false,
    };

    let _subscription_id = progress_manager
        .subscribe_to_execution(execution_id.to_string(), connection.clone(), Some(filter))
        .await;

    // Send updates with different steps
    let updates = vec![
        ("initialization", 0.1),
        ("important", 0.5),
        ("cleanup", 0.9),
        ("important", 1.0),
    ];

    for (step, progress) in updates {
        let update = ProgressUpdate {
            execution_id: execution_id.to_string(),
            task_id: "filtered-task".to_string(),
            progress,
            step: Some(step.to_string()),
            step_number: None,
            total_steps: None,
            message: Some(format!("Step: {}", step)),
            data: Some(json!({"step_data": step})),
            timestamp: chrono::Utc::now(),
        };

        progress_manager.send_progress_update(update).await.unwrap();
    }

    // Give time for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that only "important" steps were notified
    let notifications = connection.get_notifications().await;
    assert_eq!(notifications.len(), 2); // Only the two "important" steps

    for notification in &notifications {
        if let ratchet_mcp::protocol::messages::McpMethod::NotificationsTaskProgress(task_progress) =
            &notification.method
        {
            assert_eq!(task_progress.step, Some("important".to_string()));
            assert!(task_progress.data.is_none()); // Data should be filtered out
        }
    }
}

#[tokio::test]
async fn test_tool_registry_with_streaming() {
    let mut registry = RatchetToolRegistry::new();
    let executor = Arc::new(MockStreamingTaskExecutor::new()) as Arc<dyn McpTaskExecutor>;
    registry.set_executor(executor);

    // Test that progress manager is available
    let progress_manager = registry.get_progress_manager();
    assert_eq!(progress_manager.get_subscription_count("test").await, 0);

    // Test subscription
    let connection = Arc::new(MockTransportConnection::new());
    let _subscription_id = progress_manager
        .subscribe_to_execution("test-execution".to_string(), connection.clone(), None)
        .await;

    assert_eq!(progress_manager.get_subscription_count("test-execution").await, 1);
}

#[tokio::test]
async fn test_concurrent_progress_subscriptions() {
    let progress_manager = Arc::new(ProgressNotificationManager::new());
    let execution_id = "concurrent-test";

    // Create multiple connections
    let connections: Vec<Arc<MockTransportConnection>> =
        (0..5).map(|_| Arc::new(MockTransportConnection::new())).collect();

    // Subscribe all connections
    let mut subscription_ids = Vec::new();
    for connection in &connections {
        let sub_id = progress_manager
            .subscribe_to_execution(execution_id.to_string(), connection.clone(), None)
            .await;
        subscription_ids.push(sub_id);
    }

    assert_eq!(progress_manager.get_subscription_count(execution_id).await, 5);

    // Send a progress update
    let update = ProgressUpdate {
        execution_id: execution_id.to_string(),
        task_id: "concurrent-task".to_string(),
        progress: 0.5,
        step: Some("testing".to_string()),
        step_number: Some(1),
        total_steps: Some(2),
        message: Some("Concurrent test".to_string()),
        data: Some(json!({"concurrent": true})),
        timestamp: chrono::Utc::now(),
    };

    progress_manager.send_progress_update(update).await.unwrap();

    // Give time for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // All connections should have received the notification
    for connection in &connections {
        let notifications = connection.get_notifications().await;
        assert_eq!(notifications.len(), 1);
    }

    // Clean up subscriptions
    for (i, sub_id) in subscription_ids.iter().enumerate() {
        progress_manager.unsubscribe(execution_id, sub_id).await;
        assert_eq!(progress_manager.get_subscription_count(execution_id).await, 5 - i - 1);
    }
}
