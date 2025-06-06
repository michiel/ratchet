//! Schedule entity definition

use super::Entity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Schedule entity for cron-based task scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: i32,
    pub uuid: Uuid,
    pub task_id: i32,
    pub name: String,
    pub cron_expression: String,
    pub input_data: serde_json::Value,
    pub enabled: bool,
    pub status: ScheduleStatus,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub execution_count: i32,
    pub max_executions: Option<i32>,
    pub metadata: serde_json::Value,
    pub output_destinations: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ScheduleStatus {
    #[default]
    Active,
    Inactive,
    Completed,
    Failed,
}

impl Entity for Schedule {
    fn id(&self) -> i32 {
        self.id
    }
    fn uuid(&self) -> Uuid {
        self.uuid
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl Schedule {
    pub fn new(
        task_id: i32,
        name: String,
        cron_expression: String,
        input_data: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            uuid: Uuid::new_v4(),
            task_id,
            name,
            cron_expression,
            input_data,
            enabled: true,
            status: ScheduleStatus::Active,
            next_run_at: None,
            last_run_at: None,
            execution_count: 0,
            max_executions: None,
            metadata: serde_json::json!({}),
            output_destinations: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_ready_to_run(&self) -> bool {
        self.enabled
            && matches!(self.status, ScheduleStatus::Active)
            && self.next_run_at.is_some_and(|next| Utc::now() >= next)
            && self
                .max_executions
                .map_or(true, |max| self.execution_count < max)
    }
}
