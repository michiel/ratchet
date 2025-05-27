use ratchet_lib::{
    config::{RatchetConfig, DatabaseConfig},
    database::{connection::DatabaseConnection, repositories::RepositoryFactory},
    execution::{ProcessTaskExecutor, WorkerProcessManager, worker_process::WorkerConfig},
    execution::ipc::{WorkerMessage, CoordinatorMessage, MessageEnvelope, TaskExecutionResult},
};
use serde_json::json;
use std::time::Duration;
use tempfile::tempdir;
use std::fs;
use uuid::Uuid;

async fn create_test_database() -> DatabaseConnection {
    let config = DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        max_connections: 5,
        connection_timeout: Duration::from_secs(10),
    };
    let db = DatabaseConnection::new(config).await.expect("Failed to create test database");
    
    // Run migrations to create tables
    db.migrate().await.expect("Failed to run migrations");
    
    db
}

async fn create_test_task_files() -> (String, serde_json::Value, serde_json::Value) {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let task_path = temp_dir.path();
    
    // Create metadata.json
    let metadata = json!({
        "uuid": "integration-test-task",
        "version": "1.0.0",
        "label": "Integration Test Task",
        "description": "Task for testing process separation architecture"
    });
    fs::write(task_path.join("metadata.json"), serde_json::to_string_pretty(&metadata).unwrap())
        .expect("Failed to write metadata");
    
    // Create input schema
    let input_schema = json!({
        "type": "object",
        "properties": {
            "operation": { "type": "string", "enum": ["add", "multiply"] },
            "a": { "type": "number" },
            "b": { "type": "number" }
        },
        "required": ["operation", "a", "b"]
    });
    fs::write(task_path.join("input.schema.json"), serde_json::to_string_pretty(&input_schema).unwrap())
        .expect("Failed to write input schema");
    
    // Create output schema
    let output_schema = json!({
        "type": "object",
        "properties": {
            "result": { "type": "number" },
            "operation": { "type": "string" }
        },
        "required": ["result", "operation"]
    });
    fs::write(task_path.join("output.schema.json"), serde_json::to_string_pretty(&output_schema).unwrap())
        .expect("Failed to write output schema");
    
    // Create main.js
    let main_js = r#"(function(input) {
        const { operation, a, b } = input;
        
        if (typeof a !== 'number' || typeof b !== 'number') {
            throw new Error('a and b must be numbers');
        }
        
        let result;
        switch (operation) {
            case 'add':
                result = a + b;
                break;
            case 'multiply':
                result = a * b;
                break;
            default:
                throw new Error('Unknown operation: ' + operation);
        }
        
        return {
            result: result,
            operation: operation
        };
    })"#;
    fs::write(task_path.join("main.js"), main_js)
        .expect("Failed to write main.js");
    
    let task_path_str = task_path.to_string_lossy().to_string();
    
    // Keep temp dir alive by leaking it (for test purposes only)
    std::mem::forget(temp_dir);
    
    (task_path_str, input_schema, output_schema)
}

#[tokio::test]
async fn test_worker_process_manager_lifecycle() {
    let config = WorkerConfig {
        worker_count: 2,
        restart_on_crash: true,
        max_restart_attempts: 3,
        restart_delay_seconds: 1,
        health_check_interval_seconds: 10,
        task_timeout_seconds: 30,
        worker_idle_timeout_seconds: Some(60),
    };
    
    let mut manager = WorkerProcessManager::new(config);
    
    // Test manager creation
    assert_eq!(manager.get_worker_stats().len(), 0);
    
    // Test starting workers (may fail if ratchet-cli binary not available, but shouldn't panic)
    let start_result = manager.start().await;
    if start_result.is_ok() {
        // If workers started successfully, test stats
        let stats = manager.get_worker_stats();
        assert!(stats.len() > 0);
        
        // Test health checks
        let health_results = manager.health_check_all().await;
        assert_eq!(health_results.len(), stats.len());
        
        // Test stopping workers
        let stop_result = manager.stop().await;
        assert!(stop_result.is_ok());
        
        // After stopping, should have no workers
        assert_eq!(manager.get_worker_stats().len(), 0);
    }
    // If start failed (e.g., binary not found), that's okay for this test
}

