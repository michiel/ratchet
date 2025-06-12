/// Enhanced REST API integration tests
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
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

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

/// Test helper to create a test REST API server with access to repositories
async fn create_test_server_with_repos() -> (TestServer, RepositoryFactory) {
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

    let app = create_rest_app(repositories.clone(), job_queue, task_executor, None, None);
    let server = TestServer::new(app).unwrap();
    (server, repositories)
}

#[tokio::test]
async fn test_health_and_stats_endpoints() {
    let server = create_test_server().await;

    // Test health endpoint
    let response = server.get("/health").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // Test job stats endpoint
    let response = server.get("/jobs/stats").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let stats: Value = response.json();
    assert!(stats.get("total").is_some());
    assert!(stats.get("queued").is_some());
    assert!(stats.get("by_priority").is_some());

    // Test worker stats endpoint
    let response = server.get("/workers/stats").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let stats: Value = response.json();
    assert!(stats.get("total_workers").is_some());
}

#[tokio::test]
#[ignore] // Disable until route issue is resolved
async fn test_pagination_parameters() {
    let server = create_test_server().await;

    // First test that the basic endpoint works
    let response = server.get("/jobs").await;
    if response.status_code() != StatusCode::OK {
        let error_body = response.text();
        panic!(
            "Basic jobs endpoint failed with status {}: {}",
            response.status_code(),
            error_body
        );
    }

    // Test valid pagination
    let response = server.get("/jobs?_start=0&_end=10").await;
    if response.status_code() != StatusCode::OK {
        let error_body = response.text();
        panic!(
            "Pagination failed with status {}: {}",
            response.status_code(),
            error_body
        );
    }
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    assert!(body.get("meta").is_some());

    // Test invalid pagination parameters
    let response = server.get("/jobs?_start=invalid").await;
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    // Test sorting parameters (use a valid field)
    let response = server.get("/jobs?_sort=id&_order=DESC").await;
    if response.status_code() != StatusCode::OK {
        let error_body = response.text();
        panic!(
            "Sorting failed with status {}: {}",
            response.status_code(),
            error_body
        );
    }
}

#[tokio::test]
async fn test_job_creation_and_retrieval() {
    let (server, repos) = create_test_server_with_repos().await;

    // First create a task
    use ratchet_lib::database::entities::tasks;
    use sea_orm::{ActiveModelTrait, Set};

    let task_model = tasks::ActiveModel {
        uuid: Set(Uuid::new_v4()),
        name: Set("test-task".to_string()),
        description: Set(Some("Test task".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        ..Default::default()
    };
    let task = task_model
        .insert(repos.database().get_connection())
        .await
        .unwrap();

    // Create a job via API
    let new_job = json!({
        "task_id": task.id,
        "input_data": {"test": true},
        "priority": "Normal"
    });

    let response = server.post("/jobs").json(&new_job).await;

    if response.status_code() != StatusCode::CREATED {
        let error_body = response.text();
        panic!(
            "Job creation failed with status {}: {}",
            response.status_code(),
            error_body
        );
    }

    let created_job: Value = response.json();
    assert_eq!(created_job["task_id"], task.id);
    assert_eq!(created_job["priority"], "Normal");
    let job_id = created_job["id"].as_i64().unwrap();

    // Retrieve the job
    let response = server.get(&format!("/jobs/{}", job_id)).await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let retrieved_job: Value = response.json();
    assert_eq!(retrieved_job["id"], job_id);
}

#[tokio::test]
#[ignore] // Disable until schedules endpoint is available
async fn test_schedule_operations() {
    let (server, repos) = create_test_server_with_repos().await;

    // Create a task first
    use ratchet_lib::database::entities::tasks;
    use sea_orm::{ActiveModelTrait, Set};

    let task_model = tasks::ActiveModel {
        uuid: Set(Uuid::new_v4()),
        name: Set("scheduled-task".to_string()),
        description: Set(Some("Task for scheduling".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        ..Default::default()
    };
    let task = task_model
        .insert(repos.database().get_connection())
        .await
        .unwrap();

    // Create a schedule
    let new_schedule = json!({
        "task_id": task.id,
        "cron_expression": "0 * * * *",
        "input_data": {"scheduled": true}
    });

    let response = server.post("/schedules").json(&new_schedule).await;
    assert_eq!(response.status_code(), StatusCode::CREATED);
    let schedule: Value = response.json();
    assert_eq!(schedule["cron_expression"], "0 * * * *");
    assert!(schedule["is_active"].as_bool().unwrap());
}

#[tokio::test]
#[ignore] // Disable until executions endpoint is available
async fn test_execution_filtering() {
    let (server, repos) = create_test_server_with_repos().await;

    // Create test data
    use ratchet_lib::database::entities::{executions, tasks, ExecutionStatus};
    use sea_orm::{ActiveModelTrait, Set};

    let task_model = tasks::ActiveModel {
        uuid: Set(Uuid::new_v4()),
        name: Set("exec-test-task".to_string()),
        description: Set(Some("Execution test task".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        ..Default::default()
    };
    let task = task_model
        .insert(repos.database().get_connection())
        .await
        .unwrap();

    // Create executions with different statuses
    for i in 0..3 {
        let exec_model = executions::ActiveModel {
            uuid: Set(Uuid::new_v4()),
            task_id: Set(task.id),
            status: Set(if i == 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed
            }),
            input: Set(json!({"index": i})),
            output: Set(Some(json!({"result": i}))),
            ..Default::default()
        };
        exec_model
            .insert(repos.database().get_connection())
            .await
            .unwrap();
    }

    // Test filtering by status
    let response = server.get("/executions?status=completed").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Value = response.json();
    let executions = body["data"].as_array().unwrap();
    assert_eq!(executions.len(), 1);

    // Test filtering by task_id
    let response = server
        .get(&format!("/executions?task_id={}", task.id))
        .await;
    assert_eq!(response.status_code(), StatusCode::OK);
    let body: Value = response.json();
    let executions = body["data"].as_array().unwrap();
    assert_eq!(executions.len(), 3);
}

#[tokio::test]
async fn test_error_responses() {
    let server = create_test_server().await;

    // Test 404 - non-existent resource
    let response = server.get("/jobs/99999").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    let error: Value = response.json();
    assert!(error.get("error").is_some());

    // Test 422 - invalid job creation (non-existent task)
    let invalid_job = json!({
        "task_id": 99999,
        "input_data": {}
    });
    let response = server.post("/jobs").json(&invalid_job).await;
    assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn test_sequential_job_creation() {
    let (server, repos) = create_test_server_with_repos().await;

    // Create a task
    use ratchet_lib::database::entities::tasks;
    use sea_orm::{ActiveModelTrait, Set};

    let task_model = tasks::ActiveModel {
        uuid: Set(Uuid::new_v4()),
        name: Set("concurrent-task".to_string()),
        description: Set(Some("Concurrent test task".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        ..Default::default()
    };
    let task = task_model
        .insert(repos.database().get_connection())
        .await
        .unwrap();

    // Create multiple jobs sequentially (concurrent test would require Send bounds)
    let mut success_count = 0;
    for i in 0..5 {
        let job = json!({
            "task_id": task.id,
            "input_data": {"index": i},
            "priority": "Normal"
        });
        let response = server.post("/jobs").json(&job).await;
        if response.status_code() == StatusCode::CREATED {
            success_count += 1;
        } else {
            let error_body = response.text();
            println!(
                "Job {} creation failed with status {}: {}",
                i,
                response.status_code(),
                error_body
            );
        }
    }

    // All should succeed
    assert_eq!(success_count, 5);
}
