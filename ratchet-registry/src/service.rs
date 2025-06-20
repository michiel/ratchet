use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::{RegistryConfig, TaskSource};
use crate::error::{RegistryError, Result};
use crate::loaders::{filesystem::FilesystemLoader, git::GitLoader, http::HttpLoader, TaskLoader};
use crate::registry::DefaultTaskRegistry;
use crate::sync::DatabaseSync;
use crate::types::{DiscoveredTask, SyncResult, TaskDefinition, TaskReference};
use crate::watcher::RegistryWatcher;

#[async_trait]
pub trait RegistryService: Send + Sync {
    async fn discover_all_tasks(&self) -> Result<Vec<DiscoveredTask>>;
    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition>;
    async fn sync_to_database(&self) -> Result<SyncResult>;
    async fn start_watching(&self) -> Result<()>;
    async fn stop_watching(&self) -> Result<()>;
    async fn registry(&self) -> Arc<DefaultTaskRegistry>;
}

pub struct DefaultRegistryService {
    registry: Arc<DefaultTaskRegistry>,
    filesystem_loader: FilesystemLoader,
    http_loader: HttpLoader,
    git_loader: GitLoader,
    sync_service: Option<Arc<DatabaseSync>>,
    watcher: Option<Arc<RwLock<RegistryWatcher>>>,
    config: RegistryConfig,
}

impl DefaultRegistryService {
    pub fn new(config: RegistryConfig) -> Self {
        let sources = config.sources.clone();
        Self {
            registry: Arc::new(DefaultTaskRegistry::with_sources(sources)),
            filesystem_loader: FilesystemLoader::new(),
            http_loader: HttpLoader::new(),
            git_loader: GitLoader::new(),
            sync_service: None,
            watcher: None,
            config,
        }
    }

    pub fn with_sync_service(mut self, sync_service: Arc<DatabaseSync>) -> Self {
        self.sync_service = Some(sync_service);
        self
    }

    async fn discover_from_source(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        info!("Discovering tasks from source: {:?}", source);

        let discovered = match source {
            TaskSource::Filesystem { .. } => self.filesystem_loader.discover_tasks(source).await?,
            TaskSource::Http { .. } => self.http_loader.discover_tasks(source).await?,
            TaskSource::Git { .. } => self.git_loader.discover_tasks(source).await?,
        };

        info!("Discovered {} tasks from source", discovered.len());
        Ok(discovered)
    }

    async fn load_discovered_tasks(&self, discovered: Vec<DiscoveredTask>) -> Result<()> {
        for task in discovered {
            // Load the full task definition
            let task_def = self.load_task(&task.task_ref).await?;

            // Add to registry
            if let Err(e) = self.registry.add_task(task_def.clone()).await {
                error!("Failed to add task to registry: {}", e);
            } else {
                // Auto-sync to database if sync service is available
                if let Some(sync_service) = &self.sync_service {
                    let discovered_tasks = vec![task];
                    if let Err(e) = sync_service.sync_discovered_tasks(discovered_tasks).await {
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
    async fn discover_all_tasks(&self) -> Result<Vec<DiscoveredTask>> {
        let mut all_discovered = Vec::new();

        for source in &self.config.sources {
            match self.discover_from_source(source).await {
                Ok(mut discovered) => {
                    all_discovered.append(&mut discovered);
                }
                Err(e) => {
                    error!("Failed to discover tasks from source {:?}: {}", source, e);
                    // Continue with other sources
                }
            }
        }

        info!("Total discovered tasks: {}", all_discovered.len());
        Ok(all_discovered)
    }

    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition> {
        // Determine which loader to use based on the source
        if task_ref.source.starts_with("file://") {
            self.filesystem_loader.load_task(task_ref).await
        } else if task_ref.source.starts_with("http://") || task_ref.source.starts_with("https://") {
            self.http_loader.load_task(task_ref).await
        } else if task_ref.source.starts_with("git://") {
            self.git_loader.load_task(task_ref).await
        } else {
            Err(RegistryError::Configuration(format!(
                "Unsupported task source: {}",
                task_ref.source
            )))
        }
    }

    async fn sync_to_database(&self) -> Result<SyncResult> {
        if let Some(sync_service) = &self.sync_service {
            // Discover all tasks first
            let discovered = self.discover_all_tasks().await?;

            // Load and add to registry
            self.load_discovered_tasks(discovered.clone()).await?;

            // Sync to database
            sync_service.sync_discovered_tasks(discovered).await
        } else {
            Err(RegistryError::Configuration("No sync service configured".to_string()))
        }
    }

    async fn start_watching(&self) -> Result<()> {
        // Collect filesystem sources with watch enabled
        let mut watch_paths = Vec::new();

        for source in &self.config.sources {
            if let TaskSource::Filesystem { path, recursive, watch } = source {
                if *watch {
                    watch_paths.push((path.clone().into(), *recursive));
                }
            }
        }

        if watch_paths.is_empty() {
            info!("No filesystem sources configured for watching");
            return Ok(());
        }

        // Create and start watcher
        let mut watcher = RegistryWatcher::new(
            self.registry.clone(),
            self.sync_service.clone(),
            crate::config::WatcherConfig::default(),
        );

        for (path, recursive) in watch_paths {
            watcher.add_watch_path(path, recursive);
        }

        watcher.start().await?;

        // Store watcher reference (this is simplified - in practice you'd want better lifecycle management)
        // self.watcher = Some(Arc::new(RwLock::new(watcher)));

        info!("File system watcher started");
        Ok(())
    }

    async fn stop_watching(&self) -> Result<()> {
        if let Some(watcher_arc) = &self.watcher {
            let mut watcher = watcher_arc.write().await;
            watcher.stop().await?;
        }
        Ok(())
    }

    async fn registry(&self) -> Arc<DefaultTaskRegistry> {
        self.registry.clone()
    }
}
