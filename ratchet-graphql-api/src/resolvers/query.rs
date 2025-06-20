//! GraphQL query resolvers

use crate::{context::GraphQLContext, types::*};
use async_graphql::{Context, Object, Result};
use ratchet_api_types::{
    pagination::{ListInput, SortInput},
    ApiId,
};
use ratchet_interfaces::{ExecutionFilters, JobFilters, ScheduleFilters, TaskFilters};

/// Root query resolver
pub struct Query;

#[Object]
impl Query {
    /// Get all tasks with optional filtering and sorting
    async fn tasks(
        &self,
        ctx: &Context<'_>,
        filters: Option<TaskFiltersInput>,
        sort: Option<SortInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<TaskList> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();

        // Convert GraphQL filters to domain filters with comprehensive mapping
        let domain_filters = filters
            .map(|f| TaskFilters {
                // Basic filters (existing)
                name: f.name_contains.clone(),
                enabled: f.enabled,
                registry_source: f.registry_source,
                validated_after: f.validated_after,

                // Advanced string filtering
                name_exact: f.name_exact,
                name_contains: f.name_contains,
                name_starts_with: f.name_starts_with,
                name_ends_with: f.name_ends_with,

                // Version filtering
                version: f.version,
                version_in: f.version_in,

                // Extended date filtering
                created_after: f.created_after,
                created_before: f.created_before,
                updated_after: f.updated_after,
                updated_before: f.updated_before,
                validated_before: f.validated_before,

                // ID filtering
                uuid: f.uuid,
                uuid_in: f.uuid_in,
                id_in: f.id_in,

                // Advanced boolean filtering
                has_validation: f.has_validation,
                in_sync: f.in_sync,
            })
            .unwrap_or(TaskFilters {
                name: None,
                enabled: None,
                registry_source: None,
                validated_after: None,
                name_exact: None,
                name_contains: None,
                name_starts_with: None,
                name_ends_with: None,
                version: None,
                version_in: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                validated_before: None,
                uuid: None,
                uuid_in: None,
                id_in: None,
                has_validation: None,
                in_sync: None,
            });

        // Create list input with pagination and sorting
        let list_input = ListInput {
            pagination: Some(ratchet_api_types::PaginationInput {
                page: None,
                limit: Some(limit.unwrap_or(50) as u32),
                offset: Some(offset.unwrap_or(0) as u32),
            }),
            sort,
            filters: None, // Using domain filters instead
        };

        let result = task_repo.find_with_list_input(domain_filters, list_input).await?;
        let items: Vec<Task> = result.items.into_iter().collect();
        let meta = result.meta;
        Ok(TaskList { items, meta })
    }

