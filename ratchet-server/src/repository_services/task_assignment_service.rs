//! Task assignment service for repository management
//!
//! This service handles the assignment of tasks to repositories, managing
//! the relationship between tasks and their source repositories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use anyhow::{Context, Result, anyhow};

use ratchet_storage::repositories::TaskSyncService;

/// Task assignment service
#[derive(Clone)]
pub struct TaskAssignmentService {
    /// Database repository service
    db_service: Arc<ratchet_storage::seaorm::repositories::RepositoryService>,
    /// Task sync service for sync operations
    sync_service: Arc<TaskSyncService>,
}

/// Task repository assignment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRepositoryAssignment {
    /// Task ID
    pub task_id: i32,
    /// Repository ID
    pub repository_id: i32,
    /// Repository name
    pub repository_name: String,
    /// Path within repository
    pub repository_path: String,
    /// Whether task can be pushed to repository
    pub can_push: bool,
    /// Whether changes are automatically pushed
    pub auto_push: bool,
    /// Current sync status
    pub sync_status: String,
    /// Whether task needs to be pushed
    pub needs_push: bool,
    /// Last sync timestamp
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// Task assignment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTaskRequest {
    /// Task ID to assign
    pub task_id: i32,
    /// Target repository ID
    pub repository_id: i32,
    /// Custom path within repository (optional)
    pub repository_path: Option<String>,
    /// Whether to immediately sync the task
    pub sync_immediately: Option<bool>,
}

/// Task move request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveTaskRequest {
    /// Task ID to move
    pub task_id: i32,
    /// Source repository ID
    pub from_repository_id: i32,
    /// Target repository ID
    pub to_repository_id: i32,
    /// New path within target repository (optional)
    pub new_repository_path: Option<String>,
    /// Whether to remove from source repository
    pub remove_from_source: Option<bool>,
    /// Whether to immediately sync to target
    pub sync_to_target: Option<bool>,
}

impl TaskAssignmentService {
    /// Create a new task assignment service
    pub fn new(
        db_service: Arc<ratchet_storage::seaorm::repositories::RepositoryService>,
        sync_service: Arc<TaskSyncService>,
    ) -> Self {
        Self {
            db_service,
            sync_service,
        }
    }

    /// Assign a new task to a repository (for tasks created via API)
    pub async fn assign_new_task_repository(&self, task_id: i32, repo_id: Option<i32>) -> Result<TaskRepositoryAssignment> {
        info!("Assigning new task {} to repository", task_id);

        let repository_id = if let Some(repo_id) = repo_id {
            // Use specified repository
            self.validate_repository_exists(repo_id).await?;
            repo_id
        } else {
            // Use default repository
            self.get_default_repository_id().await?
        };

        // TODO: Update task in database with repository assignment
        // This would involve calling the task repository to update the task's repository_id
        // For now, we'll create a placeholder assignment

        let repository = self.db_service.get_repository(repository_id).await
            .context("Failed to get repository")?
            .ok_or_else(|| anyhow!("Repository {} not found", repository_id))?;

        let repo_name = repository.name.clone();
        let assignment = TaskRepositoryAssignment {
            task_id,
            repository_id,
            repository_name: repository.name,
            repository_path: format!("tasks/task_{}.js", task_id), // Default path
            can_push: repository.is_writable,
            auto_push: repository.push_on_change,
            sync_status: "pending".to_string(),
            needs_push: true, // New task needs to be pushed
            last_synced_at: None,
        };

        info!("Task {} assigned to repository {} ({})", task_id, repository_id, repo_name);
        Ok(assignment)
    }

