//! # Deprecated Database Module
//!
//! **This module is deprecated and will be removed in a future version.**
//!
//! Database functionality has been moved to the `ratchet-storage` crate.
//! Please migrate to using `ratchet-storage` for all database operations.
//!
//! Migration path:
//! - Replace `ratchet_lib::database::*` with `ratchet_storage::*`
//! - Use `ratchet_storage::seaorm::repositories::RepositoryFactory` instead of `ratchet_lib::database::repositories::RepositoryFactory`
//!

#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod connection;
#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod entities;
#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod filters;
#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod migrations;
#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod repositories;
#[deprecated(since = "0.1.0", note = "Use ratchet-storage crate instead")]
pub mod safe_errors;
// pub mod base_repository;

#[deprecated(since = "0.1.0", note = "Use ratchet_storage::seaorm::connection instead")]
pub use connection::{DatabaseConnection, DatabaseError};
#[deprecated(since = "0.1.0", note = "Use ratchet_storage::seaorm::filters instead")]
pub use filters::{validation, SafeFilterBuilder};
#[deprecated(since = "0.1.0", note = "Use ratchet_storage::seaorm::repositories instead")]
pub use repositories::{ExecutionRepository, JobRepository, ScheduleRepository, TaskRepository};
#[deprecated(since = "0.1.0", note = "Use ratchet_storage::seaorm::safe_errors instead")]
pub use safe_errors::{ErrorCode, SafeDatabaseError, SafeDatabaseResult, ToSafeResult};
// pub use base_repository::{BaseRepository, TransactionManager};

// Re-export commonly used Sea-ORM types
#[deprecated(since = "0.1.0", note = "Use sea_orm directly")]
pub use sea_orm::{
    ActiveModelTrait, Database, DatabaseConnection as SeaConnection, DbErr, EntityTrait,
};
