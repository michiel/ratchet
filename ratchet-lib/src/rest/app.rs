use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use std::sync::Arc;
use tower_http::{
    trace::TraceLayer,
    // timeout::TimeoutLayer,
    // limit::RequestBodyLimitLayer,
    // compression::CompressionLayer,
};

use crate::{
    database::repositories::RepositoryFactory,
    execution::{JobQueueManager, ProcessTaskExecutor},
    registry::TaskRegistry,
    services::TaskSyncService,
    rest::{
        handlers::{
            tasks::{TasksContext, list_tasks, get_task, update_task, create_task, delete_task},
            executions::{ExecutionsContext, list_executions, get_execution, update_execution, create_execution, delete_execution, retry_execution, cancel_execution},
            jobs::{JobsContext, list_jobs, get_job, create_job, update_job, delete_job, cancel_job, retry_job, get_queue_stats, test_output_destinations},
            schedules::{SchedulesContext, list_schedules, get_schedule, create_schedule, update_schedule, delete_schedule, enable_schedule, disable_schedule, trigger_schedule},
            workers::{WorkersContext, list_workers, get_worker, get_worker_pool_stats},
        },
        middleware::{cors_layer, request_id_middleware, rate_limit_middleware_with_state, RateLimitConfig, create_rate_limit_layer},
    },
};

/// REST API application state
#[derive(Clone)]
pub struct RestApiState {
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    pub task_executor: Arc<ProcessTaskExecutor>,
    pub registry: Option<Arc<TaskRegistry>>,
    pub sync_service: Option<Arc<TaskSyncService>>,
}

/// Create the REST API application
pub fn create_rest_app(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    _task_executor: Arc<ProcessTaskExecutor>,
    registry: Option<Arc<TaskRegistry>>,
    sync_service: Option<Arc<TaskSyncService>>,
) -> Router {
    create_rest_app_with_rate_limit(repositories, job_queue, _task_executor, registry, sync_service, None)
}

/// Create the REST API application with optional rate limiting
pub fn create_rest_app_with_rate_limit(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    _task_executor: Arc<ProcessTaskExecutor>,
    registry: Option<Arc<TaskRegistry>>,
    sync_service: Option<Arc<TaskSyncService>>,
    rate_limit_config: Option<RateLimitConfig>,
) -> Router {
    let tasks_context = TasksContext {
        sync_service: sync_service.clone(),
        registry: registry.clone(),
    };

    let executions_context = ExecutionsContext::new(repositories.clone(), job_queue.clone());
    
    let jobs_context = JobsContext {
        repository: repositories.clone(),
    };
    
    let schedules_context = SchedulesContext {
        repository: repositories.clone(),
    };
    
    let workers_context = WorkersContext {};

    let tasks_router = Router::new()
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task).patch(update_task).delete(delete_task))
        .with_state(tasks_context.clone());
        
    let executions_router = Router::new()
        .route("/executions", get(list_executions).post(create_execution))
        .route("/executions/:id", get(get_execution).patch(update_execution).delete(delete_execution))
        .route("/executions/:id/retry", post(retry_execution))
        .route("/executions/:id/cancel", post(cancel_execution))
        .with_state(executions_context.clone());
        
    let jobs_router = Router::new()
        .route("/jobs", get(list_jobs).post(create_job))
        .route("/jobs/stats", get(get_queue_stats))
        .route("/jobs/test-output-destinations", post(test_output_destinations))
        .route("/jobs/:id", get(get_job).patch(update_job).delete(delete_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/jobs/:id/retry", post(retry_job))
        .with_state(jobs_context.clone());
        
    let schedules_router = Router::new()
        .route("/schedules", get(list_schedules).post(create_schedule))
        .route("/schedules/:id", get(get_schedule).patch(update_schedule).delete(delete_schedule))
        .route("/schedules/:id/enable", post(enable_schedule))
        .route("/schedules/:id/disable", post(disable_schedule))
        .route("/schedules/:id/trigger", post(trigger_schedule))
        .with_state(schedules_context.clone());
        
    let workers_router = Router::new()
        .route("/workers", get(list_workers))
        .route("/workers/stats", get(get_worker_pool_stats))
        .route("/workers/:id", get(get_worker))
        .with_state(workers_context.clone());

    let mut app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        
        // Merge all sub-routers
        .merge(tasks_router)
        .merge(executions_router)
        .merge(jobs_router)
        .merge(schedules_router)
        .merge(workers_router)
        
        // Add middleware in correct order (inner to outer)
        .layer(middleware::from_fn(request_id_middleware));
        
    // Add rate limiting if configured
    if let Some(config) = rate_limit_config {
        let rate_limiter = create_rate_limit_layer(config);
        app = app.layer(middleware::from_fn(move |headers, connect_info, request, next| {
            rate_limit_middleware_with_state(rate_limiter.clone(), headers, connect_info, request, next)
        }));
    }
        
    app
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
}

/// Health check endpoint for the REST API
pub async fn health_check() -> &'static str {
    "OK"
}