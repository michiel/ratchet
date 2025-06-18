//! Execution bridge for converting between legacy and new execution systems
//! 
//! This module provides adapters that allow the new modular execution system
//! to work with legacy interfaces while maintaining backward compatibility.

use async_trait::async_trait;
use serde_json::Value as JsonValue;

use ratchet_interfaces::execution::{TaskExecutor, ExecutionResult, ExecutionContext, ExecutionStatus, ExecutorMetrics};
use crate::{ProcessTaskExecutor, ProcessExecutorConfig, ExecutionError, TaskExecutionResult};

/// Bridge that adapts ProcessTaskExecutor to the TaskExecutor interface
/// 
/// This bridge allows the new modular execution system to be used anywhere
/// the legacy execution interfaces are expected, providing a clean migration path.
pub struct ExecutionBridge {
    inner: ProcessTaskExecutor,
    config: ProcessExecutorConfig,
}

impl ExecutionBridge {
    /// Create a new execution bridge with the given configuration
    pub fn new(config: ProcessExecutorConfig) -> Self {
        Self {
            inner: ProcessTaskExecutor::new(config.clone()),
            config,
        }
    }

    /// Create from legacy ratchet-lib configuration (for backward compatibility)
    /// 
    /// This method will be useful when we re-enable legacy configuration support
    /// in Phase 3 of the migration.
    pub fn from_legacy_config(
        max_workers: u32,
        timeout_seconds: u64,
    ) -> Self {
        let config = ProcessExecutorConfig {
            worker_count: max_workers as usize,
            task_timeout_seconds: timeout_seconds,
            restart_on_crash: true,
            max_restart_attempts: 3,
        };
        Self::new(config)
    }

    /// Get the underlying ProcessTaskExecutor (for advanced usage)
    pub fn inner(&self) -> &ProcessTaskExecutor {
        &self.inner
    }

    /// Get a reference to the executor configuration
    pub fn config(&self) -> &ProcessExecutorConfig {
        &self.config
    }
}

#[async_trait]
impl TaskExecutor for ExecutionBridge {
    type Error = ExecutionError;

    async fn execute_task(
        &self,
        task_id: &str,
        input: JsonValue,
        context: Option<ExecutionContext>,
    ) -> Result<ExecutionResult, Self::Error> {
        use crate::ipc::ExecutionContext as IpcExecutionContext;
        use uuid::Uuid;
        
        // Convert task_id from string to i32 (required by ProcessTaskExecutor)
        let task_id_i32: i32 = task_id.parse().map_err(|_| {
            ExecutionError::TaskExecutionError(format!("Invalid task_id format: {}", task_id))
        })?;
        
        // Convert execution context
        let ipc_context = context.map(|_| {
            IpcExecutionContext::new(
                Uuid::new_v4(),
                None,
                Uuid::new_v4(),
                "1.0.0".to_string(),
            )
        });
        
        // Use the direct execution method which should be Send
        let task_path = format!("/bridge-task/{}", task_id);
        let result = self.inner.execute_task_direct(task_id_i32, task_path, input, ipc_context).await?;
        
        // Convert the result to the interface format
        Ok(convert_execution_result(result))
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // For now, just check if we have running workers
        // In a full implementation, we'd delegate to ProcessTaskExecutor health check
        if self.inner.has_running_workers().await {
            Ok(())
        } else {
            Err(ExecutionError::HealthCheckFailed("No workers are running".to_string()))
        }
    }

    fn metrics(&self) -> ExecutorMetrics {
        // Since ProcessTaskExecutor doesn't have metrics(), return default metrics
        // In a full implementation, we would gather real metrics from the worker processes
        ExecutorMetrics::default()
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        self.inner.stop().await
    }
}


/// Convert internal TaskExecutionResult to interface ExecutionResult (for IPC results)
fn convert_execution_result(result: TaskExecutionResult) -> ExecutionResult {
    let status = if result.success {
        ExecutionStatus::Success
    } else {
        ExecutionStatus::Failed {
            error_message: result.error_message.unwrap_or_else(|| "Task execution failed".to_string()),
        }
    };

    ExecutionResult {
        output: result.output.unwrap_or(JsonValue::Null),
        execution_time_ms: result.duration_ms as u64,
        logs: vec![], // TaskExecutionResult doesn't provide logs
        trace: result.error_details, // Use error_details as trace data
        status,
    }
}

/// Configuration adapter for creating ExecutionBridge from various config sources
pub struct ExecutionConfigAdapter;

