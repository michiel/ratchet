//! GraphQL query resolvers

use async_graphql::{Object, Context, Result};
use ratchet_interfaces::{TaskFilters, ExecutionFilters, JobFilters, ScheduleFilters};
use ratchet_api_types::{ApiId, pagination::{SortInput, ListInput}};
use crate::{
    context::GraphQLContext,
    types::*,
};

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
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| TaskFilters {
            name: f.name_contains,
            enabled: f.enabled,
            registry_source: f.registry_source,
            validated_after: f.created_after,
        }).unwrap_or(TaskFilters {
            name: None,
            enabled: None,
            registry_source: None,
            validated_after: None,
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
        let items: Vec<Task> = result.items.into_iter().map(|task| task.into()).collect();
        let meta = result.meta.into();
        Ok(TaskList { items, meta })
    }

    /// Get a single task by ID
    async fn task(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        let task_repo = context.repositories.task_repository();
        
        let api_id: ApiId = id.into();
        match task_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(task) => Ok(Some(task.into())),
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
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| ExecutionFilters {
            task_id: f.task_id.map(|id| id.into()),
            status: f.status.map(|s| s.into()),
            queued_after: f.queued_after,
            completed_after: f.completed_after,
        }).unwrap_or(ExecutionFilters {
            task_id: None,
            status: None,
            queued_after: None,
            completed_after: None,
        });
        
        let pagination = ratchet_api_types::PaginationInput { 
            page: None,
            limit: Some(limit.unwrap_or(50) as u32), 
            offset: Some(offset.unwrap_or(0) as u32),
        };
        let result = execution_repo.find_with_filters(domain_filters, pagination).await?;
        let items: Vec<Execution> = result.items.into_iter().map(|exec| exec.into()).collect();
        let meta = result.meta.into();
        Ok(ExecutionList { items, meta })
    }

    /// Get a single execution by ID
    async fn execution(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Execution>> {
        let context = ctx.data::<GraphQLContext>()?;
        let execution_repo = context.repositories.execution_repository();
        
        let api_id: ApiId = id.into();
        match execution_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
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
    ) -> Result<JobList> {
        let context = ctx.data::<GraphQLContext>()?;
        let job_repo = context.repositories.job_repository();
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| JobFilters {
            task_id: f.task_id.map(|id| id.into()),
            status: f.status.map(|s| s.into()),
            priority: f.priority.map(|p| p.into()),
            queued_after: f.queued_after,
            scheduled_before: f.scheduled_before,
        }).unwrap_or(JobFilters {
            task_id: None,
            status: None,
            priority: None,
            queued_after: None,
            scheduled_before: None,
        });
        
        let pagination = ratchet_api_types::PaginationInput { 
            page: None,
            limit: Some(limit.unwrap_or(50) as u32), 
            offset: Some(offset.unwrap_or(0) as u32),
        };
        let result = job_repo.find_with_filters(domain_filters, pagination).await?;
        let items: Vec<Job> = result.items.into_iter().map(|job| job.into()).collect();
        let meta = result.meta.into();
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
        
        // Convert GraphQL filters to domain filters
        let domain_filters = filters.map(|f| ScheduleFilters {
            task_id: f.task_id.map(|id| id.into()),
            enabled: f.enabled,
            next_run_before: f.next_run_before,
        }).unwrap_or(ScheduleFilters {
            task_id: None,
            enabled: None,
            next_run_before: None,
        });
        
        let pagination = ratchet_api_types::PaginationInput { 
            page: None,
            limit: Some(limit.unwrap_or(50) as u32), 
            offset: Some(offset.unwrap_or(0) as u32),
        };
        let result = schedule_repo.find_with_filters(domain_filters, pagination).await?;
        let items: Vec<Schedule> = result.items.into_iter().map(|schedule| schedule.into()).collect();
        let meta = result.meta.into();
        Ok(ScheduleList { items, meta })
    }

    /// Get a single schedule by ID
    async fn schedule(&self, ctx: &Context<'_>, id: GraphQLApiId) -> Result<Option<Schedule>> {
        let context = ctx.data::<GraphQLContext>()?;
        let schedule_repo = context.repositories.schedule_repository();
        
        let api_id: ApiId = id.into();
        match schedule_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await? {
            Some(schedule) => Ok(Some(schedule.into())),
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
        Ok(WorkerList { 
            items: vec![], 
            meta: meta.into() 
        })
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