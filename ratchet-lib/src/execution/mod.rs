// Process-based architecture modules for Send/Sync compliance
pub mod ipc;
pub mod process_executor;
pub mod worker;
pub mod worker_process;

// Keep common modules that are still used
pub mod executor;
pub mod job_queue;

// New improvement modules
// pub mod graceful_shutdown;
pub mod load_balancer;
pub mod retry;
pub mod task_cache;

// Process-based architecture exports (Send/Sync compliant)
pub use ipc::{CoordinatorMessage, StdioTransport, WorkerMessage};
pub use process_executor::ProcessTaskExecutor;
pub use worker::Worker;
pub use worker_process::{WorkerProcess, WorkerProcessManager, WorkerProcessStatus};

// Re-export common traits and types from executor and job_queue
pub use executor::{ExecutionContext, ExecutionError, ExecutionResult, TaskExecutor};
pub use job_queue::{JobQueue, JobQueueManager};
