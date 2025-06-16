//! GraphQL types for executions

use async_graphql::{SimpleObject, InputObject};
use ratchet_api_types::{UnifiedExecution, ExecutionStatus};
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
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
    pub task_id: Option<GraphQLApiId>,
    pub status: Option<ExecutionStatusGraphQL>,
    pub queued_after: Option<DateTime<Utc>>,
    pub completed_after: Option<DateTime<Utc>>,
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