    /// Move a task from one repository to another
    pub async fn move_task_to_repository(&self, request: MoveTaskRequest) -> Result<TaskRepositoryAssignment> {
        info!("Moving task {} from repository {} to repository {}", 
            request.task_id, request.from_repository_id, request.to_repository_id);

        // Validate repositories
        self.validate_repository_exists(request.from_repository_id).await?;
        self.validate_repository_exists(request.to_repository_id).await?;

        let target_repo = self.db_service.get_repository(request.to_repository_id).await
            .context("Failed to get target repository")?
            .ok_or_else(|| anyhow!("Target repository {} not found", request.to_repository_id))?;

        // TODO: Implement actual task moving logic
        // This would involve:
        // 1. Getting the task from the source repository
        // 2. Updating the task's repository_id in the database
        // 3. Optionally removing from source repository
        // 4. Optionally syncing to target repository

        let new_path = request.new_repository_path
            .unwrap_or_else(|| format!("tasks/task_{}.js", request.task_id));

        let repo_name = target_repo.name.clone();
        let assignment = TaskRepositoryAssignment {
            task_id: request.task_id,
            repository_id: request.to_repository_id,
            repository_name: target_repo.name,
            repository_path: new_path,
            can_push: target_repo.is_writable,
            auto_push: target_repo.push_on_change,
            sync_status: "pending_move".to_string(),
            needs_push: true,
            last_synced_at: None,
        };

        // Handle removal from source if requested
        if request.remove_from_source.unwrap_or(false) {
            // TODO: Remove task from source repository
            debug!("Would remove task {} from source repository {}", request.task_id, request.from_repository_id);
        }

        // Handle immediate sync if requested
        if request.sync_to_target.unwrap_or(false) {
            match self.sync_service.sync_repository(request.to_repository_id).await {
                Ok(_) => {
                    info!("Task {} synced to target repository {}", request.task_id, request.to_repository_id);
                }
                Err(e) => {
                    warn!("Failed to sync task {} to target repository: {}", request.task_id, e);
                }
            }
        }

        info!("Task {} moved to repository {} ({})", request.task_id, request.to_repository_id, repo_name);
        Ok(assignment)
    }

    /// Get task repository assignment information
    pub async fn get_task_repository_assignment(&self, task_id: i32) -> Result<Option<TaskRepositoryAssignment>> {
        debug!("Getting repository assignment for task {}", task_id);

        // TODO: Get actual task from database and extract repository information
        // For now, this is a placeholder implementation
        
        // Try to find which repository this task belongs to
        let repositories = self.db_service.list_repositories().await
            .context("Failed to list repositories")?;

        for repository in repositories {
            let task_count = self.db_service.count_tasks_in_repository(repository.id).await
                .unwrap_or(0);
            
            if task_count > 0 {
                // TODO: Check if this specific task is in this repository
                // For now, we'll return the first repository with tasks as a placeholder
                let assignment = TaskRepositoryAssignment {
                    task_id,
                    repository_id: repository.id,
                    repository_name: repository.name,
                    repository_path: format!("tasks/task_{}.js", task_id),
                    can_push: repository.is_writable,
                    auto_push: repository.push_on_change,
                    sync_status: repository.sync_status,
                    needs_push: false, // TODO: Get actual sync status
                    last_synced_at: repository.last_sync_at.map(|dt| dt),
                };
                
                return Ok(Some(assignment));
            }
        }

        Ok(None)
    }

    /// Assign multiple tasks to a repository in batch
    pub async fn batch_assign_tasks(&self, task_ids: Vec<i32>, repository_id: i32) -> Result<Vec<TaskRepositoryAssignment>> {
        let task_count = task_ids.len();
        info!("Batch assigning {} tasks to repository {}", task_count, repository_id);

        self.validate_repository_exists(repository_id).await?;
        
        let mut assignments = Vec::new();
        
        for task_id in &task_ids {
            match self.assign_new_task_repository(*task_id, Some(repository_id)).await {
                Ok(assignment) => assignments.push(assignment),
                Err(e) => {
                    error!("Failed to assign task {} to repository {}: {}", task_id, repository_id, e);
                    // Continue with other tasks even if one fails
                }
            }
        }

        info!("Batch assignment completed: {}/{} tasks assigned successfully", 
            assignments.len(), task_count);
        
        Ok(assignments)
    }

    /// Get all tasks assigned to a repository
    pub async fn get_repository_task_assignments(&self, repository_id: i32) -> Result<Vec<TaskRepositoryAssignment>> {
        debug!("Getting all task assignments for repository {}", repository_id);

        self.validate_repository_exists(repository_id).await?;
        
        let _repository = self.db_service.get_repository(repository_id).await
            .context("Failed to get repository")?
            .ok_or_else(|| anyhow!("Repository {} not found", repository_id))?;

        // TODO: Get actual tasks from database for this repository
        // For now, return empty list as placeholder
        let assignments = Vec::new();

        debug!("Found {} task assignments for repository {}", assignments.len(), repository_id);
        Ok(assignments)
    }

