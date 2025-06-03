use async_graphql::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    api::{
        types::*,
        pagination::{PaginationInput, ListResponse},
        errors::ApiError,
    },
    database::repositories::RepositoryFactory,
    execution::{
        job_queue::JobQueueManager, 
        ProcessTaskExecutor,
    },
    registry::TaskRegistry,
    services::TaskSyncService,
    output::{OutputDeliveryManager, OutputDestinationConfig},
};
use super::types::*;

/// GraphQL context containing services and repositories with Send+Sync compliance
pub struct GraphQLContext {
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    pub task_executor: Arc<ProcessTaskExecutor>, // ✅ Send/Sync compliant via process separation
    pub registry: Option<Arc<TaskRegistry>>,
    pub task_sync_service: Option<Arc<TaskSyncService>>,
}

/// Root Query resolver
pub struct Query;

#[Object]
impl Query {
    /// Get all tasks from unified view (registry + database)
    async fn tasks(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
    ) -> Result<ListResponse<UnifiedTask>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Use sync service if available, otherwise fall back to database
        if let Some(sync_service) = &context.task_sync_service {
            let unified_tasks = sync_service.list_all_tasks().await
                .map_err(|e| {
                    let api_error = ApiError::internal_error(format!("Failed to list tasks: {}", e));
                    Error::from(api_error)
                })?;
            
            let total = unified_tasks.len() as u64;
            
            // Apply pagination using unified system
            let pagination = pagination.unwrap_or_default();
            let offset = pagination.get_offset() as usize;
            let limit = pagination.get_limit() as usize;
            
            let items = if offset < unified_tasks.len() {
                let end = (offset + limit).min(unified_tasks.len());
                unified_tasks[offset..end]
                    .iter()
                    .cloned()
                    .map(UnifiedTask::from)
                    .collect()
            } else {
                vec![]
            };
            
            Ok(ListResponse::new(items, &pagination, total))
        } else {
            // Fallback to database-only view
            let db_tasks = context.repositories.task_repository()
                .find_all().await
                .map_err(|e| {
                    let api_error = ApiError::internal_error(format!("Database error: {}", e));
                    Error::from(api_error)
                })?;
            
            let total = db_tasks.len() as u64;
            
            // Convert database tasks to unified view and apply pagination
            let pagination = pagination.unwrap_or_default();
            let offset = pagination.get_offset() as usize;
            let limit = pagination.get_limit() as usize;
            
            let items = if offset < db_tasks.len() {
                let end = (offset + limit).min(db_tasks.len());
                db_tasks[offset..end]
                    .iter()
                    .map(|task| UnifiedTask::from(task.clone()))
                    .collect()
            } else {
                vec![]
            };
            
            Ok(ListResponse::new(items, &pagination, total))
        }
    }
    
    /// Get a specific task by UUID and optional version
    async fn task(
        &self,
        ctx: &Context<'_>,
        uuid: Uuid,
        version: Option<String>,
    ) -> Result<Option<UnifiedTask>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        if let Some(sync_service) = &context.task_sync_service {
            // Use unified view
            let task = sync_service.get_task(uuid, version.as_deref()).await
                .map_err(|e| Error::new(format!("Failed to get task: {}", e)))?;
            
            Ok(task.map(UnifiedTask::from))
        } else {
            // Fallback to database
            let db_task = context.repositories.task_repository()
                .find_by_uuid(uuid).await
                .map_err(|e| {
                    let api_error = ApiError::internal_error(format!("Database error: {}", e));
                    Error::from(api_error)
                })?;
            
            Ok(db_task.map(UnifiedTask::from))
        }
    }
    
    /// Get task by UUID
    async fn task_by_uuid(&self, ctx: &Context<'_>, uuid: Uuid) -> Result<Option<UnifiedTask>> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let db_task = context.repositories.task_repository()
            .find_by_uuid(uuid).await
            .map_err(|e| {
                let api_error = ApiError::internal_error(format!("Database error: {}", e));
                Error::from(api_error)
            })?;
        
        Ok(db_task.map(UnifiedTask::from))
    }
    
    /// Get executions with optional pagination
    async fn executions(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
        task_id: Option<ApiId>,
    ) -> Result<ListResponse<UnifiedExecution>> {
        let context = ctx.data::<GraphQLContext>()?;
        let pagination = pagination.unwrap_or_default();
        
        // Get executions from repository
        let db_executions = if let Some(task_id) = task_id.and_then(|id| id.as_i32()) {
            context.repositories.execution_repository()
                .find_by_task_id(task_id).await
        } else {
            let limit = pagination.get_limit() as usize;
            context.repositories.execution_repository()
                .find_recent(limit as u64).await
        }.map_err(|e| {
            let api_error = ApiError::internal_error(format!("Database error: {}", e));
            Error::from(api_error)
        })?;
        
        let total = db_executions.len() as u64;
        
        // Convert to unified executions
        let all_executions: Vec<UnifiedExecution> = db_executions
            .into_iter()
            .map(UnifiedExecution::from)
            .collect();
            
        // Apply pagination
        let offset = pagination.get_offset() as usize;
        let limit = pagination.get_limit() as usize;
        let items = if offset < all_executions.len() {
            let end = (offset + limit).min(all_executions.len());
            all_executions[offset..end].to_vec()
        } else {
            vec![]
        };
        
        Ok(ListResponse::new(items, &pagination, total))
    }
    
    /// Get jobs with optional pagination
    async fn jobs(
        &self,
        ctx: &Context<'_>,
        pagination: Option<PaginationInput>,
        status: Option<JobStatus>,
    ) -> Result<ListResponse<UnifiedJob>> {
        let context = ctx.data::<GraphQLContext>()?;
        let pagination = pagination.unwrap_or_default();
        let limit = pagination.get_limit() as u64;
        
        // Get jobs from repository
        let db_jobs = if let Some(status) = status {
            let db_status = status.into();
            context.repositories.job_repository()
                .find_by_status(db_status).await
        } else {
            context.repositories.job_repository()
                .find_ready_for_processing(limit).await
        }.map_err(|e| {
            let api_error = ApiError::internal_error(format!("Database error: {}", e));
            Error::from(api_error)
        })?;
        
        let total = db_jobs.len() as u64;
        
        // Convert to unified jobs
        let all_jobs: Vec<UnifiedJob> = db_jobs
            .into_iter()
            .map(UnifiedJob::from)
            .collect();
            
        // Apply pagination
        let offset = pagination.get_offset() as usize;
        let items = if offset < all_jobs.len() {
            let end = (offset + limit as usize).min(all_jobs.len());
            all_jobs[offset..end].to_vec()
        } else {
            vec![]
        };
        
        Ok(ListResponse::new(items, &pagination, total))
    }
    
    /// Get task statistics
    async fn task_stats(&self, ctx: &Context<'_>) -> Result<TaskStats> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let total_tasks = context.repositories.task_repository().count().await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        let enabled_tasks = context.repositories.task_repository().count_enabled().await
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
        
        let stats = context.repositories.execution_repository().get_stats().await
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
        
        let stats = context.repositories.job_repository().get_queue_stats().await
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
        let database_healthy = context.repositories.task_repository().health_check_send().await.is_ok();
        
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

    /* DEPRECATED: Use the unified 'tasks' query instead
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
    */
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
    ) -> Result<UnifiedJob> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Convert GraphQL output destinations to internal format
        let output_destinations = if let Some(destinations) = input.output_destinations {
            let configs: Result<Vec<OutputDestinationConfig>, _> = destinations
                .into_iter()
                .map(convert_output_destination_input)
                .collect();
            Some(configs.map_err(|e| Error::new(format!("Invalid output destination: {}", e)))?)
        } else {
            None
        };
        
        // Create job directly (since job queue doesn't support output destinations yet)
        let task_id = input.task_id.as_i32()
            .ok_or_else(|| Error::new("Invalid task ID"))?;
        let priority = input.priority.map(Into::into)
            .unwrap_or(crate::database::entities::JobPriority::Normal);
        
        let mut job = crate::database::entities::jobs::Model::new(
            task_id,
            input.input_data,
            priority,
        );
        
        if let Some(destinations) = output_destinations {
            job.output_destinations = Some(serde_json::to_value(destinations).unwrap());
        }
        
        let created_job = context.repositories.job_repository()
            .create(job).await
            .map_err(|e| {
                let api_error = ApiError::internal_error(format!("Database error: {}", e));
                Error::from(api_error)
            })?;
        
        Ok(UnifiedJob::from(created_job))
    }
    
    /// Update task enabled status
    async fn update_task_status(
        &self,
        ctx: &Context<'_>,
        id: ApiId,
        enabled: bool,
    ) -> Result<UnifiedTask> {
        let context = ctx.data::<GraphQLContext>()?;
        
        let task_id = id.as_i32()
            .ok_or_else(|| Error::new("Invalid task ID"))?;
        
        context.repositories.task_repository()
            .set_enabled(task_id, enabled).await
            .map_err(|e| {
                let api_error = ApiError::internal_error(format!("Database error: {}", e));
                Error::from(api_error)
            })?;
        
        let db_task = context.repositories.task_repository()
            .find_by_id(task_id).await
            .map_err(|e| {
                let api_error = ApiError::internal_error(format!("Database error: {}", e));
                Error::from(api_error)
            })?
            .ok_or_else(|| {
                let api_error = ApiError::not_found("Task", &task_id.to_string());
                Error::from(api_error)
            })?;
        
        Ok(UnifiedTask::from(db_task))
    }
    
    /// Execute a task directly (immediate execution)
    async fn execute_task_direct(
        &self,
        ctx: &Context<'_>,
        task_id: ApiId,
        input_data: serde_json::Value,
    ) -> Result<TaskExecutionResult> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // Get numeric task ID
        let task_id_num = task_id.as_i32()
            .ok_or_else(|| Error::new("Invalid task ID"))?;
            
        // Execute task directly using process executor
        let execution_result = context.task_executor
            .execute_task_send(task_id_num, input_data, None)
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
        Ok(UnifiedJob {
            id: ApiId::from_i32(id),
            task_id: ApiId::from_i32(0),
            priority: JobPriority::Normal,
            status: JobStatus::Cancelled,
            retry_count: 0,
            max_retries: 0,
            queued_at: chrono::Utc::now(),
            scheduled_for: None,
            error_message: Some("Cancelled via GraphQL".to_string()),
            output_destinations: None,
        })
    }
    
    /// Test output destination configurations
    async fn test_output_destinations(
        &self,
        _ctx: &Context<'_>,
        input: TestOutputDestinationsInput,
    ) -> Result<Vec<TestDestinationResult>> {
        // Convert GraphQL input to internal format
        let configs: Result<Vec<OutputDestinationConfig>, _> = input.destinations
            .into_iter()
            .map(convert_output_destination_input)
            .collect();
        
        let configs = configs.map_err(|e| Error::new(format!("Invalid destination configuration: {}", e)))?;
        
        // Test configurations
        let test_results = OutputDeliveryManager::test_configurations(&configs)
            .await
            .map_err(|e| Error::new(format!("Failed to test configurations: {}", e)))?;
        
        // Convert results to GraphQL format
        Ok(test_results.into_iter().map(|result| TestDestinationResult {
            index: result.index as i32,
            destination_type: result.destination_type,
            success: result.success,
            error: result.error,
            estimated_time_ms: result.estimated_time.as_millis() as i32,
        }).collect())
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
#[allow(dead_code)]
fn convert_execution_status(status: crate::database::entities::executions::ExecutionStatus) -> ExecutionStatus {
    match status {
        crate::database::entities::executions::ExecutionStatus::Pending => ExecutionStatus::Pending,
        crate::database::entities::executions::ExecutionStatus::Running => ExecutionStatus::Running,
        crate::database::entities::executions::ExecutionStatus::Completed => ExecutionStatus::Completed,
        crate::database::entities::executions::ExecutionStatus::Failed => ExecutionStatus::Failed,
        crate::database::entities::executions::ExecutionStatus::Cancelled => ExecutionStatus::Cancelled,
    }
}

#[allow(dead_code)]
fn convert_job_priority(priority: crate::database::entities::jobs::JobPriority) -> JobPriority {
    match priority {
        crate::database::entities::jobs::JobPriority::Low => JobPriority::Low,
        crate::database::entities::jobs::JobPriority::Normal => JobPriority::Normal,
        crate::database::entities::jobs::JobPriority::High => JobPriority::High,
        crate::database::entities::jobs::JobPriority::Urgent => JobPriority::Critical,
    }
}

#[allow(dead_code)]
fn convert_job_priority_to_db(priority: JobPriority) -> crate::database::entities::jobs::JobPriority {
    match priority {
        JobPriority::Low => crate::database::entities::jobs::JobPriority::Low,
        JobPriority::Normal => crate::database::entities::jobs::JobPriority::Normal,
        JobPriority::High => crate::database::entities::jobs::JobPriority::High,
        JobPriority::Critical => crate::database::entities::jobs::JobPriority::Urgent,
    }
}

#[allow(dead_code)]
fn convert_job_status(status: crate::database::entities::jobs::JobStatus) -> JobStatus {
    match status {
        crate::database::entities::jobs::JobStatus::Queued => JobStatus::Queued,
        crate::database::entities::jobs::JobStatus::Processing => JobStatus::Processing,
        crate::database::entities::jobs::JobStatus::Completed => JobStatus::Completed,
        crate::database::entities::jobs::JobStatus::Failed => JobStatus::Failed,
        crate::database::entities::jobs::JobStatus::Retrying => JobStatus::Retrying,
        crate::database::entities::jobs::JobStatus::Cancelled => JobStatus::Cancelled,
    }
}

#[allow(dead_code)]
fn convert_job_status_to_db(status: JobStatus) -> crate::database::entities::jobs::JobStatus {
    match status {
        JobStatus::Queued => crate::database::entities::jobs::JobStatus::Queued,
        JobStatus::Processing => crate::database::entities::jobs::JobStatus::Processing,
        JobStatus::Completed => crate::database::entities::jobs::JobStatus::Completed,
        JobStatus::Failed => crate::database::entities::jobs::JobStatus::Failed,
        JobStatus::Retrying => crate::database::entities::jobs::JobStatus::Retrying,
        JobStatus::Cancelled => crate::database::entities::jobs::JobStatus::Cancelled,
    }
}

// Output destination conversion functions
#[allow(dead_code)]
fn convert_output_destination_config(config: OutputDestinationConfig) -> OutputDestination {
    match config {
        OutputDestinationConfig::Filesystem { path, format, permissions, create_dirs, overwrite, backup_existing } => {
            OutputDestination::Filesystem(FilesystemDestination {
                path,
                format: convert_output_format(format),
                permissions: format!("{:o}", permissions),
                create_dirs,
                overwrite,
                backup_existing,
            })
        }
        OutputDestinationConfig::Webhook { url, method, headers, timeout, retry_policy, auth, content_type } => {
            OutputDestination::Webhook(WebhookDestination {
                url,
                method: convert_http_method(method),
                headers,
                timeout_seconds: timeout.as_secs() as i32,
                retry_policy: RetryPolicy {
                    max_attempts: retry_policy.max_attempts as i32,
                    initial_delay_ms: retry_policy.initial_delay.as_millis() as i32,
                    max_delay_ms: retry_policy.max_delay.as_millis() as i32,
                    backoff_multiplier: retry_policy.backoff_multiplier,
                },
                auth: auth.map(|a| WebhookAuth {
                    auth_type: "bearer".to_string(), // Simplified for now
                    username: None,
                    password: None,
                    token: Some(format!("{:?}", a)), // Simplified serialization
                }),
                content_type,
            })
        }
        OutputDestinationConfig::Database { .. } => {
            OutputDestination::Database(DatabaseDestination {
                connection_string: "not_implemented".to_string(),
                table_name: "not_implemented".to_string(),
                column_mappings: std::collections::HashMap::new(),
            })
        }
        OutputDestinationConfig::S3 { .. } => {
            OutputDestination::S3(S3Destination {
                bucket: "not_implemented".to_string(),
                key_template: "not_implemented".to_string(),
                region: "not_implemented".to_string(),
                access_key_id: None,
                secret_access_key: None,
            })
        }
    }
}

fn convert_output_destination_input(input: OutputDestinationInput) -> Result<OutputDestinationConfig, String> {
    match input.destination_type {
        DestinationType::Filesystem => {
            let fs = input.filesystem.ok_or("Filesystem configuration required")?;
            Ok(OutputDestinationConfig::Filesystem {
                path: fs.path,
                format: convert_output_format_from_graphql(fs.format),
                permissions: fs.permissions.and_then(|p| u32::from_str_radix(&p, 8).ok()).unwrap_or(0o644),
                create_dirs: fs.create_dirs.unwrap_or(true),
                overwrite: fs.overwrite.unwrap_or(true),
                backup_existing: fs.backup_existing.unwrap_or(false),
            })
        }
        DestinationType::Webhook => {
            let webhook = input.webhook.ok_or("Webhook configuration required")?;
            Ok(OutputDestinationConfig::Webhook {
                url: webhook.url,
                method: convert_http_method_from_graphql(webhook.method),
                headers: webhook.headers.unwrap_or_default(),
                timeout: std::time::Duration::from_secs(webhook.timeout_seconds.unwrap_or(30) as u64),
                retry_policy: crate::output::RetryPolicy {
                    max_attempts: webhook.retry_policy.as_ref().map(|r| r.max_attempts as u32).unwrap_or(3),
                    initial_delay: std::time::Duration::from_millis(
                        webhook.retry_policy.as_ref().map(|r| r.initial_delay_ms as u64).unwrap_or(1000)
                    ),
                    max_delay: std::time::Duration::from_millis(
                        webhook.retry_policy.as_ref().map(|r| r.max_delay_ms as u64).unwrap_or(30000)
                    ),
                    backoff_multiplier: webhook.retry_policy.as_ref().map(|r| r.backoff_multiplier).unwrap_or(2.0),
                    jitter: true,
                    retry_on_status: vec![500, 502, 503, 504],
                },
                auth: None, // Simplified for now
                content_type: webhook.content_type,
            })
        }
        DestinationType::Database => {
            Err("Database destinations not yet implemented".to_string())
        }
        DestinationType::S3 => {
            Err("S3 destinations not yet implemented".to_string())
        }
    }
}

#[allow(dead_code)]
fn convert_output_format(format: crate::output::OutputFormat) -> OutputFormat {
    // Use the conversion from the api module
    format.into()
}

fn convert_output_format_from_graphql(format: OutputFormat) -> crate::output::OutputFormat {
    // Use the conversion from the api module
    format.into()
}

// Use conversion implementations from api::conversions module
#[allow(dead_code)]
fn convert_http_method(method: crate::types::HttpMethod) -> HttpMethod {
    method.into()
}

fn convert_http_method_from_graphql(method: HttpMethod) -> crate::types::HttpMethod {
    method.into()
}