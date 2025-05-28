use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

use crate::errors::Result;
use crate::registry::{TaskRegistry, TaskSource, loaders::{TaskLoader, filesystem::FilesystemTaskLoader, http::HttpTaskLoader}};
use crate::registry::watcher::{RegistryWatcher, WatcherConfig};
use crate::services::TaskSyncService;
use crate::config::RegistrySourceConfig;

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
    watcher: Option<Arc<RwLock<RegistryWatcher>>>,
    sources: Vec<TaskSource>,
    source_configs: Vec<RegistrySourceConfig>,
}

impl DefaultRegistryService {
    pub fn new(sources: Vec<TaskSource>) -> Self {
        let sources_clone = sources.clone();
        Self {
            registry: Arc::new(TaskRegistry::with_sources(sources)),
            filesystem_loader: FilesystemTaskLoader::new(),
            http_loader: HttpTaskLoader::new(),
            sync_service: None,
            watcher: None,
            sources: sources_clone,
            source_configs: Vec::new(),
        }
    }
    
    pub fn new_with_configs(sources: Vec<TaskSource>, configs: Vec<RegistrySourceConfig>) -> Self {
        let sources_clone = sources.clone();
        Self {
            registry: Arc::new(TaskRegistry::with_sources(sources)),
            filesystem_loader: FilesystemTaskLoader::new(),
            http_loader: HttpTaskLoader::new(),
            sync_service: None,
            watcher: None,
            sources: sources_clone,
            source_configs: configs,
        }
    }
    
    pub fn with_sync_service(mut self, sync_service: Arc<TaskSyncService>) -> Self {
        self.sync_service = Some(sync_service);
        self
    }

    pub fn registry_mut(&mut self) -> &mut Arc<TaskRegistry> {
        &mut self.registry
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

impl DefaultRegistryService {
    /// Start watching filesystem sources if configured
    pub async fn start_watching(&mut self) -> Result<()> {
        // Collect filesystem sources with watch enabled
        let mut watch_paths = Vec::new();
        
        // Match sources with their configs
        for (i, source) in self.sources.iter().enumerate() {
            if let TaskSource::Filesystem { path } = source {
                // Check if this source has watch enabled in its config
                if let Some(config) = self.source_configs.get(i) {
                    if let Some(source_config) = &config.config {
                        if source_config.get("watch").and_then(|v| v.as_bool()).unwrap_or(false) {
                            watch_paths.push((path.clone(), true)); // recursive = true
                        }
                    }
                }
            }
        }

        if watch_paths.is_empty() {
            info!("No filesystem sources configured for watching");
            return Ok(());
        }

        // Create watcher config from sources
        let watcher_config = WatcherConfig {
            enabled: true,
            debounce_ms: 500,
            ignore_patterns: vec![
                "*.tmp".to_string(),
                "*.swp".to_string(),
                ".git/**".to_string(),
                ".DS_Store".to_string(),
            ],
            max_concurrent_reloads: 5,
            retry_on_error: true,
            retry_delay_ms: 1000,
        };

        // Create and start watcher with just the registry
        let mut watcher = RegistryWatcher::new(
            self.registry.clone(),
            self.sync_service.clone(),
            watcher_config,
        );

        let num_paths = watch_paths.len();
        
        for (path, recursive) in watch_paths {
            watcher.add_watch_path(path, recursive);
        }

        watcher.start().await?;

        // Store watcher reference
        self.watcher = Some(Arc::new(RwLock::new(watcher)));

        info!("File system watcher started for {} paths", num_paths);
        Ok(())
    }

    /// Stop watching for changes
    pub async fn stop_watching(&mut self) -> Result<()> {
        if let Some(watcher_arc) = self.watcher.take() {
            let mut watcher = watcher_arc.write().await;
            watcher.stop().await?;
        }
        Ok(())
    }
}