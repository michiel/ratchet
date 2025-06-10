//! GraphQL types for tasks

use async_graphql::{InputObject, SimpleObject};
use ratchet_api_types::UnifiedTask;
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// GraphQL Task type
#[derive(SimpleObject, Clone)]
pub struct Task {
    pub id: GraphQLApiId,
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub available_versions: Vec<String>,
    pub registry_source: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
    pub in_sync: bool,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

impl From<UnifiedTask> for Task {
    fn from(task: UnifiedTask) -> Self {
        Self {
            id: task.id.into(),
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            available_versions: task.available_versions,
            registry_source: task.registry_source,
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
            in_sync: task.in_sync,
            input_schema: task.input_schema,
            output_schema: task.output_schema,
            metadata: task.metadata,
        }
    }
}

/// Input type for creating tasks
#[derive(InputObject)]
pub struct CreateTaskInput {
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

/// Input type for updating tasks
#[derive(InputObject)]
pub struct UpdateTaskInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

/// Input type for task filtering
#[derive(InputObject)]
pub struct TaskFiltersInput {
    pub name_contains: Option<String>,
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
}

/// Task statistics
#[derive(SimpleObject)]
pub struct TaskStats {
    pub total_tasks: i32,
    pub enabled_tasks: i32,
    pub disabled_tasks: i32,
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub average_execution_time_ms: Option<f64>,
}

/// Task validation result
#[derive(SimpleObject)]
pub struct TaskValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}