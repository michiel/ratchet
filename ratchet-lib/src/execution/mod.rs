pub mod executor;
pub mod job_queue;
pub mod scheduler;
pub mod worker_pool_simple;

// Process-based architecture modules for Send/Sync compliance
pub mod ipc;
pub mod worker_process;
pub mod worker;
pub mod process_executor;

pub use executor::{TaskExecutor, DatabaseTaskExecutor, ExecutionContext, ExecutionResult};
pub use job_queue::{JobQueue, JobQueueManager};
pub use scheduler::{TaskScheduler, ScheduleManager};
pub use worker_pool_simple::{SimpleWorkerPool, WorkerConfig};

// Process-based architecture exports (Send/Sync compliant)
pub use ipc::{WorkerMessage, CoordinatorMessage, StdioTransport};
pub use worker_process::{WorkerProcess, WorkerProcessManager, WorkerProcessStatus};
pub use worker::Worker;
pub use process_executor::ProcessTaskExecutor;