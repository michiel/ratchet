//! Task synchronization service implementation
//!
//! This module provides the core synchronization service that coordinates
//! bidirectional sync between the database and various repository types.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use super::task_sync::{
    ConflictResolution, ConflictType, PushResult, RepositoryTask, SyncError, SyncResult,
    TaskConflict, TaskRepository, TaskVersion,
};

/// Task synchronization service
pub struct TaskSyncService {
    /// Active repository instances
    repositories: Arc<RwLock<HashMap<i32, Box<dyn TaskRepository>>>>,
    /// Conflict resolver
    conflict_resolver: Arc<ConflictResolver>,
    /// Database connection (placeholder - would use actual DB interface)
    db: Arc<dyn DatabaseInterface>,
}

/// Database interface for sync operations
#[async_trait]
pub trait DatabaseInterface: Send + Sync {
    /// Get all tasks for a repository
    async fn get_repository_tasks(&self, repository_id: i32) -> Result<Vec<DatabaseTask>>;
    
    /// Get a specific task by repository and path
    async fn get_task_by_path(&self, repository_id: i32, path: &str) -> Result<Option<DatabaseTask>>;
    
    /// Create or update a task in the database
    async fn upsert_task(&self, task: &DatabaseTask) -> Result<()>;
    
    /// Delete a task from the database
    async fn delete_task(&self, repository_id: i32, path: &str) -> Result<()>;
    
    /// Mark task as needing push
    async fn mark_task_needs_push(&self, task_id: i32, needs_push: bool) -> Result<()>;
    
    /// Update task sync status
    async fn update_sync_status(&self, task_id: i32, status: &str, synced_at: DateTime<Utc>) -> Result<()>;
    
    /// Get repository configuration
    async fn get_repository_config(&self, repository_id: i32) -> Result<Option<RepositoryConfig>>;
}

/// Database task representation
#[derive(Debug, Clone)]
pub struct DatabaseTask {
    pub id: i32,
    pub repository_id: i32,
    pub name: String,
    pub path: String,
    pub source_code: String,
    pub input_schema: JsonValue,
    pub output_schema: JsonValue,
    pub metadata: JsonValue,
    pub checksum: String,
    pub sync_status: String,
    pub needs_push: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Repository configuration
#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    pub id: i32,
    pub name: String,
    pub repository_type: String,
    pub uri: String,
    pub branch: Option<String>,
    pub sync_enabled: bool,
    pub is_writable: bool,
    pub push_on_change: bool,
}

/// Conflict resolution engine
pub struct ConflictResolver {
    /// Default resolution strategy
    default_strategy: ConflictResolution,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new(default_strategy: ConflictResolution) -> Self {
        Self { default_strategy }
    }

    /// Resolve a task conflict
    pub async fn resolve_conflict(&self, conflict: &TaskConflict) -> Result<Resolution> {
        debug!("Resolving conflict for task {}: {:?}", conflict.task_id, conflict.conflict_type);

        match &conflict.conflict_type {
            ConflictType::ModificationConflict => {
                if conflict.auto_resolvable {
                    self.auto_resolve_modification_conflict(conflict).await
                } else {
                    Ok(Resolution::RequiresManualIntervention {
                        reason: "Both versions have incompatible changes".to_string(),
                        local_version: conflict.local_version.clone(),
                        remote_version: conflict.remote_version.clone(),
                    })
                }
            }
            ConflictType::LocalOnly => {
                // Task exists locally but not in repository
                match self.default_strategy {
                    ConflictResolution::TakeLocal => Ok(Resolution::PushToRepository),
                    ConflictResolution::TakeRemote => Ok(Resolution::DeleteLocal),
                    _ => Ok(Resolution::RequiresManualIntervention {
                        reason: "Task exists locally but not in repository".to_string(),
                        local_version: conflict.local_version.clone(),
                        remote_version: conflict.remote_version.clone(),
                    }),
                }
            }
            ConflictType::RemoteOnly => {
                // Task exists in repository but not locally
                match self.default_strategy {
                    ConflictResolution::TakeRemote => Ok(Resolution::PullFromRepository),
                    ConflictResolution::TakeLocal => Ok(Resolution::DeleteFromRepository),
                    _ => Ok(Resolution::RequiresManualIntervention {
                        reason: "Task exists in repository but not locally".to_string(),
                        local_version: conflict.local_version.clone(),
                        remote_version: conflict.remote_version.clone(),
                    }),
                }
            }
            ConflictType::DeleteModifyConflict => {
                Ok(Resolution::RequiresManualIntervention {
                    reason: "Task was deleted locally but modified in repository".to_string(),
                    local_version: conflict.local_version.clone(),
                    remote_version: conflict.remote_version.clone(),
                })
            }
            ConflictType::ModifyDeleteConflict => {
                Ok(Resolution::RequiresManualIntervention {
                    reason: "Task was modified locally but deleted in repository".to_string(),
                    local_version: conflict.local_version.clone(),
                    remote_version: conflict.remote_version.clone(),
                })
            }
        }
    }

