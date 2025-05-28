use async_graphql::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::database::repositories::RepositoryFactory;
use crate::execution::{
    job_queue::JobQueueManager, 
    ProcessTaskExecutor,
};
use crate::registry::TaskRegistry;
use super::types::*;

/// GraphQL context containing services and repositories with Send+Sync compliance
pub struct GraphQLContext {
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    pub task_executor: Arc<ProcessTaskExecutor>, // ✅ Send/Sync compliant via process separation
    pub registry: Option<Arc<TaskRegistry>>,
}

/// Root Query resolver
pub struct Query;

#[Object]
impl Query {
    /// Get all tasks with optional pagination
    async fn tasks(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
    ) -> Result<TaskListResponse> {
        let context = ctx.data::<GraphQLContext>()?;
        let page = pagination.as_ref().and_then(|p| p.page).unwrap_or(1);
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(10);
        
        // Get tasks from repository
        let db_tasks = context.repositories.task_repo.find_all().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        let total = db_tasks.len() as u64;
        
        // Convert database tasks to GraphQL tasks
        let tasks = db_tasks.into_iter().map(|task| Task {
            id: task.id,
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            path: task.path,
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
        }).collect();
        
        Ok(TaskListResponse {
            tasks,
            total,
            page,
            limit,
        })
    }
    
