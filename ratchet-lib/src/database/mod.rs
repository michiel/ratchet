//! # ⚠️ DEPRECATED Database Module
//!
//! **This entire module is deprecated as of version 0.4.0 and will be removed in version 0.5.0.**
//!
//! ## Migration Required
//! Database functionality has been completely moved to the `ratchet-storage` crate.
//! All users must migrate to the modern storage layer before version 0.5.0.
//!
//! ## Quick Migration Guide
//!
//! ### 1. Update Dependencies
//! ```toml
//! # Remove from Cargo.toml:
//! # ratchet_lib = { ... }
//!
//! # Add instead:
//! ratchet-storage = { path = "../ratchet-storage", features = ["seaorm"] }
//! ratchet-interfaces = { path = "../ratchet-interfaces" }
//! ```
//!
//! ### 2. Update Imports
//! ```rust
//! // OLD (deprecated):
//! use ratchet_lib::database::repositories::RepositoryFactory;
//! use ratchet_lib::database::entities::{Task, Execution, Job, Schedule};
//!
//! // NEW (modern):
//! use ratchet_storage::adapters::UnifiedRepositoryFactory;
//! use ratchet_api_types::{UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule};
//! use ratchet_interfaces::RepositoryFactory;
//! ```
//!
//! ### 3. Update Repository Usage
//! ```rust
//! // OLD (deprecated):
//! let factory = ratchet_lib::database::repositories::RepositoryFactory::new(conn);
//! let tasks = factory.task_repository().find_all().await?;
//!
//! // NEW (modern):
//! let storage_factory = ratchet_storage::seaorm::repositories::RepositoryFactory::new(conn);
//! let unified_factory = ratchet_storage::adapters::UnifiedRepositoryFactory::new(storage_factory);
//! let tasks = unified_factory.task_repository().find_enabled().await?;
//! ```
//!
//! For detailed migration assistance, see: docs/migration/database_migration.md
//!

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::connection instead. Will be removed in 0.5.0. See migration guide: docs/migration/database_migration.md"
)]
pub mod connection;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::{UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule} instead. Will be removed in 0.5.0"
)]
pub mod entities;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::filters instead. Will be removed in 0.5.0"
)]
pub mod filters;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::migrations instead. Will be removed in 0.5.0"
)]
pub mod migrations;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_interfaces::{RepositoryFactory, TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository} instead. Will be removed in 0.5.0"
)]
pub mod repositories;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::safe_errors instead. Will be removed in 0.5.0"
)]
pub mod safe_errors;

// Legacy compatibility adapter (transitional - will be removed in 0.5.0)
pub mod legacy_adapter;
pub mod legacy_adapter_impl;

// pub mod base_repository;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::connection instead. Will be removed in 0.5.0. See migration guide: docs/migration/database_migration.md"
)]
pub use connection::{DatabaseConnection, DatabaseError};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::filters instead. Will be removed in 0.5.0. See migration guide: docs/migration/database_migration.md"
)]
pub use filters::{validation, SafeFilterBuilder};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_interfaces::{TaskRepository, ExecutionRepository, JobRepository, ScheduleRepository} instead. Will be removed in 0.5.0. See migration guide: docs/migration/database_migration.md"
)]
pub use repositories::{ExecutionRepository, JobRepository, ScheduleRepository, TaskRepository};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::seaorm::safe_errors instead. Will be removed in 0.5.0. See migration guide: docs/migration/database_migration.md"
)]
pub use safe_errors::{ErrorCode, SafeDatabaseError, SafeDatabaseResult, ToSafeResult};

// Re-export legacy adapter for backward compatibility
pub use legacy_adapter::{
    LegacyDatabaseAdapter, LegacyRepositoryFactory,
    LegacyTaskRepository, LegacyExecutionRepository, LegacyJobRepository, LegacyScheduleRepository,
    LegacyTask, LegacyExecution, LegacyJob, LegacySchedule
};

// pub use base_repository::{BaseRepository, TransactionManager};

// Re-export commonly used Sea-ORM types
#[deprecated(
    since = "0.4.0", 
    note = "Import sea_orm directly instead of via ratchet_lib::database. Will be removed in 0.5.0. Use: use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, DbErr, EntityTrait};"
)]
pub use sea_orm::{
    ActiveModelTrait, Database, DatabaseConnection as SeaConnection, DbErr, EntityTrait,
};
