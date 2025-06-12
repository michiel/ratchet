#[cfg(feature = "git")]
mod git_tests {
    use ratchet_registry::config::{TaskSource, GitConfig, GitSyncStrategy};
    use ratchet_registry::loaders::git::GitLoader;
    use ratchet_registry::loaders::TaskLoader;
    use ratchet_registry::types::TaskReference;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio_test;

    async fn create_test_git_repo(temp_dir: &TempDir) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let repo_path = temp_dir.path().join("test_repo");
        std::fs::create_dir_all(&repo_path)?;

        // Initialize Git repository
        let repo = git2::Repository::init(&repo_path)?;

        // Create .ratchet directory
        let ratchet_dir = repo_path.join(".ratchet");
        std::fs::create_dir_all(&ratchet_dir)?;

        // Create registry.yaml
        let registry_yaml = r#"
name: "Test Task Repository"
description: "Test repository for Git integration"
version: "1.0.0"
ratchet_version: ">=0.6.0"
discovery:
  auto_index: true
  include_patterns:
    - "tasks/**"
"#;
        std::fs::write(ratchet_dir.join("registry.yaml"), registry_yaml)?;

        // Create tasks directory
        let tasks_dir = repo_path.join("tasks");
        std::fs::create_dir_all(&tasks_dir)?;

        // Create a sample task
        let task_dir = tasks_dir.join("sample-task");
        std::fs::create_dir_all(&task_dir)?;

