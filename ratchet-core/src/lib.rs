//! Core domain models and types for Ratchet
//! 
//! This crate contains the fundamental types and traits used throughout
//! the Ratchet system. It has minimal dependencies and defines the
//! domain language of the application.

pub mod error;
pub mod task;
pub mod execution;
pub mod types;
pub mod config;
pub mod service;

// Re-export commonly used types at the crate root
pub use error::{RatchetError, Result};
pub use task::{Task, TaskMetadata, TaskId};
pub use execution::{Execution, ExecutionId, ExecutionStatus};
pub use types::{HttpMethod, LogLevel, Priority};
pub use service::{ServiceRegistry, ServiceProvider};