//! Database migration support
//!
//! This module provides migration support for the storage layer

use crate::StorageResult;

/// Migration trait for database schema changes
#[allow(async_fn_in_trait)]
pub trait Migration {
    /// Apply the migration
    async fn up(&self) -> StorageResult<()>;

    /// Rollback the migration
    async fn down(&self) -> StorageResult<()>;

    /// Get migration version
    fn version(&self) -> &str;

    /// Get migration name
    fn name(&self) -> &str;
}
