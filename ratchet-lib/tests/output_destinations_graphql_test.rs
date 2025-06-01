use ratchet_lib::{
    config::DatabaseConfig,
    database::{connection::DatabaseConnection, repositories::RepositoryFactory, entities::{Task, Job, JobPriority}},
    execution::{
        job_queue::{JobQueueConfig, JobQueueManager},
        process_executor::ProcessTaskExecutor,
    },
    graphql::schema::{create_schema, RatchetSchema},
    output::OutputDestinationConfig,
};
use serde_json::json;
use std::sync::Arc;
use sea_orm::prelude::{Uuid, Json};
use tempfile::TempDir;

type TestSchema = RatchetSchema;

/// Test helper to create GraphQL schema with test database
async fn create_test_schema() -> (TestSchema, RepositoryFactory, TempDir) {
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
    
    // Create required components
    let job_queue_config = JobQueueConfig {
        max_dequeue_batch_size: 10,
        max_queue_size: 1000,
        default_retry_delay: 60,
        default_max_retries: 3,
    };
    let job_queue = Arc::new(JobQueueManager::new(repositories.clone(), job_queue_config));
    
    let config = ratchet_lib::config::RatchetConfig::default();
    let task_executor = Arc::new(ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap());
    
    let schema = create_schema(
        repositories.clone(),
        job_queue,
        task_executor,
        None,  // registry
        None,  // task_sync_service
    );
    
    (schema, repositories, temp_dir)
}

#[tokio::test]
async fn test_graphql_test_output_destinations() {
    let (schema, _repos, temp_dir) = create_test_schema().await;
    
    let query = format!(r#"
        mutation {{
            testOutputDestinations(input: {{
                destinations: [
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: JSON,
                            createDirs: true,
                            overwrite: true
                        }}
                    }},
                    {{
                        destinationType: WEBHOOK,
                        webhook: {{
                            url: "https://httpbin.org/post",
                            method: POST,
                            timeoutSeconds: 30
                        }}
                    }}
                ]
            }}) {{
                index
                destinationType
                success
                error
                estimatedTimeMs
            }}
        }}
    "#, temp_dir.path().join("test.json").to_string_lossy());
    
    let result = schema.execute(&query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let test_results = data["testOutputDestinations"].as_array().unwrap();
    assert_eq!(test_results.len(), 2);
    
    // Check filesystem destination result
    let filesystem_result = &test_results[0];
    assert_eq!(filesystem_result["index"], 0);
    assert_eq!(filesystem_result["destinationType"], "filesystem");
    assert_eq!(filesystem_result["success"], true);
    assert!(filesystem_result["error"].is_null());
    
    // Check webhook destination result
    let webhook_result = &test_results[1];
    assert_eq!(webhook_result["index"], 1);
    assert_eq!(webhook_result["destinationType"], "webhook");
    assert_eq!(webhook_result["success"], true);
    assert!(webhook_result["error"].is_null());
}

#[tokio::test]
async fn test_graphql_execute_task_with_destinations() {
    let (schema, repos, temp_dir) = create_test_schema().await;
    
    // Create a test task first
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "graphql-test-task".to_string(),
        description: Some("Test task for GraphQL".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: Json::from(json!({})),
        input_schema: Json::from(json!({"type": "object"})),
        output_schema: Json::from(json!({"type": "object"})),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    let mutation = format!(r#"
        mutation {{
            executeTask(input: {{
                taskId: {},
                inputData: {{"message": "hello"}},
                priority: NORMAL,
                outputDestinations: [
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: JSON,
                            permissions: "644",
                            createDirs: true,
                            overwrite: true,
                            backupExisting: false
                        }}
                    }},
                    {{
                        destinationType: WEBHOOK,
                        webhook: {{
                            url: "https://httpbin.org/post",
                            method: POST,
                            headers: {{"X-Source": "ratchet"}},
                            timeoutSeconds: 30,
                            contentType: "application/json"
                        }}
                    }}
                ]
            }}) {{
                id
                taskId
                priority
                status
                outputDestinations {{
                    ... on FilesystemDestination {{
                        path
                        format
                        permissions
                        createDirs
                        overwrite
                    }}
                    ... on WebhookDestination {{
                        url
                        method
                        timeoutSeconds
                        contentType
                    }}
                }}
            }}
        }}
    "#, created_task.id, temp_dir.path().join("{{job_uuid}}.json").to_string_lossy());
    
    let result = schema.execute(&mutation).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let job_data = &data["executeTask"];
    
    assert_eq!(job_data["taskId"], created_task.id);
    assert_eq!(job_data["priority"], "NORMAL");
    assert_eq!(job_data["status"], "QUEUED");
    
    let destinations = job_data["outputDestinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 2);
    
    // Check filesystem destination
    let filesystem_dest = &destinations[0];
    assert!(filesystem_dest["path"].as_str().unwrap().contains("{{job_uuid}}"));
    assert_eq!(filesystem_dest["format"], "JSON");
    assert_eq!(filesystem_dest["permissions"], "644");
    assert_eq!(filesystem_dest["createDirs"], true);
    
    // Check webhook destination
    let webhook_dest = &destinations[1];
    assert_eq!(webhook_dest["url"], "https://httpbin.org/post");
    assert_eq!(webhook_dest["method"], "POST");
    assert_eq!(webhook_dest["timeoutSeconds"], 30);
    assert_eq!(webhook_dest["contentType"], "application/json");
}

