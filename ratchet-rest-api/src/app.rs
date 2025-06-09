//! Main application configuration and router setup

use axum::{
    routing::{get, post},
    Router,
};
use ratchet_interfaces::{RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator};
use ratchet_web::{
    middleware::{cors_layer, error_handler_layer, request_id_layer},
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::{
    context::{TasksContext, ExecutionsContext, JobsContext, SchedulesContext, WorkersContext},
    handlers,
};

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Enable CORS middleware
    pub enable_cors: bool,
    /// Enable request ID tracking
    pub enable_request_id: bool,
    /// Enable request tracing
    pub enable_tracing: bool,
    /// API path prefix
    pub api_prefix: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enable_cors: true,
            enable_request_id: true,
            enable_tracing: true,
            api_prefix: "/api/v1".to_string(),
        }
    }
}

/// Application context containing all dependencies
#[derive(Clone)]
pub struct AppContext {
    pub tasks: TasksContext,
    pub executions: ExecutionsContext,
    pub jobs: JobsContext,
    pub schedules: SchedulesContext,
    pub workers: WorkersContext,
}

impl AppContext {
    pub fn new(
        repositories: Arc<dyn RepositoryFactory>,
        registry: Arc<dyn TaskRegistry>,
        registry_manager: Arc<dyn RegistryManager>,
        validator: Arc<dyn TaskValidator>,
    ) -> Self {
        Self {
            tasks: TasksContext::new(
                repositories.clone(),
                registry,
                registry_manager,
                validator,
            ),
            executions: ExecutionsContext::new(repositories.clone()),
            jobs: JobsContext::new(repositories.clone()),
            schedules: SchedulesContext::new(repositories.clone()),
            workers: WorkersContext::new(),
        }
    }
}

/// Create the complete REST API application
pub fn create_rest_app(context: AppContext, config: AppConfig) -> Router<TasksContext> {
    let app = Router::new()
        // Health endpoints (no prefix)
        .route("/health", get(handlers::health_check))
        .route("/health/detailed", get(handlers::health_check_detailed))
        .route("/ready", get(handlers::readiness_check))
        .route("/live", get(handlers::liveness_check))
        // API routes with prefix
        .nest(&config.api_prefix, create_api_router())
        // Add application context
        .with_state(context.tasks);

    // Add middleware layers (applied in reverse order)
    let mut app = app;
    if config.enable_cors {
        app = app.layer(cors_layer());
    }

    if config.enable_request_id {
        app = app.layer(request_id_layer());
    }

    if config.enable_tracing {
        app = app.layer(TraceLayer::new_for_http());
    }

    app = app.layer(error_handler_layer());

    app
}

/// Create unified API router
fn create_api_router() -> Router<TasksContext> {
    Router::new()
        // Task endpoints
        .route("/tasks", get(handlers::list_tasks).post(handlers::create_task))
        .route("/tasks/stats", get(handlers::get_task_stats))
        .route("/tasks/sync", post(handlers::sync_tasks))
        .route(
            "/tasks/:id",
            get(handlers::get_task)
                .patch(handlers::update_task)
                .delete(handlers::delete_task),
        )
        .route("/tasks/:id/enable", post(handlers::enable_task))
        .route("/tasks/:id/disable", post(handlers::disable_task))
        // Placeholder endpoints
        .route("/executions", get(placeholder_handler))
        .route("/executions/:id", get(placeholder_handler))
        .route("/jobs", get(placeholder_handler))
        .route("/jobs/:id", get(placeholder_handler))
        .route("/schedules", get(placeholder_handler))
        .route("/schedules/:id", get(placeholder_handler))
        .route("/workers", get(placeholder_handler))
        .route("/workers/stats", get(placeholder_handler))
}


/// Placeholder handler for unimplemented endpoints
async fn placeholder_handler() -> axum::response::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "message": "Endpoint not yet implemented",
        "status": "placeholder"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        // This test would require mock implementations
        // For now, just test that the router can be created
        
        // let context = AppContext::new(...); // Would need mock implementations
        // let app = create_rest_app(context, AppConfig::default());
        
        // let response = app
        //     .oneshot(
        //         axum::http::Request::builder()
        //             .uri("/health")
        //             .body(axum::body::Body::empty())
        //             .unwrap(),
        //     )
        //     .await
        //     .unwrap();
        
        // assert_eq!(response.status(), StatusCode::OK);
    }
}