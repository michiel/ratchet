pub mod filesystem;
pub mod http;

use crate::errors::Result;
use crate::registry::TaskSource;
use crate::task::Task;
use async_trait::async_trait;

#[async_trait]
pub trait TaskLoader {
    async fn load_tasks(&self, source: &TaskSource) -> Result<Vec<Task>>;
}
