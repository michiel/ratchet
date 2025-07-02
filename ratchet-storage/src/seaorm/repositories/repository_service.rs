use crate::seaorm::entities::{TaskRepository, TaskRepositoryActiveModel, TaskRepositories, Tasks};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, DbErr, PaginatorTrait};
use std::sync::Arc;

// Re-export types from ratchet-api-types to avoid duplication
pub use ratchet_api_types::{CreateRepositoryRequest, UpdateRepositoryRequest, ConnectionTestResult};

#[derive(Clone)]
pub struct RepositoryService {
    db: Arc<DatabaseConnection>,
}

impl RepositoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// List all repositories
    pub async fn list_repositories(&self) -> Result<Vec<TaskRepository>, DbErr> {
        TaskRepositories::find().all(self.db.as_ref()).await
    }

    /// Get repository by ID
    pub async fn get_repository(&self, id: i32) -> Result<Option<TaskRepository>, DbErr> {
        TaskRepositories::find_by_id(id).one(self.db.as_ref()).await
    }

    /// Get default repository
    pub async fn get_default_repository(&self) -> Result<Option<TaskRepository>, DbErr> {
        TaskRepositories::find()
            .filter(crate::seaorm::entities::task_repositories::Column::IsDefault.eq(true))
            .one(self.db.as_ref())
            .await
    }

    /// Create a new repository
    pub async fn create_repository(&self, request: CreateRepositoryRequest) -> Result<TaskRepository, DbErr> {
        let now = chrono::Utc::now();
        
        // If this is set as default, unset any existing default
        if request.is_default.unwrap_or(false) {
            self.unset_all_defaults().await?;
        }

        let new_repo = TaskRepositoryActiveModel {
            name: Set(request.name),
            repository_type: Set(request.repository_type),
            uri: Set(request.uri),
            branch: Set(request.branch),
            auth_config: Set(request.auth_config),
            sync_enabled: Set(request.sync_enabled.unwrap_or(true)),
            sync_interval_minutes: Set(request.sync_interval_minutes),
            sync_status: Set("pending".to_string()),
            priority: Set(1), // Default priority
            is_default: Set(request.is_default.unwrap_or(false)),
            is_writable: Set(request.is_writable.unwrap_or(true)),
            watch_patterns: Set(serde_json::json!(request.watch_patterns.unwrap_or_else(|| vec!["**/*.js".to_string()]))),
            ignore_patterns: Set(serde_json::json!(request.ignore_patterns.unwrap_or_else(|| vec!["node_modules/**".to_string()]))),
            push_on_change: Set(request.push_on_change.unwrap_or(false)),
            metadata: Set(request.metadata.unwrap_or_else(|| serde_json::json!({}))),
            created_at: Set(chrono::DateTime::from_naive_utc_and_offset(now.naive_utc(), chrono::Utc)),
            updated_at: Set(chrono::DateTime::from_naive_utc_and_offset(now.naive_utc(), chrono::Utc)),
            ..Default::default()
        };

        let inserted = new_repo.insert(self.db.as_ref()).await?;
        Ok(inserted)
    }

    /// Update an existing repository
    pub async fn update_repository(&self, id: i32, request: UpdateRepositoryRequest) -> Result<Option<TaskRepository>, DbErr> {
        if let Some(existing) = self.get_repository(id).await? {
            let now = chrono::Utc::now();
            
            // If this is set as default, unset any existing default
            if request.is_default == Some(true) && !existing.is_default {
                self.unset_all_defaults().await?;
            }

            let mut update_model: TaskRepositoryActiveModel = existing.into();
            
            if let Some(name) = request.name {
                update_model.name = Set(name);
            }
            if let Some(uri) = request.uri {
                update_model.uri = Set(uri);
            }
            if let Some(branch) = request.branch {
                update_model.branch = Set(Some(branch));
            }
            if let Some(auth_config) = request.auth_config {
                update_model.auth_config = Set(Some(auth_config));
            }
            if let Some(sync_enabled) = request.sync_enabled {
                update_model.sync_enabled = Set(sync_enabled);
            }
            if let Some(sync_interval_minutes) = request.sync_interval_minutes {
                update_model.sync_interval_minutes = Set(Some(sync_interval_minutes));
            }
            if let Some(is_default) = request.is_default {
                update_model.is_default = Set(is_default);
            }
            if let Some(is_writable) = request.is_writable {
                update_model.is_writable = Set(is_writable);
            }
            if let Some(watch_patterns) = request.watch_patterns {
                update_model.watch_patterns = Set(serde_json::json!(watch_patterns));
            }
            if let Some(ignore_patterns) = request.ignore_patterns {
                update_model.ignore_patterns = Set(serde_json::json!(ignore_patterns));
            }
            if let Some(push_on_change) = request.push_on_change {
                update_model.push_on_change = Set(push_on_change);
            }
            if let Some(metadata) = request.metadata {
                update_model.metadata = Set(metadata);
            }
            
            update_model.updated_at = Set(chrono::DateTime::from_naive_utc_and_offset(now.naive_utc(), chrono::Utc));

            let updated = update_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    /// Delete a repository
    pub async fn delete_repository(&self, id: i32) -> Result<bool, DbErr> {
        // Check if repository has tasks
        let task_count = Tasks::find()
            .filter(crate::seaorm::entities::tasks::Column::RepositoryId.eq(id))
            .count(self.db.as_ref())
            .await?;

        if task_count > 0 {
            return Err(DbErr::Custom(format!("Cannot delete repository with {} active tasks", task_count)));
        }

        let result = TaskRepositories::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(result.rows_affected > 0)
    }

    /// Set a repository as default
    pub async fn set_default_repository(&self, id: i32) -> Result<Option<TaskRepository>, DbErr> {
        // First unset all defaults
        self.unset_all_defaults().await?;

        // Then set this one as default
        if let Some(existing) = self.get_repository(id).await? {
            let mut update_model: TaskRepositoryActiveModel = existing.into();
            update_model.is_default = Set(true);
            update_model.updated_at = Set(chrono::DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), chrono::Utc));
            
            let updated = update_model.update(self.db.as_ref()).await?;
            Ok(Some(updated))
        } else {
            Ok(None)
        }
    }

    /// Test repository connection
    pub async fn test_repository_connection(&self, _id: i32) -> Result<ConnectionTestResult, DbErr> {
        // TODO: Implement actual connection testing based on repository type
        Ok(ConnectionTestResult {
            success: true,
            message: "Connection test not yet implemented".to_string(),
            details: None,
        })
    }

    /// Count tasks in a repository
    pub async fn count_tasks_in_repository(&self, id: i32) -> Result<u64, DbErr> {
        Tasks::find()
            .filter(crate::seaorm::entities::tasks::Column::RepositoryId.eq(id))
            .count(self.db.as_ref())
            .await
    }

    /// Helper method to unset all default repositories
    async fn unset_all_defaults(&self) -> Result<(), DbErr> {
        // Get all default repositories and update them
        let default_repos = TaskRepositories::find()
            .filter(crate::seaorm::entities::task_repositories::Column::IsDefault.eq(true))
            .all(self.db.as_ref())
            .await?;

        for repo in default_repos {
            let mut update_model: TaskRepositoryActiveModel = repo.into();
            update_model.is_default = Set(false);
            update_model.updated_at = Set(chrono::DateTime::from_naive_utc_and_offset(chrono::Utc::now().naive_utc(), chrono::Utc));
            update_model.update(self.db.as_ref()).await?;
        }

        Ok(())
    }
}