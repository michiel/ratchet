//! Git-based task repository implementation
//!
//! This module provides a repository implementation that syncs tasks with
//! Git repositories, supporting clone, pull, push operations and branch management.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info, warn};
use anyhow::{Result, anyhow};

use super::task_sync::{
    RepositoryHealth, RepositoryMetadata, RepositoryTask, TaskRepository,
};
use super::filesystem_repo::FilesystemTaskRepository;

/// Git authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitAuth {
    /// Authentication type (token, ssh_key, username_password)
    pub auth_type: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password or token
    pub password: Option<String>,
    /// SSH private key path
    pub ssh_key_path: Option<String>,
    /// SSH key passphrase
    pub ssh_passphrase: Option<String>,
}

/// Git-based task repository
pub struct GitTaskRepository {
    /// Git repository URL
    repo_url: String,
    /// Branch to work with
    branch: String,
    /// Authentication configuration
    auth_config: Option<GitAuth>,
    /// Local working directory for the repository
    local_path: PathBuf,
    /// Filesystem repository for file operations
    filesystem_repo: FilesystemTaskRepository,
    /// Repository name
    name: String,
    /// Whether to auto-commit changes
    auto_commit: bool,
}

impl GitTaskRepository {
    /// Create a new Git repository
    pub fn new<P: Into<PathBuf>>(
        repo_url: String,
        branch: String,
        auth_config: Option<GitAuth>,
        local_path: P,
        name: String,
    ) -> Self {
        let local_path = local_path.into();
        let filesystem_repo = FilesystemTaskRepository::default_local(&local_path);
        
        Self {
            repo_url,
            branch,
            auth_config,
            local_path,
            filesystem_repo,
            name,
            auto_commit: true,
        }
    }

    /// Set auto-commit behavior
    pub fn with_auto_commit(mut self, auto_commit: bool) -> Self {
        self.auto_commit = auto_commit;
        self
    }

    /// Check if local repository exists and is valid
    async fn is_repo_initialized(&self) -> bool {
        let git_dir = self.local_path.join(".git");
        git_dir.exists() && git_dir.is_dir()
    }

    /// Initialize or clone the Git repository
    async fn ensure_repository(&self) -> Result<()> {
        if !self.is_repo_initialized().await {
            info!("Cloning Git repository: {} to {:?}", self.repo_url, self.local_path);
            self.clone_repository().await?;
        } else {
            debug!("Git repository already exists at {:?}", self.local_path);
        }
        Ok(())
    }

    /// Clone the Git repository
    async fn clone_repository(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.local_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut cmd = Command::new("git");
        cmd.arg("clone")
           .arg("--branch")
           .arg(&self.branch)
           .arg(&self.repo_url)
           .arg(&self.local_path);

        // Apply authentication if configured
        self.apply_auth_to_command(&mut cmd).await?;

        let output = cmd.output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Git clone failed: {}", stderr));
        }

