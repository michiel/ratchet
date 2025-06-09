//! GraphQL types for schedules

use async_graphql::{SimpleObject, InputObject};
use ratchet_api_types::UnifiedSchedule;
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

/// GraphQL Schedule type
#[derive(SimpleObject, Clone)]
pub struct Schedule {
    pub id: GraphQLApiId,
    pub task_id: GraphQLApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub timezone: Option<String>,
    pub enabled: bool,
    pub input: Option<JsonValue>,
    pub max_retries: i32,
    pub timeout_seconds: Option<i32>,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UnifiedSchedule> for Schedule {
    fn from(schedule: UnifiedSchedule) -> Self {
        Self {
            id: schedule.id.into(),
            task_id: schedule.task_id.into(),
            name: schedule.name,
            description: schedule.description,
            cron_expression: schedule.cron_expression,
            timezone: schedule.timezone,
            enabled: schedule.enabled,
            input: schedule.input,
            max_retries: schedule.max_retries,
            timeout_seconds: schedule.timeout_seconds,
            next_run: schedule.next_run,
            last_run: schedule.last_run,
            run_count: schedule.run_count,
            success_count: schedule.success_count,
            failure_count: schedule.failure_count,
            metadata: schedule.metadata,
            created_at: schedule.created_at,
            updated_at: schedule.updated_at,
        }
    }
}

/// Input type for creating schedules
#[derive(InputObject)]
pub struct CreateScheduleInput {
    pub task_id: GraphQLApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
    pub input: Option<JsonValue>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub metadata: Option<JsonValue>,
}

/// Input type for updating schedules
#[derive(InputObject)]
pub struct UpdateScheduleInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
    pub input: Option<JsonValue>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub metadata: Option<JsonValue>,
}

/// Input type for schedule filtering
#[derive(InputObject)]
pub struct ScheduleFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub enabled: Option<bool>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// Schedule statistics
#[derive(SimpleObject)]
pub struct ScheduleStats {
    pub total_schedules: i32,
    pub enabled_schedules: i32,
    pub disabled_schedules: i32,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub average_run_time_ms: Option<f64>,
}