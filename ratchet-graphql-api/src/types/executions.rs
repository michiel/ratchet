//! GraphQL types for executions

use async_graphql::{SimpleObject, InputObject, Enum};
use ratchet_api_types::{UnifiedExecution, ExecutionStatus};
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// GraphQL Execution type
#[derive(SimpleObject, Clone)]
pub struct Execution {
    pub id: GraphQLApiId,
    pub task_id: GraphQLApiId,
    pub job_id: Option<GraphQLApiId>,
    pub status: ExecutionStatusGraphQL,
    pub input: Option<JsonValue>,
    pub output: Option<JsonValue>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub worker_id: Option<String>,
    pub retry_count: i32,
    pub metadata: Option<JsonValue>,
}

/// GraphQL enum for execution status
#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatusGraphQL {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
    Retrying,
}

impl From<ExecutionStatus> for ExecutionStatusGraphQL {
    fn from(status: ExecutionStatus) -> Self {
        match status {
            ExecutionStatus::Pending => ExecutionStatusGraphQL::Pending,
            ExecutionStatus::Running => ExecutionStatusGraphQL::Running,
            ExecutionStatus::Completed => ExecutionStatusGraphQL::Completed,
            ExecutionStatus::Failed => ExecutionStatusGraphQL::Failed,
            ExecutionStatus::Cancelled => ExecutionStatusGraphQL::Cancelled,
            ExecutionStatus::Timeout => ExecutionStatusGraphQL::Timeout,
            ExecutionStatus::Retrying => ExecutionStatusGraphQL::Retrying,
        }
    }
}

impl From<ExecutionStatusGraphQL> for ExecutionStatus {
    fn from(status: ExecutionStatusGraphQL) -> Self {
        match status {
            ExecutionStatusGraphQL::Pending => ExecutionStatus::Pending,
            ExecutionStatusGraphQL::Running => ExecutionStatus::Running,
            ExecutionStatusGraphQL::Completed => ExecutionStatus::Completed,
            ExecutionStatusGraphQL::Failed => ExecutionStatus::Failed,
            ExecutionStatusGraphQL::Cancelled => ExecutionStatus::Cancelled,
            ExecutionStatusGraphQL::Timeout => ExecutionStatus::Timeout,
            ExecutionStatusGraphQL::Retrying => ExecutionStatus::Retrying,
        }
    }
}

impl From<UnifiedExecution> for Execution {
    fn from(execution: UnifiedExecution) -> Self {
        Self {
            id: execution.id.into(),
            task_id: execution.task_id.into(),
            job_id: execution.job_id.map(|id| id.into()),
            status: execution.status.into(),
            input: execution.input,
            output: execution.output,
            error_message: execution.error_message,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms,
            worker_id: execution.worker_id,
            retry_count: execution.retry_count,
            metadata: execution.metadata,
        }
    }
}

/// Input type for creating executions
#[derive(InputObject)]
pub struct CreateExecutionInput {
    pub task_id: GraphQLApiId,
    pub job_id: Option<GraphQLApiId>,
    pub input: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

/// Input type for execution filtering
#[derive(InputObject)]
pub struct ExecutionFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub job_id: Option<GraphQLApiId>,
    pub status: Option<ExecutionStatusGraphQL>,
    pub worker_id: Option<String>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
}

/// Execution statistics
#[derive(SimpleObject)]
pub struct ExecutionStats {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub pending_executions: i64,
    pub running_executions: i64,
    pub average_duration_ms: Option<f64>,
    pub total_duration_ms: i64,
}