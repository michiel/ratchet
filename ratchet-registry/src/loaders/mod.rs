pub mod filesystem;
pub mod http;
pub mod validation;

use async_trait::async_trait;

use crate::config::TaskSource;
use crate::error::Result;
use crate::types::{DiscoveredTask, TaskDefinition, TaskReference};

#[async_trait]
pub trait TaskLoader: Send + Sync {
    async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>>;
    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition>;
    async fn supports_source(&self, source: &TaskSource) -> bool;
}