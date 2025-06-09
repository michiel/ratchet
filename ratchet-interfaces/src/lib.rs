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

pub mod service;
pub mod execution;
pub mod logging;

// Re-export commonly used types
pub use service::{Service, ServiceHealth, ServiceMetrics, HealthStatus};
pub use execution::{TaskExecutor, ExecutionResult, ExecutionContext};
pub use logging::{LogEvent, LogLevel, StructuredLogger};