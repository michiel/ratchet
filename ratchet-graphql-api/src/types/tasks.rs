//! GraphQL types for tasks

use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use ratchet_api_types::UnifiedTask;
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
    // String filtering with multiple operators
    pub name_contains: Option<String>,
    pub name_exact: Option<String>,
    pub name_starts_with: Option<String>,
    pub name_ends_with: Option<String>,

    // Version filtering
    pub version: Option<String>,
    pub version_in: Option<Vec<String>>,

    // Boolean filtering
    pub enabled: Option<bool>,
    pub registry_source: Option<bool>,

    // Date range filtering
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub validated_after: Option<DateTime<Utc>>,
    pub validated_before: Option<DateTime<Utc>>,

    // ID and UUID filtering
    pub uuid: Option<String>,
    pub uuid_in: Option<Vec<String>>,
    pub id_in: Option<Vec<i32>>,

    // Advanced filtering
    pub has_validation: Option<bool>,
    pub in_sync: Option<bool>,
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

/// Task test case input
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TaskTestCaseInput {
    pub name: String,
    pub input: JsonValue,
    pub expected_output: Option<JsonValue>,
    pub should_fail: Option<bool>,
    pub description: Option<String>,
}

/// MCP task development - create task input
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct McpCreateTaskInput {
    pub name: String,
    pub description: String,
    pub code: String,
    pub input_schema: JsonValue,
    pub output_schema: JsonValue,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub test_cases: Option<Vec<TaskTestCaseInput>>,
}

/// MCP task development - edit task input
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct McpEditTaskInput {
    pub name: String,
    pub description: Option<String>,
    pub code: Option<String>,
    pub input_schema: Option<JsonValue>,
    pub output_schema: Option<JsonValue>,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub test_cases: Option<Vec<TaskTestCaseInput>>,
}

/// MCP task test results
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct McpTaskTestResults {
    pub total: i32,
    pub passed: i32,
    pub failed: i32,
    pub skipped: i32,
    pub test_results: Vec<JsonValue>,
}

/// MCP store result input
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct McpStoreResultInput {
    pub task_id: String,
    pub input: JsonValue,
    pub output: JsonValue,
    pub execution_time_ms: Option<i64>,
    pub status: Option<String>,
}
