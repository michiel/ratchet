pub mod connection;
pub mod entities;
pub mod filters;
pub mod migrations;
pub mod repositories;
pub mod safe_errors;
// pub mod base_repository;

pub use connection::{DatabaseConnection, DatabaseError};
pub use filters::{validation, SafeFilterBuilder};
pub use repositories::{ExecutionRepository, JobRepository, ScheduleRepository, TaskRepository};
pub use safe_errors::{ErrorCode, SafeDatabaseError, SafeDatabaseResult, ToSafeResult};
// pub use base_repository::{BaseRepository, TransactionManager};

// Re-export commonly used Sea-ORM types
pub use sea_orm::{
    ActiveModelTrait, Database, DatabaseConnection as SeaConnection, DbErr, EntityTrait,
};
