use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};
use chrono::Utc;
use uuid::Uuid;

use crate::config::TaskSource;
use crate::error::{RegistryError, Result};
use crate::loaders::TaskLoader;
use crate::types::{DiscoveredTask, TaskDefinition, TaskMetadata, TaskReference};

pub struct FilesystemLoader {
    base_path: Option<PathBuf>,
    recursive: bool,
}

impl Default for FilesystemLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemLoader {
    pub fn new() -> Self {
        Self {
            base_path: None,
            recursive: true,
        }
    }

    pub fn with_base_path(base_path: PathBuf, recursive: bool) -> Self {
        Self {
            base_path: Some(base_path),
            recursive,
        }
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

    async fn load_task_metadata(path: &Path) -> Result<TaskMetadata> {
        let metadata_path = path.join("metadata.json");
        let metadata_content = fs::read_to_string(metadata_path).await?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

        // Extract basic metadata fields
        let name = metadata["name"]
            .as_str()
            .ok_or_else(|| RegistryError::ValidationError("Missing 'name' in metadata".to_string()))?
            .to_string();

        let version = metadata["version"]
            .as_str()
            .ok_or_else(|| RegistryError::ValidationError("Missing 'version' in metadata".to_string()))?
            .to_string();

        let uuid = if let Some(uuid_str) = metadata["uuid"].as_str() {
            Uuid::parse_str(uuid_str)
                .map_err(|e| RegistryError::ValidationError(format!("Invalid UUID: {}", e)))?
        } else {
            Uuid::new_v4() // Generate if not present
        };

        let description = metadata["description"].as_str().map(|s| s.to_string());
        let tags = metadata["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let now = Utc::now();

        Ok(TaskMetadata {
            uuid,
            name,
            version,
            description,
            tags,
            created_at: now,
            updated_at: now,
            checksum: None, // TODO: Calculate checksum
        })
    }

    async fn load_task_definition_from_path(&self, path: &Path) -> Result<TaskDefinition> {
        let metadata = Self::load_task_metadata(path).await?;

        // Load main script
        let main_js_path = path.join("main.js");
        let script = fs::read_to_string(main_js_path).await?;

        // Load schemas (optional)
        let input_schema = if path.join("input.schema.json").exists() {
            let schema_content = fs::read_to_string(path.join("input.schema.json")).await?;
            Some(serde_json::from_str(&schema_content)?)
        } else {
            None
        };

        let output_schema = if path.join("output.schema.json").exists() {
            let schema_content = fs::read_to_string(path.join("output.schema.json")).await?;
            Some(serde_json::from_str(&schema_content)?)
        } else {
            None
        };

        let task_ref = TaskReference {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            source: format!("file://{}", path.display()),
        };

        Ok(TaskDefinition {
            reference: task_ref,
            metadata,
            script,
            input_schema,
            output_schema,
            dependencies: Vec::new(), // TODO: Extract from metadata
            environment: std::collections::HashMap::new(), // TODO: Extract from metadata
        })
    }

    fn discover_tasks_in_directory<'a>(&'a self, path: &'a Path, source: &'a TaskSource) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<DiscoveredTask>>> + Send + 'a>> {
        Box::pin(async move {
        let mut discovered = Vec::new();

        if !path.exists() {
            return Err(RegistryError::TaskNotFound(format!(
                "Path does not exist: {:?}",
                path
            )));
        }

        let metadata = fs::metadata(path).await?;

        if metadata.is_file() && Self::is_zip_file(path).await {
            // Handle ZIP file - for now, skip implementation
            warn!("ZIP file support not yet implemented: {:?}", path);
        } else if metadata.is_dir() {
            if Self::is_task_directory(path).await {
                // Single task directory
                info!("Found task directory: {:?}", path);
                let task_metadata = Self::load_task_metadata(path).await?;
                let task_ref = TaskReference {
                    name: task_metadata.name.clone(),
                    version: task_metadata.version.clone(),
                    source: format!("file://{}", path.display()),
                };

                discovered.push(DiscoveredTask {
                    task_ref,
                    metadata: task_metadata,
                    discovered_at: Utc::now(),
                });
            } else if self.recursive {
                // Scan directory recursively
                info!("Scanning directory for tasks: {:?}", path);
                let mut entries = fs::read_dir(path).await?;

                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    let entry_metadata = entry.metadata().await?;

                    if entry_metadata.is_dir() && Self::is_task_directory(&entry_path).await {
                        info!("Found task directory: {:?}", entry_path);
                        let task_metadata = Self::load_task_metadata(&entry_path).await?;
                        let task_ref = TaskReference {
                            name: task_metadata.name.clone(),
                            version: task_metadata.version.clone(),
                            source: format!("file://{}", entry_path.display()),
                        };

                        discovered.push(DiscoveredTask {
                            task_ref,
                            metadata: task_metadata,
                            discovered_at: Utc::now(),
                        });
                    } else if entry_metadata.is_file() && Self::is_zip_file(&entry_path).await {
                        // Handle ZIP files - for now, skip
                        warn!("ZIP file support not yet implemented: {:?}", entry_path);
                    } else if entry_metadata.is_dir() {
                        // Recursively scan subdirectories
                        let subdiscovered = self.discover_tasks_in_directory(&entry_path, source).await?;
                        discovered.extend(subdiscovered);
                    }
                }
            }
        }

        info!("Discovered {} tasks from {:?}", discovered.len(), path);
        Ok(discovered)
        })
    }
}

#[async_trait]
impl TaskLoader for FilesystemLoader {
    async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        match source {
            TaskSource::Filesystem { path, recursive, .. } => {
                let loader = FilesystemLoader {
                    base_path: Some(PathBuf::from(path)),
                    recursive: *recursive,
                };
                loader.discover_tasks_in_directory(&PathBuf::from(path), source).await
            }
            _ => Err(RegistryError::Configuration(
                "FilesystemLoader only supports filesystem sources".to_string(),
            )),
        }
    }

    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition> {
        if !task_ref.source.starts_with("file://") {
            return Err(RegistryError::Configuration(
                "FilesystemLoader can only load file:// sources".to_string(),
            ));
        }

        let path_str = task_ref.source.strip_prefix("file://").unwrap();
        let path = PathBuf::from(path_str);

        self.load_task_definition_from_path(&path).await
    }

    async fn supports_source(&self, source: &TaskSource) -> bool {
        matches!(source, TaskSource::Filesystem { .. })
    }
}