    /// Attempt automatic resolution of modification conflicts
    async fn auto_resolve_modification_conflict(&self, conflict: &TaskConflict) -> Result<Resolution> {
        // Simple conflict resolution: take the version with the latest modification time
        if conflict.local_version.modified_at > conflict.remote_version.modified_at {
            Ok(Resolution::UseLocal)
        } else if conflict.remote_version.modified_at > conflict.local_version.modified_at {
            Ok(Resolution::UseRemote)
        } else {
            // Same modification time - use default strategy
            match self.default_strategy {
                ConflictResolution::TakeLocal => Ok(Resolution::UseLocal),
                ConflictResolution::TakeRemote => Ok(Resolution::UseRemote),
                _ => Ok(Resolution::RequiresManualIntervention {
                    reason: "Both versions have the same modification time".to_string(),
                    local_version: conflict.local_version.clone(),
                    remote_version: conflict.remote_version.clone(),
                }),
            }
        }
    }
}

/// Conflict resolution result
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Use the local (database) version
    UseLocal,
    /// Use the remote (repository) version
    UseRemote,
    /// Push local changes to repository
    PushToRepository,
    /// Pull remote changes to database
    PullFromRepository,
    /// Delete local version
    DeleteLocal,
    /// Delete from repository
    DeleteFromRepository,
    /// Requires manual intervention
    RequiresManualIntervention {
        reason: String,
        local_version: TaskVersion,
        remote_version: TaskVersion,
    },
}

impl TaskSyncService {
    /// Create a new task sync service
    pub fn new(
        db: Arc<dyn DatabaseInterface>,
        default_resolution: ConflictResolution,
    ) -> Self {
        Self {
            repositories: Arc::new(RwLock::new(HashMap::new())),
            conflict_resolver: Arc::new(ConflictResolver::new(default_resolution)),
            db,
        }
    }

    /// Register a repository for sync operations
    pub async fn register_repository(&self, repository_id: i32, repository: Box<dyn TaskRepository>) {
        let mut repos = self.repositories.write().await;
        repos.insert(repository_id, repository);
        info!("Registered repository {} for sync operations", repository_id);
    }

    /// Unregister a repository
    pub async fn unregister_repository(&self, repository_id: i32) {
        let mut repos = self.repositories.write().await;
        repos.remove(&repository_id);
        info!("Unregistered repository {}", repository_id);
    }

