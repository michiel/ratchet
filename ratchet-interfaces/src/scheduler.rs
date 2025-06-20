//! Scheduler service interface for task scheduling
//!
//! This module defines the core interfaces for task scheduling services
//! that can be implemented by different scheduling backends like
//! tokio-cron-scheduler, cron, etc.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ratchet_api_types::{ApiId, UnifiedSchedule};

/// Error types for scheduler operations
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Internal scheduler error: {0}")]
    Internal(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Schedule not found: {0}")]
    ScheduleNotFound(ApiId),

    #[error("Scheduler is not running")]
    NotRunning,

    #[error("Scheduler is already running")]
    AlreadyRunning,

    #[error("Repository error: {0}")]
    Repository(String),
}

/// Status information for a specific schedule
#[derive(Debug, Clone)]
pub struct ScheduleStatus {
    /// Schedule ID
    pub id: ApiId,
    /// Whether the schedule is enabled
    pub enabled: bool,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled execution time
    pub next_run: Option<DateTime<Utc>>,
    /// Whether the schedule is currently running
    pub is_running: bool,
    /// Number of times this schedule has been executed
    pub run_count: u64,
}

/// Core scheduler service interface
///
/// This trait defines the contract for task scheduling services that can
/// manage cron-based schedules and trigger job execution. Implementations
/// should handle the scheduling logic while delegating job creation and
/// execution to repository and execution services.
#[async_trait]
pub trait SchedulerService: Send + Sync {
    /// Start the scheduler service
    ///
    /// This should initialize the scheduler, load existing schedules from
    /// the repository, and begin monitoring for scheduled executions.
    async fn start(&self) -> Result<(), SchedulerError>;

    /// Stop the scheduler service
    ///
    /// This should gracefully shutdown the scheduler, stopping all
    /// scheduled tasks and cleaning up resources.
    async fn stop(&self) -> Result<(), SchedulerError>;

    /// Add a new schedule to the scheduler
    ///
    /// The schedule should be persisted to the repository before calling
    /// this method. This method adds the schedule to the active scheduler
    /// for execution monitoring.
    async fn add_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;

    /// Remove a schedule from the scheduler
    ///
    /// This removes the schedule from active monitoring but does not
    /// delete it from the repository. The schedule should be removed
    /// from the repository separately if needed.
    async fn remove_schedule(&self, schedule_id: ApiId) -> Result<(), SchedulerError>;

    /// Update an existing schedule
    ///
    /// This should handle changes to the schedule's cron expression,
    /// enabled status, or other metadata. The schedule should be
    /// updated in the repository before calling this method.
    async fn update_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;

    /// Get the status of a specific schedule
    ///
    /// Returns detailed status information about the schedule including
    /// execution history and next run time.
    async fn get_schedule_status(&self, schedule_id: ApiId) -> Result<ScheduleStatus, SchedulerError>;

    /// Check if the scheduler is currently running
    fn is_running(&self) -> bool;

    /// Get the number of active schedules
    ///
    /// Returns the count of schedules currently being monitored
    /// by the scheduler.
    async fn schedule_count(&self) -> Result<usize, SchedulerError>;
}
