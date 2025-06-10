use ratchet_registry::prelude::*;
use ratchet_registry::TaskLoader;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_filesystem_loader() {
    let temp_dir = TempDir::new().unwrap();
    let task_dir = temp_dir.path().join("test-task");
    std::fs::create_dir_all(&task_dir).unwrap();

    // Create a simple task
    let metadata = serde_json::json!({
        "name": "test-task",
        "version": "1.0.0",
        "description": "A test task"
    });

    std::fs::write(
        task_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).unwrap(),
    ).unwrap();

    std::fs::write(
        task_dir.join("main.js"),
        "console.log('Hello, world!');",
    ).unwrap();

    // Test filesystem loader
    let loader = ratchet_registry::FilesystemLoader::new();
    let source = TaskSource::Filesystem {
        path: temp_dir.path().to_string_lossy().to_string(),
        recursive: true,
        watch: false,
    };

    let discovered = loader.discover_tasks(&source).await.unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].metadata.name, "test-task");
    assert_eq!(discovered[0].metadata.version, "1.0.0");
}

#[tokio::test]
async fn test_registry_service() {
    let config = RegistryConfig {
        sources: vec![],
        sync_interval: std::time::Duration::from_secs(300),
        enable_auto_sync: false,
        enable_validation: true,
        cache_config: Default::default(),
    };

    let service = DefaultRegistryService::new(config);
    let registry = service.registry().await;

    // Test that we can access the registry
    let tasks = registry.list_tasks().await.unwrap();
    assert_eq!(tasks.len(), 0); // Empty initially
}

#[tokio::test]
async fn test_task_validation() {
    let validator = ratchet_registry::loaders::validation::TaskValidator::new();
    
    let task_def = TaskDefinition {
        reference: TaskReference {
            name: "test-task".to_string(),
            version: "1.0.0".to_string(),
            source: "file:///test".to_string(),
        },
        metadata: TaskMetadata {
            uuid: uuid::Uuid::new_v4(),
            name: "test-task".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test task".to_string()),
            tags: vec!["test".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            checksum: None,
        },
        script: "console.log('Hello');".to_string(),
        input_schema: None,
        output_schema: None,
        dependencies: vec![],
        environment: std::collections::HashMap::new(),
    };

    let result = validator.validate(&task_def).await.unwrap();
    assert!(result.is_valid);
}