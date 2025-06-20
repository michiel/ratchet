//! Unified Ratchet Server
//!
//! This crate provides a unified server that combines REST and GraphQL APIs
//! along with all necessary services, demonstrating the new modular architecture.

pub mod config;
pub mod services;
pub mod startup;
pub mod bridges;
pub mod embedded;
pub mod scheduler;
pub mod heartbeat;
pub mod job_processor;

// Re-export main components
pub use config::*;
pub use services::*;
pub use startup::*;