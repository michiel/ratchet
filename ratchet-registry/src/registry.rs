use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::TaskSource;
use crate::error::{RegistryError, Result};
use crate::types::{DiscoveredTask, RegistryEvent, SyncResult, TaskDefinition, TaskReference, ValidationResult};

#[async_trait]
pub trait TaskRegistry: Send + Sync {
    async fn discover_tasks(&self) -> Result<Vec<DiscoveredTask>>;
    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition>;
    async fn validate_task(&self, task: &TaskDefinition) -> Result<ValidationResult>;
    async fn sync_with_database(&self) -> Result<SyncResult>;
    async fn get_task_versions(&self, name: &str) -> Result<Vec<String>>;
    async fn watch_for_changes(&self) -> Result<tokio::sync::mpsc::Receiver<RegistryEvent>>;
}

pub struct DefaultTaskRegistry {
    tasks: Arc<RwLock<HashMap<Uuid, HashMap<String, Arc<TaskDefinition>>>>>,
    sources: Vec<TaskSource>,
}

impl Default for DefaultTaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultTaskRegistry {
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

    pub async fn add_task(&self, task: TaskDefinition) -> Result<()> {
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

    pub async fn get_task(&self, id: Uuid, version: Option<&str>) -> Result<Option<Arc<TaskDefinition>>> {
        let tasks = self.tasks.read().await;

        if let Some(version_map) = tasks.get(&id) {
            if let Some(version) = version {
                Ok(version_map.get(version).cloned())
            } else {
                // Get latest version
                let latest = version_map.keys().max().and_then(|v| version_map.get(v)).cloned();
                Ok(latest)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn list_tasks(&self) -> Result<Vec<Arc<TaskDefinition>>> {
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

#[async_trait]
impl TaskRegistry for DefaultTaskRegistry {
    async fn discover_tasks(&self) -> Result<Vec<DiscoveredTask>> {
        // This is a placeholder implementation
        // In practice, this would use the loaders to discover tasks from sources
        Ok(Vec::new())
    }

    async fn load_task(&self, _task_ref: &TaskReference) -> Result<TaskDefinition> {
        // This is a placeholder implementation
        // In practice, this would use the loaders to load the task definition
        Err(RegistryError::NotImplemented(
            "load_task not yet implemented".to_string(),
        ))
    }

    async fn validate_task(&self, _task: &TaskDefinition) -> Result<ValidationResult> {
        // This is a placeholder implementation
        // In practice, this would validate the task using jsonschema
        Ok(ValidationResult::new())
    }

    async fn sync_with_database(&self) -> Result<SyncResult> {
        // This is a placeholder implementation
        // In practice, this would sync with the database using ratchet-storage
        Ok(SyncResult::new())
    }

    async fn get_task_versions(&self, name: &str) -> Result<Vec<String>> {
        let tasks = self.tasks.read().await;
        let mut versions = Vec::new();

        for version_map in tasks.values() {
            for task in version_map.values() {
                if task.metadata.name == name {
                    versions.push(task.metadata.version.clone());
                }
            }
        }

        versions.sort();
        versions.dedup();
        Ok(versions)
    }

    async fn watch_for_changes(&self) -> Result<tokio::sync::mpsc::Receiver<RegistryEvent>> {
        // This is a placeholder implementation
        // In practice, this would set up file watchers and return events
        let (_tx, rx) = tokio::sync::mpsc::channel(100);
        Ok(rx)
    }
}
