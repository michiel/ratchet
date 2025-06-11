//! # ⚠️ DEPRECATED Entity Models
//!
//! **All entity models in this module are deprecated as of version 0.4.0 and will be removed in version 0.5.0.**
//!
//! ## Migration Required
//! Entity models have been moved to `ratchet-storage` and unified in `ratchet-api-types`.
//!
//! ### Migration Guide
//! ```rust
//! // OLD (deprecated):
//! use ratchet_lib::database::entities::{Task, Execution, Job, Schedule, DeliveryResult};
//!
//! // NEW (modern):
//! use ratchet_api_types::{UnifiedTask, UnifiedExecution, UnifiedJob, UnifiedSchedule, UnifiedDeliveryResult};
//! use ratchet_storage::entities::{task, execution, job, schedule, delivery_result};
//! ```

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::entities::delivery_result instead. Will be removed in 0.5.0"
)]
pub mod delivery_results;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::entities::execution instead. Will be removed in 0.5.0"
)]
pub mod executions;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::entities::job instead. Will be removed in 0.5.0"
)]
pub mod jobs;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::entities::schedule instead. Will be removed in 0.5.0"
)]
pub mod schedules;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_storage::entities::task instead. Will be removed in 0.5.0"
)]
pub mod tasks;

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::UnifiedDeliveryResult instead. Will be removed in 0.5.0"
)]
pub use delivery_results::{
    ActiveModel as DeliveryResultActiveModel, Column as DeliveryResultColumn,
    Entity as DeliveryResults, Model as DeliveryResult,
};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::UnifiedExecution and ratchet_api_types::ExecutionStatus instead. Will be removed in 0.5.0"
)]
pub use executions::{
    ActiveModel as ExecutionActiveModel, Column as ExecutionColumn, Entity as Executions,
    ExecutionStatus, Model as Execution,
};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::{UnifiedJob, JobPriority, JobStatus} instead. Will be removed in 0.5.0"
)]
pub use jobs::{
    ActiveModel as JobActiveModel, Column as JobColumn, Entity as Jobs, JobPriority, JobStatus,
    Model as Job,
};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::UnifiedSchedule instead. Will be removed in 0.5.0"
)]
pub use schedules::{
    ActiveModel as ScheduleActiveModel, Column as ScheduleColumn, Entity as Schedules,
    Model as Schedule,
};

#[deprecated(
    since = "0.4.0", 
    note = "Use ratchet_api_types::UnifiedTask instead. Will be removed in 0.5.0"
)]
pub use tasks::{
    ActiveModel as TaskActiveModel, Column as TaskColumn, Entity as Tasks, Model as Task,
};
