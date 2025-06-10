use tracing::info;

use crate::sync::database::ConflictResolution;
use crate::types::DiscoveredTask;

#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    /// Always use the registry version
    PreferRegistry,
    /// Always use the database version
    PreferDatabase,
    /// Use the newer version based on timestamps
    PreferNewer,
    /// Attempt to merge changes (advanced)
    Merge,
}

impl Default for ConflictStrategy {
    fn default() -> Self {
        Self::PreferRegistry
    }
}

pub struct ConflictResolver {
    strategy: ConflictStrategy,
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver {
    pub fn new() -> Self {
        Self {
            strategy: ConflictStrategy::default(),
        }
    }

    pub fn with_strategy(mut self, strategy: ConflictStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn resolve_conflict(
        &self,
        existing: &ratchet_storage::entities::task::Task,
        discovered: &DiscoveredTask,
    ) -> ConflictResolution {
        match self.strategy {
            ConflictStrategy::PreferRegistry => {
                info!(
                    "Conflict resolution: preferring registry version for task {} {}",
                    discovered.metadata.name, discovered.metadata.version
                );
                ConflictResolution::UseRegistry
            }
            ConflictStrategy::PreferDatabase => {
                info!(
                    "Conflict resolution: preferring database version for task {} {}",
                    discovered.metadata.name, discovered.metadata.version
                );
                ConflictResolution::UseDatabase
            }
            ConflictStrategy::PreferNewer => {
                // Compare timestamps to determine which is newer
                if self.is_registry_newer(existing, discovered) {
                    info!(
                        "Conflict resolution: registry version is newer for task {} {}",
                        discovered.metadata.name, discovered.metadata.version
                    );
                    ConflictResolution::UseRegistry
                } else {
                    info!(
                        "Conflict resolution: database version is newer for task {} {}",
                        discovered.metadata.name, discovered.metadata.version
                    );
                    ConflictResolution::UseDatabase
                }
            }
            ConflictStrategy::Merge => {
                info!(
                    "Conflict resolution: attempting merge for task {} {}",
                    discovered.metadata.name, discovered.metadata.version
                );
                // For now, merging is not implemented, so fall back to registry
                ConflictResolution::UseRegistry
            }
        }
    }

    fn is_registry_newer(
        &self,
        existing: &ratchet_storage::entities::task::Task,
        discovered: &DiscoveredTask,
    ) -> bool {
        // Compare updated_at timestamps
        // For now, this is a placeholder since we need to understand the actual
        // storage entity structure better
        
        // Assuming the storage entity has an updated_at field
        discovered.metadata.updated_at > existing.updated_at
    }
}