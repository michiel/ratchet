use crate::{JsExecutionError, JsTask, ExecutionContext};
use serde_json::Value as JsonValue;
use thiserror::Error;
use tracing::debug;

/// Errors that can occur when running a JS task
#[derive(Error, Debug)]
pub enum JsTaskError {
    #[error("Task execution failed: {0}")]
    ExecutionError(String),

    #[error("JavaScript execution error: {0}")]
    JsExecutionError(#[from] JsExecutionError),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Task configuration error: {0}")]
    ConfigError(String),
}

/// JavaScript task runner
pub struct JsTaskRunner {
    #[allow(dead_code)]
    http_enabled: bool,
}

impl Default for JsTaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl JsTaskRunner {
    /// Create a new JavaScript task runner
    pub fn new() -> Self {
        Self {
            http_enabled: cfg!(feature = "http"),
        }
    }

    /// Execute a JavaScript task with input data
    pub async fn execute_task(
        &self,
        task: &JsTask,
        input_data: JsonValue,
        execution_context: Option<ExecutionContext>,
    ) -> Result<JsonValue, JsTaskError> {
        debug!("Executing JS task: {}", task.name);

        // Create HTTP manager if HTTP feature is enabled
        #[cfg(feature = "http")]
        let http_manager = ratchet_http::HttpManager::new();
        
        #[cfg(not(feature = "http"))]
        let http_manager = ();

        // Execute the task
        let result = crate::execution::execute_js_with_content(
            &task.content,
            input_data,
            task.input_schema.as_ref(),
            task.output_schema.as_ref(),
            &http_manager,
            execution_context.as_ref(),
        )
        .await
        .map_err(JsTaskError::from)?;

        Ok(result)
    }

    /// Execute JavaScript code directly with input data
    pub async fn execute_code(
        &self,
        code: &str,
        input_data: JsonValue,
        input_schema: Option<&JsonValue>,
        output_schema: Option<&JsonValue>,
        execution_context: Option<ExecutionContext>,
    ) -> Result<JsonValue, JsTaskError> {
        debug!("Executing JS code directly");

        // Create HTTP manager if HTTP feature is enabled
        #[cfg(feature = "http")]
        let http_manager = ratchet_http::HttpManager::new();
        
        #[cfg(not(feature = "http"))]
        let http_manager = ();

        // Execute the code
        let result = crate::execution::execute_js_with_content(
            code,
            input_data,
            input_schema,
            output_schema,
            &http_manager,
            execution_context.as_ref(),
        )
        .await
        .map_err(JsTaskError::from)?;

        Ok(result)
    }
}

/// For unit tests and simple use cases
pub async fn add_two_numbers(num1: i32, num2: i32) -> Result<i32, JsExecutionError> {
    let js_code = r#"
        function main(input) {
            return input.num1 + input.num2;
        }
    "#;

    let input_data = serde_json::json!({
        "num1": num1,
        "num2": num2
    });

    let runner = JsTaskRunner::new();
    let result = runner
        .execute_code(js_code, input_data, None, None, None)
        .await
        .map_err(|e| JsExecutionError::ExecutionError(e.to_string()))?;

    result
        .as_i64()
        .map(|n| n as i32)
        .ok_or_else(|| JsExecutionError::OutputError("Expected integer result".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_two_numbers() {
        let result = add_two_numbers(5, 7).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12);
    }

    #[tokio::test]
    async fn test_js_task_runner() {
        let task = JsTask {
            name: "test_task".to_string(),
            content: r#"
                function main(input) {
                    return { result: input.a + input.b };
                }
            "#.to_string(),
            input_schema: None,
            output_schema: None,
        };

        let input_data = serde_json::json!({
            "a": 10,
            "b": 20
        });

        let runner = JsTaskRunner::new();
        let result = runner.execute_task(&task, input_data, None).await;
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["result"], 30);
    }
}