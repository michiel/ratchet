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

// Testing utilities (feature-gated for testing)
#[cfg(any(test, feature = "testing"))]
pub mod testing;

// Migration utilities (feature-gated)
#[cfg(feature = "seaorm")]
pub mod migration;

// Repository adapters for interface unification (temporarily disabled due to interface mismatches)
// #[cfg(feature = "seaorm")]
// pub mod adapters;

// Re-export core types for convenience
pub use config::StorageConfig;
pub use connection::{Connection, ConnectionManager};
pub use error::{StorageError, StorageResult};
pub use filters::SafeFilterBuilder;
pub use transaction::{Transaction, TransactionManager};

// Entity re-exports
pub use entities::{
    delivery_result::DeliveryResult,
    execution::{Execution, ExecutionStatus},
    job::{Job, JobPriority, JobStatus},
    schedule::{Schedule, ScheduleStatus},
    task::{Task, TaskStatus},
    Query, // Common query types
};

// Repository re-exports
pub use repositories::{
    BaseRepository, Repository, BaseRepositoryImpl, RepositoryFactory,
    delivery_result::DeliveryResultRepository, 
    execution::{ExecutionRepository, ExecutionStatistics}, 
    job::{JobRepository, QueueStats},
    schedule::ScheduleRepository, 
    task::TaskRepository,
};
