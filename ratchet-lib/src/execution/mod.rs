// Process-based architecture modules for Send/Sync compliance
pub mod ipc;
pub mod worker_process;
pub mod worker;
pub mod process_executor;

// Keep common modules that are still used
pub mod executor;
pub mod job_queue;

// Process-based architecture exports (Send/Sync compliant)
pub use ipc::{WorkerMessage, CoordinatorMessage, StdioTransport};
pub use worker_process::{WorkerProcess, WorkerProcessManager, WorkerProcessStatus};
pub use worker::Worker;
pub use process_executor::ProcessTaskExecutor;

// Re-export common traits and types from executor and job_queue
pub use executor::{TaskExecutor, ExecutionContext, ExecutionResult, ExecutionError};
pub use job_queue::{JobQueue, JobQueueManager};