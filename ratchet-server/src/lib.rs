//! Unified Ratchet Server
//!
//! This crate provides a unified server that combines REST and GraphQL APIs
//! along with all necessary services, demonstrating the new modular architecture.

pub mod bridges;
pub mod config;
pub mod embedded;
pub mod heartbeat;
pub mod job_processor;
pub mod mcp_handler;
pub mod monitoring;
pub mod repository_services;
pub mod scheduler;
pub mod security;
pub mod services;
pub mod startup;
pub mod task_service;
pub mod watchers;

// Re-export main components
pub use config::*;
pub use services::*;
pub use startup::*;
