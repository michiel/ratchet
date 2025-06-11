//! GraphQL types for schedules

use async_graphql::{SimpleObject, InputObject};
use ratchet_api_types::UnifiedSchedule;
use super::scalars::GraphQLApiId;
use chrono::{DateTime, Utc};

/// GraphQL Schedule type - using UnifiedSchedule directly for API consistency
pub type Schedule = UnifiedSchedule;

/// Input type for creating schedules
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CreateScheduleInput {
    pub task_id: GraphQLApiId,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub enabled: Option<bool>,
}

/// Input type for updating schedules
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct UpdateScheduleInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}

/// Input type for schedule filtering
#[derive(InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ScheduleFiltersInput {
    pub task_id: Option<GraphQLApiId>,
    pub enabled: Option<bool>,
    pub next_run_before: Option<DateTime<Utc>>,
}

/// Schedule statistics
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ScheduleStats {
    pub total_schedules: i32,
    pub enabled_schedules: i32,
    pub disabled_schedules: i32,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub average_run_time_ms: Option<f64>,
}