use std::sync::Arc;
use tracing::{error, info};

use crate::error::{RegistryError, Result};
use crate::sync::ConflictResolver;
use crate::types::{DiscoveredTask, SyncResult, TaskReference};
use ratchet_storage::repositories::BaseRepository;

pub struct DatabaseSync {
    task_repo: Arc<ratchet_storage::repositories::TaskRepository>,
    conflict_resolver: ConflictResolver,
}

impl DatabaseSync {
    pub fn new(task_repo: Arc<ratchet_storage::repositories::TaskRepository>) -> Self {
        Self {
            task_repo,
            conflict_resolver: ConflictResolver::new(),
        }
    }

    pub fn with_conflict_resolver(mut self, resolver: ConflictResolver) -> Self {
        self.conflict_resolver = resolver;
        self
    }

    pub async fn sync_discovered_tasks(&self, tasks: Vec<DiscoveredTask>) -> Result<SyncResult> {
        let mut result = SyncResult::new();

        for discovered in tasks {
            let task_ref = TaskReference {
                name: discovered.metadata.name.clone(),
                version: discovered.metadata.version.clone(),
                source: discovered.task_ref.source.clone(),
            };
            
            match self.sync_single_task(discovered).await {
                Ok(sync_type) => match sync_type {
                    SyncType::Added => result.tasks_added += 1,
                    SyncType::Updated => result.tasks_updated += 1,
                    SyncType::Skipped => {
                        // No change to counters
                    }
                },
                Err(e) => {
                    error!("Failed to sync task: {}", e);
                    result.add_error(task_ref, e.to_string());
                }
            }
        }

        info!(
            "Sync completed: {} added, {} updated, {} errors",
            result.tasks_added,
            result.tasks_updated,
            result.errors.len()
        );

        Ok(result)
    }

    pub async fn cleanup_removed_tasks(&self, active_tasks: &[TaskReference]) -> Result<()> {
        // TODO: Implement cleanup logic
        // This would:
        // 1. Query all tasks from database
        // 2. Compare with active_tasks list
        // 3. Mark tasks as unavailable if not in active list
        // 4. Preserve historical data

        info!(
            "Cleanup would process {} active tasks (not yet implemented)",
            active_tasks.len()
        );

        Ok(())
    }

    async fn sync_single_task(&self, discovered: DiscoveredTask) -> Result<SyncType> {
        // Check if task already exists in database
        // For now, just check by UUID since find_by_name_and_version doesn't exist yet
        let existing_task = self
            .task_repo
            .find_by_uuid(discovered.metadata.uuid)
            .await
            .map_err(RegistryError::Storage)?;

        match existing_task {
            Some(existing) => {
                // Task exists, check if we need to update
                if self.should_update_task(&existing, &discovered) {
                    // Apply conflict resolution strategy
                    match self.conflict_resolver.resolve_conflict(&existing, &discovered) {
                        ConflictResolution::UseRegistry => {
                            self.update_task_in_database(discovered).await?;
                            Ok(SyncType::Updated)
                        }
                        ConflictResolution::UseDatabase => {
                            info!(
                                "Keeping database version of task {} {}",
                                discovered.metadata.name, discovered.metadata.version
                            );
                            Ok(SyncType::Skipped)
                        }
                        ConflictResolution::Merge => {
                            // TODO: Implement merge strategy
                            self.update_task_in_database(discovered).await?;
                            Ok(SyncType::Updated)
                        }
                    }
                } else {
                    Ok(SyncType::Skipped)
                }
            }
            None => {
                // Task doesn't exist, add it
                self.add_task_to_database(discovered).await?;
                Ok(SyncType::Added)
            }
        }
    }

    fn should_update_task(
        &self,
        _existing: &ratchet_storage::entities::task::Task,
        _discovered: &DiscoveredTask,
    ) -> bool {
        // TODO: Implement proper comparison logic
        // This could compare:
        // - Last modified timestamps
        // - Checksums
        // - Version numbers
        // For now, always assume we should update
        true
    }

    async fn add_task_to_database(&self, discovered: DiscoveredTask) -> Result<()> {
        // Convert discovered task to storage entity
        let task_entity = self.convert_to_entity(discovered)?;

        self.task_repo
            .create(&task_entity)
            .await
            .map_err(RegistryError::Storage)?;

        Ok(())
    }

    async fn update_task_in_database(&self, discovered: DiscoveredTask) -> Result<()> {
        // Convert discovered task to storage entity
        let task_entity = self.convert_to_entity(discovered)?;

        self.task_repo
            .update(&task_entity)
            .await
            .map_err(RegistryError::Storage)?;

        Ok(())
    }

    fn convert_to_entity(
        &self,
        discovered: DiscoveredTask,
    ) -> Result<ratchet_storage::entities::task::Task> {
        // Convert DiscoveredTask to storage Task entity
        let task = ratchet_storage::entities::task::Task::new(
            discovered.metadata.name,
            discovered.metadata.version,
            discovered.task_ref.source,
            serde_json::json!({}), // input_schema placeholder
            serde_json::json!({}), // output_schema placeholder
        );
        
        Ok(task)
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