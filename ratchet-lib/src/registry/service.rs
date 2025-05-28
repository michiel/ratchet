use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, error};

use crate::errors::Result;
use crate::registry::{TaskRegistry, TaskSource, loaders::{TaskLoader, filesystem::FilesystemTaskLoader, http::HttpTaskLoader}};

#[async_trait]
pub trait RegistryService: Send + Sync {
    async fn load_all_sources(&self) -> Result<()>;
    async fn registry(&self) -> Arc<TaskRegistry>;
}

pub struct DefaultRegistryService {
    registry: Arc<TaskRegistry>,
    filesystem_loader: FilesystemTaskLoader,
    http_loader: HttpTaskLoader,
}

impl DefaultRegistryService {
    pub fn new(sources: Vec<TaskSource>) -> Self {
        Self {
            registry: Arc::new(TaskRegistry::with_sources(sources)),
            filesystem_loader: FilesystemTaskLoader::new(),
            http_loader: HttpTaskLoader::new(),
        }
    }

    async fn load_source(&self, source: &TaskSource) -> Result<()> {
        info!("Loading tasks from source: {:?}", source);
        
        let tasks = match source {
            TaskSource::Filesystem { .. } => self.filesystem_loader.load_tasks(source).await?,
            TaskSource::Http { .. } => self.http_loader.load_tasks(source).await?,
        };

        for task in tasks {
            if let Err(e) = self.registry.add_task(task).await {
                error!("Failed to add task to registry: {}", e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl RegistryService for DefaultRegistryService {
    async fn load_all_sources(&self) -> Result<()> {
        let sources = self.registry.sources().to_vec();
        
        for source in sources {
            if let Err(e) = self.load_source(&source).await {
                error!("Failed to load source {:?}: {}", source, e);
                // Continue loading other sources
            }
        }

        let task_count = self.registry.list_tasks().await?.len();
        info!("Registry loaded with {} tasks", task_count);
        
        Ok(())
    }

    async fn registry(&self) -> Arc<TaskRegistry> {
        self.registry.clone()
    }
}