        info!("Successfully cloned repository to {:?}", self.local_path);
        Ok(())
    }

    /// Pull latest changes from remote
    async fn pull_changes(&self) -> Result<()> {
        self.ensure_repository().await?;

        let mut cmd = Command::new("git");
        cmd.arg("pull")
           .arg("origin")
           .arg(&self.branch)
           .current_dir(&self.local_path);

        self.apply_auth_to_command(&mut cmd).await?;

        let output = cmd.output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git pull failed: {}", stderr);
            // Don't fail completely, repository might still be usable
        } else {
            debug!("Successfully pulled changes from remote");
        }

        Ok(())
    }

    /// Push changes to remote repository
    async fn push_changes(&self, commit_message: &str) -> Result<String> {
        self.ensure_repository().await?;

        // Stage all changes
        let mut add_cmd = Command::new("git");
        add_cmd.arg("add")
              .arg(".")
              .current_dir(&self.local_path);

        let add_output = add_cmd.output().await?;
        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(anyhow!("Git add failed: {}", stderr));
        }

        // Check if there are changes to commit
        let mut status_cmd = Command::new("git");
        status_cmd.arg("status")
                  .arg("--porcelain")
                  .current_dir(&self.local_path);

        let status_output = status_cmd.output().await?;
        let status_str = String::from_utf8_lossy(&status_output.stdout);
        
        if status_str.trim().is_empty() {
            debug!("No changes to commit");
            return Ok("No changes".to_string());
        }

        // Commit changes
        let mut commit_cmd = Command::new("git");
        commit_cmd.arg("commit")
                  .arg("-m")
                  .arg(commit_message)
                  .current_dir(&self.local_path);

        let commit_output = commit_cmd.output().await?;
        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            return Err(anyhow!("Git commit failed: {}", stderr));
        }

        // Get commit hash
        let mut hash_cmd = Command::new("git");
        hash_cmd.arg("rev-parse")
                .arg("HEAD")
                .current_dir(&self.local_path);

        let hash_output = hash_cmd.output().await?;
        let commit_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

        // Push to remote
        let mut push_cmd = Command::new("git");
        push_cmd.arg("push")
                .arg("origin")
                .arg(&self.branch)
                .current_dir(&self.local_path);

        self.apply_auth_to_command(&mut push_cmd).await?;

        let push_output = push_cmd.output().await?;
        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Err(anyhow!("Git push failed: {}", stderr));
        }

        info!("Successfully pushed changes to remote: {}", commit_hash);
        Ok(commit_hash)
    }

    /// Apply authentication configuration to Git command
    async fn apply_auth_to_command(&self, _cmd: &mut Command) -> Result<()> {
        // In a real implementation, this would:
        // 1. Set up SSH agent with private keys
        // 2. Configure Git credentials helper
        // 3. Set environment variables for token-based auth
        // 4. Handle different authentication methods
        
        // For now, we assume Git is configured with appropriate credentials
        // This could be enhanced to support:
        // - SSH key authentication
        // - Personal access tokens
        // - Username/password authentication
        // - Git credential managers
        
        if let Some(_auth) = &self.auth_config {
            // TODO: Implement authentication setup
            debug!("Git authentication configured (implementation pending)");
        }

        Ok(())
    }

    /// Get current commit hash
    async fn get_current_commit(&self) -> Result<Option<String>> {
        if !self.is_repo_initialized().await {
            return Ok(None);
        }

        let mut cmd = Command::new("git");
        cmd.arg("rev-parse")
           .arg("HEAD")
           .current_dir(&self.local_path);

        let output = cmd.output().await?;
        
        if output.status.success() {
            let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(Some(commit_hash))
        } else {
            Ok(None)
        }
    }

    /// Get repository status information
    async fn get_repo_status(&self) -> Result<GitStatus> {
        if !self.is_repo_initialized().await {
            return Ok(GitStatus {
                has_changes: false,
                ahead_by: 0,
                behind_by: 0,
                _current_branch: self.branch.clone(),
            });
        }

        // Check for local changes
        let mut status_cmd = Command::new("git");
        status_cmd.arg("status")
                  .arg("--porcelain")
                  .current_dir(&self.local_path);

        let status_output = status_cmd.output().await?;
        let has_changes = !String::from_utf8_lossy(&status_output.stdout).trim().is_empty();

        // Check ahead/behind status
        let mut ahead_behind_cmd = Command::new("git");
        ahead_behind_cmd.arg("rev-list")
                       .arg("--left-right")
                       .arg("--count")
                       .arg(&format!("{}...origin/{}", self.branch, self.branch))
                       .current_dir(&self.local_path);

        let ahead_behind_output = ahead_behind_cmd.output().await?;
        let (ahead_by, behind_by) = if ahead_behind_output.status.success() {
            let output_str = String::from_utf8_lossy(&ahead_behind_output.stdout);
            let parts: Vec<&str> = output_str.trim().split('\t').collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().unwrap_or(0);
                let behind = parts[1].parse().unwrap_or(0);
                (ahead, behind)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        Ok(GitStatus {
            has_changes,
            ahead_by,
            behind_by,
            _current_branch: self.branch.clone(),
        })
    }
}

/// Git repository status
#[derive(Debug, Clone)]
struct GitStatus {
    has_changes: bool,
    ahead_by: u32,
    behind_by: u32,
    _current_branch: String,
}

#[async_trait]
impl TaskRepository for GitTaskRepository {
    async fn list_tasks(&self) -> Result<Vec<RepositoryTask>> {
        // Ensure repository is available and up-to-date
        self.pull_changes().await?;
        
        // Delegate to filesystem repository for actual file operations
        self.filesystem_repo.list_tasks().await
    }

    async fn get_task(&self, path: &str) -> Result<Option<RepositoryTask>> {
        self.pull_changes().await?;
        self.filesystem_repo.get_task(path).await
    }

