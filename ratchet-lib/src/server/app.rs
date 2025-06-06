use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;

use crate::database::repositories::RepositoryFactory;
use crate::execution::{JobQueueManager, ProcessTaskExecutor};
use crate::graphql::{create_schema, RatchetSchema};
use crate::registry::TaskRegistry;
use crate::rest::create_rest_app;
use crate::services::TaskSyncService;

use super::{
    handlers::{graphql_handler, graphql_playground, health_handler, version_handler},
    middleware::{cors_layer, logging_middleware, trace_layer},
};

/// Server application state with Send+Sync compliance
#[derive(Clone)]
pub struct ServerState {
    pub schema: RatchetSchema,
    pub repositories: RepositoryFactory,
    pub job_queue: Arc<JobQueueManager>,
    pub task_executor: Arc<ProcessTaskExecutor>, // ✅ Send/Sync compliant
    pub registry: Option<Arc<TaskRegistry>>,
    pub task_sync_service: Option<Arc<TaskSyncService>>,
}

/// Create the main Axum application with all routes and middleware
pub fn create_app(
    repositories: RepositoryFactory,
    job_queue: Arc<JobQueueManager>,
    task_executor: Arc<ProcessTaskExecutor>,
    registry: Option<Arc<TaskRegistry>>,
    task_sync_service: Option<Arc<TaskSyncService>>,
    mcp_routes: Option<Router>,
) -> Router {
    // Create GraphQL schema with process-based executor
    let schema = create_schema(
        repositories.clone(),
        job_queue.clone(),
        task_executor.clone(),
        registry.clone(),
        task_sync_service.clone(),
    );

    // Create server state
    // Create REST API first, before moving values into ServerState
    let rest_api = create_rest_app(
        repositories.clone(),
        job_queue.clone(),
        task_executor.clone(),
        registry.clone(),
        task_sync_service.clone(),
    );

    let state = ServerState {
        schema,
        repositories,
        job_queue,
        task_executor,
        registry,
        task_sync_service,
    };

    // Build the router with all routes
    let mut app = Router::new()
        // GraphQL routes
        .route("/graphql", post(graphql_handler))
        .route("/playground", get(graphql_playground))
        // API routes
        .route("/health", get(health_handler))
        .route("/version", get(version_handler))
        .route(
            "/api-docs",
            get(|| async { axum::response::Redirect::permanent("/docs/openapi-viewer.html") }),
        )
        // Root route
        .route("/", get(|| async { "Ratchet API Server" }))
        // Add state for GraphQL routes
        .with_state(state)
        // Nest REST API under /api/v1
        .nest("/api/v1", rest_api)
        // Serve static documentation files
        .nest_service("/docs", ServeDir::new("docs"));

    // Add MCP routes if provided
    if let Some(mcp_router) = mcp_routes {
        app = app.nest("/mcp", mcp_router);
    }

    // Add middleware stack
    app.layer(
        ServiceBuilder::new()
            .layer(trace_layer())
            .layer(cors_layer())
            .layer(middleware::from_fn(logging_middleware)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, RatchetConfig, ServerConfig};
    use crate::database::DatabaseConnection;
    use axum_test::TestServer;
    use std::time::Duration;

    async fn create_test_app() -> Router {
        let mut config = RatchetConfig::default();
        config.server = Some(ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            database: DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                max_connections: 5,
                connection_timeout: Duration::from_secs(10),
            },
            // JWT configuration removed
        });

        let db = DatabaseConnection::new(config.server.as_ref().unwrap().database.clone())
            .await
            .unwrap();
        db.migrate().await.unwrap();

        let repositories = RepositoryFactory::new(db);
        let job_queue = Arc::new(JobQueueManager::with_default_config(repositories.clone()));
        let task_executor = Arc::new(
            ProcessTaskExecutor::new(repositories.clone(), config)
                .await
                .unwrap(),
        );

        create_app(repositories, job_queue, task_executor, None, None, None)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/health").await;
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_version_endpoint() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/version").await;
        assert_eq!(response.status_code(), 200);

        let json: serde_json::Value = response.json();
        assert!(json.get("version").is_some());
        assert!(json.get("name").is_some());
    }

    #[tokio::test]
    async fn test_graphql_playground() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();

        let response = server.get("/playground").await;
        assert_eq!(response.status_code(), 200);
        assert!(response.text().contains("GraphQL Playground"));
    }
}
