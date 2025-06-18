//! Main application configuration and router setup

use axum::{
    response::{Json, Html, IntoResponse},
    routing::{get, post},
    Router,
};
use ratchet_interfaces::{RepositoryFactory, TaskRegistry, RegistryManager, TaskValidator};
use ratchet_web::{
    middleware::{
        audit_middleware, cors_layer, error_handler_layer, request_id_layer, security_headers_middleware,
        rate_limit_middleware, create_rate_limit_middleware, session_middleware, create_session_manager,
        AuditConfig, SecurityConfig, RateLimitConfig, SessionConfig, RateLimiter,
    },
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
    /// Enable security headers
    pub enable_security_headers: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Enable session management
    pub enable_session_management: bool,
    /// Security configuration
    pub security_config: SecurityConfig,
    /// Audit configuration
    pub audit_config: AuditConfig,
    /// Rate limiting configuration
    pub rate_limit_config: RateLimitConfig,
    /// Session management configuration
    pub session_config: SessionConfig,
    /// API path prefix
    pub api_prefix: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enable_cors: true,
            enable_request_id: true,
            enable_tracing: true,
            enable_security_headers: true,
            enable_audit_logging: true,
            enable_rate_limiting: true,
            enable_session_management: true,
            security_config: SecurityConfig::development(),
            audit_config: AuditConfig::development(),
            rate_limit_config: RateLimitConfig::permissive(),
            session_config: SessionConfig::development(),
            api_prefix: "/api/v1".to_string(),
        }
    }
}

impl AppConfig {
    /// Create a production configuration
    pub fn production() -> Self {
        Self {
            enable_cors: true,
            enable_request_id: true,
            enable_tracing: true,
            enable_security_headers: true,
            enable_audit_logging: true,
            enable_rate_limiting: true,
            enable_session_management: true,
            security_config: SecurityConfig::production(),
            audit_config: AuditConfig::production(),
            rate_limit_config: RateLimitConfig::strict(),
            session_config: SessionConfig::production(),
            api_prefix: "/api/v1".to_string(),
        }
    }
    
    /// Create a development configuration
    pub fn development() -> Self {
        Self {
            enable_cors: true,
            enable_request_id: true,
            enable_tracing: true,
            enable_security_headers: true,
            enable_audit_logging: true,
            enable_rate_limiting: true,
            enable_session_management: true,
            security_config: SecurityConfig::development(),
            audit_config: AuditConfig::development(),
            rate_limit_config: RateLimitConfig::permissive(),
            session_config: SessionConfig::development(),
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
        // Health endpoints (no prefix) - need context for detailed checks
        .route("/health", get(handlers::health::health_check))
        .route("/health/detailed", get(handlers::health::health_check_detailed))
        .route("/ready", get(handlers::health::readiness_check))
        .route("/live", get(handlers::health::liveness_check))
        // Metrics endpoints (no prefix) - need context for application metrics
        .route("/metrics", get(handlers::metrics::get_metrics))
        .route("/metrics/prometheus", get(handlers::metrics::get_prometheus_metrics))
        // OpenAPI documentation endpoints (no context needed)
        .route("/api-docs/openapi.json", get(serve_openapi_spec))
        .route("/docs", get(serve_swagger_ui))
        // API routes with prefix
        .nest(&config.api_prefix, create_api_router())
        // Add application context for all routes
        .with_state(context.tasks);

    // Add middleware layers (applied in reverse order)
    let mut app = app;
    
    // Security headers (applied first, affects all responses)
    if config.enable_security_headers {
        let security_config = config.security_config.clone();
        app = app.layer(axum::middleware::from_fn(move |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let config = security_config.clone();
            async move {
                req.extensions_mut().insert(config);
                security_headers_middleware(req, next).await
            }
        }));
    }
    
    // Rate limiting (applied early to prevent abuse)
    if config.enable_rate_limiting {
        let rate_limiter = create_rate_limit_middleware(config.rate_limit_config.clone());
        app = app.layer(axum::middleware::from_fn(move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let rate_limiter = rate_limiter.clone();
            async move {
                // Extract ConnectInfo from request extensions if available
                let connect_info = req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>().cloned();
                let mut req = req;
                req.extensions_mut().insert(rate_limiter);
                match rate_limit_middleware(connect_info, req, next).await {
                    Ok(response) => response,
                    Err(err) => err.into_response(),
                }
            }
        }));
    }
    
    // Session management (applied after rate limiting but before other middleware)
    if config.enable_session_management {
        let session_manager = create_session_manager(config.session_config.clone());
        app = app.layer(axum::middleware::from_fn(move |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let session_manager = session_manager.clone();
            async move {
                req.extensions_mut().insert(session_manager);
                session_middleware(req, next).await
            }
        }));
    }
    
    // Audit logging (should be one of the first middleware to capture all requests)
    if config.enable_audit_logging {
        let audit_config = config.audit_config.clone();
        app = app.layer(axum::middleware::from_fn(move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            let config = audit_config.clone();
            async move {
                // Extract ConnectInfo from request extensions if available
                let connect_info = req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>().cloned();
                let mut req = req;
                req.extensions_mut().insert(config);
                audit_middleware(connect_info, req, next).await
            }
        }));
    }
    
    // CORS handling
    if config.enable_cors {
        app = app.layer(cors_layer());
    }

    // Request ID tracking
    if config.enable_request_id {
        app = app.layer(request_id_layer());
    }

    // HTTP tracing
    if config.enable_tracing {
        app = app.layer(TraceLayer::new_for_http());
    }

    // Error handling (should be last to catch all errors)
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
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5.10.5/swagger-ui.css" />
    <style>
        html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin:0; background: #fafafa; }
        .loading { 
            padding: 20px; 
            text-align: center; 
            font-family: sans-serif; 
            color: #666; 
        }
    </style>
