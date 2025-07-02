pub mod api_key_repository;
pub mod execution_repository;
pub mod job_repository;
pub mod repository_service;
pub mod schedule_repository;
pub mod session_repository;
pub mod task_repository;
pub mod user_repository;

pub use api_key_repository::SeaOrmApiKeyRepository;
pub use execution_repository::ExecutionRepository;
pub use job_repository::JobRepository;
pub use repository_service::RepositoryService;
pub use schedule_repository::ScheduleRepository;
pub use session_repository::SeaOrmSessionRepository;
pub use task_repository::TaskRepository;
pub use user_repository::SeaOrmUserRepository;

use crate::seaorm::connection::DatabaseError;
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
    pub user_repo: SeaOrmUserRepository,
    pub session_repo: SeaOrmSessionRepository,
    pub api_key_repo: SeaOrmApiKeyRepository,
    pub repository_service: RepositoryService,
    db: crate::seaorm::connection::DatabaseConnection,
}

impl RepositoryFactory {
    /// Create a new repository factory with shared database connection
    pub fn new(db: crate::seaorm::connection::DatabaseConnection) -> Self {
        Self {
            task_repo: TaskRepository::new(db.clone()),
            execution_repo: ExecutionRepository::new(db.clone()),
            schedule_repo: ScheduleRepository::new(db.clone()),
            job_repo: JobRepository::new(db.clone()),
            user_repo: SeaOrmUserRepository::new(db.clone()),
            session_repo: SeaOrmSessionRepository::new(db.clone()),
            api_key_repo: SeaOrmApiKeyRepository::new(db.clone()),
            repository_service: RepositoryService::new(std::sync::Arc::new(db.get_connection().clone())),
            db,
        }
    }

    /// Get the database connection
    pub fn database(&self) -> &crate::seaorm::connection::DatabaseConnection {
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

    /// Get the user repository
    pub fn user_repository(&self) -> SeaOrmUserRepository {
        self.user_repo.clone()
    }

    /// Get the session repository
    pub fn session_repository(&self) -> SeaOrmSessionRepository {
        self.session_repo.clone()
    }

    /// Get the API key repository
    pub fn api_key_repository(&self) -> SeaOrmApiKeyRepository {
        self.api_key_repo.clone()
    }

    /// Get the repository service
    pub fn repository_service(&self) -> RepositoryService {
        self.repository_service.clone()
    }
}
