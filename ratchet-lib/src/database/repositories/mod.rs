pub mod task_repository;
pub mod execution_repository;
pub mod schedule_repository;
pub mod job_repository;

pub use task_repository::TaskRepository;
pub use execution_repository::ExecutionRepository;
pub use schedule_repository::ScheduleRepository;
pub use job_repository::JobRepository;

use crate::database::DatabaseError;
use async_trait::async_trait;

/// Common repository trait for all database operations
#[async_trait]
pub trait Repository: Send + Sync {
    /// Health check for the repository
    async fn health_check(&self) -> Result<(), DatabaseError>;
}

/// Repository factory for creating all repositories with shared connection
#[derive(Clone)]
pub struct RepositoryFactory {
    pub task_repo: TaskRepository,
    pub execution_repo: ExecutionRepository,
    pub schedule_repo: ScheduleRepository,
    pub job_repo: JobRepository,
}

impl RepositoryFactory {
    /// Create a new repository factory with shared database connection
    pub fn new(db: crate::database::DatabaseConnection) -> Self {
        Self {
            task_repo: TaskRepository::new(db.clone()),
            execution_repo: ExecutionRepository::new(db.clone()),
            schedule_repo: ScheduleRepository::new(db.clone()),
            job_repo: JobRepository::new(db),
        }
    }
}