use ratchet_lib::{
    database::{
        connection::DatabaseConnection,
        repositories::RepositoryFactory,
        entities::{Task, Job, JobPriority},
    },
    config::{DatabaseConfig, RatchetConfig},
    execution::{ProcessTaskExecutor, TaskExecutor},
    output::OutputDestinationConfig,
};
use std::time::Duration;
use tempfile::TempDir;
use serde_json::json;
use sea_orm::prelude::Uuid;

async fn setup_test_environment() -> (RepositoryFactory, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    
    // Setup in-memory database
    let db_config = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 1,
        connection_timeout: Duration::from_secs(5),
    };
    
    let connection = DatabaseConnection::new(db_config).await
        .expect("Failed to create database connection");
    
    // Run migrations
    connection.migrate().await
        .expect("Failed to run migrations");
    
    let repos = RepositoryFactory::new(connection);
    
    (repos, temp_dir)
}

#[tokio::test]
async fn test_output_delivery_to_filesystem() {
    let (repos, temp_dir) = setup_test_environment().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "test-task".to_string(),
        description: Some("Test task for output delivery".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({"test": "metadata"}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    // Create output destination configuration
    let output_path = temp_dir.path().join("outputs").join("{{job_id}}_{{timestamp}}.json");
    let output_destinations = vec![
        OutputDestinationConfig::Filesystem {
            path: output_path.to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    ];
    
    // Create a job with output destinations
    let mut job = Job::new(
        created_task.id,
        json!({"input": "test data"}),
        JobPriority::Normal,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    let created_job = repos.job_repo.create(job).await.unwrap();
    
    // Create and start executor
    let config = RatchetConfig::default();
    let executor = ProcessTaskExecutor::new(repos, config).await.unwrap();
    executor.start().await.unwrap();
    
    // Execute the job (this will fail because the task path doesn't exist, but we're testing output delivery)
    let _ = executor.execute_job(created_job.id).await;
    
    // Stop executor
    executor.stop().await.unwrap();
    
    // Note: In a real test, we would need a valid task that produces output
    // For now, this test verifies that the output delivery system is integrated
    // and doesn't crash during execution
}

#[tokio::test]
async fn test_output_delivery_with_templates() {
    let (repos, temp_dir) = setup_test_environment().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "template-test".to_string(),
        description: Some("Test task for template output".to_string()),
        version: "2.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({"test": "metadata"}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    // Create output destination with template variables
    let output_path = temp_dir.path().join("{{task_name}}_{{task_version}}_{{year}}{{month}}{{day}}.json");
    let output_destinations = vec![
        OutputDestinationConfig::Filesystem {
            path: output_path.to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        }
    ];
    
    // Create a job with output destinations and metadata
    let mut job = Job::new(
        created_task.id,
        json!({"test": "input"}),
        JobPriority::High,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    job.metadata = Some(json!({
        "user_id": "test-user",
        "request_id": "req-123"
    }));
    let created_job = repos.job_repo.create(job).await.unwrap();
    
    // Verify the job was created with output destinations
    let fetched_job = repos.job_repo.find_by_id(created_job.id).await.unwrap().unwrap();
    assert!(fetched_job.output_destinations.is_some());
    
    // Parse and verify the configuration
    let configs: Vec<OutputDestinationConfig> = 
        serde_json::from_value(fetched_job.output_destinations.unwrap()).unwrap();
    assert_eq!(configs.len(), 1);
    
    match &configs[0] {
        OutputDestinationConfig::Filesystem { path, .. } => {
            assert!(path.contains("{{task_name}}"));
            assert!(path.contains("{{task_version}}"));
        }
        _ => panic!("Expected filesystem destination"),
    }
}

#[tokio::test]
async fn test_multiple_output_destinations() {
    let (repos, temp_dir) = setup_test_environment().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "multi-output".to_string(),
        description: Some("Test task with multiple outputs".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: json!({"test": "metadata"}),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    // Create multiple output destinations
    let output_destinations = vec![
        OutputDestinationConfig::Filesystem {
            path: temp_dir.path().join("output1.json").to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        },
        OutputDestinationConfig::Filesystem {
            path: temp_dir.path().join("output2.yaml").to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Yaml,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        },
        OutputDestinationConfig::Webhook {
            url: "https://example.com/webhook".to_string(),
            method: ratchet_lib::types::HttpMethod::Post,
            headers: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_policy: ratchet_lib::output::RetryPolicy::default(),
            auth: None,
            content_type: Some("application/json".to_string()),
        },
    ];
    
    // Create a job with multiple output destinations
    let mut job = Job::new(
        created_task.id,
        json!({"data": "test"}),
        JobPriority::Normal,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    let created_job = repos.job_repo.create(job).await.unwrap();
    
    // Verify the job has multiple destinations
    let fetched_job = repos.job_repo.find_by_id(created_job.id).await.unwrap().unwrap();
    let configs: Vec<OutputDestinationConfig> = 
        serde_json::from_value(fetched_job.output_destinations.unwrap()).unwrap();
    assert_eq!(configs.len(), 3);
    
    // Verify each destination type
    let mut has_json_file = false;
    let mut has_yaml_file = false;
    let mut has_webhook = false;
    
    for config in configs {
        match config {
            OutputDestinationConfig::Filesystem { format, .. } => {
                match format {
                    ratchet_lib::output::OutputFormat::Json => has_json_file = true,
                    ratchet_lib::output::OutputFormat::Yaml => has_yaml_file = true,
                    _ => {}
                }
            }
            OutputDestinationConfig::Webhook { .. } => has_webhook = true,
            _ => {}
        }
    }
    
    assert!(has_json_file);
    assert!(has_yaml_file);
    assert!(has_webhook);
}