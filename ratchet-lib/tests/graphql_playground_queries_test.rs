use ratchet_lib::{
    config::{DatabaseConfig, RatchetConfig},
    database::{
        connection::DatabaseConnection, 
        repositories::RepositoryFactory,
        entities::tasks::ActiveModel as TaskActiveModel,
    },
    execution::{
        job_queue::{JobQueueConfig, JobQueueManager},
        process_executor::ProcessTaskExecutor,
    },
    graphql::schema::{create_schema, RatchetSchema},
};
use async_graphql::{Request, Variables};
use serde_json::json;
use std::sync::Arc;
use regex::Regex;
use sea_orm::{ActiveModelTrait, Set};
use chrono::Utc;

type TestSchema = RatchetSchema;

/// Test helper to create GraphQL schema with test database
async fn create_test_schema() -> (TestSchema, RepositoryFactory) {
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
    
    let config = RatchetConfig::default();
    let task_executor = Arc::new(ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap());
    
    let schema = create_schema(
        repositories.clone(),
        job_queue,
        task_executor,
        None, // No registry for tests
        None, // No task sync service for tests
    );
    
    (schema, repositories)
}

/// Extract GraphQL queries and variables from the playground handler HTML
fn extract_playground_queries() -> Vec<(String, String, String)> {
    let handler_content = include_str!("../src/server/handlers.rs");
    
    // Find the tabs array
    let tabs_start = handler_content.find("tabs: [").expect("tabs array not found");
    let tabs_end = handler_content[tabs_start..].find("]").expect("tabs array end not found") + tabs_start;
    let tabs_content = &handler_content[tabs_start..tabs_end];
    
    let mut queries = Vec::new();
    
    // Extract each tab
    let tab_regex = Regex::new(r"name:\s*'([^']+)'[^}]*query:\s*`([^`]+)`(?:[^}]*variables:\s*'([^']+)')?").unwrap();
    
    for cap in tab_regex.captures_iter(tabs_content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let query = cap.get(2).unwrap().as_str().to_string();
        let variables = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_else(|| "{}".to_string());
        queries.push((name, query, variables));
    }
    
    queries
}

#[tokio::test]
async fn test_list_all_tasks_query() {
    let (schema, _repos) = create_test_schema().await;
    
    let query = r#"
        query ListAllTasks {
            tasks {
                items {
                    id
                    uuid
                    name
                    description
                    version
                    availableVersions
                    registrySource
                    enabled
                    createdAt
                    updatedAt
                    validatedAt
                    inSync
                }
                meta {
                    page
                    limit
                    total
                    totalPages
                    hasNext
                    hasPrevious
                }
            }
        }
    "#;
    
    let request = Request::new(query);
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Query should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Query should return data");
}

