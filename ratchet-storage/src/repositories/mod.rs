//! Repository pattern implementation

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    connection::{Connection, ConnectionManager},
    entities::{Entity, Query},
    StorageResult,
};

pub mod delivery_result;
pub mod execution;
pub mod job;
pub mod schedule;
pub mod task;

// Re-export concrete repositories
pub use delivery_result::DeliveryResultRepository;
pub use execution::{ExecutionRepository, ExecutionStatistics};
pub use job::{JobRepository, QueueStats};
pub use schedule::ScheduleRepository;
pub use task::TaskRepository;


/// Base repository trait with generic CRUD operations
#[async_trait]
pub trait Repository<T>: Send + Sync
where
    T: Entity + Send + Sync,
{
    /// Health check for the repository
    async fn health_check(&self) -> StorageResult<bool>;

    /// Get connection statistics
    async fn stats(&self) -> StorageResult<crate::connection::ConnectionStats>;
}

/// Base repository trait with CRUD operations
#[async_trait]
pub trait BaseRepository<T>: Repository<T>
where
    T: Entity + Send + Sync + Clone,
{
    /// Create a new entity
    async fn create(&self, entity: &T) -> StorageResult<T>;

    /// Find entity by ID
    async fn find_by_id(&self, id: i32) -> StorageResult<Option<T>>;

    /// Find entity by UUID
    async fn find_by_uuid(&self, uuid: Uuid) -> StorageResult<Option<T>>;

    /// Update an entity
    async fn update(&self, entity: &T) -> StorageResult<T>;

    /// Delete an entity by ID
    async fn delete(&self, id: i32) -> StorageResult<bool>;

    /// Delete an entity by UUID
    async fn delete_by_uuid(&self, uuid: Uuid) -> StorageResult<bool>;

    /// Find all entities with query parameters
    async fn find_all(&self, query: &Query) -> StorageResult<Vec<T>>;

    /// Count entities matching query
    async fn count(&self, query: &Query) -> StorageResult<u64>;

    /// Check if entity exists by ID
    async fn exists(&self, id: i32) -> StorageResult<bool>;

    /// Check if entity exists by UUID
    async fn exists_by_uuid(&self, uuid: Uuid) -> StorageResult<bool>;

    /// Batch create entities
    async fn batch_create(&self, entities: &[T]) -> StorageResult<Vec<T>>;

    /// Batch update entities
    async fn batch_update(&self, entities: &[T]) -> StorageResult<Vec<T>>;

    /// Batch delete entities by IDs
    async fn batch_delete(&self, ids: &[i32]) -> StorageResult<u64>;
}

/// Base repository implementation with common functionality
#[derive(Clone)]
pub struct BaseRepositoryImpl<T> {
    connection_manager: Arc<dyn ConnectionManager>,
    table_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> BaseRepositoryImpl<T>
where
    T: Entity + Send + Sync + Clone,
{
    /// Create a new base repository
    pub fn new(
        connection_manager: Arc<dyn ConnectionManager>,
        table_name: impl Into<String>,
    ) -> Self {
        Self {
            connection_manager,
            table_name: table_name.into(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a database connection
    pub async fn get_connection(&self) -> StorageResult<Arc<dyn Connection>> {
        self.connection_manager.get_connection().await
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[async_trait]
impl<T> Repository<T> for BaseRepositoryImpl<T>
where
    T: Entity + Send + Sync,
{
    async fn health_check(&self) -> StorageResult<bool> {
        self.connection_manager.health_check().await
    }

    async fn stats(&self) -> StorageResult<crate::connection::ConnectionStats> {
        self.connection_manager.pool_stats().await
    }
}

/// Repository factory for creating repository instances
#[derive(Clone)]
pub struct RepositoryFactory {
    connection_manager: Arc<dyn ConnectionManager>,
}

impl RepositoryFactory {
    /// Create a new repository factory
    pub fn new(connection_manager: Arc<dyn ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    /// Create a task repository
    pub fn task_repository(&self) -> TaskRepository {
        TaskRepository::new(self.connection_manager.clone())
    }

    /// Create an execution repository
    pub fn execution_repository(&self) -> ExecutionRepository {
        ExecutionRepository::new(self.connection_manager.clone())
    }

    /// Create a job repository
    pub fn job_repository(&self) -> JobRepository {
        JobRepository::new(self.connection_manager.clone())
    }

    /// Create a schedule repository
    pub fn schedule_repository(&self) -> ScheduleRepository {
        ScheduleRepository::new(self.connection_manager.clone())
    }

    /// Create a delivery result repository
    pub fn delivery_result_repository(&self) -> DeliveryResultRepository {
        DeliveryResultRepository::new(self.connection_manager.clone())
    }

    /// Get the connection manager
    pub fn connection_manager(&self) -> Arc<dyn ConnectionManager> {
        self.connection_manager.clone()
    }

    /// Get a database connection (legacy compatibility method)
    pub async fn database(&self) -> crate::StorageResult<Arc<dyn crate::Connection>> {
        self.connection_manager.get_connection().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::InMemoryConnectionManager;

    #[tokio::test]
    async fn test_repository_factory() {
        let connection_manager = Arc::new(InMemoryConnectionManager::new());
        let factory = RepositoryFactory::new(connection_manager);

        // Test repository creation
        let task_repo = factory.task_repository();
        assert!(task_repo.health_check().await.unwrap());

        let execution_repo = factory.execution_repository();
        assert!(execution_repo.health_check().await.unwrap());

        let job_repo = factory.job_repository();
        assert!(job_repo.health_check().await.unwrap());

        let schedule_repo = factory.schedule_repository();
        assert!(schedule_repo.health_check().await.unwrap());

        let delivery_repo = factory.delivery_result_repository();
        assert!(delivery_repo.health_check().await.unwrap());
    }
}
