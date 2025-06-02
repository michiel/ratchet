use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{
    api::types::{UnifiedTask, ApiId},
    services::UnifiedTask as ServiceUnifiedTask,
};

/// REST API representation of a Task (now unified)
pub type TaskResponse = UnifiedTask;

impl From<ServiceUnifiedTask> for TaskResponse {
    fn from(task: ServiceUnifiedTask) -> Self {
        UnifiedTask::from(task)
    }
}

/// Detailed task response with schemas (now just an alias)
pub type TaskDetailResponse = UnifiedTask;

/// Task update request
#[derive(Debug, Deserialize)]
pub struct TaskUpdateRequest {
    pub enabled: Option<bool>,
}

/// Task filter parameters
#[derive(Debug, Deserialize)]
pub struct TaskFilters {
    pub uuid: Option<String>,
    pub name: Option<String>, // Changed from 'label' to 'name' for consistency
    pub version: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "registrySource")]
    pub registry_source: Option<bool>,
    pub name_like: Option<String>, // Changed from 'label_like' to 'name_like'
}

impl TaskFilters {
    pub fn matches_uuid(&self, uuid: &Uuid) -> bool {
        match &self.uuid {
            Some(filter_uuid) => uuid.to_string().contains(filter_uuid),
            None => true,
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        match &self.name_like {
            Some(filter) => name.to_lowercase().contains(&filter.to_lowercase()),
            None => match &self.name {
                Some(exact) => name == exact,
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