    /// Unassign task from repository (move to default repository)
    pub async fn unassign_task_from_repository(&self, task_id: i32) -> Result<TaskRepositoryAssignment> {
        info!("Unassigning task {} from current repository", task_id);

        let default_repo_id = self.get_default_repository_id().await?;
        
        // TODO: Get current repository assignment and move to default
        // For now, assign to default repository
        self.assign_new_task_repository(task_id, Some(default_repo_id)).await
    }

    /// Set task sync status
    pub async fn set_task_sync_status(&self, task_id: i32, status: &str, needs_push: bool) -> Result<()> {
        debug!("Setting sync status for task {}: {}, needs_push: {}", task_id, status, needs_push);
        
        // TODO: Update task sync status in database
        // This would involve calling the task repository to update the task's sync status
        
        Ok(())
    }

    /// Get tasks that need to be pushed to repositories
    pub async fn get_tasks_needing_push(&self, repository_id: Option<i32>) -> Result<Vec<TaskRepositoryAssignment>> {
        debug!("Getting tasks that need to be pushed");

        if let Some(repo_id) = repository_id {
            self.validate_repository_exists(repo_id).await?;
        }

        // TODO: Query database for tasks with needs_push = true
        // Filter by repository if specified
        
        let assignments = Vec::new(); // Placeholder
        
        debug!("Found {} tasks needing push", assignments.len());
        Ok(assignments)
    }

    /// Validate that a repository exists
    async fn validate_repository_exists(&self, repository_id: i32) -> Result<()> {
        let exists = self.db_service.get_repository(repository_id).await
            .context("Failed to check repository existence")?
            .is_some();

        if !exists {
            return Err(anyhow!("Repository {} does not exist", repository_id));
        }

        Ok(())
    }

    /// Get the default repository ID
    async fn get_default_repository_id(&self) -> Result<i32> {
        let default_repo = self.db_service.get_default_repository().await
            .context("Failed to get default repository")?
            .ok_or_else(|| anyhow!("No default repository configured"))?;

        Ok(default_repo.id)
    }
}

/// Task assignment error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskAssignmentError {
    /// Task not found
    TaskNotFound { task_id: i32 },
    /// Repository not found
    RepositoryNotFound { repository_id: i32 },
    /// No default repository configured
    NoDefaultRepository,
    /// Repository is not writable
    RepositoryNotWritable { repository_id: i32 },
    /// Task already assigned to repository
    TaskAlreadyAssigned { task_id: i32, repository_id: i32 },
    /// Sync operation failed
    SyncFailed { task_id: i32, repository_id: i32, error: String },
}

impl std::fmt::Display for TaskAssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskAssignmentError::TaskNotFound { task_id } => {
                write!(f, "Task {} not found", task_id)
            }
            TaskAssignmentError::RepositoryNotFound { repository_id } => {
                write!(f, "Repository {} not found", repository_id)
            }
            TaskAssignmentError::NoDefaultRepository => {
                write!(f, "No default repository configured")
            }
            TaskAssignmentError::RepositoryNotWritable { repository_id } => {
                write!(f, "Repository {} is not writable", repository_id)
            }
            TaskAssignmentError::TaskAlreadyAssigned { task_id, repository_id } => {
                write!(f, "Task {} is already assigned to repository {}", task_id, repository_id)
            }
            TaskAssignmentError::SyncFailed { task_id, repository_id, error } => {
                write!(f, "Failed to sync task {} to repository {}: {}", task_id, repository_id, error)
            }
        }
    }
}

impl std::error::Error for TaskAssignmentError {}

#[cfg(test)]
mod tests {
    use super::{TaskAssignmentService, TaskRepositoryAssignment, AssignTaskRequest};
    
    // TODO: Add comprehensive tests for task assignment operations
    // This would include:
    // - Task assignment to repositories
    // - Task moving between repositories
    // - Batch operations
    // - Error handling scenarios
    // - Repository validation
}