        // metadata.json
        let metadata = r#"{
  "name": "sample-task",
  "version": "1.0.0",
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "description": "A sample task for testing",
  "tags": ["test", "sample"]
}"#;
        std::fs::write(task_dir.join("metadata.json"), metadata)?;

        // main.js
        let main_js = r#"function(input) {
  return {
    message: "Hello from Git task!",
    input: input
  };
}"#;
        std::fs::write(task_dir.join("main.js"), main_js)?;

        // input.schema.json
        let input_schema = r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}"#;
        std::fs::write(task_dir.join("input.schema.json"), input_schema)?;

        // output.schema.json
        let output_schema = r#"{
  "type": "object",
  "properties": {
    "message": { "type": "string" },
    "input": { "type": "object" }
  }
}"#;
        std::fs::write(task_dir.join("output.schema.json"), output_schema)?;

        // Commit files to Git
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let signature = git2::Signature::now("Test User", "test@example.com")?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit with sample task",
            &tree,
            &[],
        )?;

        Ok(repo_path)
    }

    #[tokio::test]
    async fn test_git_loader_discover_tasks() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = create_test_git_repo(&temp_dir).await
            .expect("Failed to create test Git repository");

        let git_config = GitConfig {
            branch: "main".to_string(),
            subdirectory: None,
            shallow: true,
            depth: Some(1),
            sync_strategy: GitSyncStrategy::Fetch,
            cleanup_on_error: true,
            verify_signatures: false,
            allowed_refs: None,
            timeout: std::time::Duration::from_secs(60),
            max_repo_size: None,
            local_cache_path: None,
            cache_ttl: std::time::Duration::from_secs(3600),
            keep_history: false,
        };

        let source = TaskSource::Git {
            url: format!("file://{}", repo_path.display()),
            auth: None,
            config: git_config,
        };

        let loader = GitLoader::new();
        let discovered = loader.discover_tasks(&source).await
            .expect("Failed to discover tasks");

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].metadata.name, "sample-task");
        assert_eq!(discovered[0].metadata.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_git_loader_load_task() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = create_test_git_repo(&temp_dir).await
            .expect("Failed to create test Git repository");

        let git_config = GitConfig {
            branch: "main".to_string(),
            subdirectory: None,
            shallow: true,
            depth: Some(1),
            sync_strategy: GitSyncStrategy::Fetch,
            cleanup_on_error: true,
            verify_signatures: false,
            allowed_refs: None,
            timeout: std::time::Duration::from_secs(60),
            max_repo_size: None,
            local_cache_path: None,
            cache_ttl: std::time::Duration::from_secs(3600),
            keep_history: false,
        };

        let source = TaskSource::Git {
            url: format!("file://{}", repo_path.display()),
            auth: None,
            config: git_config,
        };

        let loader = GitLoader::new();

        // First discover tasks to get a task reference
        let discovered = loader.discover_tasks(&source).await
            .expect("Failed to discover tasks");

        assert!(!discovered.is_empty());
        let task_ref = &discovered[0].task_ref;

        // Now load the full task definition
        let task_def = loader.load_task(task_ref).await
            .expect("Failed to load task");

        assert_eq!(task_def.metadata.name, "sample-task");
        assert_eq!(task_def.metadata.version, "1.0.0");
        assert!(task_def.script.contains("Hello from Git task!"));
        assert!(task_def.input_schema.is_some());
        assert!(task_def.output_schema.is_some());
    }

    #[tokio::test]
    async fn test_git_loader_supports_source() {
        let loader = GitLoader::new();

        let git_source = TaskSource::Git {
            url: "https://github.com/example/repo.git".to_string(),
            auth: None,
            config: GitConfig::default(),
        };

        let filesystem_source = TaskSource::Filesystem {
            path: "/tmp/tasks".to_string(),
            recursive: true,
            watch: false,
        };

        assert!(loader.supports_source(&git_source).await);
        assert!(!loader.supports_source(&filesystem_source).await);
    }

    #[tokio::test]
    async fn test_git_loader_with_subdirectory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = create_test_git_repo(&temp_dir).await
            .expect("Failed to create test Git repository");

        // Create a subdirectory with another task
        let production_dir = repo_path.join("production");
        let production_tasks_dir = production_dir.join("tasks");
        std::fs::create_dir_all(&production_tasks_dir).expect("Failed to create production tasks dir");

        let prod_task_dir = production_tasks_dir.join("prod-task");
        std::fs::create_dir_all(&prod_task_dir).expect("Failed to create prod task dir");

        let metadata = r#"{
  "name": "prod-task",
  "version": "2.0.0", 
  "uuid": "550e8400-e29b-41d4-a716-446655440001",
  "description": "A production task",
  "tags": ["production"]
}"#;
        std::fs::write(prod_task_dir.join("metadata.json"), metadata)
            .expect("Failed to write metadata");

        let main_js = r#"function(input) {
  return { production: true, input: input };
}"#;
        std::fs::write(prod_task_dir.join("main.js"), main_js)
            .expect("Failed to write main.js");

        // Commit the new files
        let repo = git2::Repository::open(&repo_path).expect("Failed to open repo");
        let mut index = repo.index().expect("Failed to get index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .expect("Failed to add files");
        index.write().expect("Failed to write index");

        let signature = git2::Signature::now("Test User", "test@example.com")
            .expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let parent_commit = repo.head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel to commit");

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Add production task",
            &tree,
            &[&parent_commit],
        ).expect("Failed to commit");

        let git_config = GitConfig {
            branch: "main".to_string(),
            subdirectory: Some("production".to_string()),
            shallow: true,
            depth: Some(1),
            sync_strategy: GitSyncStrategy::Fetch,
            cleanup_on_error: true,
            verify_signatures: false,
            allowed_refs: None,
            timeout: std::time::Duration::from_secs(60),
            max_repo_size: None,
            local_cache_path: None,
            cache_ttl: std::time::Duration::from_secs(3600),
            keep_history: false,
        };

        let source = TaskSource::Git {
            url: format!("file://{}", repo_path.display()),
            auth: None,
            config: git_config,
        };

        let loader = GitLoader::new();
        let discovered = loader.discover_tasks(&source).await
            .expect("Failed to discover tasks");

        // Should only find the production task, not the root level task
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].metadata.name, "prod-task");
        assert_eq!(discovered[0].metadata.version, "2.0.0");
    }
}

#[cfg(not(feature = "git"))]
mod git_feature_disabled_tests {
    use ratchet_registry::config::{TaskSource, GitConfig};
    use ratchet_registry::loaders::git::GitLoader;
    use ratchet_registry::loaders::TaskLoader;
    use ratchet_registry::error::RegistryError;

    #[tokio::test]
    async fn test_git_loader_without_feature() {
        let loader = GitLoader::new();
        
        let source = TaskSource::Git {
            url: "https://github.com/example/repo.git".to_string(),
            auth: None,
            config: GitConfig::default(),
        };

        let result = loader.discover_tasks(&source).await;
        assert!(result.is_err());
        
        if let Err(RegistryError::NotImplemented(msg)) = result {
            assert!(msg.contains("Git support is not compiled in"));
        } else {
            panic!("Expected NotImplemented error");
        }
    }
}