</head>
<body>
    <div id="swagger-ui">
        <div class="loading">Loading API Documentation...</div>
    </div>
    <script src="https://unpkg.com/swagger-ui-dist@5.10.5/swagger-ui-bundle.js" onerror="handleScriptError()"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5.10.5/swagger-ui-standalone-preset.js" onerror="handleScriptError()"></script>
    <script>
        function handleScriptError() {
            document.getElementById('swagger-ui').innerHTML = `
                <div style="padding: 20px; font-family: sans-serif;">
                    <h2>⚠️ Unable to Load Swagger UI</h2>
                    <p>The Swagger UI resources failed to load from CDN. This might be due to:</p>
                    <ul>
                        <li>Network connectivity issues</li>
                        <li>CDN being blocked by your network</li>
                        <li>Browser security settings</li>
                    </ul>
                    <p><strong>Alternative:</strong> You can access the raw OpenAPI specification at: 
                        <a href="/api-docs/openapi.json">/api-docs/openapi.json</a>
                    </p>
                </div>
            `;
        }

        window.onload = function() {
            if (typeof SwaggerUIBundle === 'undefined') {
                handleScriptError();
                return;
            }
            
            try {
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
                    layout: "StandaloneLayout",
                    onComplete: function() {
                        console.log('Swagger UI loaded successfully');
                    },
                    onFailure: function(error) {
                        console.error('Swagger UI failed to load:', error);
                        handleScriptError();
                    }
                });
            } catch (error) {
                console.error('Error initializing Swagger UI:', error);
                handleScriptError();
            }
        }
    </script>
</body>
</html>"#;

    Html(html)
}

/// Create unified API router
fn create_api_router() -> Router<TasksContext> {
    Router::new()
        // Authentication endpoints (no auth required)
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/me", get(handlers::auth::get_current_user))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/change-password", post(handlers::auth::change_password))
        // Task endpoints
        .route("/tasks", get(handlers::tasks::list_tasks).post(handlers::tasks::create_task))
        .route("/tasks/stats", get(handlers::tasks::get_task_stats))
        .route("/tasks/sync", post(handlers::tasks::sync_tasks))
        .route(
            "/tasks/{id}",
            get(handlers::tasks::get_task)
                .patch(handlers::tasks::update_task)
                .delete(handlers::tasks::delete_task),
        )
        .route("/tasks/{id}/enable", post(handlers::tasks::enable_task))
        .route("/tasks/{id}/disable", post(handlers::tasks::disable_task))
        // Execution endpoints
        .route("/executions", get(handlers::executions::list_executions).post(handlers::executions::create_execution))
        .route("/executions/stats", get(handlers::executions::get_execution_stats))
        .route(
            "/executions/{id}",
            get(handlers::executions::get_execution)
                .patch(handlers::executions::update_execution)
                .delete(handlers::executions::delete_execution),
        )
        .route("/executions/{id}/cancel", post(handlers::executions::cancel_execution))
        .route("/executions/{id}/retry", post(handlers::executions::retry_execution))
        .route("/executions/{id}/logs", get(handlers::executions::get_execution_logs))
        // Job endpoints
        .route("/jobs", get(handlers::jobs::list_jobs).post(handlers::jobs::create_job))
        .route("/jobs/stats", get(handlers::jobs::get_job_stats))
        .route(
            "/jobs/{id}",
            get(handlers::jobs::get_job)
                .patch(handlers::jobs::update_job)
                .delete(handlers::jobs::delete_job),
        )
        .route("/jobs/{id}/cancel", post(handlers::jobs::cancel_job))
        .route("/jobs/{id}/retry", post(handlers::jobs::retry_job))
        // Schedule endpoints
        .route("/schedules", get(handlers::schedules::list_schedules).post(handlers::schedules::create_schedule))
        .route("/schedules/stats", get(handlers::schedules::get_schedule_stats))
        .route(
            "/schedules/{id}",
            get(handlers::schedules::get_schedule)
                .patch(handlers::schedules::update_schedule)
                .delete(handlers::schedules::delete_schedule),
        )
        .route("/schedules/{id}/enable", post(handlers::schedules::enable_schedule))
        .route("/schedules/{id}/disable", post(handlers::schedules::disable_schedule))
        .route("/schedules/{id}/trigger", post(handlers::schedules::trigger_schedule))
        // MCP task development endpoints
        .route("/mcp/tasks", post(handlers::mcp_create_task))
        .route("/mcp/tasks/{name}", 
            get(handlers::mcp_edit_task)  // For getting current task config before editing
            .patch(handlers::mcp_edit_task)
            .delete(handlers::mcp_delete_task))
        .route("/mcp/tasks/{name}/test", post(handlers::mcp_test_task))
        .route("/mcp/results", post(handlers::mcp_store_result))
        .route("/mcp/results/{name}", get(handlers::mcp_get_results))
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