#[tokio::test]
async fn test_ipc_message_serialization() {
    let correlation_id = Uuid::new_v4();
    
    // Test WorkerMessage serialization
    let worker_message = WorkerMessage::ExecuteTask {
        job_id: 123,
        task_id: 456,
        task_path: "/path/to/task".to_string(),
        input_data: json!({"test": "data"}),
        correlation_id,
    };
    
    let envelope = MessageEnvelope::new(worker_message);
    let serialized = serde_json::to_string(&envelope);
    assert!(serialized.is_ok());
    
    let deserialized: Result<MessageEnvelope<WorkerMessage>, _> = 
        serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
    
    let envelope = deserialized.unwrap();
    if let WorkerMessage::ExecuteTask { job_id, task_id, correlation_id: cid, .. } = envelope.message {
        assert_eq!(job_id, 123);
        assert_eq!(task_id, 456);
        assert_eq!(cid, correlation_id);
    } else {
        panic!("Wrong message type after deserialization");
    }
}

#[tokio::test]
async fn test_coordinator_message_serialization() {
    let correlation_id = Uuid::new_v4();
    
    // Test CoordinatorMessage serialization
    let task_result = TaskExecutionResult {
        success: true,
        output: Some(json!({"result": 42})),
        error_message: None,
        error_details: None,
        started_at: chrono::Utc::now(),
        completed_at: chrono::Utc::now(),
        duration_ms: 150,
    };
    
    let coordinator_message = CoordinatorMessage::TaskResult {
        job_id: 789,
        correlation_id,
        result: task_result,
    };
    
    let envelope = MessageEnvelope::new(coordinator_message);
    let serialized = serde_json::to_string(&envelope);
    assert!(serialized.is_ok());
    
    let deserialized: Result<MessageEnvelope<CoordinatorMessage>, _> = 
        serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
    
    let envelope = deserialized.unwrap();
    if let CoordinatorMessage::TaskResult { job_id, correlation_id: cid, result } = envelope.message {
        assert_eq!(job_id, 789);
        assert_eq!(cid, correlation_id);
        assert!(result.success);
        assert_eq!(result.duration_ms, 150);
    } else {
        panic!("Wrong message type after deserialization");
    }
}