    /// Sync a specific repository (bidirectional)
    pub async fn sync_repository(&self, repository_id: i32) -> Result<SyncResult> {
        let start_time = std::time::Instant::now();
        let mut result = SyncResult::new(repository_id);

        info!("Starting sync for repository {}", repository_id);

        // Get repository instance
        let repository = {
            let repos = self.repositories.read().await;
            repos.get(&repository_id)
                .ok_or_else(|| anyhow!("Repository {} not found", repository_id))?
                .as_ref() as *const dyn TaskRepository
        };

        // SAFETY: The repository is guaranteed to live as long as it's registered
        let repository = unsafe { &*repository };

        // Get repository configuration
        let repo_config = self.db.get_repository_config(repository_id).await?
            .ok_or_else(|| anyhow!("Repository configuration not found for {}", repository_id))?;

        if !repo_config.sync_enabled {
            warn!("Sync is disabled for repository {}", repository_id);
            return Ok(result);
        }

        // Get tasks from both sources
        let (db_tasks, repo_tasks): (Vec<DatabaseTask>, Vec<RepositoryTask>) = tokio::try_join!(
            self.db.get_repository_tasks(repository_id),
            repository.list_tasks()
        )?;

        // Create lookup maps for efficient comparison
        let db_task_map: HashMap<String, &DatabaseTask> = 
            db_tasks.iter().map(|t| (t.path.clone(), t)).collect();
        let repo_task_map: HashMap<String, &RepositoryTask> = 
            repo_tasks.iter().map(|t| (t.path.clone(), t)).collect();

        // Detect changes and conflicts
        let mut conflicts = Vec::new();
        let mut operations = Vec::new();

        // Check each repository task
        for (path, repo_task) in &repo_task_map {
            match db_task_map.get(path) {
                Some(db_task) => {
                    // Task exists in both places - check for conflicts
                    if db_task.checksum != repo_task.checksum {
                        let conflict = self.detect_conflict(db_task, repo_task).await?;
                        conflicts.push(conflict);
                    }
                }
                None => {
                    // Task only exists in repository - pull it
                    operations.push(SyncOperation::PullFromRepository((*repo_task).clone()));
                }
            }
        }

        // Check each database task
        for (path, db_task) in &db_task_map {
            if !repo_task_map.contains_key(path) {
                // Task only exists in database
                if db_task.needs_push && repo_config.is_writable {
                    // Push to repository
                    let repo_task = self.db_task_to_repo_task(db_task)?;
                    operations.push(SyncOperation::PushToRepository(repo_task));
                } else {
                    // Create conflict for manual resolution
                    let conflict = TaskConflict {
                        task_id: db_task.id,
                        repository_id,
                        conflict_type: ConflictType::LocalOnly,
                        local_version: self.db_task_to_task_version(db_task)?,
                        remote_version: TaskVersion {
                            source_code: String::new(),
                            input_schema: JsonValue::Null,
                            output_schema: JsonValue::Null,
                            metadata: super::task_sync::TaskMetadata::minimal("0.0.0".to_string()),
                            checksum: String::new(),
                            modified_at: Utc::now(),
                        },
                        auto_resolvable: false,
                        detected_at: Utc::now(),
                    };
                    conflicts.push(conflict);
                }
            }
        }

        // Resolve conflicts
        for conflict in &conflicts {
            match self.conflict_resolver.resolve_conflict(conflict).await? {
                Resolution::UseLocal => {
                    if repo_config.is_writable {
                        let db_task = db_tasks.iter().find(|t| t.id == conflict.task_id).unwrap();
                        let repo_task = self.db_task_to_repo_task(db_task)?;
                        operations.push(SyncOperation::PushToRepository(repo_task));
                    }
                }
                Resolution::UseRemote => {
                    let repo_task = repo_tasks.iter().find(|t| t.path == 
                        db_tasks.iter().find(|d| d.id == conflict.task_id).unwrap().path).unwrap();
                    operations.push(SyncOperation::PullFromRepository(repo_task.clone()));
                }
                Resolution::PushToRepository => {
                    if repo_config.is_writable {
                        let db_task = db_tasks.iter().find(|t| t.id == conflict.task_id).unwrap();
                        let repo_task = self.db_task_to_repo_task(db_task)?;
                        operations.push(SyncOperation::PushToRepository(repo_task));
                    }
                }
                Resolution::PullFromRepository => {
                    let repo_task = repo_tasks.iter().find(|t| t.path == 
                        db_tasks.iter().find(|d| d.id == conflict.task_id).unwrap().path).unwrap();
                    operations.push(SyncOperation::PullFromRepository(repo_task.clone()));
                }
                Resolution::DeleteLocal => {
                    let db_task = db_tasks.iter().find(|t| t.id == conflict.task_id).unwrap();
                    operations.push(SyncOperation::DeleteFromDatabase(db_task.path.clone()));
                }
                Resolution::DeleteFromRepository => {
                    if repo_config.is_writable {
                        let db_task = db_tasks.iter().find(|t| t.id == conflict.task_id).unwrap();
                        operations.push(SyncOperation::DeleteFromRepository(db_task.path.clone()));
                    }
                }
                Resolution::RequiresManualIntervention { .. } => {
                    result.conflicts.push(conflict.clone());
                }
            }
        }

        // Execute sync operations
        for operation in operations {
            match operation {
                SyncOperation::PullFromRepository(repo_task) => {
                    match self.pull_task_from_repository(repository_id, &repo_task).await {
                        Ok(_) => result.tasks_added += 1,
                        Err(e) => result.errors.push(SyncError {
                            error_type: "pull_error".to_string(),
                            message: e.to_string(),
                            task_path: Some(repo_task.path),
                            occurred_at: Utc::now(),
                        }),
                    }
                }
                SyncOperation::PushToRepository(repo_task) => {
                    match repository.put_task(&repo_task).await {
                        Ok(_) => {
                            result.tasks_updated += 1;
                            // Mark as synced in database
                            if let Err(e) = self.db.update_sync_status(
                                db_tasks.iter().find(|t| t.path == repo_task.path).unwrap().id,
                                "synced",
                                Utc::now()
                            ).await {
                                warn!("Failed to update sync status: {}", e);
                            }
                        }
                        Err(e) => result.errors.push(SyncError {
                            error_type: "push_error".to_string(),
                            message: e.to_string(),
                            task_path: Some(repo_task.path),
                            occurred_at: Utc::now(),
                        }),
                    }
                }
                SyncOperation::DeleteFromDatabase(path) => {
                    match self.db.delete_task(repository_id, &path).await {
                        Ok(_) => result.tasks_deleted += 1,
                        Err(e) => result.errors.push(SyncError {
                            error_type: "delete_db_error".to_string(),
                            message: e.to_string(),
                            task_path: Some(path),
                            occurred_at: Utc::now(),
                        }),
                    }
                }
                SyncOperation::DeleteFromRepository(path) => {
                    match repository.delete_task(&path).await {
                        Ok(_) => result.tasks_deleted += 1,
                        Err(e) => result.errors.push(SyncError {
                            error_type: "delete_repo_error".to_string(),
                            message: e.to_string(),
                            task_path: Some(path),
                            occurred_at: Utc::now(),
                        }),
                    }
                }
            }
        }

        result.duration_ms = start_time.elapsed().as_millis() as u64;
        info!("Completed sync for repository {} in {}ms - Added: {}, Updated: {}, Deleted: {}, Conflicts: {}, Errors: {}",
            repository_id, result.duration_ms, result.tasks_added, result.tasks_updated, 
            result.tasks_deleted, result.conflicts.len(), result.errors.len());

        Ok(result)
    }

