//! GraphQL types for jobs

use async_graphql::{SimpleObject, InputObject, Enum};
use ratchet_api_types::{UnifiedJob, JobStatus, JobPriority};
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// GraphQL Job type
#[derive(SimpleObject, Clone)]
pub struct Job {
    pub id: GraphQLApiId,
    pub task_id: GraphQLApiId,
    pub schedule_id: Option<GraphQLApiId>,
    pub status: JobStatusGraphQL,
    pub priority: JobPriorityGraphQL,
    pub input: Option<JsonValue>,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub max_retries: i32,
    pub retry_count: i32,
    pub timeout_seconds: Option<i32>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

/// GraphQL enum for job status
#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum JobStatusGraphQL {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
    Retrying,
}

impl From<JobStatus> for JobStatusGraphQL {
    fn from(status: JobStatus) -> Self {
        match status {
            JobStatus::Pending => JobStatusGraphQL::Pending,
            JobStatus::Running => JobStatusGraphQL::Running,
            JobStatus::Completed => JobStatusGraphQL::Completed,
            JobStatus::Failed => JobStatusGraphQL::Failed,
            JobStatus::Cancelled => JobStatusGraphQL::Cancelled,
            JobStatus::Timeout => JobStatusGraphQL::Timeout,
            JobStatus::Retrying => JobStatusGraphQL::Retrying,
        }
    }
}

impl From<JobStatusGraphQL> for JobStatus {
    fn from(status: JobStatusGraphQL) -> Self {
        match status {
            JobStatusGraphQL::Pending => JobStatus::Pending,
            JobStatusGraphQL::Running => JobStatus::Running,
            JobStatusGraphQL::Completed => JobStatus::Completed,
            JobStatusGraphQL::Failed => JobStatus::Failed,
            JobStatusGraphQL::Cancelled => JobStatus::Cancelled,
            JobStatusGraphQL::Timeout => JobStatus::Timeout,
            JobStatusGraphQL::Retrying => JobStatus::Retrying,
        }
    }
}

/// GraphQL enum for job priority
#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum JobPriorityGraphQL {
    Low,
    Normal,
    High,
    Critical,
}

impl From<JobPriority> for JobPriorityGraphQL {
    fn from(priority: JobPriority) -> Self {
        match priority {
            JobPriority::Low => JobPriorityGraphQL::Low,
            JobPriority::Normal => JobPriorityGraphQL::Normal,
            JobPriority::High => JobPriorityGraphQL::High,
            JobPriority::Critical => JobPriorityGraphQL::Critical,
        }
    }
}

impl From<JobPriorityGraphQL> for JobPriority {
    fn from(priority: JobPriorityGraphQL) -> Self {
        match priority {
            JobPriorityGraphQL::Low => JobPriority::Low,
            JobPriorityGraphQL::Normal => JobPriority::Normal,
            JobPriorityGraphQL::High => JobPriority::High,
            JobPriorityGraphQL::Critical => JobPriority::Critical,
        }
    }
}

impl From<UnifiedJob> for Job {
    fn from(job: UnifiedJob) -> Self {
        Self {
            id: job.id.into(),
            task_id: job.task_id.into(),
            schedule_id: job.schedule_id.map(|id| id.into()),
            status: job.status.into(),
            priority: job.priority.into(),
            input: job.input,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            max_retries: job.max_retries,
            retry_count: job.retry_count,
            timeout_seconds: job.timeout_seconds,
            metadata: job.metadata,
            created_at: job.created_at,
        }
    }
}

/// Input type for creating jobs
#[derive(InputObject)]
pub struct CreateJobInput {
    pub task_id: GraphQLApiId,
    pub schedule_id: Option<GraphQLApiId>,
    pub priority: Option<JobPriorityGraphQL>,
    pub input: Option<JsonValue>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub metadata: Option<JsonValue>,
}

/// Input type for job filtering
#[derive(InputObject)]
pub struct JobFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub schedule_id: Option<GraphQLApiId>,
    pub status: Option<JobStatusGraphQL>,
    pub priority: Option<JobPriorityGraphQL>,
    pub scheduled_after: Option<DateTime<Utc>>,
    pub scheduled_before: Option<DateTime<Utc>>,
}

/// Job statistics
#[derive(SimpleObject)]
pub struct JobStats {
    pub total_jobs: i64,
    pub pending_jobs: i64,
    pub running_jobs: i64,
    pub completed_jobs: i64,
    pub failed_jobs: i64,
    pub cancelled_jobs: i64,
    pub average_processing_time_ms: Option<f64>,
}