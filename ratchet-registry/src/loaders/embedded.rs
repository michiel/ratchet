//! Embedded task loader
//!
//! This loader provides access to tasks that are embedded directly in the Ratchet binary.
//! These tasks are always available and cannot be modified or deleted.

use crate::{
    config::TaskSource,
    error::RegistryError,
    types::{DiscoveredTask, TaskDefinition, TaskMetadata, TaskReference},
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

/// Embedded task data structure (copied from ratchet-server to avoid circular dependency)
#[derive(Debug, Clone)]
pub struct EmbeddedTask {
    pub name: String,
    pub metadata: &'static str,
    pub input_schema: &'static str,
    pub output_schema: &'static str,
    pub main_js: &'static str,
}

/// Loader for embedded tasks
#[derive(Debug, Clone)]
pub struct EmbeddedLoader {
    tasks: HashMap<String, EmbeddedTask>,
}

impl EmbeddedLoader {
    /// Create a new embedded loader with built-in tasks
    pub fn new() -> Self {
        let mut tasks = HashMap::new();

        // Add heartbeat task (embedded directly here to avoid circular dependency)
        let heartbeat_task = EmbeddedTask {
            name: "heartbeat".to_string(),
            metadata: include_str!("../../../ratchet-server/src/embedded/heartbeat/metadata.json"),
            input_schema: include_str!("../../../ratchet-server/src/embedded/heartbeat/input.schema.json"),
            output_schema: include_str!("../../../ratchet-server/src/embedded/heartbeat/output.schema.json"),
            main_js: include_str!("../../../ratchet-server/src/embedded/heartbeat/main.js"),
        };

        tasks.insert(heartbeat_task.name.clone(), heartbeat_task);

        Self { tasks }
    }

    /// Get all embedded task names
    pub fn task_names(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Check if a task is embedded
    pub fn contains_task(&self, name: &str) -> bool {
        self.tasks.contains_key(name)
    }

    /// Convert embedded task to TaskDefinition
    fn convert_to_task_definition(
        &self,
        embedded_task: &EmbeddedTask,
        reference: TaskReference,
    ) -> crate::error::Result<TaskDefinition> {
        // Parse metadata
        let metadata_json: serde_json::Value = serde_json::from_str(embedded_task.metadata)
            .map_err(|e| RegistryError::ValidationError(format!("Invalid metadata JSON: {}", e)))?;

        // Parse schemas
        let input_schema: serde_json::Value = serde_json::from_str(embedded_task.input_schema)
            .map_err(|e| RegistryError::ValidationError(format!("Invalid input schema JSON: {}", e)))?;

        let output_schema: serde_json::Value = serde_json::from_str(embedded_task.output_schema)
            .map_err(|e| RegistryError::ValidationError(format!("Invalid output schema JSON: {}", e)))?;

        // Extract required fields from metadata
        let uuid_str = metadata_json
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RegistryError::ValidationError("Missing 'uuid' in metadata".to_string()))?;

        let uuid =
            Uuid::parse_str(uuid_str).map_err(|e| RegistryError::ValidationError(format!("Invalid UUID: {}", e)))?;

        let description = metadata_json
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = metadata_json
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        // Create TaskMetadata
        let metadata = TaskMetadata {
            uuid,
            name: reference.name.clone(),
            version: reference.version.clone(),
            description,
            tags,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
        };

        // Create TaskDefinition
        Ok(TaskDefinition {
            reference,
            metadata,
            script: embedded_task.main_js.to_string(),
            input_schema: Some(input_schema),
            output_schema: Some(output_schema),
            dependencies: vec![],
            environment: HashMap::new(),
        })
    }
}

impl Default for EmbeddedLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::loaders::TaskLoader for EmbeddedLoader {
    async fn discover_tasks(&self, _source: &TaskSource) -> crate::error::Result<Vec<DiscoveredTask>> {
        let mut tasks = Vec::new();

        for embedded_task in self.tasks.values() {
            // Parse metadata to extract task information
            let metadata_json: serde_json::Value = serde_json::from_str(embedded_task.metadata)
                .map_err(|e| RegistryError::ValidationError(format!("Invalid metadata JSON: {}", e)))?;

            let name = metadata_json
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RegistryError::ValidationError("Missing 'name' in metadata".to_string()))?
                .to_string();

            let version = metadata_json
                .get("version")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RegistryError::ValidationError("Missing 'version' in metadata".to_string()))?
                .to_string();

            let task_ref = TaskReference {
                name: name.clone(),
                version: version.clone(),
                source: format!("embedded://{}", name),
            };

            // Create metadata from parsed JSON
            let uuid_str = metadata_json
                .get("uuid")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RegistryError::ValidationError("Missing 'uuid' in metadata".to_string()))?;

            let uuid = Uuid::parse_str(uuid_str)
                .map_err(|e| RegistryError::ValidationError(format!("Invalid UUID: {}", e)))?;

            let description = metadata_json
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let tags = metadata_json
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let metadata = TaskMetadata {
                uuid,
                name,
                version,
                description,
                tags,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                checksum: None,
            };

            tasks.push(DiscoveredTask {
                task_ref,
                metadata,
                discovered_at: Utc::now(),
            });
        }

        Ok(tasks)
    }

    async fn load_task(&self, task_ref: &TaskReference) -> crate::error::Result<TaskDefinition> {
        let embedded_task = self
            .tasks
            .get(&task_ref.name)
            .ok_or_else(|| RegistryError::TaskNotFound(task_ref.name.clone()))?;

        self.convert_to_task_definition(embedded_task, task_ref.clone())
    }

    async fn supports_source(&self, source: &TaskSource) -> bool {
        // Embedded loader supports special "embedded://" sources
        match source {
            TaskSource::Git { url, .. } => url.starts_with("embedded://"),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GitConfig, TaskSource};
    use crate::loaders::TaskLoader;

    #[tokio::test]
    async fn test_embedded_loader_creation() {
        let loader = EmbeddedLoader::new();
        assert!(loader.contains_task("heartbeat"));
        assert_eq!(loader.task_names().len(), 1);
    }

    #[tokio::test]
    async fn test_discover_embedded_tasks() {
        let loader = EmbeddedLoader::new();
        let source = TaskSource::Git {
            url: "embedded://".to_string(),
            auth: None,
            config: GitConfig::default(),
        };

        let tasks = loader.discover_tasks(&source).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_ref.name, "heartbeat");
    }

    #[tokio::test]
    async fn test_load_heartbeat_task() {
        let loader = EmbeddedLoader::new();

        let task_ref = TaskReference {
            name: "heartbeat".to_string(),
            version: "1.0.0".to_string(),
            source: "embedded://heartbeat".to_string(),
        };

        let task = loader.load_task(&task_ref).await.unwrap();
        assert_eq!(task.reference.name, "heartbeat");
        assert!(task.input_schema.is_some());
        assert!(task.output_schema.is_some());
    }

    #[tokio::test]
    async fn test_supports_embedded_source() {
        let loader = EmbeddedLoader::new();

        let embedded_source = TaskSource::Git {
            url: "embedded://".to_string(),
            auth: None,
            config: GitConfig::default(),
        };

        let filesystem_source = TaskSource::Filesystem {
            path: "/some/path".to_string(),
            recursive: true,
            watch: false,
        };

        assert!(loader.supports_source(&embedded_source).await);
        assert!(!loader.supports_source(&filesystem_source).await);
    }
}
