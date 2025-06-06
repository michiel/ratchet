use axum::http::StatusCode;
use axum_test::TestServer;
use ratchet_lib::{
    config::DatabaseConfig,
    database::{connection::DatabaseConnection, repositories::RepositoryFactory},
    execution::{
        job_queue::{JobQueueConfig, JobQueueManager},
        process_executor::ProcessTaskExecutor,
    },
    rest::app::create_rest_app,
};
use serde_json::Value;
use std::sync::Arc;

/// Test helper to create a test REST API server
async fn create_test_server() -> TestServer {
    // Use in-memory SQLite for testing
    let db_config = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
        connection_timeout: std::time::Duration::from_secs(5),
    };

    let db_connection = DatabaseConnection::new(db_config).await.unwrap();
    let repositories = RepositoryFactory::new(db_connection);

    // Run migrations
    use ratchet_lib::database::migrations::Migrator;
    use sea_orm_migration::MigratorTrait;
    Migrator::up(repositories.database().get_connection(), None)
        .await
        .unwrap();

    // Create minimal required components
    let job_queue_config = JobQueueConfig {
        max_dequeue_batch_size: 10,
        max_queue_size: 1000,
        default_retry_delay: 60,
        default_max_retries: 3,
    };
    let job_queue = Arc::new(JobQueueManager::new(repositories.clone(), job_queue_config));

    // Create a basic ProcessTaskExecutor with default config
    let config = ratchet_lib::config::RatchetConfig::default();
    let task_executor = Arc::new(
        ProcessTaskExecutor::new(repositories.clone(), config)
            .await
            .unwrap(),
    );

    let app = create_rest_app(repositories, job_queue, task_executor, None, None);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_health_check_endpoint() {
    let server = create_test_server().await;

    let response = server.get("/health").await;
    assert_eq!(response.status_code(), StatusCode::OK);
}

// Note: Tasks endpoint test disabled because it requires TaskSyncService and TaskRegistry
// which need complex setup. This is acceptable for basic REST API testing.
// TODO: Add proper task service mocking for comprehensive testing

#[tokio::test]
async fn test_jobs_endpoints() {
    let server = create_test_server().await;

    // Test GET /jobs
    let response = server.get("/jobs").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return empty data array initially
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    let data = body["data"].as_array().unwrap();
    assert!(data.is_empty());

    // Test GET /jobs/stats
    let response = server.get("/jobs/stats").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return job queue statistics
    let body: Value = response.json();
    assert!(body.get("total").is_some());
    assert!(body.get("queued").is_some());
    assert!(body.get("by_priority").is_some());
}

#[tokio::test]
async fn test_schedules_endpoints() {
    let server = create_test_server().await;

    // Test GET /schedules
    let response = server.get("/schedules").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return empty data array initially
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    let data = body["data"].as_array().unwrap();
    assert!(data.is_empty());
}

#[tokio::test]
async fn test_workers_endpoints() {
    let server = create_test_server().await;

    // Test GET /workers
    let response = server.get("/workers").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return mock worker data
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    let data = body["data"].as_array().unwrap();
    assert_eq!(data.len(), 2); // Mock returns 2 workers

    // Test GET /workers/stats
    let response = server.get("/workers/stats").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return worker pool statistics
    let body: Value = response.json();
    assert!(body.get("total_workers").is_some());
    assert!(body.get("idle_workers").is_some());
    assert!(body.get("running_workers").is_some());
}

#[tokio::test]
async fn test_executions_endpoints() {
    let server = create_test_server().await;

    // Test GET /executions
    let response = server.get("/executions").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Should return empty data array initially
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    let data = body["data"].as_array().unwrap();
    assert!(data.is_empty());
}
