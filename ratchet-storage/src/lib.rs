//! Storage abstraction and repository pattern for Ratchet
//!
//! This crate provides a generic repository pattern and storage abstractions
//! that can work with multiple database backends while maintaining type safety
//! and consistency across the Ratchet application.

pub mod config;
pub mod connection;
pub mod entities;
pub mod error;
pub mod filters;
pub mod migrations;
pub mod repositories;
pub mod transaction;

// SeaORM implementation (feature-gated)
#[cfg(feature = "seaorm")]
pub mod seaorm;

// Database module (migration compatibility layer)
#[cfg(feature = "seaorm")]
pub mod database;

// Re-export core types for convenience
pub use config::StorageConfig;
pub use connection::{Connection, ConnectionManager};
pub use error::{StorageError, StorageResult};
pub use repositories::Repository;
pub use filters::SafeFilterBuilder;
pub use transaction::{Transaction, TransactionManager};

// Entity re-exports
pub use entities::{
    task::{Task, TaskStatus},
    execution::{Execution, ExecutionStatus},
    job::{Job, JobStatus, JobPriority},
    schedule::{Schedule, ScheduleStatus},
    delivery_result::DeliveryResult,
};

// Repository re-exports
pub use repositories::{
    task::TaskRepository,
    execution::ExecutionRepository,
    job::JobRepository,
    schedule::ScheduleRepository,
    delivery_result::DeliveryResultRepository,
};