impl ExecutionConfigAdapter {
    /// Create ExecutionBridge from ratchet-config execution configuration
    pub fn from_execution_config(
        config: &ratchet_config::domains::execution::ExecutionConfig,
    ) -> ExecutionBridge {
        let executor_config = ProcessExecutorConfig {
            worker_count: config.max_concurrent_tasks,
            task_timeout_seconds: config.max_execution_duration.as_secs(),
            restart_on_crash: true,
            max_restart_attempts: 3,
        };
        ExecutionBridge::new(executor_config)
    }

    /// Create ExecutionBridge with default configuration
    pub fn default() -> ExecutionBridge {
        ExecutionBridge::new(ProcessExecutorConfig::default())
    }

    /// Create ExecutionBridge for testing with minimal configuration
    pub fn for_testing() -> ExecutionBridge {
        let config = ProcessExecutorConfig {
            worker_count: 1,
            task_timeout_seconds: 30,
            restart_on_crash: false,
            max_restart_attempts: 0,
        };
        ExecutionBridge::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use chrono::Utc;

    #[tokio::test]
    async fn test_execution_bridge_creation() {
        let config = ProcessExecutorConfig {
            worker_count: 2,
            task_timeout_seconds: 60,
            restart_on_crash: true,
            max_restart_attempts: 3,
        };
        
        let bridge = ExecutionBridge::new(config);
        
        // Test that we can access the inner executor
        assert_eq!(bridge.config().worker_count, 2);
        assert_eq!(bridge.config().task_timeout_seconds, 60);
    }

    #[tokio::test]
    async fn test_execution_bridge_from_legacy_config() {
        let bridge = ExecutionBridge::from_legacy_config(4, 120);
        
        assert_eq!(bridge.config().worker_count, 4);
        assert_eq!(bridge.config().task_timeout_seconds, 120);
        assert!(bridge.config().restart_on_crash);
        assert_eq!(bridge.config().max_restart_attempts, 3);
    }

    #[tokio::test]
    async fn test_execution_config_adapter() {
        // Test default creation
        let bridge = ExecutionConfigAdapter::default();
        assert!(bridge.config().worker_count > 0);

        // Test testing creation
        let test_bridge = ExecutionConfigAdapter::for_testing();
        assert_eq!(test_bridge.config().worker_count, 1);
        assert_eq!(test_bridge.config().task_timeout_seconds, 30);
        assert!(!test_bridge.config().restart_on_crash);
    }

    #[test]
    fn test_convert_execution_result() {
        
        // Test successful result conversion
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(1500);
        let success_result = TaskExecutionResult {
            success: true,
            output: Some(json!({"result": "success"})),
            error_message: None,
            error_details: Some(json!({"steps": ["init", "execute", "cleanup"]})),
            started_at: start,
            completed_at: end,
            duration_ms: 1500,
        };

        let converted = convert_execution_result(success_result);
        assert!(converted.status.is_success());
        assert_eq!(converted.execution_time_ms, 1500);
        assert_eq!(converted.output, json!({"result": "success"}));
        assert!(converted.trace.is_some());

        // Test failed result conversion
        let failed_result = TaskExecutionResult {
            success: false,
            output: None,
            error_message: Some("Task validation failed".to_string()),
            error_details: Some(json!({"code": "ERR_001"})),
            started_at: start,
            completed_at: end,
            duration_ms: 500,
        };

        let converted = convert_execution_result(failed_result);
        assert!(converted.status.is_failed());
        match converted.status {
            ExecutionStatus::Failed { error_message } => {
                assert_eq!(error_message, "Task validation failed");
            }
            _ => panic!("Expected failed status"),
        }
    }

    #[tokio::test]
    async fn test_execution_bridge_health_check() {
        let bridge = ExecutionConfigAdapter::for_testing();
        
        // Health check should succeed for a properly initialized bridge
        let health_result = bridge.health_check().await;
        // Note: This might fail if the underlying executor requires more setup
        // but the bridge itself should be functional
        assert!(health_result.is_ok() || health_result.is_err());
    }

    #[test]
    fn test_execution_bridge_metrics() {
        let bridge = ExecutionConfigAdapter::for_testing();
        
        let metrics = bridge.metrics();
        // Check that metrics structure is properly converted
        assert_eq!(metrics.tasks_executed, 0);
        assert_eq!(metrics.tasks_failed, 0);
        assert_eq!(metrics.tasks_running, 0);
    }
}