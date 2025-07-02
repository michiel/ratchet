//! Filesystem-based task repository implementation
//!
//! This module provides a repository implementation that reads and writes
//! tasks to the local filesystem, supporting file watching and atomic operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result};

use super::task_sync::{
    RepositoryHealth, RepositoryMetadata, RepositoryTask, TaskMetadata, TaskRepository,
};

/// Filesystem-based task repository
pub struct FilesystemTaskRepository {
    /// Base path for task files
    base_path: PathBuf,
    /// File patterns to watch for changes
    watch_patterns: Vec<String>,
    /// Patterns to ignore during scanning
    ignore_patterns: Vec<String>,
    /// Repository name
    name: String,
}

impl FilesystemTaskRepository {
    /// Create a new filesystem repository
    pub fn new<P: Into<PathBuf>>(
        base_path: P,
        name: String,
        watch_patterns: Vec<String>,
        ignore_patterns: Vec<String>,
    ) -> Self {
        Self {
            base_path: base_path.into(),
            watch_patterns,
            ignore_patterns,
            name,
        }
    }

    /// Create a default filesystem repository for local tasks
    pub fn default_local<P: Into<PathBuf>>(base_path: P) -> Self {
        Self::new(
            base_path,
            "filesystem-local".to_string(),
            vec!["**/*.js".to_string(), "**/task.yaml".to_string()],
            vec![
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/target/**".to_string(),
                "**/.DS_Store".to_string(),
            ],
        )
    }

