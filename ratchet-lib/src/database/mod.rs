pub mod entities;
pub mod migrations;
pub mod repositories;
pub mod connection;

pub use connection::{DatabaseConnection, DatabaseError};
pub use repositories::{TaskRepository, ExecutionRepository, ScheduleRepository, JobRepository};

// Re-export commonly used Sea-ORM types
pub use sea_orm::{Database, DatabaseConnection as SeaConnection, DbErr, ActiveModelTrait, EntityTrait};