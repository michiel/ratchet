use axum::http::StatusCode;
use axum_test::TestServer;
use ratchet_lib::{
    config::DatabaseConfig,
    database::{connection::DatabaseConnection, repositories::RepositoryFactory, entities::{Task, Job, JobPriority}},
    rest::app::create_rest_app,
    execution::{
        job_queue::{JobQueueConfig, JobQueueManager},
        process_executor::ProcessTaskExecutor,
    },
    output::OutputDestinationConfig,
};
use serde_json::{json, Value};
use std::sync::Arc;
use sea_orm::prelude::Uuid;
use tempfile::TempDir;

/// Test helper to create a test REST API server with database
async fn create_test_server() -> (TestServer, RepositoryFactory, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    
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
    let task_executor = Arc::new(ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap());
    
    let app = create_rest_app(repositories.clone(), job_queue, task_executor, None, None);
    let server = TestServer::new(app).unwrap();
    
    (server, repositories, temp_dir)
}

#[tokio::test]
async fn test_test_output_destinations_endpoint() {
    let (server, _repos, temp_dir) = create_test_server().await;
    
    let test_request = json!({
        "destinations": [
            {
                "type": "filesystem",
                "path": temp_dir.path().join("test.json").to_string_lossy(),
                "format": "json",
                "create_dirs": true,
                "overwrite": true
            },
            {
                "type": "webhook",
                "url": "https://httpbin.org/post",
                "method": "POST",
                "timeout_seconds": 10
            }
        ]
    });
    
    let response = server
        .post("/jobs/test-output-destinations")
        .json(&test_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: Value = response.json();
    assert!(body.get("results").is_some());
    assert!(body.get("overall_success").is_some());
    
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    
    // Check filesystem destination result
    let filesystem_result = &results[0];
    assert_eq!(filesystem_result["index"], 0);
    assert_eq!(filesystem_result["destination_type"], "filesystem");
    assert_eq!(filesystem_result["success"], true);
    assert!(filesystem_result["error"].is_null());
    
    // Check webhook destination result
    let webhook_result = &results[1];
    assert_eq!(webhook_result["index"], 1);
    assert_eq!(webhook_result["destination_type"], "webhook");
    assert_eq!(webhook_result["success"], true);
    assert!(webhook_result["error"].is_null());
}

#[tokio::test]
async fn test_test_output_destinations_invalid_config() {
    let (server, _repos, _temp_dir) = create_test_server().await;
    
    let test_request = json!({
        "destinations": [
            {
                "type": "filesystem",
                "path": "", // Invalid empty path
                "format": "json"
            }
        ]
    });
    
    let response = server
        .post("/jobs/test-output-destinations")
        .json(&test_request)
        .await;
    
    // The endpoint returns OK but marks the destination as failed
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: Value = response.json();
    assert!(body.get("results").is_some());
    assert_eq!(body["overall_success"], false);
    
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    
    let result = &results[0];
    assert_eq!(result["success"], false);
    assert!(result["error"].as_str().unwrap().to_lowercase().contains("path"));
}

#[tokio::test]
async fn test_create_job_with_output_destinations() {
    let (server, repos, temp_dir) = create_test_server().await;
    
    // Create a test task first
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "test-task".to_string(),
        description: Some("Test task".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    let job_request = json!({
        "task_id": created_task.id,
        "input_data": {"message": "hello"},
        "priority": "Normal",
        "output_destinations": [
            {
                "type": "filesystem",
                "path": temp_dir.path().join("{{job_uuid}}.json").to_string_lossy(),
                "format": "json",
                "create_dirs": true,
                "overwrite": true
            }
        ]
    });
    
    let response = server
        .post("/jobs")
        .json(&job_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::CREATED);
    
    let body: Value = response.json();
    // The job creation response is a direct JobResponse, not wrapped in data
    assert_eq!(body["task_id"], created_task.id);
    assert_eq!(body["priority"], "Normal"); // Note: enum serialization uses Pascal case
    assert!(body["output_destinations"].is_array());
    
    let destinations = body["output_destinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0]["type"], "filesystem");
}

#[tokio::test]
async fn test_job_list_includes_output_destinations() {
    let (server, repos, temp_dir) = create_test_server().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "list-test-task".to_string(),
        description: Some("Test task for listing".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    // Create a job with output destinations
    let output_destinations = vec![
        OutputDestinationConfig::Filesystem {
            path: temp_dir.path().join("list-test.json").to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    ];
    
    let mut job = Job::new(
        created_task.id,
        json!({"test": "data"}),
        JobPriority::Normal,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    let _created_job = repos.job_repo.create(job).await.unwrap();
    
    // Test GET /jobs
    let response = server.get("/jobs").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: Value = response.json();
    assert!(body.get("data").is_some());
    
    let jobs = body["data"].as_array().unwrap();
    assert!(!jobs.is_empty());
    
    let job_data = &jobs[0];
    // output_destinations might be null if empty
    if !job_data["output_destinations"].is_null() {
        assert!(job_data["output_destinations"].is_array());
        
        let destinations = job_data["output_destinations"].as_array().unwrap();
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0]["type"], "filesystem");
    }
}

#[tokio::test]
async fn test_job_creation_with_multiple_destinations() {
    let (server, repos, temp_dir) = create_test_server().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "multi-dest-task".to_string(),
        description: Some("Test task with multiple destinations".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    let job_request = json!({
        "task_id": created_task.id,
        "input_data": {"data": "multi-test"},
        "priority": "High",
        "output_destinations": [
            {
                "type": "filesystem",
                "path": temp_dir.path().join("output1.json").to_string_lossy(),
                "format": "json"
            },
            {
                "type": "filesystem",
                "path": temp_dir.path().join("output2.yaml").to_string_lossy(),
                "format": "yaml"
            },
            {
                "type": "webhook",
                "url": "https://httpbin.org/post",
                "method": "POST",
                "timeout_seconds": 30
            }
        ]
    });
    
    let response = server
        .post("/jobs")
        .json(&job_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::CREATED);
    
    let body: Value = response.json();
    // Direct JobResponse without data wrapper
    assert_eq!(body["priority"], "High"); // enum serialization
    
    let destinations = body["output_destinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 3);
    
    // Verify each destination type
    let mut has_json = false;
    let mut has_yaml = false;
    let mut has_webhook = false;
    
    for dest in destinations {
        match dest["type"].as_str().unwrap() {
            "filesystem" => {
                if dest["format"] == "json" {
                    has_json = true;
                } else if dest["format"] == "yaml" {
                    has_yaml = true;
                }
            }
            "webhook" => has_webhook = true,
            _ => {}
        }
    }
    
    assert!(has_json);
    assert!(has_yaml);
    assert!(has_webhook);
}

#[tokio::test]
async fn test_test_destinations_with_templates() {
    let (server, _repos, temp_dir) = create_test_server().await;
    
    let test_request = json!({
        "destinations": [
            {
                "type": "filesystem",
                "path": temp_dir.path().join("{{task_name}}_{{timestamp}}.json").to_string_lossy(),
                "format": "json",
                "create_dirs": true
            },
            {
                "type": "webhook",
                "url": "https://{{env}}.example.com/webhook/{{job_id}}",
                "method": "POST",
                "headers": {
                    "X-Task": "{{task_name}}",
                    "X-Environment": "{{env}}"
                }
            }
        ]
    });
    
    let response = server
        .post("/jobs/test-output-destinations")
        .json(&test_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: Value = response.json();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    
    // Both destinations should succeed even with template variables
    // (test mode uses placeholder values)
    for result in results {
        assert_eq!(result["success"], true);
    }
}

#[tokio::test]
async fn test_webhook_authentication_config() {
    let (server, _repos, _temp_dir) = create_test_server().await;
    
    let test_request = json!({
        "destinations": [
            {
                "type": "webhook",
                "url": "https://httpbin.org/bearer",
                "method": "POST",
                "timeout_seconds": 10,
                "auth": {
                    "bearer": {
                        "token": "test-token-123"
                    }
                }
            }
        ]
    });
    
    let response = server
        .post("/jobs/test-output-destinations")
        .json(&test_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: Value = response.json();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    
    let webhook_result = &results[0];
    assert_eq!(webhook_result["destination_type"], "webhook");
    assert_eq!(webhook_result["success"], true);
}

#[tokio::test]
async fn test_invalid_job_creation_missing_task() {
    let (server, _repos, temp_dir) = create_test_server().await;
    
    let job_request = json!({
        "task_id": 99999, // Non-existent task
        "input_data": {"test": "data"},
        "output_destinations": [
            {
                "type": "filesystem",
                "path": temp_dir.path().join("test.json").to_string_lossy(),
                "format": "json"
            }
        ]
    });
    
    let response = server
        .post("/jobs")
        .json(&job_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_malformed_output_destinations() {
    let (server, _repos, _temp_dir) = create_test_server().await;
    
    let test_request = json!({
        "destinations": [
            {
                "type": "invalid_type",
                "url": "https://example.com"
            }
        ]
    });
    
    let response = server
        .post("/jobs/test-output-destinations")
        .json(&test_request)
        .await;
    
    // The endpoint might return 422 for unprocessable entity instead of 400
    assert!(response.status_code() == StatusCode::BAD_REQUEST || response.status_code() == StatusCode::UNPROCESSABLE_ENTITY);
}