    /// Push task changes to repository
    pub async fn push_task_changes(&self, task_id: i32) -> Result<PushResult> {
        // Implementation would get the task from database and push to its repository
        todo!("Implement push_task_changes")
    }

    /// Pull task from repository to database
    async fn pull_task_from_repository(&self, repository_id: i32, repo_task: &RepositoryTask) -> Result<()> {
        let db_task = DatabaseTask {
            id: 0, // Will be set by database
            repository_id,
            name: repo_task.name.clone(),
            path: repo_task.path.clone(),
            source_code: repo_task.source_code.clone(),
            input_schema: repo_task.input_schema.clone(),
            output_schema: repo_task.output_schema.clone(),
            metadata: serde_json::to_value(&repo_task.metadata)?,
            checksum: repo_task.checksum.clone(),
            sync_status: "synced".to_string(),
            needs_push: false,
            last_synced_at: Some(Utc::now()),
            created_at: repo_task.created_at,
            updated_at: repo_task.modified_at,
        };

        self.db.upsert_task(&db_task).await?;
        debug!("Pulled task {} from repository", repo_task.name);
        Ok(())
    }

    /// Detect conflict between database and repository task
    async fn detect_conflict(&self, db_task: &DatabaseTask, repo_task: &RepositoryTask) -> Result<TaskConflict> {
        let conflict_type = if db_task.updated_at > db_task.last_synced_at.unwrap_or(DateTime::UNIX_EPOCH) &&
                              repo_task.modified_at > db_task.last_synced_at.unwrap_or(DateTime::UNIX_EPOCH) {
            ConflictType::ModificationConflict
        } else {
            ConflictType::ModificationConflict // Simplified for now
        };

        Ok(TaskConflict {
            task_id: db_task.id,
            repository_id: db_task.repository_id,
            conflict_type,
            local_version: self.db_task_to_task_version(db_task)?,
            remote_version: TaskVersion {
                source_code: repo_task.source_code.clone(),
                input_schema: repo_task.input_schema.clone(),
                output_schema: repo_task.output_schema.clone(),
                metadata: repo_task.metadata.clone(),
                checksum: repo_task.checksum.clone(),
                modified_at: repo_task.modified_at,
            },
            auto_resolvable: true, // Could be determined by analyzing the differences
            detected_at: Utc::now(),
        })
    }

    /// Convert database task to repository task
    fn db_task_to_repo_task(&self, db_task: &DatabaseTask) -> Result<RepositoryTask> {
        let metadata: super::task_sync::TaskMetadata = serde_json::from_value(db_task.metadata.clone())?;
        
        Ok(RepositoryTask {
            path: db_task.path.clone(),
            name: db_task.name.clone(),
            source_code: db_task.source_code.clone(),
            input_schema: db_task.input_schema.clone(),
            output_schema: db_task.output_schema.clone(),
            metadata,
            checksum: db_task.checksum.clone(),
            modified_at: db_task.updated_at,
            created_at: db_task.created_at,
        })
    }

    /// Convert database task to task version
    fn db_task_to_task_version(&self, db_task: &DatabaseTask) -> Result<TaskVersion> {
        let metadata: super::task_sync::TaskMetadata = serde_json::from_value(db_task.metadata.clone())?;
        
        Ok(TaskVersion {
            source_code: db_task.source_code.clone(),
            input_schema: db_task.input_schema.clone(),
            output_schema: db_task.output_schema.clone(),
            metadata,
            checksum: db_task.checksum.clone(),
            modified_at: db_task.updated_at,
        })
    }
}

/// Internal sync operations
#[derive(Debug, Clone)]
enum SyncOperation {
    PullFromRepository(RepositoryTask),
    PushToRepository(RepositoryTask),
    DeleteFromDatabase(String),
    DeleteFromRepository(String),
}