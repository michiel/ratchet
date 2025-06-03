//! REST API route definitions

use axum::{
    Router,
    routing::{get, post, put, delete},
};
use std::sync::Arc;

use super::handlers;
use super::handlers::health::AppState;

/// Create all API routes
pub fn create_api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        
        // Authentication endpoints
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/me", get(handlers::auth::me))
        .route("/api/v1/auth/profile", get(handlers::auth::profile))
        .route("/api/v1/auth/admin", get(handlers::auth::admin_only))
        .route("/api/v1/auth/api-key", get(handlers::auth::api_key_protected))
        .route("/api/v1/auth/flexible", get(handlers::auth::flexible_auth))
        .route("/api/v1/auth/public", get(handlers::auth::public_info))
        .route("/api/v1/auth/health", get(handlers::auth::health_with_auth))
        
        // Task endpoints
        .route("/api/v1/tasks", get(handlers::tasks::list_tasks))
        .route("/api/v1/tasks", post(handlers::tasks::create_task))
        .route("/api/v1/tasks/:id", get(handlers::tasks::get_task))
        .route("/api/v1/tasks/:id", put(handlers::tasks::update_task))
        .route("/api/v1/tasks/:id", delete(handlers::tasks::delete_task))
        .route("/api/v1/tasks/:id/execute", post(handlers::tasks::execute_task))
        
        // Execution endpoints
        .route("/api/v1/executions", get(handlers::executions::list_executions))
        .route("/api/v1/executions/:id", get(handlers::executions::get_execution))
        .route("/api/v1/executions/:id/cancel", post(handlers::executions::cancel_execution))
        .route("/api/v1/executions/:id/logs", get(handlers::executions::get_execution_logs))
        
        // Job endpoints
        .route("/api/v1/jobs", get(handlers::jobs::list_jobs))
        .route("/api/v1/jobs", post(handlers::jobs::create_job))
        .route("/api/v1/jobs/:id", get(handlers::jobs::get_job))
        .route("/api/v1/jobs/:id", put(handlers::jobs::update_job))
        .route("/api/v1/jobs/:id", delete(handlers::jobs::delete_job))
        .route("/api/v1/jobs/:id/pause", post(handlers::jobs::pause_job))
        .route("/api/v1/jobs/:id/resume", post(handlers::jobs::resume_job))
        
        // Schedule endpoints
        .route("/api/v1/schedules", get(handlers::schedules::list_schedules))
        .route("/api/v1/schedules", post(handlers::schedules::create_schedule))
        .route("/api/v1/schedules/:id", get(handlers::schedules::get_schedule))
        .route("/api/v1/schedules/:id", put(handlers::schedules::update_schedule))
        .route("/api/v1/schedules/:id", delete(handlers::schedules::delete_schedule))
        .route("/api/v1/schedules/:id/pause", post(handlers::schedules::pause_schedule))
        .route("/api/v1/schedules/:id/resume", post(handlers::schedules::resume_schedule))
        
        // Worker endpoints
        .route("/api/v1/workers", get(handlers::workers::list_workers))
        .route("/api/v1/workers/:id", get(handlers::workers::get_worker))
        .route("/api/v1/workers/:id/stats", get(handlers::workers::get_worker_metrics))
}