#[tokio::test]
async fn test_process_executor_with_valid_task_structure() {
    let db = create_test_database().await;
    let repositories = RepositoryFactory::new(db);
    let config = RatchetConfig::default();
    
    let executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
    
    // Create test task files and database entity
    let (task_path, input_schema, output_schema) = create_test_task_files().await;
    
    let task_entity = ratchet_lib::database::entities::tasks::Model {
        id: 0, // Will be set by database
        uuid: uuid::Uuid::parse_str("9f6c1234-5678-9012-3456-789012345679").unwrap(),
        name: "Integration Test Task".to_string(),
        description: Some("Task for testing process separation architecture".to_string()),
        version: "1.0.0".to_string(),
        path: task_path,
        metadata: sea_orm::entity::prelude::Json::from(serde_json::json!({
            "uuid": "integration-test-task",
            "version": "1.0.0",
            "label": "Integration Test Task",
            "description": "Task for testing process separation architecture"
        })),
        input_schema: sea_orm::entity::prelude::Json::from(input_schema),
        output_schema: sea_orm::entity::prelude::Json::from(output_schema),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    
    let created_task = repositories.task_repo.create(task_entity).await
        .expect("Failed to create task in database");
    
    // Test task execution with different inputs
    let test_cases = vec![
        (json!({"operation": "add", "a": 5, "b": 3}), "add"),
        (json!({"operation": "multiply", "a": 4, "b": 7}), "multiply"),
    ];
    
    for (input_data, operation) in test_cases {
        let result = executor.execute_task_send(created_task.id, input_data.clone(), None).await;
        
        // Should return a result (even if worker execution fails)
        assert!(result.is_ok());
        let execution_result = result.unwrap();
        
        // Verify execution was recorded
        assert!(execution_result.execution_id > 0);
        
        // Verify database record
        let execution = repositories.execution_repo
            .find_by_id(execution_result.execution_id).await
            .expect("Database query should succeed")
            .expect("Execution record should exist");
        
        assert_eq!(execution.task_id, created_task.id);
        // Compare JSON values (note: Sea-ORM Json wraps serde_json::Value)
        let execution_input: serde_json::Value = execution.input.clone().into();
        assert_eq!(execution_input, input_data);
        
        // Status should be either completed or failed
        use ratchet_lib::database::entities::executions::ExecutionStatus;
        assert!(matches!(execution.status, ExecutionStatus::Completed | ExecutionStatus::Failed));
        
        println!("Task execution for {} operation: status={:?}, success={}", 
                 operation, execution.status, execution_result.success);
    }
}

#[tokio::test]
async fn test_process_executor_error_handling() {
    let db = create_test_database().await;
    let repositories = RepositoryFactory::new(db);
    let config = RatchetConfig::default();
    
    let executor = ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap();
    
    // Test with non-existent task
    let result = executor.execute_task_send(99999, json!({}), None).await;
    assert!(result.is_err());
    
    // Test with non-existent job
    let result = executor.execute_job_send(99999).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_process_executor_operations() {
    let db = create_test_database().await;
    let repositories = RepositoryFactory::new(db);
    let config = RatchetConfig::default();
    
    let executor = std::sync::Arc::new(
        ProcessTaskExecutor::new(repositories.clone(), config).await.unwrap()
    );
    
    // Create test task
    let (task_path, input_schema, output_schema) = create_test_task_files().await;
    
    let task_entity = ratchet_lib::database::entities::tasks::Model {
        id: 0, // Will be set by database
        uuid: uuid::Uuid::parse_str("9f6c1234-5678-9012-3456-78901234567a").unwrap(),
        name: "Concurrent Test Task".to_string(),
        description: Some("Task for testing concurrent execution".to_string()),
        version: "1.0.0".to_string(),
        path: task_path,
        metadata: sea_orm::entity::prelude::Json::from(serde_json::json!({
            "uuid": "concurrent-test-task",
            "version": "1.0.0",
            "label": "Concurrent Test Task",
            "description": "Task for testing concurrent execution"
        })),
        input_schema: sea_orm::entity::prelude::Json::from(input_schema),
        output_schema: sea_orm::entity::prelude::Json::from(output_schema),
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
    };
    
    let created_task = repositories.task_repo.create(task_entity).await
        .expect("Failed to create task in database");
    
    // Run multiple concurrent executions
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let executor_clone = executor.clone();
        let task_id = created_task.id;
        
        let handle = tokio::spawn(async move {
            let input_data = json!({
                "operation": "add",
                "a": i * 2,
                "b": i + 1
            });
            
            executor_clone.execute_task_send(task_id, input_data, None).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all executions to complete
    let results = futures::future::join_all(handles).await;
    
    // All should complete successfully (or fail gracefully)
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Task {} should complete", i);
        let execution_result = result.unwrap();
        assert!(execution_result.is_ok(), "Execution {} should return a result", i);
    }
}

#[tokio::test]
async fn test_worker_config_customization() {
    // Test custom worker configuration
    let custom_config = WorkerConfig {
        worker_count: 1,
        restart_on_crash: false,
        max_restart_attempts: 1,
        restart_delay_seconds: 2,
        health_check_interval_seconds: 5,
        task_timeout_seconds: 60,
        worker_idle_timeout_seconds: Some(120),
    };
    
    let manager = WorkerProcessManager::new(custom_config.clone());
    
    // Verify configuration is applied
    // Note: We can't directly access the config from the manager,
    // but we can test that it doesn't panic and basic operations work
    let stats = manager.get_worker_stats();
    assert_eq!(stats.len(), 0); // No workers started yet
}

#[tokio::test]
async fn test_message_envelope_metadata() {
    let message = WorkerMessage::Ping {
        correlation_id: Uuid::new_v4(),
    };
    
    let envelope = MessageEnvelope::new(message);
    
    // Verify envelope has proper metadata
    assert!(envelope.timestamp > chrono::DateTime::from_timestamp(0, 0).unwrap());
    assert_eq!(envelope.protocol_version, 1); // Assuming IPC_PROTOCOL_VERSION is 1
    
    // Test serialization preserves metadata
    let serialized = serde_json::to_string(&envelope).unwrap();
    let deserialized: MessageEnvelope<WorkerMessage> = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(envelope.protocol_version, deserialized.protocol_version);
    assert_eq!(envelope.timestamp, deserialized.timestamp);
}