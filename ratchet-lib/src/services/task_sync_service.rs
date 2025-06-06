use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::database::entities::Task as TaskEntity;
use crate::database::repositories::TaskRepository;
use crate::errors::Result;
use crate::registry::TaskRegistry;
use crate::task::Task;

/// Service that synchronizes tasks between the registry and database
/// Provides a unified view of tasks from both sources
pub struct TaskSyncService {
    task_repo: TaskRepository,
    registry: Arc<TaskRegistry>,
}

impl TaskSyncService {
    pub fn new(task_repo: TaskRepository, registry: Arc<TaskRegistry>) -> Self {
        Self {
            task_repo,
            registry,
        }
    }

    /// Synchronize a task from registry to database
    /// This is called when a task is loaded into the registry
    pub async fn sync_task_to_db(&self, task: &Task) -> Result<()> {
        let task_uuid = task.metadata.uuid;
        let version = &task.metadata.version;

        // Check if this exact version already exists in database
        if let Ok(Some(existing)) = self.task_repo.find_by_uuid(task_uuid).await {
            if existing.version == *version {
                info!(
                    "Task {} version {} already exists in database",
                    task_uuid, version
                );
                return Ok(());
            }
        }

        // Create or update the task in database
        let task_entity = TaskEntity::from_ratchet_task(task);

        match self.task_repo.create(task_entity).await {
            Ok(_) => {
                info!(
                    "Synchronized task {} version {} to database",
                    task_uuid, version
                );
                Ok(())
            }
            Err(e) => {
                warn!("Failed to sync task {} to database: {}", task_uuid, e);
                Err(crate::errors::RatchetError::Database(e.to_string()))
            }
        }
    }

    /// Synchronize all tasks from registry to database
    pub async fn sync_all_registry_tasks(&self) -> Result<()> {
        info!("Starting registry to database synchronization");

        let tasks = self.registry.list_tasks().await?;
        let mut synced = 0;
        let mut failed = 0;

        for task in tasks {
            match self.sync_task_to_db(&task).await {
                Ok(_) => synced += 1,
                Err(e) => {
                    error!("Failed to sync task: {}", e);
                    failed += 1;
                }
            }
        }

        info!(
            "Registry sync complete: {} synced, {} failed",
            synced, failed
        );
        Ok(())
    }

    /// Get a unified list of all tasks
    /// Registry is the authoritative source - database provides additional metadata
    pub async fn list_all_tasks(&self) -> Result<Vec<UnifiedTask>> {
        let mut unified_tasks = Vec::new();

        // Get all tasks from registry
        let registry_tasks = self.registry.list_tasks().await?;

        // Get all tasks from database for additional metadata
        let db_tasks = self
            .task_repo
            .find_all()
            .await
            .map_err(|e| crate::errors::RatchetError::Database(e.to_string()))?;

        // Create a map of database tasks by UUID for quick lookup
        let db_task_map: std::collections::HashMap<Uuid, TaskEntity> =
            db_tasks.into_iter().map(|t| (t.uuid, t)).collect();

        // Build unified view with registry as source of truth
        for registry_task in registry_tasks {
            let task_uuid = registry_task.metadata.uuid;
            let db_task = db_task_map.get(&task_uuid);

            unified_tasks.push(UnifiedTask {
                // Core data from registry
                id: db_task.map(|t| t.id),
                uuid: task_uuid,
                version: registry_task.metadata.version.clone(),
                label: registry_task.metadata.label.clone(),
                description: registry_task.metadata.description.clone(),

                // Registry info
                available_versions: self.registry.list_versions(task_uuid).await?,
                registry_source: true,

                // Database metadata
                enabled: db_task.map(|t| t.enabled).unwrap_or(true),
                created_at: db_task.map(|t| t.created_at),
                updated_at: db_task.map(|t| t.updated_at),
                validated_at: db_task.and_then(|t| t.validated_at),

                // Computed fields
                in_sync: db_task.is_some(),
            });
        }

        Ok(unified_tasks)
    }

    /// Get a specific task by UUID and optional version
    pub async fn get_task(&self, uuid: Uuid, version: Option<&str>) -> Result<Option<UnifiedTask>> {
        // Try to get from registry first
        if let Some(registry_task) = self.registry.get_task(uuid, version).await? {
            // Get database info if available
            let db_task = self
                .task_repo
                .find_by_uuid(uuid)
                .await
                .map_err(|e| crate::errors::RatchetError::Database(e.to_string()))?;

            let unified = UnifiedTask {
                id: db_task.as_ref().map(|t| t.id),
                uuid: registry_task.metadata.uuid,
                version: registry_task.metadata.version.clone(),
                label: registry_task.metadata.label.clone(),
                description: registry_task.metadata.description.clone(),

                available_versions: self.registry.list_versions(uuid).await?,
                registry_source: true,

                enabled: db_task.as_ref().map(|t| t.enabled).unwrap_or(true),
                created_at: db_task.as_ref().map(|t| t.created_at),
                updated_at: db_task.as_ref().map(|t| t.updated_at),
                validated_at: db_task.as_ref().and_then(|t| t.validated_at),

                in_sync: db_task.is_some(),
            };

            return Ok(Some(unified));
        }

        Ok(None)
    }
}

/// Unified task representation combining registry and database information
#[derive(Debug, Clone)]
pub struct UnifiedTask {
    // Database ID (if task exists in database)
    pub id: Option<i32>,

    // Core task data (from registry)
    pub uuid: Uuid,
    pub version: String,
    pub label: String,
    pub description: String,

    // Registry information
    pub available_versions: Vec<String>,
    pub registry_source: bool,

    // Database metadata
    pub enabled: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub validated_at: Option<chrono::DateTime<chrono::Utc>>,

    // Sync status
    pub in_sync: bool,
}