#[tokio::test]
async fn test_graphql_query_jobs_with_destinations() {
    let (schema, repos, temp_dir) = create_test_schema().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "query-test-task".to_string(),
        description: Some("Test task for job query".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: Json::from(json!({})),
        input_schema: Json::from(json!({"type": "object"})),
        output_schema: Json::from(json!({"type": "object"})),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    // Create a job with output destinations
    let output_destinations = vec![
        OutputDestinationConfig::Filesystem {
            path: temp_dir.path().join("query-test.json").to_string_lossy().to_string(),
            format: ratchet_lib::output::OutputFormat::Json,
            permissions: 0o644,
            create_dirs: true,
            overwrite: true,
            backup_existing: false,
        },
        OutputDestinationConfig::Webhook {
            url: "https://example.com/webhook".to_string(),
            method: ratchet_lib::types::HttpMethod::Post,
            headers: std::collections::HashMap::new(),
            timeout: std::time::Duration::from_secs(30),
            retry_policy: ratchet_lib::output::RetryPolicy::default(),
            auth: None,
            content_type: Some("application/json".to_string()),
        }
    ];
    
    let mut job = Job::new(
        created_task.id,
        json!({"test": "query-data"}),
        JobPriority::High,
    );
    job.output_destinations = Some(serde_json::to_value(&output_destinations).unwrap());
    let _created_job = repos.job_repo.create(job).await.unwrap();
    
    let query = r#"
        query {
            jobs(pagination: {page: 1, limit: 10}) {
                jobs {
                    id
                    taskId
                    priority
                    status
                    outputDestinations {
                        ... on FilesystemDestination {
                            path
                            format
                        }
                        ... on WebhookDestination {
                            url
                            method
                            contentType
                        }
                    }
                }
                total
                page
                limit
            }
        }
    "#;
    
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let jobs_data = &data["jobs"];
    
    assert!(jobs_data["total"].as_u64().unwrap() > 0);
    
    let jobs = jobs_data["jobs"].as_array().unwrap();
    assert!(!jobs.is_empty());
    
    let job_data = &jobs[0];
    assert_eq!(job_data["taskId"], created_task.id);
    assert_eq!(job_data["priority"], "HIGH");
    
    let destinations = job_data["outputDestinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 2);
    
    // Check that both destination types are present
    let mut has_filesystem = false;
    let mut has_webhook = false;
    
    for dest in destinations {
        if dest.get("path").is_some() {
            has_filesystem = true;
            assert_eq!(dest["format"], "JSON");
        }
        if dest.get("url").is_some() {
            has_webhook = true;
            assert_eq!(dest["method"], "POST");
            assert_eq!(dest["contentType"], "application/json");
        }
    }
    
    assert!(has_filesystem);
    assert!(has_webhook);
}

#[tokio::test]
async fn test_graphql_test_destinations_with_templates() {
    let (schema, _repos, temp_dir) = create_test_schema().await;
    
    let query = format!(r#"
        mutation {{
            testOutputDestinations(input: {{
                destinations: [
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: JSON,
                            createDirs: true
                        }}
                    }},
                    {{
                        destinationType: WEBHOOK,
                        webhook: {{
                            url: "https://{{{{env}}}}.example.com/webhook/{{{{job_id}}}}",
                            method: POST,
                            headers: {{
                                "X-Task": "{{{{task_name}}}}",
                                "X-Environment": "{{{{env}}}}"
                            }},
                            timeoutSeconds: 30
                        }}
                    }}
                ]
            }}) {{
                index
                destinationType
                success
                error
            }}
        }}
    "#, temp_dir.path().join("{{task_name}}_{{timestamp}}.json").to_string_lossy());
    
    let result = schema.execute(&query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let test_results = data["testOutputDestinations"].as_array().unwrap();
    assert_eq!(test_results.len(), 2);
    
    // Both destinations should succeed with template validation
    for result in test_results {
        assert_eq!(result["success"], true);
        assert!(result["error"].is_null());
    }
}

