//! Enhanced console commands with full MCP integration

pub mod enhanced_task;
pub mod template;
pub mod execution;
pub mod monitor;
pub mod job;

pub use enhanced_task::EnhancedTaskCommand;
pub use template::TemplateCommand;
pub use execution::ExecutionCommand;
pub use monitor::MonitorCommand;
pub use job::JobCommand;