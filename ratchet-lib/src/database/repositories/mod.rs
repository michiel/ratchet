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
#[async_trait(?Send)]
pub trait Repository {
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
    db: crate::database::DatabaseConnection,
}

impl RepositoryFactory {
    /// Create a new repository factory with shared database connection
    pub fn new(db: crate::database::DatabaseConnection) -> Self {
        Self {
            task_repo: TaskRepository::new(db.clone()),
            execution_repo: ExecutionRepository::new(db.clone()),
            schedule_repo: ScheduleRepository::new(db.clone()),
            job_repo: JobRepository::new(db.clone()),
            db,
        }
    }
    
    /// Get the database connection
    pub fn database(&self) -> &crate::database::DatabaseConnection {
        &self.db
    }
    
    /// Get the execution repository
    pub fn execution_repository(&self) -> ExecutionRepository {
        self.execution_repo.clone()
    }
    
    /// Get the task repository
    pub fn task_repository(&self) -> TaskRepository {
        self.task_repo.clone()
    }
    
    /// Get the schedule repository
    pub fn schedule_repository(&self) -> ScheduleRepository {
        self.schedule_repo.clone()
    }
    
    /// Get the job repository
    pub fn job_repository(&self) -> JobRepository {
        self.job_repo.clone()
    }
}