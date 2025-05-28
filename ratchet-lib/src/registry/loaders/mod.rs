pub mod filesystem;
pub mod http;

use async_trait::async_trait;
use crate::errors::Result;
use crate::registry::TaskSource;
use crate::task::Task;

#[async_trait]
pub trait TaskLoader {
    async fn load_tasks(&self, source: &TaskSource) -> Result<Vec<Task>>;
}