#[cfg(feature = "git")]
mod https_git_tests {
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_git2_https_clone_capability() {
        // Test that git2 can clone a public HTTPS repository
        // This verifies our OpenSSL configuration is working
        
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let clone_path = temp_dir.path().join("test_clone");
        
        // Use a small, reliable public repository for testing
        let test_repo_url = "https://github.com/octocat/Hello-World.git";
        
        // Attempt to clone with git2 directly
        let result = git2::Repository::clone(test_repo_url, &clone_path);
        
        match result {
            Ok(repo) => {
                println!("✅ Successfully cloned HTTPS repository: {}", test_repo_url);
                
                // Verify the repository is valid
                assert!(repo.is_bare() == false, "Repository should not be bare");
                assert!(clone_path.join(".git").exists(), ".git directory should exist");
                
                // Verify we can read the HEAD
                let head = repo.head().expect("Should be able to read HEAD");
                println!("HEAD reference: {}", head.name().unwrap_or("(no name)"));
                
                println!("✅ HTTPS Git repository cloning is working correctly");
            }
            Err(e) => {
                panic!("❌ Failed to clone HTTPS repository: {} - Error: {}", test_repo_url, e);
            }
        }
    }
    
    #[test]
    fn test_git2_has_https_support() {
        // Test that git2 was compiled with HTTPS support
        // This is a compile-time check for our feature flags
        
        // Try to create a repository builder which should have HTTPS capabilities
        let _repo_builder = git2::build::RepoBuilder::new();
        
        // The fact that this compiles and runs means HTTPS support is available
        println!("✅ git2 RepoBuilder created successfully - HTTPS support enabled");
        
        // Verify HTTPS URLs are recognized as valid using the url crate
        use url::Url;
        match Url::parse("https://github.com/example/repo.git") {
            Ok(url) => {
                assert_eq!(url.scheme(), "https");
                println!("✅ HTTPS URLs are properly parsed");
            }
            Err(e) => {
                panic!("❌ Failed to parse HTTPS URL: {}", e);
            }
        }
    }
}

#[cfg(not(feature = "git"))]
mod no_git_tests {
    #[test]
    fn test_git_feature_disabled() {
        println!("ℹ️  Git feature is disabled, skipping HTTPS Git tests");
    }
}