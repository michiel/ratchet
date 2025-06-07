//! Types for JavaScript execution

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// JavaScript task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsTask {
    /// Task name
    pub name: String,
    
    /// JavaScript code content
    pub content: String,
    
    /// Input JSON schema (optional)
    pub input_schema: Option<JsonValue>,
    
    /// Output JSON schema (optional)
    pub output_schema: Option<JsonValue>,
}

/// Execution context for JavaScript tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub execution_id: String,
    
    /// Task ID
    pub task_id: String,
    
    /// Task version
    pub task_version: String,
    
    /// Optional job ID
    pub job_id: Option<String>,
}

impl ExecutionContext {
    pub fn new(execution_id: String, task_id: String, task_version: String) -> Self {
        Self {
            execution_id,
            task_id,
            task_version,
            job_id: None,
        }
    }
    
    pub fn with_job_id(mut self, job_id: String) -> Self {
        self.job_id = Some(job_id);
        self
    }
}