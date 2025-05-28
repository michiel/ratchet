use std::path::Path;
use async_trait::async_trait;
use tracing::{info, warn};
use tokio::fs;

use crate::errors::{Result, RatchetError};
use crate::registry::{TaskSource, loaders::TaskLoader};
use crate::task::Task;

pub struct FilesystemTaskLoader;

impl FilesystemTaskLoader {
    pub fn new() -> Self {
        Self
    }

    async fn is_task_directory(path: &Path) -> bool {
        let metadata_path = path.join("metadata.json");
        fs::try_exists(metadata_path).await.unwrap_or(false)
    }

    async fn is_zip_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            ext == "zip"
        } else {
            false
        }
    }

    async fn load_from_path(&self, path: &Path) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();

        if !path.exists() {
            return Err(RatchetError::TaskNotFound(format!("Path does not exist: {:?}", path)));
        }

        let metadata = fs::metadata(path).await?;

        if metadata.is_file() && Self::is_zip_file(path).await {
            // Load single ZIP file
            info!("Loading task from ZIP file: {:?}", path);
            match Task::from_fs(path) {
                Ok(task) => tasks.push(task),
                Err(e) => warn!("Failed to load task from {:?}: {}", path, e),
            }
        } else if metadata.is_dir() {
            if Self::is_task_directory(path).await {
                // Load single task directory
                info!("Loading task from directory: {:?}", path);
                match Task::from_fs(path) {
                    Ok(task) => tasks.push(task),
                    Err(e) => warn!("Failed to load task from {:?}: {}", path, e),
                }
            } else {
                // Scan directory for tasks
                info!("Scanning directory for tasks: {:?}", path);
                let mut entries = fs::read_dir(path).await?;
                
                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    let entry_metadata = entry.metadata().await?;
                    
                    if entry_metadata.is_dir() && Self::is_task_directory(&entry_path).await {
                        info!("Found task directory: {:?}", entry_path);
                        match Task::from_fs(&entry_path) {
                            Ok(task) => tasks.push(task),
                            Err(e) => warn!("Failed to load task from {:?}: {}", entry_path, e),
                        }
                    } else if entry_metadata.is_file() && Self::is_zip_file(&entry_path).await {
                        info!("Found task ZIP file: {:?}", entry_path);
                        match Task::from_fs(&entry_path) {
                            Ok(task) => tasks.push(task),
                            Err(e) => warn!("Failed to load task from {:?}: {}", entry_path, e),
                        }
                    }
                }
            }
        }

        info!("Loaded {} tasks from {:?}", tasks.len(), path);
        Ok(tasks)
    }
}

#[async_trait]
impl TaskLoader for FilesystemTaskLoader {
    async fn load_tasks(&self, source: &TaskSource) -> Result<Vec<Task>> {
        match source {
            TaskSource::Filesystem { path } => self.load_from_path(path).await,
            _ => Err(RatchetError::NotImplemented("FilesystemTaskLoader only supports filesystem sources".to_string())),
        }
    }
}