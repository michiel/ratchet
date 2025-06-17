//! Scheduler module for task scheduling and execution

pub mod interface;
pub mod repository_bridge;

pub use interface::{SchedulerService, SchedulerError, ScheduleStatus};
pub use repository_bridge::RepositoryBridge;

// Re-export the current implementation for now
pub use crate::scheduler_legacy::{SchedulerService as LegacySchedulerService, SchedulerConfig};