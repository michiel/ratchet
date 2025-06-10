//! GraphQL types for jobs

use async_graphql::{SimpleObject, InputObject, Enum};
use ratchet_api_types::{UnifiedJob, JobStatus, JobPriority};
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};

/// GraphQL Job type
#[derive(SimpleObject, Clone)]
pub struct Job {
    pub id: GraphQLApiId,
    pub task_id: GraphQLApiId,
    pub priority: JobPriorityGraphQL,
    pub status: JobStatusGraphQL,
    pub retry_count: i32,
    pub max_retries: i32,
    pub queued_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

/// GraphQL enum for job status
#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum JobStatusGraphQL {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

impl From<JobStatus> for JobStatusGraphQL {
    fn from(status: JobStatus) -> Self {
        match status {
            JobStatus::Queued => JobStatusGraphQL::Queued,
            JobStatus::Processing => JobStatusGraphQL::Processing,
            JobStatus::Completed => JobStatusGraphQL::Completed,
            JobStatus::Failed => JobStatusGraphQL::Failed,
            JobStatus::Cancelled => JobStatusGraphQL::Cancelled,
            JobStatus::Retrying => JobStatusGraphQL::Retrying,
        }
    }
}

impl From<JobStatusGraphQL> for JobStatus {
    fn from(status: JobStatusGraphQL) -> Self {
        match status {
            JobStatusGraphQL::Queued => JobStatus::Queued,
            JobStatusGraphQL::Processing => JobStatus::Processing,
            JobStatusGraphQL::Completed => JobStatus::Completed,
            JobStatusGraphQL::Failed => JobStatus::Failed,
            JobStatusGraphQL::Cancelled => JobStatus::Cancelled,
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
            priority: job.priority.into(),
            status: job.status.into(),
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            queued_at: job.queued_at,
            scheduled_for: job.scheduled_for,
            error_message: job.error_message,
        }
    }
}

/// Input type for creating jobs
#[derive(InputObject)]
pub struct CreateJobInput {
    pub task_id: GraphQLApiId,
    pub priority: Option<JobPriorityGraphQL>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub max_retries: Option<i32>,
}

/// Input type for job filtering
#[derive(InputObject)]
pub struct JobFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub status: Option<JobStatusGraphQL>,
    pub priority: Option<JobPriorityGraphQL>,
    pub queued_after: Option<DateTime<Utc>>,
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