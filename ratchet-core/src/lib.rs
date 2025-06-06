//! Core domain models and types for Ratchet
//!
//! This crate contains the fundamental types and traits used throughout
//! the Ratchet system. It has minimal dependencies and defines the
//! domain language of the application.

pub mod config;
pub mod error;
pub mod execution;
pub mod service;
pub mod task;
pub mod types;
pub mod validation;

// Re-export commonly used types at the crate root
pub use error::{RatchetError, Result};
pub use execution::{Execution, ExecutionId, ExecutionStatus};
pub use service::{ServiceProvider, ServiceRegistry};
pub use task::{Task, TaskId, TaskMetadata};
pub use types::{HttpMethod, LogLevel, Priority};
pub use validation::{
    parse_schema, validate_json, validate_json_type, validate_json_with_schema_file,
    validate_required_fields,
};