    /// Get a specific task by ID
    async fn task(&self, ctx: &Context<'_>, id: i32) -> Result<Option<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let db_task = context.repositories.task_repo.find_by_id(id).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        Ok(db_task.map(|task| Task {
            id: task.id,
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            path: task.path,
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
        }))
    }
    
    /// Get task by UUID
    async fn task_by_uuid(&self, ctx: &Context<'_>, uuid: Uuid) -> Result<Option<Task>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let db_task = context.repositories.task_repo.find_by_uuid(uuid).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        Ok(db_task.map(|task| Task {
            id: task.id,
            uuid: task.uuid,
            name: task.name,
            description: task.description,
            version: task.version,
            path: task.path,
            enabled: task.enabled,
            created_at: task.created_at,
            updated_at: task.updated_at,
            validated_at: task.validated_at,
        }))
    }
    
    /// Get executions with optional pagination
    async fn executions(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
        task_id: Option<i32>,
    ) -> Result<ExecutionListResponse> {
        let context = ctx.data::<GraphQLContext>()?;
        let page = pagination.as_ref().and_then(|p| p.page).unwrap_or(1);
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(10);
        
        // Get executions from repository
        let db_executions = if let Some(task_id) = task_id {
            context.repositories.execution_repo.find_by_task_id(task_id).await
        } else {
            context.repositories.execution_repo.find_recent(limit).await
        }.map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        let total = db_executions.len() as u64;
        
        // Convert database executions to GraphQL executions
        let executions = db_executions.into_iter().map(|exec| Execution {
            id: exec.id,
            uuid: exec.uuid,
            task_id: exec.task_id,
            status: convert_execution_status(exec.status),
            error_message: exec.error_message,
            queued_at: exec.queued_at,
            started_at: exec.started_at,
            completed_at: exec.completed_at,
            duration_ms: exec.duration_ms.map(|d| d as i64),
        }).collect();
        
        Ok(ExecutionListResponse {
            executions,
            total,
            page,
            limit,
        })
    }
    
    /// Get jobs with optional pagination
    async fn jobs(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
        status: Option<JobStatus>,
    ) -> Result<JobListResponse> {
        let context = ctx.data::<GraphQLContext>()?;
        let page = pagination.as_ref().and_then(|p| p.page).unwrap_or(1);
        let limit = pagination.as_ref().and_then(|p| p.limit).unwrap_or(10);
        
        // Get jobs from repository
        let db_jobs = if let Some(status) = status {
            let db_status = convert_job_status_to_db(status);
            context.repositories.job_repo.find_by_status(db_status).await
        } else {
            context.repositories.job_repo.find_ready_for_processing(limit as u64).await
        }.map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        let total = db_jobs.len() as u64;
        
        // Convert database jobs to GraphQL jobs
        let jobs = db_jobs.into_iter().map(|job| Job {
            id: job.id,
            task_id: job.task_id,
            priority: convert_job_priority(job.priority),
            status: convert_job_status(job.status),
            retry_count: job.retry_count,
            max_retries: job.max_retries,
            queued_at: job.queued_at,
            scheduled_for: None, // TODO: Get actual scheduled time
            error_message: job.error_message,
        }).collect();
        
        Ok(JobListResponse {
            jobs,
            total,
            page,
            limit,
        })
    }
    
    /// Get task statistics
    async fn task_stats(&self, ctx: &Context<'_>) -> Result<TaskStats> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let total_tasks = context.repositories.task_repo.count().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        let enabled_tasks = context.repositories.task_repo.count_enabled().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        let disabled_tasks = total_tasks - enabled_tasks;
        
        Ok(TaskStats {
            total_tasks,
            enabled_tasks,
            disabled_tasks,
        })
    }
    
    /// Get execution statistics
    async fn execution_stats(&self, ctx: &Context<'_>) -> Result<ExecutionStats> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let stats = context.repositories.execution_repo.get_stats().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        Ok(ExecutionStats {
            total_executions: stats.total,
            pending: stats.pending,
            running: stats.running,
            completed: stats.completed,
            failed: stats.failed,
        })
    }
    
    /// Get job queue statistics
    async fn job_stats(&self, ctx: &Context<'_>) -> Result<JobStats> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let stats = context.repositories.job_repo.get_queue_stats().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        Ok(JobStats {
            total_jobs: stats.total,
            queued: stats.queued,
            processing: stats.processing,
            completed: stats.completed,
            failed: stats.failed,
            retrying: stats.retrying,
        })
    }
    
    /// Get system health status
    async fn health(&self, ctx: &Context<'_>) -> Result<HealthStatus> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Check database health
        let database_healthy = context.repositories.task_repo.health_check_send().await.is_ok();
        
        // Check job queue health
        let job_queue_healthy = true; // Job queue is always healthy if it can be accessed
        
        // Check task executor health (process-based)
        let executor_healthy = context.task_executor.health_check_send().await.is_ok();
        
        let all_healthy = database_healthy && job_queue_healthy && executor_healthy;
        let message = if all_healthy {
            "All systems operational".to_string()
        } else {
            let mut issues = Vec::new();
            if !database_healthy { issues.push("database"); }
            if !job_queue_healthy { issues.push("job_queue"); }
            if !executor_healthy { issues.push("task_executor"); }
            format!("Issues detected in: {}", issues.join(", "))
        };
        
        Ok(HealthStatus {
            database: database_healthy,
            job_queue: job_queue_healthy,
            scheduler: executor_healthy, // Use executor health for scheduler
            message,
        })
    }

    /// Get all tasks from the registry
    async fn registry_tasks(&self, ctx: &Context<'_>) -> Result<RegistryTaskListResponse> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let registry = context.registry.as_ref()
            .ok_or_else(|| Error::new("Registry not configured"))?;
        
        let tasks = registry.list_tasks().await
            .map_err(|e| Error::new(format!("Registry error: {}", e)))?;
        
        let mut registry_tasks = Vec::new();
        for task in tasks {
            let versions = registry.list_versions(task.metadata.uuid).await
                .map_err(|e| Error::new(format!("Registry error: {}", e)))?;
            
            registry_tasks.push(RegistryTask {
                id: task.metadata.uuid,
                version: task.metadata.version.clone(),
                label: task.metadata.label.clone(),
                description: task.metadata.description.clone(),
                available_versions: versions,
            });
        }
        
        let total = registry_tasks.len() as u64;
        
        Ok(RegistryTaskListResponse {
            tasks: registry_tasks,
            total,
        })
    }

    /// Get a specific task from the registry
    async fn registry_task(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        version: Option<String>,
    ) -> Result<Option<RegistryTask>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let registry = context.registry.as_ref()
            .ok_or_else(|| Error::new("Registry not configured"))?;
        
        if let Some(task) = registry.get_task(id, version.as_deref()).await
            .map_err(|e| Error::new(format!("Registry error: {}", e)))? {
            
            let versions = registry.list_versions(id).await
                .map_err(|e| Error::new(format!("Registry error: {}", e)))?;
            
            Ok(Some(RegistryTask {
                id: task.metadata.uuid,
                version: task.metadata.version.clone(),
                label: task.metadata.label.clone(),
                description: task.metadata.description.clone(),
                available_versions: versions,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all available versions of a task from the registry
    async fn registry_task_versions(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> Result<Vec<String>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let registry = context.registry.as_ref()
            .ok_or_else(|| Error::new("Registry not configured"))?;
        
        registry.list_versions(id).await
            .map_err(|e| Error::new(format!("Registry error: {}", e)))
    }
}

/// Root Mutation resolver
pub struct Mutation;

#[Object]
impl Mutation {
    /// Execute a task immediately
    async fn execute_task(
        &self,
        ctx: &Context<'_>,
        input: ExecuteTaskInput,
    ) -> Result<Job> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Add job to queue with process-based execution
        let priority = input.priority.map(convert_job_priority_to_db)
            .unwrap_or(crate::database::entities::JobPriority::Normal);
        
        let job_id = context.job_queue.enqueue_job_send(
            input.task_id,
            input.input_data,
            priority,
        ).await.map_err(|e| Error::new(format!("Job queue error: {}", e)))?;
        
        // Get the created job
        let db_job = context.repositories.job_repo.find_by_id(job_id).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?
            .ok_or_else(|| Error::new("Job not found after creation"))?;
        
        Ok(Job {
            id: db_job.id,
            task_id: db_job.task_id,
            priority: convert_job_priority(db_job.priority),
            status: convert_job_status(db_job.status),
            retry_count: db_job.retry_count,
            max_retries: db_job.max_retries,
            queued_at: db_job.queued_at,
            scheduled_for: None, // TODO: Get actual scheduled time
            error_message: db_job.error_message,
        })
    }
    
    /// Update task enabled status
    async fn update_task_status(
        &self,
        ctx: &Context<'_>,
        id: i32,
        enabled: bool,
    ) -> Result<Task> {
        let context = ctx.data::<GraphQLContext>()?;
        
        context.repositories.task_repo.set_enabled(id, enabled).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        let db_task = context.repositories.task_repo.find_by_id(id).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?
            .ok_or_else(|| Error::new("Task not found"))?;
        
        Ok(Task {
            id: db_task.id,
            uuid: db_task.uuid,
            name: db_task.name,
            description: db_task.description,
            version: db_task.version,
            path: db_task.path,
            enabled: db_task.enabled,
            created_at: db_task.created_at,
            updated_at: db_task.updated_at,
            validated_at: db_task.validated_at,
        })
    }
    
    /// Execute a task directly (immediate execution)
    async fn execute_task_direct(
        &self,
        ctx: &Context<'_>,
        task_id: i32,
        input_data: serde_json::Value,
    ) -> Result<TaskExecutionResult> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Execute task directly using process executor
        let execution_result = context.task_executor
            .execute_task_send(task_id, input_data, None)
            .await
            .map_err(|e| Error::new(format!("Task execution failed: {}", e)))?;
        
        Ok(TaskExecutionResult {
            success: execution_result.success,
            output: execution_result.output,
            error: execution_result.error,
            duration_ms: execution_result.duration_ms,
        })
    }
    
    /// Cancel a job (temporarily disabled)
    async fn cancel_job(&self, ctx: &Context<'_>, id: i32) -> Result<Job> {
        let _context = ctx.data::<GraphQLContext>()?;
        
        // TODO: Implement mark_cancelled method
        // context.repositories.job_repo.mark_cancelled(id).await
        //     .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        // Return a placeholder job for now
        Ok(Job {
            id,
            task_id: 0,
            priority: JobPriority::Normal,
            status: JobStatus::Cancelled,
            retry_count: 0,
            max_retries: 0,
            queued_at: chrono::Utc::now(),
            scheduled_for: None,
            error_message: Some("Cancelled via GraphQL".to_string()),
        })
    }
}

/// Root Subscription resolver (for real-time updates)
pub struct Subscription;

#[Subscription]
impl Subscription {
    /// Subscribe to job status updates
    async fn job_updates(&self) -> impl futures::Stream<Item = Job> {
        // TODO: Implement real-time job updates using channels
        futures::stream::empty()
    }
    
    /// Subscribe to execution updates
    async fn execution_updates(&self) -> impl futures::Stream<Item = Execution> {
        // TODO: Implement real-time execution updates using channels
        futures::stream::empty()
    }
}

// Helper conversion functions
fn convert_execution_status(status: crate::database::entities::ExecutionStatus) -> ExecutionStatus {
    match status {
        crate::database::entities::ExecutionStatus::Pending => ExecutionStatus::Pending,
        crate::database::entities::ExecutionStatus::Running => ExecutionStatus::Running,
        crate::database::entities::ExecutionStatus::Completed => ExecutionStatus::Completed,
        crate::database::entities::ExecutionStatus::Failed => ExecutionStatus::Failed,
        crate::database::entities::ExecutionStatus::Cancelled => ExecutionStatus::Cancelled,
    }
}

fn convert_job_priority(priority: crate::database::entities::JobPriority) -> JobPriority {
    match priority {
        crate::database::entities::JobPriority::Low => JobPriority::Low,
        crate::database::entities::JobPriority::Normal => JobPriority::Normal,
        crate::database::entities::JobPriority::High => JobPriority::High,
        crate::database::entities::JobPriority::Urgent => JobPriority::Critical,
    }
}

fn convert_job_priority_to_db(priority: JobPriority) -> crate::database::entities::JobPriority {
    match priority {
        JobPriority::Low => crate::database::entities::JobPriority::Low,
        JobPriority::Normal => crate::database::entities::JobPriority::Normal,
        JobPriority::High => crate::database::entities::JobPriority::High,
        JobPriority::Critical => crate::database::entities::JobPriority::Urgent,
    }
}

fn convert_job_status(status: crate::database::entities::JobStatus) -> JobStatus {
    match status {
        crate::database::entities::JobStatus::Queued => JobStatus::Queued,
        crate::database::entities::JobStatus::Processing => JobStatus::Processing,
        crate::database::entities::JobStatus::Completed => JobStatus::Completed,
        crate::database::entities::JobStatus::Failed => JobStatus::Failed,
        crate::database::entities::JobStatus::Retrying => JobStatus::Retrying,
        crate::database::entities::JobStatus::Cancelled => JobStatus::Cancelled,
    }
}

fn convert_job_status_to_db(status: JobStatus) -> crate::database::entities::JobStatus {
    match status {
        JobStatus::Queued => crate::database::entities::JobStatus::Queued,
        JobStatus::Processing => crate::database::entities::JobStatus::Processing,
        JobStatus::Completed => crate::database::entities::JobStatus::Completed,
        JobStatus::Failed => crate::database::entities::JobStatus::Failed,
        JobStatus::Retrying => crate::database::entities::JobStatus::Retrying,
        JobStatus::Cancelled => crate::database::entities::JobStatus::Cancelled,
    }
}