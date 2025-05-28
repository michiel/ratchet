use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, error};

use crate::errors::Result;
use crate::registry::{TaskRegistry, TaskSource, loaders::{TaskLoader, filesystem::FilesystemTaskLoader, http::HttpTaskLoader}};
use crate::services::TaskSyncService;

#[async_trait]
pub trait RegistryService: Send + Sync {
    async fn load_all_sources(&self) -> Result<()>;
    async fn registry(&self) -> Arc<TaskRegistry>;
}

pub struct DefaultRegistryService {
    registry: Arc<TaskRegistry>,
    filesystem_loader: FilesystemTaskLoader,
    http_loader: HttpTaskLoader,
    sync_service: Option<Arc<TaskSyncService>>,
}

impl DefaultRegistryService {
    pub fn new(sources: Vec<TaskSource>) -> Self {
        Self {
            registry: Arc::new(TaskRegistry::with_sources(sources)),
            filesystem_loader: FilesystemTaskLoader::new(),
            http_loader: HttpTaskLoader::new(),
            sync_service: None,
        }
    }
    
    pub fn with_sync_service(mut self, sync_service: Arc<TaskSyncService>) -> Self {
        self.sync_service = Some(sync_service);
        self
    }

    async fn load_source(&self, source: &TaskSource) -> Result<()> {
        info!("Loading tasks from source: {:?}", source);
        
        let tasks = match source {
            TaskSource::Filesystem { .. } => self.filesystem_loader.load_tasks(source).await?,
            TaskSource::Http { .. } => self.http_loader.load_tasks(source).await?,
        };

        for task in tasks {
            if let Err(e) = self.registry.add_task(task.clone()).await {
                error!("Failed to add task to registry: {}", e);
            } else {
                // Auto-sync to database if sync service is available
                if let Some(sync_service) = &self.sync_service {
                    if let Err(e) = sync_service.sync_task_to_db(&task).await {
                        error!("Failed to sync task to database: {}", e);
                    }
                }
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