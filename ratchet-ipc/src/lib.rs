//! Inter-process communication for Ratchet
//! 
//! This crate provides the IPC protocol and transport abstractions used for
//! communication between the coordinator and worker processes.

pub mod protocol;
pub mod transport;
pub mod error;

// Re-export commonly used types
pub use protocol::{
    WorkerMessage, CoordinatorMessage, ExecutionContext,
    TaskExecutionResult, TaskValidationResult, WorkerStatus,
    WorkerError, MessageEnvelope, IPC_PROTOCOL_VERSION,
};
pub use transport::{IpcTransport, StdioTransport};
pub use error::IpcError;