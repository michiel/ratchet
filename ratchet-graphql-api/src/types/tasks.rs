//! GraphQL types for tasks

use async_graphql::{InputObject, SimpleObject};
use ratchet_api_types::UnifiedTask;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// GraphQL Task type - using UnifiedTask directly for API consistency
pub type Task = UnifiedTask;

/// Input type for creating tasks
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
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
#[graphql(rename_fields = "camelCase")]
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
#[graphql(rename_fields = "camelCase")]
pub struct TaskFiltersInput {
    pub name_contains: Option<String>,
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
}

/// Task statistics
#[derive(async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
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
#[graphql(rename_fields = "camelCase")]
pub struct TaskValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}