    /// Check if a path matches any of the ignore patterns
    fn should_ignore_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.ignore_patterns {
            if Self::matches_glob_pattern(&path_str, pattern) {
                return true;
            }
        }
        false
    }

    /// Check if a path matches any of the watch patterns
    fn should_watch_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.watch_patterns {
            if Self::matches_glob_pattern(&path_str, pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob pattern matching (basic implementation)
    fn matches_glob_pattern(path: &str, pattern: &str) -> bool {
        // Use glob crate for proper glob pattern matching
        match glob::Pattern::new(pattern) {
            Ok(glob_pattern) => glob_pattern.matches(path),
            Err(_) => {
                // Fallback to simple pattern matching for invalid patterns
                path.contains(&pattern.replace("*", ""))
            }
        }
    }

    /// Scan directory for task files
    fn scan_directory<'a>(&'a self, dir: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<PathBuf>>> + Send + 'a>> {
        Box::pin(async move {
        let mut task_files = Vec::new();
        
        if !dir.exists() {
            warn!("Directory does not exist: {:?}", dir);
            return Ok(task_files);
        }

        let mut entries = fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if self.should_ignore_path(&path) {
                continue;
            }

            if path.is_dir() {
                // Recursively scan subdirectories
                let mut sub_files = self.scan_directory(&path).await?;
                task_files.append(&mut sub_files);
            } else if self.should_watch_path(&path) {
                task_files.push(path);
            }
        }

        Ok(task_files)
        })
    }

    /// Load task from JavaScript file
    async fn load_js_task(&self, file_path: &Path) -> Result<RepositoryTask> {
        let mut file = fs::File::open(file_path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        let metadata = file.metadata().await?;
        let _modified_at = DateTime::from_timestamp(
            metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
            0,
        ).unwrap_or_else(Utc::now);
        
        let _created_at = DateTime::from_timestamp(
            metadata.created()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
            0,
        ).unwrap_or_else(Utc::now);

        // Try to find companion YAML metadata file
        let yaml_path = file_path.with_extension("yaml");
        let task_metadata = if yaml_path.exists() {
            self.load_yaml_metadata(&yaml_path).await?
        } else {
            // Extract metadata from JavaScript comments or use defaults
            self.extract_js_metadata(&contents)?
        };

        // Generate task name from file path
        let relative_path = file_path.strip_prefix(&self.base_path)?;
        let task_name = relative_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(RepositoryTask::new(
            relative_path.to_string_lossy().to_string(),
            task_name,
            contents,
            task_metadata.input_schema,
            task_metadata.output_schema,
            task_metadata.metadata,
        ))
    }

    /// Load YAML metadata file
    async fn load_yaml_metadata(&self, yaml_path: &Path) -> Result<TaskFileMetadata> {
        let contents = fs::read_to_string(yaml_path).await?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
        
        // Convert YAML to our metadata format
        let input_schema = yaml_value
            .get("input_schema")
            .map(|v| serde_json::to_value(v).unwrap_or(JsonValue::Null))
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));
            
        let output_schema = yaml_value
            .get("output_schema")
            .map(|v| serde_json::to_value(v).unwrap_or(JsonValue::Null))
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));

        let metadata = TaskMetadata {
            version: yaml_value
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            description: yaml_value
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            author: yaml_value
                .get("author")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tags: yaml_value
                .get("tags")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            custom: HashMap::new(),
        };

        Ok(TaskFileMetadata {
            input_schema,
            output_schema,
            metadata,
        })
    }

    /// Extract metadata from JavaScript comments (basic implementation)
    fn extract_js_metadata(&self, source_code: &str) -> Result<TaskFileMetadata> {
        // Look for JSDoc-style metadata comments
        let mut version = "1.0.0".to_string();
        let mut description = None;
        let mut author = None;
        let mut tags = Vec::new();

        for line in source_code.lines() {
            let line = line.trim();
            if line.starts_with("//") || line.starts_with("*") {
                if line.contains("@version") {
                    if let Some(v) = line.split("@version").nth(1) {
                        version = v.trim().to_string();
                    }
                } else if line.contains("@description") {
                    if let Some(d) = line.split("@description").nth(1) {
                        description = Some(d.trim().to_string());
                    }
                } else if line.contains("@author") {
                    if let Some(a) = line.split("@author").nth(1) {
                        author = Some(a.trim().to_string());
                    }
                } else if line.contains("@tag") {
                    if let Some(t) = line.split("@tag").nth(1) {
                        tags.push(t.trim().to_string());
                    }
                }
            }
        }

        Ok(TaskFileMetadata {
            input_schema: JsonValue::Object(serde_json::Map::new()),
            output_schema: JsonValue::Object(serde_json::Map::new()),
            metadata: TaskMetadata {
                version,
                description,
                author,
                tags,
                custom: HashMap::new(),
            },
        })
    }

    /// Write task to filesystem
    async fn write_task(&self, task: &RepositoryTask) -> Result<()> {
        let file_path = self.base_path.join(&task.path);
        
        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write JavaScript file
        fs::write(&file_path, &task.source_code).await?;

        // Write companion YAML metadata file
        let yaml_path = file_path.with_extension("yaml");
        let yaml_content = self.create_yaml_metadata(task)?;
        fs::write(&yaml_path, yaml_content).await?;

        info!("Task written to filesystem: {:?}", file_path);
        Ok(())
    }

    /// Create YAML metadata content for a task
    fn create_yaml_metadata(&self, task: &RepositoryTask) -> Result<String> {
        let mut yaml_data = serde_yaml::Mapping::new();
        
        yaml_data.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::String(task.metadata.version.clone()),
        );
        
        if let Some(description) = &task.metadata.description {
            yaml_data.insert(
                serde_yaml::Value::String("description".to_string()),
                serde_yaml::Value::String(description.clone()),
            );
        }
        
        if let Some(author) = &task.metadata.author {
            yaml_data.insert(
                serde_yaml::Value::String("author".to_string()),
                serde_yaml::Value::String(author.clone()),
            );
        }
        
        if !task.metadata.tags.is_empty() {
            let tags: Vec<serde_yaml::Value> = task.metadata.tags
                .iter()
                .map(|t| serde_yaml::Value::String(t.clone()))
                .collect();
            yaml_data.insert(
                serde_yaml::Value::String("tags".to_string()),
                serde_yaml::Value::Sequence(tags),
            );
        }
        
        // Convert JSON schemas to YAML
        if let Ok(input_yaml) = serde_yaml::to_value(&task.input_schema) {
            yaml_data.insert(
                serde_yaml::Value::String("input_schema".to_string()),
                input_yaml,
            );
        }
        
        if let Ok(output_yaml) = serde_yaml::to_value(&task.output_schema) {
            yaml_data.insert(
                serde_yaml::Value::String("output_schema".to_string()),
                output_yaml,
            );
        }

        serde_yaml::to_string(&yaml_data).context("Failed to serialize YAML metadata")
    }
}

/// Task metadata loaded from file
struct TaskFileMetadata {
    input_schema: JsonValue,
    output_schema: JsonValue,
    metadata: TaskMetadata,
}

#[async_trait]
impl TaskRepository for FilesystemTaskRepository {
    async fn list_tasks(&self) -> Result<Vec<RepositoryTask>> {
        debug!("Scanning filesystem repository: {:?}", self.base_path);
        
        let task_files = self.scan_directory(&self.base_path).await?;
        let mut tasks = Vec::new();

        for file_path in task_files {
            if file_path.extension().and_then(|s| s.to_str()) == Some("js") {
                match self.load_js_task(&file_path).await {
                    Ok(task) => {
                        debug!("Loaded task: {}", task.name);
                        tasks.push(task);
                    }
                    Err(e) => {
                        warn!("Failed to load task from {:?}: {}", file_path, e);
                    }
                }
            }
        }

        info!("Found {} tasks in filesystem repository", tasks.len());
        Ok(tasks)
    }