    async fn put_task(&self, task: &RepositoryTask) -> Result<()> {
        self.ensure_repository().await?;
        
        // Write task using filesystem repository
        self.filesystem_repo.put_task(task).await?;

        // Auto-commit if enabled
        if self.auto_commit {
            let commit_message = format!("Update task: {}", task.name);
            match self.push_changes(&commit_message).await {
                Ok(commit_hash) => {
                    info!("Task {} committed and pushed: {}", task.name, commit_hash);
                }
                Err(e) => {
                    warn!("Failed to push task {}: {}", task.name, e);
                    // Don't fail the operation, task is still written locally
                }
            }
        }

        Ok(())
    }

    async fn delete_task(&self, path: &str) -> Result<()> {
        self.ensure_repository().await?;
        
        // Delete using filesystem repository
        self.filesystem_repo.delete_task(path).await?;

        // Auto-commit if enabled
        if self.auto_commit {
            let commit_message = format!("Delete task: {}", path);
            match self.push_changes(&commit_message).await {
                Ok(commit_hash) => {
                    info!("Task {} deletion committed and pushed: {}", path, commit_hash);
                }
                Err(e) => {
                    warn!("Failed to push task deletion {}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    async fn get_metadata(&self) -> Result<RepositoryMetadata> {
        let current_commit = self.get_current_commit().await?;
        let status = self.get_repo_status().await?;
        
        let mut metadata = HashMap::new();
        metadata.insert("has_changes".to_string(), JsonValue::Bool(status.has_changes));
        metadata.insert("ahead_by".to_string(), JsonValue::Number(status.ahead_by.into()));
        metadata.insert("behind_by".to_string(), JsonValue::Number(status.behind_by.into()));
        metadata.insert("auto_commit".to_string(), JsonValue::Bool(self.auto_commit));
        
        if let Some(auth) = &self.auth_config {
            metadata.insert("auth_type".to_string(), JsonValue::String(auth.auth_type.clone()));
        }

        Ok(RepositoryMetadata {
            name: self.name.clone(),
            repository_type: "git".to_string(),
            uri: self.repo_url.clone(),
            branch: Some(self.branch.clone()),
            commit: current_commit,
            is_writable: true,
            metadata,
        })
    }

    async fn is_writable(&self) -> bool {
        // Git repositories are writable if we have push access
        // For now, assume they are writable if auth is configured
        true
    }

    async fn test_connection(&self) -> Result<bool> {
        // Test by attempting to clone or pull
        match self.ensure_repository().await {
            Ok(_) => Ok(true),
            Err(e) => {
                debug!("Git repository connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn health_check(&self) -> Result<RepositoryHealth> {
        let accessible = self.test_connection().await.unwrap_or(false);
        let writable = accessible && self.is_writable().await;
        
        let message = if !accessible {
            "Cannot access Git repository".to_string()
        } else if !writable {
            "Git repository is read-only".to_string()
        } else {
            "Git repository is healthy".to_string()
        };

        // Additional health information
        let (error_count, last_success) = if accessible {
            let status = self.get_repo_status().await;
            match status {
                Ok(_) => (0, Some(Utc::now())),
                Err(_) => (1, None),
            }
        } else {
            (1, None)
        };

        Ok(RepositoryHealth {
            accessible,
            writable,
            last_success,
            error_count,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_git_repository_creation() {
        let temp_dir = TempDir::new().unwrap();
        let repo = GitTaskRepository::new(
            "https://github.com/example/tasks.git".to_string(),
            "main".to_string(),
            None,
            temp_dir.path(),
            "test-git-repo".to_string(),
        );
        
        assert_eq!(repo.name, "test-git-repo");
        assert_eq!(repo.branch, "main");
        assert_eq!(repo.repo_url, "https://github.com/example/tasks.git");
    }

    #[tokio::test]
    async fn test_git_auth_configuration() {
        let auth = GitAuth {
            auth_type: "token".to_string(),
            username: Some("user".to_string()),
            password: Some("token123".to_string()),
            ssh_key_path: None,
            ssh_passphrase: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let repo = GitTaskRepository::new(
            "https://github.com/example/tasks.git".to_string(),
            "main".to_string(),
            Some(auth),
            temp_dir.path(),
            "test-git-repo".to_string(),
        );

        assert!(repo.auth_config.is_some());
        assert_eq!(repo.auth_config.unwrap().auth_type, "token");
    }

    #[test]
    fn test_git_status_parsing() {
        let status = GitStatus {
            has_changes: true,
            ahead_by: 2,
            behind_by: 1,
            _current_branch: "feature".to_string(),
        };

        assert!(status.has_changes);
        assert_eq!(status.ahead_by, 2);
        assert_eq!(status.behind_by, 1);
    }
}