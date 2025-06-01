use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Job priority enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum JobPriority {
    #[sea_orm(string_value = "low")]
    Low,
    #[sea_orm(string_value = "normal")]
    Normal,
    #[sea_orm(string_value = "high")]
    High,
    #[sea_orm(string_value = "urgent")]
    Urgent,
}

/// Job status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum JobStatus {
    #[sea_orm(string_value = "queued")]
    Queued,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    #[sea_orm(string_value = "retrying")]
    Retrying,
}

/// Job entity representing a queued task execution job
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// Unique identifier for the job
    #[sea_orm(unique)]
    pub uuid: Uuid,
    
    /// Foreign key to tasks table
    pub task_id: i32,
    
    /// Foreign key to executions table (null until execution starts)
    pub execution_id: Option<i32>,
    
    /// Foreign key to schedules table (null for manual jobs)
    pub schedule_id: Option<i32>,
    
    /// Job priority
    pub priority: JobPriority,
    
    /// Job status
    pub status: JobStatus,
    
    /// Input data as JSON
    pub input_data: Json,
    
    /// Number of retry attempts made
    pub retry_count: i32,
    
    /// Maximum number of retry attempts
    pub max_retries: i32,
    
    /// Delay before next retry attempt in seconds
    pub retry_delay_seconds: i32,
    
    /// Error message from last attempt (if failed)
    pub error_message: Option<String>,
    
    /// Error details as JSON from last attempt (if failed)
    pub error_details: Option<Json>,
    
    /// When the job was queued
    pub queued_at: ChronoDateTimeUtc,
    
    /// When the job should be processed (for delayed jobs)
    pub process_at: Option<ChronoDateTimeUtc>,
    
    /// When the job started processing (null if not started)
    pub started_at: Option<ChronoDateTimeUtc>,
    
    /// When the job completed processing (null if not completed)
    pub completed_at: Option<ChronoDateTimeUtc>,
    
    /// Job metadata as JSON
    pub metadata: Option<Json>,
    
    /// Output destinations configuration as JSON
    pub output_destinations: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id"
    )]
    Task,
    
    #[sea_orm(
        belongs_to = "super::executions::Entity",
        from = "Column::ExecutionId",
        to = "super::executions::Column::Id"
    )]
    Execution,
    
    #[sea_orm(
        belongs_to = "super::schedules::Entity",
        from = "Column::ScheduleId",
        to = "super::schedules::Column::Id"
    )]
    Schedule,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl Related<super::executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Execution.def()
    }
}

impl Related<super::schedules::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Schedule.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Default for JobPriority {
    fn default() -> Self {
        JobPriority::Normal
    }
}

impl JobPriority {
    /// Get numeric priority value for ordering (higher number = higher priority)
    pub fn to_numeric(&self) -> i32 {
        match self {
            JobPriority::Low => 1,
            JobPriority::Normal => 2,
            JobPriority::High => 3,
            JobPriority::Urgent => 4,
        }
    }
}

impl FromStr for JobPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(JobPriority::Low),
            "normal" => Ok(JobPriority::Normal),
            "high" => Ok(JobPriority::High),
            "urgent" => Ok(JobPriority::Urgent),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(JobStatus::Queued),
            "processing" => Ok(JobStatus::Processing),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            "retrying" => Ok(JobStatus::Retrying),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

impl Model {
    /// Create a new job for immediate execution
    pub fn new(task_id: i32, input_data: serde_json::Value, priority: JobPriority) -> Self {
        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            task_id,
            execution_id: None,
            schedule_id: None,
            priority,
            status: JobStatus::Queued,
            input_data: Json::from(input_data),
            retry_count: 0,
            max_retries: 3, // Default retry limit
            retry_delay_seconds: 60, // Default 1 minute delay
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now(),
            process_at: None,
            started_at: None,
            completed_at: None,
            metadata: None,
            output_destinations: None,
        }
    }
    
    /// Create a new scheduled job
    pub fn new_scheduled(
        task_id: i32,
        schedule_id: i32,
        input_data: serde_json::Value,
        process_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let mut job = Self::new(task_id, input_data, JobPriority::Normal);
        job.schedule_id = Some(schedule_id);
        job.process_at = Some(process_at);
        job
    }
    
    /// Mark job as processing
    pub fn start_processing(&mut self, execution_id: i32) {
        self.status = JobStatus::Processing;
        self.execution_id = Some(execution_id);
        self.started_at = Some(chrono::Utc::now());
    }
    
    /// Mark job as completed
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }
    
    /// Mark job as failed and prepare for retry
    pub fn fail(&mut self, error: String, details: Option<serde_json::Value>) -> bool {
        self.error_message = Some(error);
        self.error_details = details.map(Json::from);
        self.retry_count += 1;
        
        if self.retry_count < self.max_retries {
            // Schedule retry with exponential backoff
            self.status = JobStatus::Retrying;
            let delay_seconds = self.retry_delay_seconds * (2_i32.pow(self.retry_count as u32 - 1));
            self.process_at = Some(chrono::Utc::now() + chrono::Duration::seconds(delay_seconds as i64));
            true // Will retry
        } else {
            // No more retries
            self.status = JobStatus::Failed;
            self.completed_at = Some(chrono::Utc::now());
            false // No more retries
        }
    }
    
    /// Check if job is ready to be processed
    pub fn is_ready_for_processing(&self) -> bool {
        match self.status {
            JobStatus::Queued => {
                if let Some(process_at) = self.process_at {
                    process_at <= chrono::Utc::now()
                } else {
                    true
                }
            }
            JobStatus::Retrying => {
                if let Some(process_at) = self.process_at {
                    process_at <= chrono::Utc::now()
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}