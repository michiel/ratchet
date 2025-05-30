pub mod entities;
pub mod migrations;
pub mod repositories;
pub mod connection;
pub mod filters;
pub mod safe_errors;
// pub mod base_repository;

pub use connection::{DatabaseConnection, DatabaseError};
pub use repositories::{TaskRepository, ExecutionRepository, ScheduleRepository, JobRepository};
pub use filters::{SafeFilterBuilder, validation};
pub use safe_errors::{SafeDatabaseError, SafeDatabaseResult, ErrorCode, ToSafeResult};
// pub use base_repository::{BaseRepository, TransactionManager};

// Re-export commonly used Sea-ORM types
pub use sea_orm::{Database, DatabaseConnection as SeaConnection, DbErr, ActiveModelTrait, EntityTrait};