//! GraphQL types for schedules

use super::scalars::GraphQLApiId;
use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use ratchet_api_types::UnifiedSchedule;

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
    // ID filtering
    pub task_id: Option<GraphQLApiId>,
    pub task_id_in: Option<Vec<GraphQLApiId>>,
    pub id_in: Option<Vec<GraphQLApiId>>,

    // Name filtering
    pub name_contains: Option<String>,
    pub name_exact: Option<String>,
    pub name_starts_with: Option<String>,
    pub name_ends_with: Option<String>,

    // Status filtering
    pub enabled: Option<bool>,

    // Cron expression filtering
    pub cron_expression_contains: Option<String>,
    pub cron_expression_exact: Option<String>,

    // Schedule timing filtering
    pub next_run_after: Option<DateTime<Utc>>,
    pub next_run_before: Option<DateTime<Utc>>,
    pub last_run_after: Option<DateTime<Utc>>,
    pub last_run_before: Option<DateTime<Utc>>,

    // Date range filtering
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,

    // Advanced filtering
    pub has_next_run: Option<bool>,
    pub has_last_run: Option<bool>,
    pub is_due: Option<bool>,  // next_run <= now
    pub overdue: Option<bool>, // next_run < now and enabled
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