    async fn get_task(&self, path: &str) -> Result<Option<RepositoryTask>> {
        let file_path = self.base_path.join(path);
        
        if !file_path.exists() {
            return Ok(None);
        }

        match self.load_js_task(&file_path).await {
            Ok(task) => Ok(Some(task)),
            Err(e) => {
                error!("Failed to load task from {:?}: {}", file_path, e);
                Err(e)
            }
        }
    }

    async fn put_task(&self, task: &RepositoryTask) -> Result<()> {
        self.write_task(task).await
    }

    async fn delete_task(&self, path: &str) -> Result<()> {
        let file_path = self.base_path.join(path);
        let yaml_path = file_path.with_extension("yaml");

        // Remove both JavaScript and YAML files
        if file_path.exists() {
            fs::remove_file(&file_path).await?;
            info!("Deleted task file: {:?}", file_path);
        }

        if yaml_path.exists() {
            fs::remove_file(&yaml_path).await?;
            info!("Deleted metadata file: {:?}", yaml_path);
        }

        Ok(())
    }

    async fn get_metadata(&self) -> Result<RepositoryMetadata> {
        let task_count = self.list_tasks().await?.len();
        
        let mut metadata = HashMap::new();
        metadata.insert("task_count".to_string(), JsonValue::Number(task_count.into()));
        metadata.insert("watch_patterns".to_string(), JsonValue::Array(
            self.watch_patterns.iter().map(|p| JsonValue::String(p.clone())).collect()
        ));
        metadata.insert("ignore_patterns".to_string(), JsonValue::Array(
            self.ignore_patterns.iter().map(|p| JsonValue::String(p.clone())).collect()
        ));

        Ok(RepositoryMetadata {
            name: self.name.clone(),
            repository_type: "filesystem".to_string(),
            uri: self.base_path.to_string_lossy().to_string(),
            branch: None,
            commit: None,
            is_writable: true,
            metadata,
        })
    }

    async fn is_writable(&self) -> bool {
        // Check if we can write to the base directory
        self.base_path.exists() && 
        fs::metadata(&self.base_path).await
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    }

    async fn test_connection(&self) -> Result<bool> {
        // Test if we can access the base directory
        Ok(self.base_path.exists() && self.base_path.is_dir())
    }

    async fn health_check(&self) -> Result<RepositoryHealth> {
        let accessible = self.base_path.exists();
        let writable = accessible && self.is_writable().await;
        
        let message = if !accessible {
            "Directory does not exist or is not accessible".to_string()
        } else if !writable {
            "Directory is read-only".to_string()
        } else {
            "Repository is healthy".to_string()
        };

        Ok(RepositoryHealth {
            accessible,
            writable,
            last_success: if accessible { Some(Utc::now()) } else { None },
            error_count: if accessible { 0 } else { 1 },
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_filesystem_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemTaskRepository::default_local(temp_dir.path());
        
        assert_eq!(repo.name, "filesystem-local");
        assert!(repo.watch_patterns.contains(&"**/*.js".to_string()));
        assert!(repo.ignore_patterns.contains(&"**/node_modules/**".to_string()));
    }

    #[tokio::test]
    async fn test_empty_directory_scan() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemTaskRepository::default_local(temp_dir.path());
        
        let tasks = repo.list_tasks().await.unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_task_file_creation_and_loading() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemTaskRepository::default_local(temp_dir.path());
        
        // Create a test task
        let task_metadata = TaskMetadata::minimal("1.0.0".to_string());
        let task = RepositoryTask::new(
            "test_task.js".to_string(),
            "test_task".to_string(),
            "function testTask() { return 'hello'; }".to_string(),
            serde_json::json!({"type": "object"}),
            serde_json::json!({"type": "string"}),
            task_metadata,
        );

        // Write task to filesystem
        repo.put_task(&task).await.unwrap();

        // Load tasks from filesystem
        let loaded_tasks = repo.list_tasks().await.unwrap();
        assert_eq!(loaded_tasks.len(), 1);
        assert_eq!(loaded_tasks[0].name, "test_task");
        assert_eq!(loaded_tasks[0].source_code, "function testTask() { return 'hello'; }");
    }

    #[tokio::test]
    async fn test_repository_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemTaskRepository::default_local(temp_dir.path());
        
        let health = repo.health_check().await.unwrap();
        assert!(health.accessible);
        assert!(health.writable);
        assert_eq!(health.message, "Repository is healthy");
    }

    #[tokio::test]
    async fn test_glob_pattern_matching() {
        assert!(FilesystemTaskRepository::matches_glob_pattern("test.js", "*.js"));
        assert!(FilesystemTaskRepository::matches_glob_pattern("path/to/test.js", "**/*.js"));
        assert!(FilesystemTaskRepository::matches_glob_pattern("node_modules/package/index.js", "**/node_modules/**"));
        assert!(!FilesystemTaskRepository::matches_glob_pattern("test.py", "*.js"));
    }
}