//! JavaScript task execution compatibility layer
//!
//! This module provides a compatibility layer for CLI tools to execute JavaScript tasks
//! using modern ratchet-js while maintaining compatibility with legacy ratchet_lib API.

use anyhow::Result;
use serde_json::Value as JsonValue;
use tracing::{debug, info};

#[cfg(feature = "javascript")]
use ratchet_js::load_and_execute_task;

/// Execution mode for task execution
#[derive(Debug, Clone)]
pub enum ExecutionMode {
    /// Use legacy ratchet_lib execution
    Legacy,
    /// Use modern ratchet-js execution
    Modern,
}

/// Input for task execution
#[derive(Debug, Clone)]
pub struct TaskInput {
    pub data: JsonValue,
    pub execution_mode: ExecutionMode,
}

impl TaskInput {
    /// Create new task input with modern execution mode
    pub fn new(data: JsonValue) -> Self {
        Self {
            data,
            execution_mode: ExecutionMode::Modern,
        }
    }

    /// Create task input with legacy execution mode
    pub fn legacy(data: JsonValue) -> Self {
        Self {
            data,
            execution_mode: ExecutionMode::Legacy,
        }
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }
}

/// Execute a JavaScript task with compatibility layer for ratchet_lib API
///
/// This function provides a bridge between the CLI tools and the underlying
/// JavaScript execution engines, supporting both legacy and modern execution modes.
#[cfg(feature = "javascript")]
pub async fn execute_task_with_lib_compatibility(task_path: &str, input: TaskInput) -> Result<JsonValue> {
    match input.execution_mode {
        ExecutionMode::Modern => {
            info!("Executing task using modern ratchet-js engine: {}", task_path);
            execute_task_modern(task_path, &input.data).await
        }
        ExecutionMode::Legacy => {
            info!("Executing task using legacy ratchet_lib engine: {}", task_path);
            execute_task_legacy(task_path, &input.data).await
        }
    }
}

#[cfg(not(feature = "javascript"))]
pub async fn execute_task_with_lib_compatibility(_task_path: &str, _input: TaskInput) -> Result<JsonValue> {
    Err(anyhow::anyhow!(
        "JavaScript execution not available. Build with --features=javascript"
    ))
}

/// Execute task using modern ratchet-js engine
#[cfg(feature = "javascript")]
async fn execute_task_modern(task_path: &str, input: &JsonValue) -> Result<JsonValue> {
    debug!("Loading and executing task with ratchet-js: {}", task_path);

    let result = load_and_execute_task(task_path, input.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Modern task execution failed: {}", e))?;

    Ok(result)
}

/// Execute task using legacy ratchet_lib engine
#[cfg(feature = "javascript")]
async fn execute_task_legacy(task_path: &str, _input: &JsonValue) -> Result<JsonValue> {
    debug!("Legacy ratchet_lib execution no longer available: {}", task_path);

    // Legacy execution is no longer available - ratchet_lib has been removed
    Err(anyhow::anyhow!(
        "Legacy ratchet_lib execution is no longer available. Use modern execution instead."
    ))
}

/// Default execution function using modern engine
#[cfg(feature = "javascript")]
pub async fn execute_task(task_path: &str, input: JsonValue) -> Result<JsonValue> {
    let task_input = TaskInput::new(input);
    execute_task_with_lib_compatibility(task_path, task_input).await
}

#[cfg(not(feature = "javascript"))]
pub async fn execute_task(_task_path: &str, _input: JsonValue) -> Result<JsonValue> {
    Err(anyhow::anyhow!(
        "JavaScript execution not available. Build with --features=javascript"
    ))
}
