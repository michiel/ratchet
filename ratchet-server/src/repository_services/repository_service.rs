//! Enhanced repository service with sync engine integration
//!
//! This service provides comprehensive repository management including
//! CRUD operations, sync coordination, and repository health monitoring.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use ratchet_storage::repositories::{
    TaskSyncService, TaskRepository, FilesystemTaskRepository, GitTaskRepository, HttpTaskRepository,
    HttpRepositoryConfig, GitAuth, HttpAuth, SyncResult, PushResult, RepositoryHealth,
    DatabaseInterface, DatabaseTask, RepositoryConfig, ConflictResolution,
};
use ratchet_api_types::{
    CreateRepositoryRequest, UpdateRepositoryRequest, ConnectionTestResult, UnifiedTaskRepository,
};

/// Enhanced repository service with sync capabilities
#[derive(Clone)]
pub struct EnhancedRepositoryService {
    /// Database repository service from storage layer
    db_service: Arc<ratchet_storage::seaorm::repositories::RepositoryService>,
    /// Task sync service for repository operations
    pub sync_service: Arc<TaskSyncService>,
    /// Active repository instances
    active_repositories: Arc<RwLock<HashMap<i32, Box<dyn TaskRepository>>>>,
}

/// Repository sync status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySyncStatus {
    pub repository_id: i32,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    pub sync_error: Option<String>,
    pub health: RepositoryHealth,
    pub task_count: u64,
}

/// Repository creation/update request with sync options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepositoryWithSyncRequest {
    #[serde(flatten)]
    pub repository: CreateRepositoryRequest,
    /// Whether to immediately test connection after creation
    pub test_connection: Option<bool>,
    /// Whether to perform initial sync after creation
    pub initial_sync: Option<bool>,
}

/// Repository update request with sync options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRepositoryWithSyncRequest {
    #[serde(flatten)]
    pub repository: UpdateRepositoryRequest,
    /// Whether to test connection after update
    pub test_connection: Option<bool>,
    /// Whether to perform sync after update
    pub sync_after_update: Option<bool>,
}