    /// Get a single task by ID
    async fn task(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();

        let api_id: ApiId = id.into();
        match task_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(task) => Ok(Some(task)),
            None => Ok(None),
        }
    }

    /// Get task statistics
    async fn task_stats(&self, ctx: &Context<'_>) -> Result<TaskStats> {
        let _context = ctx.data::<GraphQLContext>()?;

        // This would be implemented based on your repository interface
        // For now, return placeholder values
        Ok(TaskStats {
            total_tasks: 0,
            enabled_tasks: 0,
            disabled_tasks: 0,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time_ms: None,
        })
    }

    /// Get all executions with optional filtering
    async fn executions(
        &self,
        ctx: &Context<'_>,
        filters: Option<ExecutionFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ExecutionList> {
        let context = ctx.data::<GraphQLContext>()?;
        let execution_repo = context.repositories.execution_repository();

        // Convert GraphQL filters to domain filters with comprehensive mapping
        let domain_filters = filters
            .map(|f| ExecutionFilters {
                // Basic filters (existing)
                task_id: f.task_id.map(|id| id.into()),
                status: f.status,
                queued_after: f.queued_after,
                completed_after: f.completed_after,

                // Advanced ID filtering
                task_id_in: f.task_id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),
                id_in: f.id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),

                // Advanced status filtering
                status_in: f.status_in.map(|statuses| statuses.into_iter().collect()),
                status_not: f.status_not,

                // Extended date filtering
                queued_before: f.queued_before,
                started_after: f.started_after,
                started_before: f.started_before,
                completed_before: f.completed_before,

                // Duration filtering
                duration_min_ms: f.duration_min_ms,
                duration_max_ms: f.duration_max_ms,

                // Progress filtering
                progress_min: f.progress_min,
                progress_max: f.progress_max,
                has_progress: f.has_progress,

                // Error filtering
                has_error: f.has_error,
                error_message_contains: f.error_message_contains,

                // Advanced boolean filtering
                can_retry: f.can_retry,
                can_cancel: f.can_cancel,
            })
            .unwrap_or(ExecutionFilters {
                task_id: None,
                status: None,
                queued_after: None,
                completed_after: None,
                task_id_in: None,
                id_in: None,
                status_in: None,
                status_not: None,
                queued_before: None,
                started_after: None,
                started_before: None,
                completed_before: None,
                duration_min_ms: None,
                duration_max_ms: None,
                progress_min: None,
                progress_max: None,
                has_progress: None,
                has_error: None,
                error_message_contains: None,
                can_retry: None,
                can_cancel: None,
            });

        // Create list input with pagination and sorting
        let list_input = ListInput {
            pagination: Some(ratchet_api_types::PaginationInput {
                page: None,
                limit: Some(limit.unwrap_or(50) as u32),
                offset: Some(offset.unwrap_or(0) as u32),
            }),
            sort: None,    // Can be added later for execution sorting
            filters: None, // Using domain filters instead
        };

        let result = execution_repo.find_with_list_input(domain_filters, list_input).await?;
        let items: Vec<Execution> = result.items.into_iter().collect();
        let meta = result.meta;
        Ok(ExecutionList { items, meta })
    }

    /// Get a single execution by ID
    async fn execution(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Execution>> {
        let context = ctx.data::<GraphQLContext>()?;
        let execution_repo = context.repositories.execution_repository();

        let api_id: ApiId = id.into();
        match execution_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(execution) => Ok(Some(execution)),
            None => Ok(None),
        }
    }

    /// Get all jobs with optional filtering
    async fn jobs(
        &self,
        ctx: &Context<'_>,
        filters: Option<JobFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<JobList> {
        let context = ctx.data::<GraphQLContext>()?;
        let job_repo = context.repositories.job_repository();

        // Convert GraphQL filters to domain filters with comprehensive mapping
        let domain_filters = filters
            .map(|f| JobFilters {
                // Basic filters (existing)
                task_id: f.task_id.map(|id| id.into()),
                status: f.status,
                priority: f.priority,
                queued_after: f.queued_after,
                scheduled_before: f.scheduled_before,

                // Advanced ID filtering
                task_id_in: f.task_id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),
                id_in: f.id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),

                // Advanced status filtering
                status_in: f.status_in.map(|statuses| statuses.into_iter().collect()),
                status_not: f.status_not,

                // Advanced priority filtering
                priority_in: f.priority_in.map(|priorities| priorities.into_iter().collect()),
                priority_min: f.priority_min,

                // Extended date filtering
                queued_before: f.queued_before,
                scheduled_after: f.scheduled_after,

                // Retry filtering
                retry_count_min: f.retry_count_min,
                retry_count_max: f.retry_count_max,
                max_retries_min: f.max_retries_min,
                max_retries_max: f.max_retries_max,
                has_retries_remaining: f.has_retries_remaining,

                // Error filtering
                has_error: f.has_error,
                error_message_contains: f.error_message_contains,

                // Scheduling filtering
                is_scheduled: f.is_scheduled,
                due_now: f.due_now,
            })
            .unwrap_or(JobFilters {
                task_id: None,
                status: None,
                priority: None,
                queued_after: None,
                scheduled_before: None,
                task_id_in: None,
                id_in: None,
                status_in: None,
                status_not: None,
                priority_in: None,
                priority_min: None,
                queued_before: None,
                scheduled_after: None,
                retry_count_min: None,
                retry_count_max: None,
                max_retries_min: None,
                max_retries_max: None,
                has_retries_remaining: None,
                has_error: None,
                error_message_contains: None,
                is_scheduled: None,
                due_now: None,
            });

        // Create list input with pagination and sorting
        let list_input = ListInput {
            pagination: Some(ratchet_api_types::PaginationInput {
                page: None,
                limit: Some(limit.unwrap_or(50) as u32),
                offset: Some(offset.unwrap_or(0) as u32),
            }),
            sort: None,    // Can be added later for job sorting
            filters: None, // Using domain filters instead
        };

        let result = job_repo.find_with_list_input(domain_filters, list_input).await?;
        let items: Vec<Job> = result.items.into_iter().map(|job| job.into()).collect();
        let meta = result.meta;
        Ok(JobList { items, meta })
    }

    /// Get a single job by ID
    async fn job(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Job>> {
        let context = ctx.data::<GraphQLContext>()?;
        let job_repo = context.repositories.job_repository();

        let api_id: ApiId = id.into();
        match job_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(job) => Ok(Some(job.into())),
            None => Ok(None),
        }
    }

    /// Get all schedules with optional filtering
    async fn schedules(
        &self,
        ctx: &Context<'_>,
        filters: Option<ScheduleFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ScheduleList> {
        let context = ctx.data::<GraphQLContext>()?;
        let schedule_repo = context.repositories.schedule_repository();

        // Convert GraphQL filters to domain filters with comprehensive mapping
        let domain_filters = filters
            .map(|f| ScheduleFilters {
                // Basic filters (existing)
                task_id: f.task_id.map(|id| id.into()),
                enabled: f.enabled,
                next_run_before: f.next_run_before,

                // Advanced ID filtering
                task_id_in: f.task_id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),
                id_in: f.id_in.map(|ids| ids.into_iter().map(|id| id.into()).collect()),

                // Name filtering
                name_contains: f.name_contains,
                name_exact: f.name_exact,
                name_starts_with: f.name_starts_with,
                name_ends_with: f.name_ends_with,

                // Cron expression filtering
                cron_expression_contains: f.cron_expression_contains,
                cron_expression_exact: f.cron_expression_exact,

                // Schedule timing filtering
                next_run_after: f.next_run_after,
                last_run_after: f.last_run_after,
                last_run_before: f.last_run_before,

                // Date range filtering
                created_after: f.created_after,
                created_before: f.created_before,
                updated_after: f.updated_after,
                updated_before: f.updated_before,

                // Advanced filtering
                has_next_run: f.has_next_run,
                has_last_run: f.has_last_run,
                is_due: f.is_due,
                overdue: f.overdue,
            })
            .unwrap_or(ScheduleFilters {
                task_id: None,
                enabled: None,
                next_run_before: None,
                task_id_in: None,
                id_in: None,
                name_contains: None,
                name_exact: None,
                name_starts_with: None,
                name_ends_with: None,
                cron_expression_contains: None,
                cron_expression_exact: None,
                next_run_after: None,
                last_run_after: None,
                last_run_before: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                has_next_run: None,
                has_last_run: None,
                is_due: None,
                overdue: None,
            });

        // Create list input with pagination and sorting
        let list_input = ListInput {
            pagination: Some(ratchet_api_types::PaginationInput {
                page: None,
                limit: Some(limit.unwrap_or(50) as u32),
                offset: Some(offset.unwrap_or(0) as u32),
            }),
            sort: None,    // Can be added later for schedule sorting
            filters: None, // Using domain filters instead
        };

        let result = schedule_repo.find_with_list_input(domain_filters, list_input).await?;
        let items: Vec<Schedule> = result.items.into_iter().collect();
        let meta = result.meta;
        Ok(ScheduleList { items, meta })
    }

    /// Get a single schedule by ID
    async fn schedule(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Schedule>> {
        let context = ctx.data::<GraphQLContext>()?;
        let schedule_repo = context.repositories.schedule_repository();

        let api_id: ApiId = id.into();
        match schedule_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(schedule) => Ok(Some(schedule)),
            None => Ok(None),
        }
    }

    /// Get all workers with optional filtering
    async fn workers(
        &self,
        ctx: &Context<'_>,
        _filters: Option<WorkerFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<WorkerList> {
        let _context = ctx.data::<GraphQLContext>()?;

        // For now, return empty list as worker management is not yet implemented
        let pagination = ratchet_api_types::PaginationInput {
            page: None,
            limit: Some(limit.unwrap_or(50) as u32),
            offset: Some(offset.unwrap_or(0) as u32),
        };
        let meta = ratchet_api_types::pagination::PaginationMeta::new(&pagination, 0);
        Ok(WorkerList { items: vec![], meta })
    }

    /// Get worker statistics
    async fn worker_stats(&self, ctx: &Context<'_>) -> Result<WorkerStats> {
        let _context = ctx.data::<GraphQLContext>()?;

        // Return placeholder worker stats
        Ok(WorkerStats {
            total_workers: 0,
            active_workers: 0,
            idle_workers: 0,
            running_workers: 0,
            stopping_workers: 0,
            error_workers: 0,
            total_tasks: 0,
            average_uptime_seconds: None,
            total_memory_usage_mb: None,
        })
    }

    /// Get system health status
    async fn health(&self, ctx: &Context<'_>) -> Result<HealthStatus> {
        let _context = ctx.data::<GraphQLContext>()?;

        // Basic health check - could be enhanced with actual database connectivity checks
        Ok(HealthStatus {
            database: true,
            message: "System is operational".to_string(),
        })
    }
}
