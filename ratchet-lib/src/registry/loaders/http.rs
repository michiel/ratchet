use async_trait::async_trait;
use tracing::warn;

use crate::errors::{Result, RatchetError};
use crate::registry::{TaskSource, loaders::TaskLoader};
use crate::task::Task;

pub struct HttpTaskLoader;

impl Default for HttpTaskLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTaskLoader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskLoader for HttpTaskLoader {
    async fn load_tasks(&self, source: &TaskSource) -> Result<Vec<Task>> {
        match source {
            TaskSource::Http { url } => {
                warn!("HTTP task loading not yet implemented for URL: {}", url);
                Err(RatchetError::NotImplemented("HTTP task loading is not yet implemented".to_string()))
            },
            _ => Err(RatchetError::NotImplemented("HttpTaskLoader only supports HTTP sources".to_string())),
        }
    }
}