impl EnhancedRepositoryService {
    /// Create a new enhanced repository service
    pub fn new(
        db_service: Arc<ratchet_storage::seaorm::repositories::RepositoryService>,
        db_interface: Arc<dyn DatabaseInterface>,
    ) -> Self {
        let sync_service = Arc::new(TaskSyncService::new(
            db_interface,
            ConflictResolution::TakeLocal, // Default conflict resolution
        ));

        Self {
            db_service,
            sync_service,
            active_repositories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// List all repositories with enhanced information
    pub async fn list_repositories(&self) -> Result<Vec<UnifiedTaskRepository>> {
        let repositories = self.db_service.list_repositories().await
            .context("Failed to list repositories from database")?;

        let mut unified_repos = Vec::new();
        
        for repo in repositories {
            let task_count = self.db_service.count_tasks_in_repository(repo.id).await
                .unwrap_or(0) as u32;

            let unified_repo = UnifiedTaskRepository {
                id: ratchet_api_types::ApiId::from_i32(repo.id),
                name: repo.name,
                repository_type: repo.repository_type,
                uri: repo.uri,
                branch: repo.branch,
                sync_enabled: repo.sync_enabled,
                sync_interval_minutes: repo.sync_interval_minutes,
                last_sync_at: repo.last_sync_at.map(|dt| dt),
                sync_status: repo.sync_status,
                is_default: repo.is_default,
                is_writable: repo.is_writable,
                watch_patterns: serde_json::from_value(repo.watch_patterns).unwrap_or_default(),
                ignore_patterns: serde_json::from_value(repo.ignore_patterns).unwrap_or_default(),
                push_on_change: repo.push_on_change,
                task_count,
                created_at: repo.created_at,
                updated_at: repo.updated_at,
            };
            
            unified_repos.push(unified_repo);
        }

        Ok(unified_repos)
    }

    /// Get repository by ID with enhanced information
    pub async fn get_repository(&self, id: i32) -> Result<Option<UnifiedTaskRepository>> {
        let repo = self.db_service.get_repository(id).await
            .context("Failed to get repository from database")?;

        if let Some(repo) = repo {
            let task_count = self.db_service.count_tasks_in_repository(repo.id).await
                .unwrap_or(0) as u32;

            let unified_repo = UnifiedTaskRepository {
                id: ratchet_api_types::ApiId::from_i32(repo.id),
                name: repo.name,
                repository_type: repo.repository_type,
                uri: repo.uri,
                branch: repo.branch,
                sync_enabled: repo.sync_enabled,
                sync_interval_minutes: repo.sync_interval_minutes,
                last_sync_at: repo.last_sync_at.map(|dt| dt),
                sync_status: repo.sync_status,
                is_default: repo.is_default,
                is_writable: repo.is_writable,
                watch_patterns: serde_json::from_value(repo.watch_patterns).unwrap_or_default(),
                ignore_patterns: serde_json::from_value(repo.ignore_patterns).unwrap_or_default(),
                push_on_change: repo.push_on_change,
                task_count,
                created_at: repo.created_at,
                updated_at: repo.updated_at,
            };
            
            Ok(Some(unified_repo))
        } else {
            Ok(None)
        }
    }

    /// Create repository with sync setup
    pub async fn create_repository(&self, request: CreateRepositoryWithSyncRequest) -> Result<UnifiedTaskRepository> {
        info!("Creating repository: {}", request.repository.name);

        // Create repository in database
        let created_repo = self.db_service.create_repository(request.repository.clone()).await
            .context("Failed to create repository in database")?;

        // Create and register repository instance
        let repo_instance = self.create_repository_instance(&created_repo).await?;
        self.sync_service.register_repository(created_repo.id, repo_instance).await;

        // Test connection if requested
        if request.test_connection.unwrap_or(false) {
            match self.test_repository_connection(created_repo.id).await {
                Ok(test_result) => {
                    if !test_result.success {
                        warn!("Repository connection test failed: {}", test_result.message);
                    }
                }
                Err(e) => {
                    warn!("Failed to test repository connection: {}", e);
                }
            }
        }

        // Perform initial sync if requested
        if request.initial_sync.unwrap_or(false) && created_repo.sync_enabled {
            match self.sync_service.sync_repository(created_repo.id).await {
                Ok(sync_result) => {
                    info!("Initial sync completed for repository {}: {} tasks processed", 
                        created_repo.id, sync_result.total_tasks_processed());
                }
                Err(e) => {
                    warn!("Initial sync failed for repository {}: {}", created_repo.id, e);
                }
            }
        }

        // Return unified repository representation
        self.get_repository(created_repo.id).await?
            .ok_or_else(|| anyhow!("Failed to retrieve created repository"))
    }

    /// Update repository with sync coordination
    pub async fn update_repository(&self, id: i32, request: UpdateRepositoryWithSyncRequest) -> Result<Option<UnifiedTaskRepository>> {
        info!("Updating repository: {}", id);

        let updated_repo = self.db_service.update_repository(id, request.repository).await
            .context("Failed to update repository in database")?;

        if let Some(repo) = updated_repo {
            // Recreate repository instance with updated configuration
            let repo_instance = self.create_repository_instance(&repo).await?;
            self.sync_service.register_repository(repo.id, repo_instance).await;

            // Test connection if requested
            if request.test_connection.unwrap_or(false) {
                match self.test_repository_connection(repo.id).await {
                    Ok(test_result) => {
                        if !test_result.success {
                            warn!("Repository connection test failed after update: {}", test_result.message);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to test repository connection after update: {}", e);
                    }
                }
            }

            // Perform sync if requested
            if request.sync_after_update.unwrap_or(false) && repo.sync_enabled {
                match self.sync_service.sync_repository(repo.id).await {
                    Ok(sync_result) => {
                        info!("Post-update sync completed for repository {}: {} tasks processed", 
                            repo.id, sync_result.total_tasks_processed());
                    }
                    Err(e) => {
                        warn!("Post-update sync failed for repository {}: {}", repo.id, e);
                    }
                }
            }

            Ok(Some(self.get_repository(repo.id).await?.unwrap()))
        } else {
            Ok(None)
        }
    }

    /// Delete repository with cleanup
    pub async fn delete_repository(&self, id: i32) -> Result<bool> {
        info!("Deleting repository: {}", id);

        // Unregister from sync service
        self.sync_service.unregister_repository(id).await;

        // Delete from database
        let deleted = self.db_service.delete_repository(id).await
            .context("Failed to delete repository from database")?;

        if deleted {
            info!("Repository {} deleted successfully", id);
        }

        Ok(deleted)
    }

    /// Set repository as default
    pub async fn set_default_repository(&self, id: i32) -> Result<Option<UnifiedTaskRepository>> {
        let updated_repo = self.db_service.set_default_repository(id).await
            .context("Failed to set default repository")?;

        if let Some(_) = updated_repo {
            info!("Repository {} set as default", id);
            self.get_repository(id).await
        } else {
            Ok(None)
        }
    }

    /// Test repository connection
    pub async fn test_repository_connection(&self, id: i32) -> Result<ConnectionTestResult> {
        debug!("Testing connection for repository: {}", id);

        let repos = self.active_repositories.read().await;
        if let Some(repository) = repos.get(&id) {
            match repository.test_connection().await {
                Ok(success) => Ok(ConnectionTestResult {
                    success,
                    message: if success { "Connection successful".to_string() } else { "Connection failed".to_string() },
                    details: None,
                }),
                Err(e) => Ok(ConnectionTestResult {
                    success: false,
                    message: format!("Connection test failed: {}", e),
                    details: Some(serde_json::json!({"error": e.to_string()})),
                }),
            }
        } else {
            Err(anyhow!("Repository {} not found or not initialized", id))
        }
    }

    /// Sync repository
    pub async fn sync_repository(&self, id: i32) -> Result<SyncResult> {
        info!("Starting sync for repository: {}", id);
        
        let result = self.sync_service.sync_repository(id).await
            .context("Repository sync failed")?;

        info!("Sync completed for repository {}: Added: {}, Updated: {}, Deleted: {}, Conflicts: {}, Errors: {}",
            id, result.tasks_added, result.tasks_updated, result.tasks_deleted, 
            result.conflicts.len(), result.errors.len());

        Ok(result)
    }

    /// Push repository changes
    pub async fn push_repository_changes(&self, id: i32) -> Result<Vec<PushResult>> {
        info!("Starting push for repository: {}", id);
        
        // TODO: Implement push logic when available in sync service
        // For now, return empty result
        warn!("Repository push not yet implemented for repository {}", id);
        Ok(Vec::new())
    }

    /// Get repository health status
    pub async fn get_repository_health(&self, id: i32) -> Result<RepositoryHealth> {
        let repos = self.active_repositories.read().await;
        if let Some(repository) = repos.get(&id) {
            repository.health_check().await
                .context("Failed to check repository health")
        } else {
            Err(anyhow!("Repository {} not found or not initialized", id))
        }
    }

    /// Get repository sync status
    pub async fn get_repository_sync_status(&self, id: i32) -> Result<RepositorySyncStatus> {
        let repo = self.db_service.get_repository(id).await
            .context("Failed to get repository from database")?
            .ok_or_else(|| anyhow!("Repository {} not found", id))?;

        let health = self.get_repository_health(id).await.unwrap_or_else(|_| RepositoryHealth {
            accessible: false,
            writable: false,
            last_success: None,
            error_count: 1,
            message: "Repository not accessible".to_string(),
        });

        let task_count = self.db_service.count_tasks_in_repository(id).await.unwrap_or(0);

        Ok(RepositorySyncStatus {
            repository_id: id,
            last_sync_at: repo.last_sync_at.map(|dt| dt),
            sync_status: repo.sync_status,
            sync_error: repo.sync_error,
            health,
            task_count,
        })
    }

    /// Get default repository
    pub async fn get_default_repository(&self) -> Result<Option<UnifiedTaskRepository>> {
        let default_repo = self.db_service.get_default_repository().await
            .context("Failed to get default repository")?;

        if let Some(repo) = default_repo {
            self.get_repository(repo.id).await
        } else {
            Ok(None)
        }
    }

    /// Initialize all repositories on service startup
    pub async fn initialize_repositories(&self) -> Result<()> {
        info!("Initializing all repositories");

        let repositories = self.db_service.list_repositories().await
            .context("Failed to list repositories for initialization")?;

        for repo in repositories {
            if repo.sync_enabled {
                match self.create_repository_instance(&repo).await {
                    Ok(repo_instance) => {
                        self.sync_service.register_repository(repo.id, repo_instance).await;
                        debug!("Initialized repository: {} ({})", repo.name, repo.id);
                    }
                    Err(e) => {
                        error!("Failed to initialize repository {} ({}): {}", repo.name, repo.id, e);
                    }
                }
            }
        }

        info!("Repository initialization completed");
        Ok(())
    }

    /// Create repository instance based on type
    async fn create_repository_instance(&self, repo: &ratchet_storage::seaorm::entities::TaskRepository) -> Result<Box<dyn TaskRepository>> {
        match repo.repository_type.as_str() {
            "filesystem" => {
                let watch_patterns: Vec<String> = serde_json::from_value(repo.watch_patterns.clone())
                    .unwrap_or_else(|_| vec!["**/*.js".to_string()]);
                let ignore_patterns: Vec<String> = serde_json::from_value(repo.ignore_patterns.clone())
                    .unwrap_or_else(|_| vec!["**/node_modules/**".to_string()]);

                let filesystem_repo = FilesystemTaskRepository::new(
                    &repo.uri,
                    repo.name.clone(),
                    watch_patterns,
                    ignore_patterns,
                );

                Ok(Box::new(filesystem_repo))
            }
            "git" => {
                let auth_config: Option<GitAuth> = repo.auth_config.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                let git_repo = GitTaskRepository::new(
                    repo.uri.clone(),
                    repo.branch.clone().unwrap_or_else(|| "main".to_string()),
                    auth_config,
                    format!("/tmp/ratchet-git-{}", repo.id), // TODO: Make configurable
                    repo.name.clone(),
                ).with_auto_commit(repo.push_on_change);

                Ok(Box::new(git_repo))
            }
            "http" => {
                let auth_config: Option<HttpAuth> = repo.auth_config.as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok());

                let http_config = HttpRepositoryConfig {
                    base_url: repo.uri.clone(),
                    auth: auth_config,
                    timeout_seconds: Some(30),
                    max_retries: Some(3),
                    default_headers: None,
                };

                let http_repo = HttpTaskRepository::new(http_config, repo.name.clone())
                    .context("Failed to create HTTP repository")?;

                Ok(Box::new(http_repo))
            }
            _ => {
                Err(anyhow!("Unsupported repository type: {}", repo.repository_type))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: Add comprehensive tests for repository service operations
    // This would include:
    // - Repository CRUD operations
    // - Sync coordination
    // - Connection testing
    // - Health monitoring
    // - Error handling scenarios
}