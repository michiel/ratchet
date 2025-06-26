use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info};

use crate::error::{RegistryError, Result};
use crate::sync::ConflictResolver;
use crate::types::{DiscoveredTask, SyncError, SyncResult, TaskReference};

// SeaORM repository imports
use ratchet_storage::seaorm::entities::tasks;
use ratchet_storage::seaorm::repositories::RepositoryFactory;

pub struct DatabaseSync {
    repository_factory: Arc<RepositoryFactory>,
    conflict_resolver: ConflictResolver,
}

impl DatabaseSync {
    pub fn new(repository_factory: Arc<RepositoryFactory>) -> Self {
        Self {
            repository_factory,
            conflict_resolver: ConflictResolver::new(),
        }
    }

    pub fn with_conflict_resolver(mut self, resolver: ConflictResolver) -> Self {
        self.conflict_resolver = resolver;
        self
    }

    pub async fn sync_discovered_tasks(&self, tasks: Vec<DiscoveredTask>) -> Result<SyncResult> {
        info!("Starting database sync of {} discovered tasks", tasks.len());
        let mut sync_result = SyncResult {
            tasks_added: 0,
            tasks_updated: 0,
            tasks_removed: 0,
            errors: Vec::new(),
        };

        for discovered_task in tasks {
            match self.sync_single_task(&discovered_task).await {
                Ok(sync_type) => {
                    match sync_type {
                        SyncType::Added => sync_result.tasks_added += 1,
                        SyncType::Updated => sync_result.tasks_updated += 1,
                        SyncType::Skipped => {
                            // No change needed, don't increment counters
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to sync task {} v{}: {}",
                        discovered_task.metadata.name, discovered_task.metadata.version, e
                    );
                    sync_result.errors.push(SyncError {
                        task_ref: discovered_task.task_ref.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        info!(
            "Database sync completed: {} added, {} updated, {} errors",
            sync_result.tasks_added,
            sync_result.tasks_updated,
            sync_result.errors.len()
        );

        Ok(sync_result)
    }

    async fn sync_single_task(&self, discovered_task: &DiscoveredTask) -> Result<SyncType> {
        let task_repo = self.repository_factory.task_repository();

        // Check if task already exists by finding all tasks with matching name and version
        // Note: This is inefficient but TaskRepository doesn't have find_by_name_and_version
        let all_tasks = task_repo
            .find_all()
            .await
            .map_err(|e| RegistryError::Other(e.to_string()))?;

        let existing_task = all_tasks.into_iter().find(|task| {
            task.name == discovered_task.metadata.name && task.version == discovered_task.metadata.version
        });

        match existing_task {
            Some(existing) => {
                // Task exists, check for conflicts
                let resolution = self.conflict_resolver.resolve_conflict(&(), discovered_task);

                match resolution {
                    ConflictResolution::UseRegistry => {
                        // Update the existing task with registry data
                        let updated_task = self.convert_discovered_to_task_model(discovered_task, Some(existing.id))?;
                        task_repo
                            .update(updated_task)
                            .await
                            .map_err(|e| RegistryError::Other(e.to_string()))?;

                        info!(
                            "Updated task {} v{}",
                            discovered_task.metadata.name, discovered_task.metadata.version
                        );
                        Ok(SyncType::Updated)
                    }
                    ConflictResolution::UseDatabase => {
                        info!(
                            "Skipped task {} v{} (database version preferred)",
                            discovered_task.metadata.name, discovered_task.metadata.version
                        );
                        Ok(SyncType::Skipped)
                    }
                    ConflictResolution::Merge => {
                        // For now, merge is the same as UseRegistry
                        let updated_task = self.convert_discovered_to_task_model(discovered_task, Some(existing.id))?;
                        task_repo
                            .update(updated_task)
                            .await
                            .map_err(|e| RegistryError::Other(e.to_string()))?;

                        info!(
                            "Merged task {} v{}",
                            discovered_task.metadata.name, discovered_task.metadata.version
                        );
                        Ok(SyncType::Updated)
                    }
                }
            }
            None => {
                // Task doesn't exist, add it
                let new_task = self.convert_discovered_to_task_model(discovered_task, None)?;
                task_repo
                    .create(new_task)
                    .await
                    .map_err(|e| RegistryError::Other(e.to_string()))?;

                info!(
                    "Added task {} v{}",
                    discovered_task.metadata.name, discovered_task.metadata.version
                );
                Ok(SyncType::Added)
            }
        }
    }

    fn convert_discovered_to_task_model(
        &self,
        discovered: &DiscoveredTask,
        existing_id: Option<i32>,
    ) -> Result<tasks::Model> {
        let now = chrono::Utc::now();

        // Generate a path from the task reference and source
        let path = format!(
            "{}/{}/{}",
            discovered.task_ref.source, discovered.task_ref.name, discovered.task_ref.version
        );

        Ok(tasks::Model {
            id: existing_id.unwrap_or(0), // Will be auto-generated for new tasks
            uuid: existing_id
                .map(|_| discovered.metadata.uuid)
                .unwrap_or(discovered.metadata.uuid),
            name: discovered.metadata.name.clone(),
            description: discovered.metadata.description.clone(),
            version: discovered.metadata.version.clone(),
            path,
            metadata: serde_json::to_value(&discovered.metadata).map_err(|e| RegistryError::Json(e))?,
            input_schema: serde_json::json!({}), // Default empty schema - TODO: Load from task definition
            output_schema: serde_json::json!({}), // Default empty schema - TODO: Load from task definition
            enabled: true,                       // New tasks are enabled by default
            created_at: existing_id
                .map(|_| discovered.metadata.created_at)
                .unwrap_or(discovered.metadata.created_at),
            updated_at: discovered.metadata.updated_at,
            validated_at: Some(now), // Mark as validated since it came from registry
        })
    }

    pub async fn cleanup_removed_tasks(&self, active_tasks: &[TaskReference]) -> Result<()> {
        info!("Starting cleanup of removed tasks");

        let task_repo = self.repository_factory.task_repository();

        // Get all tasks from database
        let all_tasks = task_repo
            .find_all()
            .await
            .map_err(|e| RegistryError::Other(e.to_string()))?;

        // Create a set of active task references for efficient lookup
        let active_set: HashSet<(String, String)> = active_tasks
            .iter()
            .map(|task_ref| (task_ref.name.clone(), task_ref.version.clone()))
            .collect();

        let mut removed_count = 0;

        // Find tasks that are no longer active and disable them
        for db_task in all_tasks {
            let task_key = (db_task.name.clone(), db_task.version.clone());

            if !active_set.contains(&task_key) && db_task.enabled {
                // Task is no longer in registry but still enabled in database
                info!("Disabling removed task: {} v{}", db_task.name, db_task.version);

                let mut updated_task = db_task.clone();
                updated_task.enabled = false;
                updated_task.updated_at = chrono::Utc::now();

                task_repo
                    .update(updated_task)
                    .await
                    .map_err(|e| RegistryError::Other(e.to_string()))?;

                removed_count += 1;
            }
        }

        info!("Cleanup completed: {} tasks disabled", removed_count);
        Ok(())
    }
}

#[derive(Debug)]
enum SyncType {
    Added,
    Updated,
    Skipped,
}

#[derive(Debug)]
pub enum ConflictResolution {
    UseRegistry,
    UseDatabase,
    Merge,
}
