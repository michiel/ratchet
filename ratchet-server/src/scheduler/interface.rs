//! Scheduler service interface and error types

use std::fmt;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ratchet_api_types::{UnifiedSchedule, ApiId};

/// Errors that can occur in the scheduler service
#[derive(Debug)]
pub enum SchedulerError {
    /// Database operation failed
    Database(String),
    /// Invalid cron expression
    InvalidCron(String),
    /// Schedule not found
    ScheduleNotFound(ApiId),
    /// Scheduler is not running
    NotRunning,
    /// Internal scheduler error
    Internal(String),
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerError::Database(msg) => write!(f, "Database error: {}", msg),
            SchedulerError::InvalidCron(expr) => write!(f, "Invalid cron expression: {}", expr),
            SchedulerError::ScheduleNotFound(id) => write!(f, "Schedule not found: {}", id),
            SchedulerError::NotRunning => write!(f, "Scheduler is not running"),
            SchedulerError::Internal(msg) => write!(f, "Internal scheduler error: {}", msg),
        }
    }
}

impl std::error::Error for SchedulerError {}

/// Status of a schedule in the scheduler
#[derive(Debug, Clone)]
pub struct ScheduleStatus {
    pub id: ApiId,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub is_running: bool,
    pub run_count: u64,
}

/// Abstract scheduler service interface
#[async_trait]
pub trait SchedulerService: Send + Sync {
    /// Start the scheduler service
    async fn start(&self) -> Result<(), SchedulerError>;
    
    /// Stop the scheduler service
    async fn stop(&self) -> Result<(), SchedulerError>;
    
    /// Add a new schedule to the scheduler
    async fn add_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;
    
    /// Remove a schedule from the scheduler
    async fn remove_schedule(&self, schedule_id: ApiId) -> Result<(), SchedulerError>;
    
    /// Update an existing schedule
    async fn update_schedule(&self, schedule: UnifiedSchedule) -> Result<(), SchedulerError>;
    
    /// Get the status of a specific schedule
    async fn get_schedule_status(&self, schedule_id: ApiId) -> Result<ScheduleStatus, SchedulerError>;
    
    /// Check if the scheduler is running
    fn is_running(&self) -> bool;
    
    /// Get the number of active schedules
    async fn schedule_count(&self) -> Result<usize, SchedulerError>;
}