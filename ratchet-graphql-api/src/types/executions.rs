//! GraphQL types for executions

use super::scalars::GraphQLApiId;
use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use ratchet_api_types::{ExecutionStatus, UnifiedExecution};
use serde_json::Value as JsonValue;

/// GraphQL Execution type - using UnifiedExecution directly for API consistency
pub type Execution = UnifiedExecution;

/// GraphQL ExecutionStatus - using unified ExecutionStatus directly
pub type ExecutionStatusGraphQL = ExecutionStatus;

/// Input type for creating executions
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CreateExecutionInput {
    pub task_id: GraphQLApiId,
    pub input: JsonValue,
}

/// Input type for updating executions
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct UpdateExecutionInput {
    pub status: Option<ExecutionStatusGraphQL>,
    pub output: Option<JsonValue>,
    pub error_message: Option<String>,
    pub error_details: Option<JsonValue>,
    pub progress: Option<f32>,
}

/// Input type for execution filtering
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ExecutionFiltersInput {
    // ID filtering
    pub task_id: Option<GraphQLApiId>,
    pub task_id_in: Option<Vec<GraphQLApiId>>,
    pub id_in: Option<Vec<GraphQLApiId>>,

    // Status filtering
    pub status: Option<ExecutionStatusGraphQL>,
    pub status_in: Option<Vec<ExecutionStatusGraphQL>>,
    pub status_not: Option<ExecutionStatusGraphQL>,

    // Date range filtering
    pub queued_after: Option<DateTime<Utc>>,
    pub queued_before: Option<DateTime<Utc>>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub completed_after: Option<DateTime<Utc>>,
    pub completed_before: Option<DateTime<Utc>>,

    // Duration filtering
    pub duration_min_ms: Option<i32>,
    pub duration_max_ms: Option<i32>,

    // Progress filtering
    pub progress_min: Option<f32>,
    pub progress_max: Option<f32>,
    pub has_progress: Option<bool>,

    // Error filtering
    pub has_error: Option<bool>,
    pub error_message_contains: Option<String>,

    // Advanced filtering
    pub can_retry: Option<bool>,
    pub can_cancel: Option<bool>,
}

/// Execution statistics
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ExecutionStats {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub pending_executions: i64,
    pub running_executions: i64,
    pub average_duration_ms: Option<f64>,
    pub total_duration_ms: i64,
}
