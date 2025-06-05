//! SeaORM implementation for Ratchet storage layer
//! 
//! This module provides a complete SeaORM-based database implementation including
//! entities, migrations, repositories, and connection management.

#[cfg(feature = "seaorm")]
pub mod config;
#[cfg(feature = "seaorm")]
pub mod connection;
#[cfg(feature = "seaorm")]
pub mod entities;
#[cfg(feature = "seaorm")]
pub mod migrations;
#[cfg(feature = "seaorm")]
pub mod repositories;
#[cfg(feature = "seaorm")]
pub mod filters;
#[cfg(feature = "seaorm")]
pub mod safe_errors;

#[cfg(feature = "seaorm")]
pub use config::DatabaseConfig;
#[cfg(feature = "seaorm")]
pub use connection::{DatabaseConnection, DatabaseError};
#[cfg(feature = "seaorm")]
pub use entities::*;
#[cfg(feature = "seaorm")]
pub use filters::{SafeFilterBuilder, validation};
#[cfg(feature = "seaorm")]
pub use safe_errors::SafeDatabaseError;

// Re-export common SeaORM types for convenience
#[cfg(feature = "seaorm")]
pub use sea_orm::{
    Database, DatabaseConnection as SeaOrmConnection, 
    ConnectOptions, ConnectionTrait, EntityTrait, ModelTrait,
    ActiveModelTrait, QueryFilter, QueryOrder, PaginatorTrait,
    TransactionTrait, DatabaseTransaction,
};

#[cfg(feature = "seaorm")]
pub use sea_orm_migration::{
    MigratorTrait, Migration, MigrationTrait,
    SchemaManager,
};
pub use sea_orm::Schema;