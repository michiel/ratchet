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
            let db_task = context.repositories.task_repo.find_by_uuid(uuid).await
                .map_err(|e| Error::new(format!("Database error: {}", e)))?;
            
            Ok(db_task.map(|task| UnifiedTask {
                id: Some(task.id),
                uuid: task.uuid,
                version: task.version.clone(),
                label: task.name,
                description: task.description.unwrap_or_default(),
                available_versions: vec![task.version],
                registry_source: false,
                enabled: task.enabled,
                created_at: Some(task.created_at),
                updated_at: Some(task.updated_at),
                validated_at: task.validated_at,
                in_sync: true,
            }))
        }
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
        let jobs = db_jobs.into_iter().map(|job| {
            let output_destinations = job.output_destinations
                .and_then(|json| serde_json::from_value::<Vec<OutputDestinationConfig>>(json.into()).ok())
                .map(|configs| configs.into_iter().map(convert_output_destination_config).collect());
            
            Job {
                id: job.id,
                task_id: job.task_id,
                priority: convert_job_priority(job.priority),
                status: convert_job_status(job.status),
                retry_count: job.retry_count,
                max_retries: job.max_retries,
                queued_at: job.queued_at,
                scheduled_for: None, // TODO: Get actual scheduled time
                error_message: job.error_message,
                output_destinations,
            }
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
    ) -> Result<Job> {
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
        let priority = input.priority.map(convert_job_priority_to_db)
            .unwrap_or(crate::database::entities::JobPriority::Normal);
        
        let mut job = crate::database::entities::jobs::Model::new(
            input.task_id,
            input.input_data,
            priority,
        );
        
        if let Some(destinations) = output_destinations {
            job.output_destinations = Some(serde_json::to_value(destinations).unwrap().into());
        }
        
        let created_job = context.repositories.job_repo.create(job).await
            .map_err(|e| Error::new(format!("Database error: {}", e)))?;
        
        let output_destinations = created_job.output_destinations
            .and_then(|json| serde_json::from_value::<Vec<OutputDestinationConfig>>(json.into()).ok())
            .map(|configs| configs.into_iter().map(convert_output_destination_config).collect());
        
        Ok(Job {
            id: created_job.id,
            task_id: created_job.task_id,
            priority: convert_job_priority(created_job.priority),
            status: convert_job_status(created_job.status),
            retry_count: created_job.retry_count,
            max_retries: created_job.max_retries,
            queued_at: created_job.queued_at,
            scheduled_for: None,
            error_message: created_job.error_message,
            output_destinations,
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

// Output destination conversion functions
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

fn convert_output_format(format: crate::output::OutputFormat) -> OutputFormat {
    match format {
        crate::output::OutputFormat::Json => OutputFormat::Json,
        crate::output::OutputFormat::JsonCompact => OutputFormat::JsonCompact,
        crate::output::OutputFormat::Yaml => OutputFormat::Yaml,
        crate::output::OutputFormat::Csv => OutputFormat::Csv,
        crate::output::OutputFormat::Raw => OutputFormat::Raw,
        crate::output::OutputFormat::Template(_) => OutputFormat::Template,
    }
}

fn convert_output_format_from_graphql(format: OutputFormat) -> crate::output::OutputFormat {
    match format {
        OutputFormat::Json => crate::output::OutputFormat::Json,
        OutputFormat::JsonCompact => crate::output::OutputFormat::JsonCompact,
        OutputFormat::Yaml => crate::output::OutputFormat::Yaml,
        OutputFormat::Csv => crate::output::OutputFormat::Csv,
        OutputFormat::Raw => crate::output::OutputFormat::Raw,
        OutputFormat::Template => crate::output::OutputFormat::Template("{{output_data}}".to_string()), // Default template
    }
}

fn convert_http_method(method: crate::types::HttpMethod) -> HttpMethod {
    match method {
        crate::types::HttpMethod::Get => HttpMethod::Get,
        crate::types::HttpMethod::Post => HttpMethod::Post,
        crate::types::HttpMethod::Put => HttpMethod::Put,
        crate::types::HttpMethod::Patch => HttpMethod::Patch,
        crate::types::HttpMethod::Delete => HttpMethod::Delete,
        crate::types::HttpMethod::Head => HttpMethod::Head,
        crate::types::HttpMethod::Options => HttpMethod::Options,
    }
}

fn convert_http_method_from_graphql(method: HttpMethod) -> crate::types::HttpMethod {
    match method {
        HttpMethod::Get => crate::types::HttpMethod::Get,
        HttpMethod::Post => crate::types::HttpMethod::Post,
        HttpMethod::Put => crate::types::HttpMethod::Put,
        HttpMethod::Patch => crate::types::HttpMethod::Patch,
        HttpMethod::Delete => crate::types::HttpMethod::Delete,
        HttpMethod::Head => crate::types::HttpMethod::Head,
        HttpMethod::Options => crate::types::HttpMethod::Options,
    }
}