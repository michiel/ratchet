use async_trait::async_trait;
use std::sync::Arc;
use tracing::warn;

use crate::config::TaskSource;
use crate::error::{RegistryError, Result};
use crate::loaders::TaskLoader;
use crate::types::{DiscoveredTask, TaskDefinition, TaskReference};

pub struct HttpLoader {
    client: Arc<ratchet_http::HttpManager>,
}

impl Default for HttpLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpLoader {
    pub fn new() -> Self {
        Self {
            client: Arc::new(ratchet_http::HttpManager::new()),
        }
    }

    pub fn with_client(client: Arc<ratchet_http::HttpManager>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TaskLoader for HttpLoader {
    async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        match source {
            TaskSource::Http { url, .. } => {
                warn!("HTTP task discovery not yet implemented for URL: {}", url);
                // TODO: Implement HTTP discovery
                // This would typically:
                // 1. GET /tasks endpoint to list available tasks
                // 2. Parse response to extract task metadata
                // 3. Convert to DiscoveredTask structs
                Err(RegistryError::NotImplemented(
                    "HTTP task discovery is not yet implemented".to_string(),
                ))
            }
            _ => Err(RegistryError::Configuration(
                "HttpLoader only supports HTTP sources".to_string(),
            )),
        }
    }

    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition> {
        if !task_ref.source.starts_with("http://") && !task_ref.source.starts_with("https://") {
            return Err(RegistryError::Configuration(
                "HttpLoader can only load HTTP/HTTPS sources".to_string(),
            ));
        }

        warn!(
            "HTTP task loading not yet implemented for: {}",
            task_ref.source
        );
        
        // TODO: Implement HTTP loading
        // This would typically:
        // 1. GET the task endpoint (e.g., /tasks/{name}/{version})
        // 2. Download and parse the task definition
        // 3. Handle different content types (JSON, ZIP, etc.)
        // 4. Apply authentication if configured
        
        Err(RegistryError::NotImplemented(
            "HTTP task loading is not yet implemented".to_string(),
        ))
    }

    async fn supports_source(&self, source: &TaskSource) -> bool {
        matches!(source, TaskSource::Http { .. })
    }
}