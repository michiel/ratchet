use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use std::str::FromStr;
use ratchet_storage::{repositories::RepositoryFactory, Schedule, ScheduleStatus};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    rest::{
        extractors::ListQueryExtractor,
        middleware::error_handler::RestError,
        middleware::pagination::WithPaginationHeaders,
        models::{
            common::ApiResponse,
            schedules::{
                ScheduleCreateRequest, ScheduleResponse,
                ScheduleUpdateRequest,
            },
        },
    },
};

#[derive(Clone)]
pub struct SchedulesContext {
    pub repository: RepositoryFactory,
}

pub async fn list_schedules(
    State(_ctx): State<SchedulesContext>,
    ListQueryExtractor(query): ListQueryExtractor,
) -> Result<Response, RestError> {
    // Simplified stub implementation - return empty list
    let schedules: Vec<Schedule> = Vec::new();
    let total = 0u64;
    
    let schedule_responses: Vec<ScheduleResponse> = schedules.into_iter().map(Into::into).collect();
    let pagination = query.pagination();

    Ok(Json(ApiResponse {
        data: schedule_responses,
    })
    .with_pagination_headers(
        total,
        pagination.offset(),
        pagination.limit(),
        "schedules",
    ))
}

pub async fn get_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
) -> Result<Json<ScheduleResponse>, RestError> {
    // Simplified stub implementation - return not found
    Err(RestError::NotFound("Schedule not found".to_string()))
}

pub async fn create_schedule(
    State(_ctx): State<SchedulesContext>,
    Json(payload): Json<ScheduleCreateRequest>,
) -> Result<(StatusCode, Json<ScheduleResponse>), RestError> {
    // Validate cron expression
    if cron::Schedule::from_str(&payload.cron_expression).is_err() {
        return Err(RestError::BadRequest("Invalid cron expression".to_string()));
    }
    
    // Simplified stub implementation - create a mock schedule
    let now = Utc::now();
    
    let created_schedule = Schedule {
        id: 1, // Mock ID
        uuid: Uuid::new_v4(),
        task_id: payload.task_id,
        name: payload.name,
        cron_expression: payload.cron_expression,
        input_data: payload.input_data,
        enabled: payload.enabled.unwrap_or(true),
        status: ScheduleStatus::Active,
        next_run_at: Some(now + chrono::Duration::hours(1)), // Mock next run
        last_run_at: None,
        execution_count: 0,
        max_executions: payload.max_executions,
        metadata: payload.metadata.unwrap_or_else(|| serde_json::json!({})),
        output_destinations: None,
        created_at: now,
        updated_at: now,
    };

    Ok((
        StatusCode::CREATED,
        Json(ScheduleResponse::from(created_schedule)),
    ))
}

pub async fn update_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
    Json(_payload): Json<ScheduleUpdateRequest>,
) -> Result<Json<ScheduleResponse>, RestError> {
    // Simplified stub implementation - return not found
    Err(RestError::NotFound("Schedule not found".to_string()))
}

pub async fn delete_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
) -> Result<StatusCode, RestError> {
    // Simplified stub implementation - return success
    Ok(StatusCode::NO_CONTENT)
}

pub async fn enable_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
) -> Result<Json<ScheduleResponse>, RestError> {
    // Simplified stub implementation - return not found
    Err(RestError::NotFound("Schedule not found".to_string()))
}

pub async fn disable_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
) -> Result<Json<ScheduleResponse>, RestError> {
    // Simplified stub implementation - return not found
    Err(RestError::NotFound("Schedule not found".to_string()))
}

pub async fn trigger_schedule(
    State(_ctx): State<SchedulesContext>,
    Path(_id): Path<i32>,
) -> Result<Json<ScheduleResponse>, RestError> {
    // Simplified stub implementation - return not found
    Err(RestError::NotFound("Schedule not found".to_string()))
}
