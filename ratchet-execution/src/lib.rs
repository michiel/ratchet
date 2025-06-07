//! Ratchet Execution Engine
//! 
//! This crate provides the core task execution functionality for Ratchet,
//! including the ProcessTaskExecutor and related components that were
//! extracted from ratchet-lib to break circular dependencies.

pub mod error;
pub mod executor;
pub mod ipc;
pub mod process;
pub mod worker;

// Re-export main types
pub use error::{ExecutionError, ExecutionResult};
pub use executor::{LocalExecutionContext, TaskExecutor};
pub use process::{ProcessTaskExecutor, ProcessExecutorConfig};
pub use worker::{WorkerConfig, WorkerProcess, WorkerProcessManager, WorkerProcessStatus};

// Re-export IPC types for backward compatibility
pub use ipc::{
    CoordinatorMessage, ExecutionContext as IpcExecutionContext, 
    MessageEnvelope, StdioTransport, TaskExecutionResult, TaskValidationResult,
    WorkerMessage, WorkerStatus
};