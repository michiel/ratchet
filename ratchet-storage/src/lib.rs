//! Storage abstraction and repository pattern for Ratchet
//!
//! This crate provides a generic repository pattern and storage abstractions
//! that can work with multiple database backends while maintaining type safety
//! and consistency across the Ratchet application.
//!
//! ## Features
//!
//! - `seaorm` - Enables SeaORM database integration with SQLite, PostgreSQL, and MySQL support
//! - `testing` - Enables comprehensive testing utilities including mock repositories, 
//!   test database fixtures, and builder patterns. Requires `seaorm` feature.
//! - `database` - Core database functionality (included by `seaorm`)
//!
//! ## Testing
//!
//! To use the testing utilities, enable both the `testing` and `seaorm` features:
//!
//! ```toml
//! [dependencies]
//! ratchet-storage = { path = "../ratchet-storage", features = ["testing"] }
//! ```
//!
//! The testing module provides:
//! - `TestDatabase` - Isolated test database with automatic cleanup
//! - `MockFactory` - Mock repository implementations using mockall
//! - Builder patterns for creating test entities
//! - Test fixtures and utilities

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

// Repository adapters removed - unified interface approach abandoned

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
