use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::RegistrySourceConfig;
use crate::errors::Result;
use crate::task::Task;

#[derive(Debug, Clone)]
pub enum TaskSource {
    Filesystem { path: PathBuf },
    Http { url: String },
}

pub struct TaskRegistry {
    tasks: Arc<RwLock<HashMap<Uuid, HashMap<String, Arc<Task>>>>>,
    sources: Vec<TaskSource>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            sources: Vec::new(),
        }
    }

    pub fn with_sources(sources: Vec<TaskSource>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            sources,
        }
    }

    pub async fn add_task(&self, task: Task) -> Result<()> {
        let task_id = task.metadata.uuid;
        let version = task.metadata.version.clone();

        let mut tasks = self.tasks.write().await;
        let version_map = tasks.entry(task_id).or_insert_with(HashMap::new);

        if version_map.contains_key(&version) {
            warn!(
                "Task {} version {} already exists in registry, skipping",
                task_id, version
            );
            return Ok(());
        }

        info!("Adding task {} version {} to registry", task_id, version);
        version_map.insert(version, Arc::new(task));
        Ok(())
    }

    pub async fn get_task(&self, id: Uuid, version: Option<&str>) -> Result<Option<Arc<Task>>> {
        let tasks = self.tasks.read().await;

        if let Some(version_map) = tasks.get(&id) {
            if let Some(version) = version {
                Ok(version_map.get(version).cloned())
            } else {
                // Get latest version
                let latest = version_map
                    .keys()
                    .max()
                    .and_then(|v| version_map.get(v))
                    .cloned();
                Ok(latest)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn list_tasks(&self) -> Result<Vec<Arc<Task>>> {
        let tasks = self.tasks.read().await;
        let mut all_tasks = Vec::new();

        for version_map in tasks.values() {
            // Get latest version of each task
            if let Some(latest) = version_map.keys().max().and_then(|v| version_map.get(v)) {
                all_tasks.push(latest.clone());
            }
        }

        Ok(all_tasks)
    }

    pub async fn list_versions(&self, id: Uuid) -> Result<Vec<String>> {
        let tasks = self.tasks.read().await;

        if let Some(version_map) = tasks.get(&id) {
            let mut versions: Vec<String> = version_map.keys().cloned().collect();
            versions.sort();
            Ok(versions)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn remove_task(&self, id: &Uuid) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if tasks.remove(id).is_some() {
            info!("Removed task {} from registry", id);
            Ok(())
        } else {
            warn!("Task {} not found in registry", id);
            Ok(())
        }
    }

    pub fn sources(&self) -> &[TaskSource] {
        &self.sources
    }
}

impl TaskSource {
    /// Create a TaskSource from a RegistrySourceConfig
    pub fn from_config(config: &RegistrySourceConfig) -> Result<Self> {
        if config.uri.starts_with("file://") {
            let path_str = config.uri.strip_prefix("file://").unwrap();
            let path = PathBuf::from(path_str);
            Ok(TaskSource::Filesystem { path })
        } else if config.uri.starts_with("http://") || config.uri.starts_with("https://") {
            Ok(TaskSource::Http {
                url: config.uri.clone(),
            })
        } else {
            Err(crate::errors::RatchetError::Configuration(format!(
                "Unsupported registry source URI: {}",
                config.uri
            )))
        }
    }
}
