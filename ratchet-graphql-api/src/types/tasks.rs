//! GraphQL types for tasks

use async_graphql::{Object, InputObject, Enum, SimpleObject};
use ratchet_api_types::UnifiedTask;
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// GraphQL Task type
#[derive(SimpleObject, Clone)]
pub struct Task {
    pub id: GraphQLApiId,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: String,
    pub registry_source: bool,
    pub in_sync: bool,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

impl From<UnifiedTask> for Task {
    fn from(task: UnifiedTask) -> Self {
        Self {
            id: task.id.into(),
            name: task.name,
            description: task.description,
            enabled: task.enabled,
            version: task.version,
            registry_source: task.registry_source,
            in_sync: task.in_sync,
            input_schema: task.input_schema,
            output_schema: task.output_schema,
            metadata: task.metadata,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
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
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,
    pub in_sync: Option<bool>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
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