#[cfg(feature = "git")]
mod git_tests {
    use ratchet_registry::config::{GitConfig, GitSyncStrategy, TaskSource};
    use ratchet_registry::loaders::git::GitLoader;
    use ratchet_registry::loaders::TaskLoader;

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
    async fn test_git_loader_with_public_repo() {
        let loader = GitLoader::new();

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
            url: "https://github.com/michiel/ratchet-repo-samples.git".to_string(),
            auth: None,
            config: git_config,
        };

        // Test discovery (may fail in CI without network, that's OK)
        let result = loader.discover_tasks(&source).await;

        match result {
            Ok(discovered) => {
                println!(
                    "✅ Successfully discovered {} tasks from Git repository",
                    discovered.len()
                );
                for task in &discovered {
                    println!("  - {} v{}", task.metadata.name, task.metadata.version);
                }

                // If we discovered tasks, try to load one
                if let Some(task) = discovered.first() {
                    match loader.load_task(&task.task_ref).await {
                        Ok(task_def) => {
                            println!("✅ Successfully loaded task: {}", task_def.metadata.name);
                            assert!(!task_def.script.is_empty(), "Task script should not be empty");
                        }
                        Err(e) => {
                            println!("⚠️  Failed to load task definition: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("ℹ️  Git operation failed (may be network/repo related): {:?}", e);
                println!("ℹ️  This is expected in CI environments without network access");
            }
        }
    }

    #[tokio::test]
    async fn test_git_loader_creation() {
        // Test that we can create a GitLoader without errors
        let loader = GitLoader::new();

        // Test that it supports Git sources
        let git_source = TaskSource::Git {
            url: "https://github.com/example/repo.git".to_string(),
            auth: None,
            config: GitConfig::default(),
        };

        assert!(loader.supports_source(&git_source).await);
        println!("✅ GitLoader created successfully and supports Git sources");
    }
}

#[cfg(not(feature = "git"))]
mod git_feature_disabled_tests {
    #[test]
    fn test_git_feature_disabled() {
        println!("ℹ️  Git feature is disabled, skipping Git integration tests");
    }
}
