//! GraphQL type definitions

use async_graphql::SimpleObject;
use ratchet_api_types::pagination::PaginationMeta;

pub mod scalars;
pub mod tasks;
pub mod executions;
pub mod jobs;
pub mod schedules;
pub mod workers;

// Re-export all types
pub use scalars::*;
pub use tasks::*;
pub use executions::*;
pub use jobs::*;
pub use schedules::*;
pub use workers::*;

/// Pagination metadata for GraphQL responses
#[derive(SimpleObject)]
pub struct PaginationMetaGraphQL {
    pub page: i32,
    pub limit: i32,
    pub total: i64,
    pub total_pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
}

impl From<PaginationMeta> for PaginationMetaGraphQL {
    fn from(meta: PaginationMeta) -> Self {
        Self {
            page: meta.page as i32,
            limit: meta.limit as i32,
            total: meta.total as i64,
            total_pages: meta.total_pages as i32,
            has_next: meta.has_next,
            has_previous: meta.has_previous,
        }
    }
}

/// Paginated task response
#[derive(SimpleObject)]
pub struct TaskList {
    pub items: Vec<Task>,
    pub meta: PaginationMetaGraphQL,
}

/// Paginated execution response
#[derive(SimpleObject)]
pub struct ExecutionList {
    pub items: Vec<Execution>,
    pub meta: PaginationMetaGraphQL,
}

/// Paginated job response
#[derive(SimpleObject)]
pub struct JobList {
    pub items: Vec<Job>,
    pub meta: PaginationMetaGraphQL,
}

/// Paginated schedule response
#[derive(SimpleObject)]
pub struct ScheduleList {
    pub items: Vec<Schedule>,
    pub meta: PaginationMetaGraphQL,
}

/// Paginated worker response
#[derive(SimpleObject)]
pub struct WorkerList {
    pub items: Vec<Worker>,
    pub meta: PaginationMetaGraphQL,
}