#[tokio::test]
async fn test_task_executions_query() {
    let (schema, _repos) = create_test_schema().await;
    
    let query = r#"
        query TaskExecutions($taskId: String) {
            executions(taskId: $taskId) {
                items {
                    id
                    uuid
                    taskId
                    input
                    output
                    status
                    errorMessage
                    queuedAt
                    startedAt
                    completedAt
                    durationMs
                }
                meta {
                    page
                    limit
                    total
                    totalPages
                    hasNext
                    hasPrevious
                }
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "taskId": null
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Query should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Query should return data");
}

#[tokio::test]
async fn test_execute_task_mutation() {
    let (schema, repos) = create_test_schema().await;
    
    // First create a test task
    let task_uuid = uuid::Uuid::new_v4();
    let task = TaskActiveModel {
        uuid: Set(task_uuid),
        name: Set("Test Task".to_string()),
        description: Set(Some("Test task description".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(None),
        ..Default::default()
    };
    let created_task = task.insert(repos.database().get_connection()).await.unwrap();
    
    let query = r#"
        mutation ExecuteTask($input: ExecuteTaskInput!) {
            executeTask(input: $input) {
                id
                taskId
                priority
                status
                retryCount
                maxRetries
                queuedAt
                scheduledFor
                errorMessage
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "input": {
            "taskId": created_task.id.to_string(),
            "inputData": {},
            "priority": "NORMAL"
        }
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Mutation should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Mutation should return data");
}

#[tokio::test]
async fn test_execute_task_direct_mutation() {
    let (schema, repos) = create_test_schema().await;
    
    // First create a test task
    let task_uuid = uuid::Uuid::new_v4();
    let task = TaskActiveModel {
        uuid: Set(task_uuid),
        name: Set("Test Task".to_string()),
        description: Set(Some("Test task description".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(None),
        ..Default::default()
    };
    let created_task = task.insert(repos.database().get_connection()).await.unwrap();
    
    let query = r#"
        mutation ExecuteTaskDirect($taskId: String!, $inputData: JSON!) {
            executeTaskDirect(taskId: $taskId, inputData: $inputData) {
                success
                output
                error
                durationMs
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "taskId": created_task.id.to_string(),
        "inputData": {}
    })));
    
    let response = schema.execute(request).await;
    
    // This might fail due to missing task implementation, but should not have schema errors
    if !response.errors.is_empty() {
        // Check if it's a schema error or execution error
        let error_message = response.errors[0].message.to_string();
        assert!(!error_message.contains("Cannot query field"), 
            "Should not have schema errors: {:?}", response.errors);
    }
}

#[tokio::test]
async fn test_system_health_query() {
    let (schema, _repos) = create_test_schema().await;
    
    let query = r#"
        query SystemHealth {
            health {
                database
                jobQueue
                scheduler
                message
            }
            taskStats {
                totalTasks
                enabledTasks
                disabledTasks
            }
            executionStats {
                totalExecutions
                pending
                running
                completed
                failed
            }
            jobStats {
                totalJobs
                queued
                processing
                completed
                failed
                retrying
            }
        }
    "#;
    
    let request = Request::new(query);
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Query should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Query should return data");
}

#[tokio::test]
async fn test_jobs_queue_query() {
    let (schema, _repos) = create_test_schema().await;
    
    let query = r#"
        query JobsQueue($status: JobStatus) {
            jobs(status: $status) {
                items {
                    id
                    taskId
                    priority
                    status
                    retryCount
                    maxRetries
                    queuedAt
                    scheduledFor
                    errorMessage
                    outputDestinations {
                        destinationType
                        template
                    }
                }
                meta {
                    page
                    limit
                    total
                    totalPages
                    hasNext
                    hasPrevious
                }
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "status": null
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Query should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Query should return data");
}

#[tokio::test]
async fn test_get_task_by_uuid_query() {
    let (schema, repos) = create_test_schema().await;
    
    // Create a test task
    let task_uuid = uuid::Uuid::new_v4();
    let task = TaskActiveModel {
        uuid: Set(task_uuid),
        name: Set("Test Task".to_string()),
        description: Set(Some("Test task description".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(None),
        ..Default::default()
    };
    task.insert(repos.database().get_connection()).await.unwrap();
    
    let query = r#"
        query GetTaskByUUID($uuid: UUID!, $version: String) {
            task(uuid: $uuid, version: $version) {
                id
                uuid
                name
                description
                version
                availableVersions
                registrySource
                enabled
                createdAt
                updatedAt
                validatedAt
                inSync
                inputSchema
                outputSchema
                metadata
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "uuid": task_uuid.to_string(),
        "version": null
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Query should not have errors: {:?}", response.errors);
}

#[tokio::test]
async fn test_update_task_status_mutation() {
    let (schema, repos) = create_test_schema().await;
    
    // Create a test task
    let task_uuid = uuid::Uuid::new_v4();
    let task = TaskActiveModel {
        uuid: Set(task_uuid),
        name: Set("Test Task".to_string()),
        description: Set(Some("Test task description".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(None),
        ..Default::default()
    };
    let created_task = task.insert(repos.database().get_connection()).await.unwrap();
    
    let query = r#"
        mutation UpdateTaskStatus($id: String!, $enabled: Boolean!) {
            updateTaskStatus(id: $id, enabled: $enabled) {
                id
                uuid
                name
                enabled
                updatedAt
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "id": created_task.id.to_string(),
        "enabled": true
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Mutation should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Mutation should return data");
}

#[tokio::test]
async fn test_test_output_destinations_mutation() {
    let (schema, _repos) = create_test_schema().await;
    
    let query = r#"
        mutation TestOutputDestinations($input: TestOutputDestinationsInput!) {
            testOutputDestinations(input: $input) {
                index
                destinationType
                success
                error
                estimatedTimeMs
            }
        }
    "#;
    
    let mut request = Request::new(query);
    request = request.variables(Variables::from_json(json!({
        "input": {
            "destinations": [{
                "destinationType": "FILESYSTEM",
                "filesystem": {
                    "path": "/tmp/test.json",
                    "format": "JSON"
                }
            }]
        }
    })));
    
    let response = schema.execute(request).await;
    
    assert!(response.errors.is_empty(), "Mutation should not have errors: {:?}", response.errors);
    assert!(response.data != async_graphql::Value::Null, "Mutation should return data");
}

#[tokio::test]
async fn test_all_playground_queries_are_valid() {
    let (schema, repos) = create_test_schema().await;
    let queries = extract_playground_queries();
    
    assert!(!queries.is_empty(), "Should extract playground queries");
    
    // Create a test task for queries that need it
    let task_uuid = uuid::Uuid::new_v4();
    let task = TaskActiveModel {
        uuid: Set(task_uuid),
        name: Set("Test Task".to_string()),
        description: Set(Some("Test task description".to_string())),
        version: Set("1.0.0".to_string()),
        path: Set("/test/path".to_string()),
        metadata: Set(json!({"test": true})),
        input_schema: Set(json!({"type": "object"})),
        output_schema: Set(json!({"type": "object"})),
        enabled: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        validated_at: Set(None),
        ..Default::default()
    };
    let created_task = task.insert(repos.database().get_connection()).await.unwrap();
    
    for (name, query, variables_str) in queries {
        println!("Testing query: {}", name);
        
        // Parse and fix variables if needed
        let mut variables_json: serde_json::Value = serde_json::from_str(&variables_str)
            .unwrap_or_else(|_| json!({}));
        
        // Replace placeholder task IDs with real ones
        if let Some(input) = variables_json.get_mut("input") {
            if let Some(task_id) = input.get_mut("taskId") {
                if task_id == "1" || task_id == 1 {
                    *task_id = json!(created_task.id.to_string());
                }
            }
        }
        if let Some(task_id) = variables_json.get_mut("taskId") {
            if task_id == "1" || task_id == 1 {
                *task_id = json!(created_task.id.to_string());
            }
        }
        if let Some(id) = variables_json.get_mut("id") {
            if id == "1" || id == 1 {
                *id = json!(created_task.id.to_string());
            }
        }
        
        let mut request = Request::new(&query);
        if variables_str != "{}" {
            request = request.variables(Variables::from_json(variables_json));
        }
        
        let response = schema.execute(request).await;
        
        // Check for schema-level errors (field not found, wrong types, etc.)
        if !response.errors.is_empty() {
            let error_messages: Vec<String> = response.errors.iter()
                .map(|e| e.message.clone())
                .collect();
            
            // Filter out runtime errors, only fail on schema errors
            let schema_errors: Vec<String> = error_messages.iter()
                .filter(|msg| {
                    msg.contains("Cannot query field") ||
                    msg.contains("Unknown field") ||
                    msg.contains("Unknown argument") ||
                    msg.contains("Invalid value for argument") ||
                    msg.contains("Expected type") ||
                    msg.contains("Cannot return null for non-nullable field")
                })
                .cloned()
                .collect();
            
            assert!(
                schema_errors.is_empty(),
                "Query '{}' has schema errors: {:?}",
                name,
                schema_errors
            );
        }
    }
}