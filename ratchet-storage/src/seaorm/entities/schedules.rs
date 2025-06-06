use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Schedule entity representing a cron-like schedule for task execution
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "schedules")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Unique identifier for the schedule
    #[sea_orm(unique)]
    pub uuid: Uuid,

    /// Foreign key to tasks table
    pub task_id: i32,

    /// Schedule name
    pub name: String,

    /// Cron expression for scheduling
    pub cron_expression: String,

    /// Input data as JSON for scheduled executions
    pub input_data: Json,

    /// Whether the schedule is enabled
    pub enabled: bool,

    /// Next scheduled run time
    pub next_run_at: Option<ChronoDateTimeUtc>,

    /// Last time the schedule was executed
    pub last_run_at: Option<ChronoDateTimeUtc>,

    /// Number of times this schedule has been executed
    pub execution_count: i32,

    /// Maximum number of executions (null for unlimited)
    pub max_executions: Option<i32>,

    /// Schedule metadata as JSON
    pub metadata: Option<Json>,

    /// Output destinations configuration as JSON
    pub output_destinations: Option<Json>,

    /// When the schedule was created
    pub created_at: ChronoDateTimeUtc,

    /// When the schedule was last updated
    pub updated_at: ChronoDateTimeUtc,
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
    /// Create a new schedule
    pub fn new(
        task_id: i32,
        name: String,
        cron_expression: String,
        input_data: serde_json::Value,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            uuid: Uuid::new_v4(),
            task_id,
            name,
            cron_expression,
            input_data,
            enabled: true,
            next_run_at: None, // Will be calculated by scheduler
            last_run_at: None,
            execution_count: 0,
            max_executions: None,
            metadata: None,
            output_destinations: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Update last run time and increment execution count
    pub fn record_execution(&mut self) {
        self.last_run_at = Some(chrono::Utc::now());
        self.execution_count += 1;
        self.updated_at = chrono::Utc::now();
    }

    /// Check if schedule has reached maximum executions
    pub fn is_exhausted(&self) -> bool {
        if let Some(max) = self.max_executions {
            self.execution_count >= max
        } else {
            false
        }
    }

    /// Parse cron expression and get next run time
    pub fn calculate_next_run(&self) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
        use cron::Schedule;
        use std::str::FromStr;

        if !self.enabled || self.is_exhausted() {
            return Ok(None);
        }

        let schedule = Schedule::from_str(&self.cron_expression)
            .map_err(|e| format!("Invalid cron expression: {}", e))?;

        let next = schedule.upcoming(chrono::Utc).next();
        Ok(next)
    }
}
