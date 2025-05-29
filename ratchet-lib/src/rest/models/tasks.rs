use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::services::UnifiedTask;

/// REST API representation of a Task
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub uuid: String,
    pub version: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    #[serde(rename = "registrySource")]
    pub registry_source: bool,
    #[serde(rename = "availableVersions")]
    pub available_versions: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "validatedAt")]
    pub validated_at: Option<DateTime<Utc>>,
    #[serde(rename = "inSync")]
    pub in_sync: bool,
}

impl From<UnifiedTask> for TaskResponse {
    fn from(task: UnifiedTask) -> Self {
        Self {
            id: task.uuid.to_string(),
            uuid: task.uuid.to_string(),
            version: task.version,
            label: task.label,
            description: task.description,
            enabled: task.enabled,
            registry_source: task.registry_source,
            available_versions: task.available_versions,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
            in_sync: task.in_sync,
        }
    }
}

/// Detailed task response with schemas
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDetailResponse {
    #[serde(flatten)]
    pub task: TaskResponse,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
    #[serde(rename = "outputSchema")]
    pub output_schema: Option<serde_json::Value>,
}

/// Task update request
#[derive(Debug, Deserialize)]
pub struct TaskUpdateRequest {
    pub enabled: Option<bool>,
}

/// Task filter parameters
#[derive(Debug, Deserialize)]
pub struct TaskFilters {
    pub uuid: Option<String>,
    pub label: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "registrySource")]
    pub registry_source: Option<bool>,
    pub label_like: Option<String>,
}

impl TaskFilters {
    pub fn matches_uuid(&self, uuid: &Uuid) -> bool {
        match &self.uuid {
            Some(filter_uuid) => uuid.to_string().contains(filter_uuid),
            None => true,
        }
    }

    pub fn matches_label(&self, label: &str) -> bool {
        match &self.label_like {
            Some(filter) => label.to_lowercase().contains(&filter.to_lowercase()),
            None => match &self.label {
                Some(exact) => label == exact,
                None => true,
            }
        }
    }

    pub fn matches_version(&self, version: &str) -> bool {
        match &self.version {
            Some(filter) => version == filter,
            None => true,
        }
    }

    pub fn matches_enabled(&self, enabled: bool) -> bool {
        match self.enabled {
            Some(filter) => enabled == filter,
            None => true,
        }
    }

    pub fn matches_registry_source(&self, registry_source: bool) -> bool {
        match self.registry_source {
            Some(filter) => registry_source == filter,
            None => true,
        }
    }
}