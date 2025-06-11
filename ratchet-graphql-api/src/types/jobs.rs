//! GraphQL types for jobs

use async_graphql::{SimpleObject, InputObject};
use ratchet_api_types::{UnifiedJob, JobStatus, JobPriority};
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};

/// GraphQL Job type - using UnifiedJob directly for API consistency
pub type Job = UnifiedJob;

/// GraphQL JobStatus - using unified JobStatus directly
pub type JobStatusGraphQL = JobStatus;

/// GraphQL JobPriority - using unified JobPriority directly
pub type JobPriorityGraphQL = JobPriority;

/// Input type for creating jobs
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CreateJobInput {
    pub task_id: GraphQLApiId,
    pub priority: Option<JobPriorityGraphQL>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub max_retries: Option<i32>,
}

/// Input type for job filtering
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct JobFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub status: Option<JobStatusGraphQL>,
    pub priority: Option<JobPriorityGraphQL>,
    pub queued_after: Option<DateTime<Utc>>,
    pub scheduled_before: Option<DateTime<Utc>>,
}

/// Job statistics
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct JobStats {
    pub total_jobs: i64,
    pub pending_jobs: i64,
    pub running_jobs: i64,
    pub completed_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
    pub average_processing_time_ms: Option<f64>,
}