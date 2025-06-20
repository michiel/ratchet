//! # Ratchet Interfaces
//!
//! Core interfaces and traits for Ratchet modular architecture.
//!
//! This crate provides the fundamental interfaces that are shared across
//! the entire Ratchet ecosystem, breaking circular dependencies between
//! the legacy ratchet-lib and new modular crates.
//!
//! ## Purpose
//!
//! During the migration from monolithic `ratchet-lib` to modular architecture,
//! this crate serves as the neutral ground for shared interfaces that both
//! legacy and new systems can depend on without creating circular dependencies.
//!
//! ## Main Interfaces
//!
//! - [`Service`] - Base service trait for all Ratchet services
//! - [`TaskExecutor`] - Core task execution interface
//! - [`StructuredLogger`] - Logging interface for structured events

pub mod database;
pub mod execution;
pub mod logging;
pub mod registry;
pub mod scheduler;
pub mod service;

// Re-export commonly used types
pub use database::{
    ApiKeyRepository, CrudRepository, DatabaseError, ExecutionFilters, ExecutionRepository, FilteredRepository,
    JobFilters, JobRepository, Repository, RepositoryFactory, ScheduleFilters, ScheduleRepository, SessionRepository,
    TaskFilters, TaskRepository, TransactionContext, TransactionManager, UserFilters, UserRepository,
};
pub use execution::{ExecutionContext, ExecutionResult, TaskExecutor};
pub use logging::{LogEvent, LogLevel, StructuredLogger};
pub use registry::{
    FilesystemRegistry, HttpCredentials, HttpRegistry, RegistryError, RegistryManager, SyncResult, TaskMetadata,
    TaskRegistry, TaskValidator, ValidationResult,
};
pub use scheduler::{ScheduleStatus, SchedulerError, SchedulerService};
pub use service::{HealthStatus, Service, ServiceHealth, ServiceMetrics};
