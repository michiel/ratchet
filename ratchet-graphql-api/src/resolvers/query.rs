//! GraphQL query resolvers

use async_graphql::{Object, Context, Result};
use ratchet_interfaces::{TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters};
use crate::{
    context::GraphQLContext,
    types::*,
    errors::ApiError,
};

/// Root query resolver
pub struct Query;

#[Object]
impl Query {
    /// Get all tasks with optional filtering
    async fn tasks(
        &self,
        ctx: &Context<'_>,
        filters: Option<TaskFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| TaskFilters {
            enabled: f.enabled,
            registry_source: f.registry_source,
            in_sync: f.in_sync,
            name_contains: f.name_contains,
            created_after: f.created_after,
            created_before: f.created_before,
        }).unwrap_or_default();
        
        let unified_tasks = task_repo.find_filtered(&domain_filters, limit, offset).await?;
        Ok(unified_tasks.into_iter().map(|task| task.into()).collect())
    }

    /// Get a single task by ID
    async fn task(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();
        
        match task_repo.find_by_id(id.into()).await? {
            Some(task) => Ok(Some(task.into())),
            None => Ok(None),
        }
    }

    /// Get task statistics
    async fn task_stats(&self, ctx: &Context<'_>) -> Result<TaskStats> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();
        
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
    ) -> Result<Vec<Execution>> {
        let context = ctx.data::<GraphQLContext>()?;
        let execution_repo = context.repositories.execution_repository();
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| ExecutionFilters {
            task_id: f.task_id.map(|id| id.into()),
            job_id: f.job_id.map(|id| id.into()),
            status: f.status.map(|s| s.into()),
            worker_id: f.worker_id,
            started_after: f.started_after,
            started_before: f.started_before,
        }).unwrap_or_default();
        
        let unified_executions = execution_repo.find_filtered(&domain_filters, limit, offset).await?;
        Ok(unified_executions.into_iter().map(|exec| exec.into()).collect())
    }

    /// Get a single execution by ID
    async fn execution(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Execution>> {
        let context = ctx.data::<GraphQLContext>()?;
        let execution_repo = context.repositories.execution_repository();
        
        match execution_repo.find_by_id(id.into()).await? {
            Some(execution) => Ok(Some(execution.into())),
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
    ) -> Result<Vec<Job>> {
        let context = ctx.data::<GraphQLContext>()?;
        let job_repo = context.repositories.job_repository();
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| JobFilters {
            task_id: f.task_id.map(|id| id.into()),
            schedule_id: f.schedule_id.map(|id| id.into()),
            status: f.status.map(|s| s.into()),
            priority: f.priority.map(|p| p.into()),
            scheduled_after: f.scheduled_after,
            scheduled_before: f.scheduled_before,
        }).unwrap_or_default();
        
        let unified_jobs = job_repo.find_filtered(&domain_filters, limit, offset).await?;
        Ok(unified_jobs.into_iter().map(|job| job.into()).collect())
    }

    /// Get a single job by ID
    async fn job(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Job>> {
        let context = ctx.data::<GraphQLContext>()?;
        let job_repo = context.repositories.job_repository();
        
        match job_repo.find_by_id(id.into()).await? {
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
    ) -> Result<Vec<Schedule>> {
        let context = ctx.data::<GraphQLContext>()?;
        let schedule_repo = context.repositories.schedule_repository();
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| ScheduleFilters {
            task_id: f.task_id.map(|id| id.into()),
            enabled: f.enabled,
            name_contains: f.name_contains,
            created_after: f.created_after,
            created_before: f.created_before,
        }).unwrap_or_default();
        
        let unified_schedules = schedule_repo.find_filtered(&domain_filters, limit, offset).await?;
        Ok(unified_schedules.into_iter().map(|schedule| schedule.into()).collect())
    }

    /// Get a single schedule by ID
    async fn schedule(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Schedule>> {
        let context = ctx.data::<GraphQLContext>()?;
        let schedule_repo = context.repositories.schedule_repository();
        
        match schedule_repo.find_by_id(id.into()).await? {
            Some(schedule) => Ok(Some(schedule.into())),
            None => Ok(None),
        }
    }

    /// Get all workers with optional filtering
    async fn workers(
        &self,
        ctx: &Context<'_>,
        filters: Option<WorkerFiltersInput>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Worker>> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // For now, return empty list as worker management is not yet implemented
        Ok(vec![])
    }

    /// Get worker statistics
    async fn worker_stats(&self, ctx: &Context<'_>) -> Result<WorkerStats> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // Return placeholder worker stats
        Ok(WorkerStats {
            total_workers: 0,
            active_workers: 0,
            idle_workers: 0,
            busy_workers: 0,
            offline_workers: 0,
            error_workers: 0,
            total_executions: 0,
            average_execution_time_ms: None,
            throughput_per_hour: None,
        })
    }
}