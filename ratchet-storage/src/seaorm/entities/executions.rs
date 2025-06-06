use sea_orm::entity::prelude::*;
use sea_orm::sea_query::StringLen;
use serde::{Deserialize, Serialize};

/// Execution status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ExecutionStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// Task execution entity representing a single execution of a task
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "executions")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Unique identifier for the execution
    #[sea_orm(unique)]
    pub uuid: Uuid,

    /// Foreign key to tasks table
    pub task_id: i32,

    /// Input data as JSON
    pub input: Json,

    /// Output data as JSON (null if not completed)
    pub output: Option<Json>,

    /// Execution status
    pub status: ExecutionStatus,

    /// Error message if execution failed
    pub error_message: Option<String>,

    /// Error details as JSON if execution failed
    pub error_details: Option<Json>,

    /// When the execution was queued
    pub queued_at: ChronoDateTimeUtc,

    /// When the execution started (null if not started)
    pub started_at: Option<ChronoDateTimeUtc>,

    /// When the execution completed (null if not completed)
    pub completed_at: Option<ChronoDateTimeUtc>,

    /// Execution duration in milliseconds (null if not completed)
    pub duration_ms: Option<i32>,

    /// HTTP requests made during execution as JSON
    pub http_requests: Option<Json>,

    /// Recording directory path if recording was enabled
    pub recording_path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tasks::Entity",
        from = "Column::TaskId",
        to = "super::tasks::Column::Id"
    )]
    Task,
}

impl Related<super::tasks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Create a new execution record
    pub fn new(task_id: i32, input: serde_json::Value) -> Self {
        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            task_id,
            input,
            output: None,
            status: ExecutionStatus::Pending,
            error_message: None,
            error_details: None,
            queued_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            http_requests: None,
            recording_path: None,
        }
    }

    /// Mark execution as started
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark execution as completed with output
    pub fn complete(&mut self, output: serde_json::Value) {
        self.status = ExecutionStatus::Completed;
        self.output = Some(output);
        self.completed_at = Some(chrono::Utc::now());

        // Calculate duration if we have start time
        if let Some(started) = self.started_at {
            let duration = chrono::Utc::now().signed_duration_since(started);
            self.duration_ms = Some(duration.num_milliseconds() as i32);
        }
    }

    /// Mark execution as failed with error
    pub fn fail(&mut self, error: String, details: Option<serde_json::Value>) {
        self.status = ExecutionStatus::Failed;
        self.error_message = Some(error);
        self.error_details = details;
        self.completed_at = Some(chrono::Utc::now());

        // Calculate duration if we have start time
        if let Some(started) = self.started_at {
            let duration = chrono::Utc::now().signed_duration_since(started);
            self.duration_ms = Some(duration.num_milliseconds() as i32);
        }
    }
}
