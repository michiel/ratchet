//! Inter-process communication for Ratchet
//!
//! This crate provides the IPC protocol and transport abstractions used for
//! communication between the coordinator and worker processes.

pub mod error;
pub mod protocol;
pub mod transport;

// Re-export commonly used types
pub use error::IpcError;
pub use protocol::{
    CoordinatorMessage, ExecutionContext, MessageEnvelope, TaskExecutionResult, TaskValidationResult, WorkerError,
    WorkerMessage, WorkerStatus, IPC_PROTOCOL_VERSION,
};
pub use transport::{IpcTransport, StdioTransport};