#[tokio::test]
async fn test_graphql_test_destinations_validation_error() {
    let (schema, _repos, _temp_dir) = create_test_schema().await;
    
    let query = r#"
        mutation {
            testOutputDestinations(input: {
                destinations: [
                    {
                        destinationType: FILESYSTEM,
                        filesystem: {
                            path: "",
                            format: JSON
                        }
                    }
                ]
            }) {
                index
                destinationType
                success
                error
            }
        }
    "#;
    
    let result = schema.execute(query).await;
    assert!(!result.errors.is_empty());
    
    let error_message = &result.errors[0].message;
    assert!(error_message.contains("Invalid destination"));
}

#[tokio::test]
async fn test_graphql_webhook_with_retry_policy() {
    let (schema, _repos, _temp_dir) = create_test_schema().await;
    
    let query = r#"
        mutation {
            testOutputDestinations(input: {
                destinations: [
                    {
                        destinationType: WEBHOOK,
                        webhook: {
                            url: "https://httpbin.org/status/500",
                            method: POST,
                            timeoutSeconds: 10,
                            retryPolicy: {
                                maxAttempts: 3,
                                initialDelayMs: 1000,
                                maxDelayMs: 5000,
                                backoffMultiplier: 2.0
                            }
                        }
                    }
                ]
            }) {
                index
                destinationType
                success
                error
                estimatedTimeMs
            }
        }
    "#;
    
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let test_results = data["testOutputDestinations"].as_array().unwrap();
    assert_eq!(test_results.len(), 1);
    
    let webhook_result = &test_results[0];
    assert_eq!(webhook_result["destinationType"], "webhook");
    assert_eq!(webhook_result["success"], true);
}

#[tokio::test]
async fn test_graphql_webhook_with_authentication() {
    let (schema, _repos, _temp_dir) = create_test_schema().await;
    
    let query = r#"
        mutation {
            testOutputDestinations(input: {
                destinations: [
                    {
                        destinationType: WEBHOOK,
                        webhook: {
                            url: "https://httpbin.org/bearer",
                            method: POST,
                            timeoutSeconds: 10,
                            auth: {
                                authType: "bearer",
                                token: "test-token-123"
                            }
                        }
                    }
                ]
            }) {
                index
                destinationType
                success
                error
            }
        }
    "#;
    
    let result = schema.execute(query).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let test_results = data["testOutputDestinations"].as_array().unwrap();
    assert_eq!(test_results.len(), 1);
    
    let webhook_result = &test_results[0];
    assert_eq!(webhook_result["destinationType"], "webhook");
    assert_eq!(webhook_result["success"], true);
}

#[tokio::test]
async fn test_graphql_multiple_output_formats() {
    let (schema, repos, temp_dir) = create_test_schema().await;
    
    // Create a test task
    let task = Task {
        id: 0,
        uuid: Uuid::new_v4(),
        name: "format-test-task".to_string(),
        description: Some("Test task for multiple formats".to_string()),
        version: "1.0.0".to_string(),
        path: "/test/path".to_string(),
        metadata: Json::from(json!({})),
        input_schema: Json::from(json!({"type": "object"})),
        output_schema: Json::from(json!({"type": "object"})),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    let created_task = repos.task_repo.create(task).await.unwrap();
    
    let mutation = format!(r#"
        mutation {{
            executeTask(input: {{
                taskId: {},
                inputData: {{"test": "formats"}},
                priority: NORMAL,
                outputDestinations: [
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: JSON
                        }}
                    }},
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: YAML
                        }}
                    }},
                    {{
                        destinationType: FILESYSTEM,
                        filesystem: {{
                            path: "{}",
                            format: CSV
                        }}
                    }}
                ]
            }}) {{
                id
                outputDestinations {{
                    ... on FilesystemDestination {{
                        path
                        format
                    }}
                }}
            }}
        }}
    "#, 
        created_task.id,
        temp_dir.path().join("output.json").to_string_lossy(),
        temp_dir.path().join("output.yaml").to_string_lossy(),
        temp_dir.path().join("output.csv").to_string_lossy()
    );
    
    let result = schema.execute(&mutation).await;
    assert!(result.errors.is_empty());
    
    let data = result.data.into_json().unwrap();
    let job_data = &data["executeTask"];
    
    let destinations = job_data["outputDestinations"].as_array().unwrap();
    assert_eq!(destinations.len(), 3);
    
    // Check that all formats are represented
    let mut formats = std::collections::HashSet::new();
    for dest in destinations {
        formats.insert(dest["format"].as_str().unwrap());
    }
    
    assert!(formats.contains("JSON"));
    assert!(formats.contains("YAML"));
    assert!(formats.contains("CSV"));
}