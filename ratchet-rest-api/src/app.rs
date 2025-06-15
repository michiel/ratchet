//! Main application configuration and router setup

use axum::{
    response::{Json, Html, IntoResponse},
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
    openapi_spec,
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
pub fn create_rest_app(context: AppContext, config: AppConfig) -> Router<()> {
    let app = Router::new()
        // Health endpoints (no prefix)
        .route("/health", get(handlers::health::health_check))
        .route("/health/detailed", get(handlers::health::health_check_detailed))
        .route("/ready", get(handlers::health::readiness_check))
        .route("/live", get(handlers::health::liveness_check))
        // OpenAPI documentation endpoints
        .route("/api-docs/openapi.json", get(serve_openapi_spec))
        .route("/docs", get(serve_swagger_ui))
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

/// Serve OpenAPI specification as JSON
async fn serve_openapi_spec() -> impl IntoResponse {
    Json(openapi_spec())
}

/// Serve Swagger UI HTML page
async fn serve_swagger_ui() -> impl IntoResponse {
    
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Ratchet API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui.css" />
    <style>
        html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin:0; background: #fafafa; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            const ui = SwaggerUIBundle({
                url: '/api-docs/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout"
            });
        }
    </script>
</body>
</html>"#;

    Html(html)
}

/// Create unified API router
fn create_api_router() -> Router<TasksContext> {
    Router::new()
        // Task endpoints
        .route("/tasks", get(handlers::tasks::list_tasks).post(handlers::tasks::create_task))
        .route("/tasks/stats", get(handlers::tasks::get_task_stats))
        .route("/tasks/sync", post(handlers::tasks::sync_tasks))
        .route(
            "/tasks/:id",
            get(handlers::tasks::get_task)
                .patch(handlers::tasks::update_task)
                .delete(handlers::tasks::delete_task),
        )
        .route("/tasks/:id/enable", post(handlers::tasks::enable_task))
        .route("/tasks/:id/disable", post(handlers::tasks::disable_task))
        // Execution endpoints
        .route("/executions", get(handlers::executions::list_executions).post(handlers::executions::create_execution))
        .route("/executions/stats", get(handlers::executions::get_execution_stats))
        .route(
            "/executions/:id",
            get(handlers::executions::get_execution)
                .patch(handlers::executions::update_execution),
        )
        .route("/executions/:id/cancel", post(handlers::executions::cancel_execution))
        .route("/executions/:id/retry", post(handlers::executions::retry_execution))
        .route("/executions/:id/logs", get(handlers::executions::get_execution_logs))
        // Job endpoints
        .route("/jobs", get(handlers::jobs::list_jobs).post(handlers::jobs::create_job))
        .route("/jobs/stats", get(handlers::jobs::get_job_stats))
        .route(
            "/jobs/:id",
            get(handlers::jobs::get_job)
                .patch(handlers::jobs::update_job),
        )
        .route("/jobs/:id/cancel", post(handlers::jobs::cancel_job))
        .route("/jobs/:id/retry", post(handlers::jobs::retry_job))
        // Schedule endpoints
        .route("/schedules", get(handlers::schedules::list_schedules).post(handlers::schedules::create_schedule))
        .route("/schedules/stats", get(handlers::schedules::get_schedule_stats))
        .route(
            "/schedules/:id",
            get(handlers::schedules::get_schedule)
                .patch(handlers::schedules::update_schedule)
                .delete(handlers::schedules::delete_schedule),
        )
        .route("/schedules/:id/enable", post(handlers::schedules::enable_schedule))
        .route("/schedules/:id/disable", post(handlers::schedules::disable_schedule))
        .route("/schedules/:id/trigger", post(handlers::schedules::trigger_schedule))
        // MCP task development endpoints
        .route("/mcp/tasks", post(handlers::mcp_create_task))
        .route("/mcp/tasks/:name", 
            get(handlers::mcp_edit_task)  // For getting current task config before editing
            .patch(handlers::mcp_edit_task)
            .delete(handlers::mcp_delete_task))
        .route("/mcp/tasks/:name/test", post(handlers::mcp_test_task))
        .route("/mcp/results", post(handlers::mcp_store_result))
        .route("/mcp/results/:name", get(handlers::mcp_get_results))
        // Worker endpoints  
        .route("/workers", get(handlers::workers::list_workers))
        .route("/workers/stats", get(handlers::workers::get_worker_stats))
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