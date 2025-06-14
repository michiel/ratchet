//! Schedule management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::ScheduleFilters;
use ratchet_web::{QueryParams, ApiResponse};
use ratchet_core::validation::{InputValidator, ErrorSanitizer};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{
        schedules::{CreateScheduleRequest, UpdateScheduleRequest, ScheduleStats},
        common::StatsResponse
    },
};

/// List all schedules with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/schedules",
    tag = "schedules",
    operation_id = "listSchedules",
    params(
        ("page" = Option<u32>, Query, description = "Page number (0-based)"),
        ("limit" = Option<u32>, Query, description = "Number of items per page"),
        ("enabled" = Option<bool>, Query, description = "Filter by enabled status"),
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("sort" = Option<String>, Query, description = "Sort expression")
    ),
    responses(
        (status = 200, description = "List of schedules retrieved successfully"),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_schedules(
    State(ctx): State<TasksContext>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("Listing schedules with query: {:?}", query.0);
    
    let list_input = query.0.to_list_input();
    let pagination = list_input.pagination.unwrap_or_default();
    
    // Convert query filters to schedule filters
    let filters = ScheduleFilters {
        task_id: None, // TODO: Extract from query filters
        enabled: None,
        next_run_before: None,
    };
    
    let schedule_repo = ctx.repositories.schedule_repository();
    let list_response = schedule_repo
        .find_with_filters(filters, pagination.clone())
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific schedule by ID
#[utoipa::path(
    get,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    operation_id = "getSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    responses(
        (status = 200, description = "Schedule retrieved successfully"),
        (status = 400, description = "Invalid schedule ID"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting schedule with ID: {}", schedule_id);
    
    // Validate schedule ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&schedule_id, "schedule_id") {
        warn!("Invalid schedule ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    let schedule = schedule_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Schedule", &schedule_id))?;
    
    Ok(Json(ApiResponse::new(schedule)))
}

/// Create a new schedule
#[utoipa::path(
    post,
    path = "/schedules",
    tag = "schedules",
    operation_id = "createSchedule",
    request_body = CreateScheduleRequest,
    responses(
        (status = 201, description = "Schedule created successfully"),
        (status = 400, description = "Invalid schedule data"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_schedule(
    State(_ctx): State<TasksContext>,
    Json(request): Json<CreateScheduleRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating schedule: {:?}", request.name);
    
    // Validate the request input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // Validate cron expression format
    if let Err(validation_err) = validator.validate_string(&request.cron_expression, "cron_expression") {
        warn!("Invalid cron expression provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate schedule name
    if let Err(validation_err) = validator.validate_string(&request.name, "name") {
        warn!("Invalid schedule name provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate task exists and is enabled
    // 2. Validate cron expression is valid
    // 3. Create schedule in database
    // 4. Calculate next run time
    // 5. Return the created schedule
    
    Err(RestError::InternalError(
        "Schedule creation not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Update an existing schedule
#[utoipa::path(
    patch,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    operation_id = "updateSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    request_body = UpdateScheduleRequest,
    responses(
        (status = 200, description = "Schedule updated successfully"),
        (status = 400, description = "Invalid schedule data"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_schedule(
    State(_ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
    Json(_request): Json<UpdateScheduleRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating schedule with ID: {}", schedule_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate schedule exists
    // 2. Update schedule name, description, cron expression, or enabled status
    // 3. Recalculate next run time if cron expression changed
    // 4. Return the updated schedule
    
    Err(RestError::InternalError(
        "Schedule update not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Delete a schedule
#[utoipa::path(
    delete,
    path = "/schedules/{schedule_id}",
    tag = "schedules",
    operation_id = "deleteSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    responses(
        (status = 204, description = "Schedule deleted successfully"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting schedule with ID: {}", schedule_id);
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    schedule_repo
        .delete(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Schedule {} deleted", schedule_id)
    })))
}

/// Enable a schedule
#[utoipa::path(
    post,
    path = "/schedules/{schedule_id}/enable",
    tag = "schedules",
    operation_id = "enableSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    responses(
        (status = 200, description = "Schedule enabled successfully"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn enable_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Enabling schedule with ID: {}", schedule_id);
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    schedule_repo
        .set_enabled(api_id, true)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Schedule {} enabled", schedule_id)
    })))
}

/// Disable a schedule
#[utoipa::path(
    post,
    path = "/schedules/{schedule_id}/disable",
    tag = "schedules",
    operation_id = "disableSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    responses(
        (status = 200, description = "Schedule disabled successfully"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn disable_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Disabling schedule with ID: {}", schedule_id);
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    schedule_repo
        .set_enabled(api_id, false)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Schedule {} disabled", schedule_id)
    })))
}

/// Trigger a schedule manually
#[utoipa::path(
    post,
    path = "/schedules/{schedule_id}/trigger",
    tag = "schedules",
    operation_id = "triggerSchedule",
    params(
        ("schedule_id" = String, Path, description = "Unique schedule identifier")
    ),
    responses(
        (status = 201, description = "Schedule triggered successfully"),
        (status = 400, description = "Schedule cannot be triggered"),
        (status = 404, description = "Schedule not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trigger_schedule(
    State(_ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Triggering schedule with ID: {}", schedule_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate schedule exists and is enabled
    // 2. Create immediate job/execution for the scheduled task
    // 3. Record the execution for the schedule
    // 4. Return the created job/execution
    
    Err(RestError::InternalError(
        "Schedule trigger not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Get schedule statistics
#[utoipa::path(
    get,
    path = "/schedules/stats",
    tag = "schedules",
    operation_id = "getScheduleStats",
    responses(
        (status = 200, description = "Schedule statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_schedule_stats(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Getting schedule statistics");
    
    let schedule_repo = ctx.repositories.schedule_repository();
    
    // Get basic counts
    let total_schedules = schedule_repo.count().await.map_err(RestError::Database)?;
    let enabled_schedules = schedule_repo.find_enabled().await
        .map_err(RestError::Database)?
        .len() as u64;
    let schedules_ready = schedule_repo.find_ready_to_run().await
        .map_err(RestError::Database)?
        .len() as u64;
    
    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = ScheduleStats {
        total_schedules,
        enabled_schedules,
        disabled_schedules: total_schedules - enabled_schedules,
        schedules_ready_to_run: schedules_ready,
        average_execution_interval_minutes: None, // TODO: Implement
        last_execution: None,    // TODO: Implement
        next_execution: None,    // TODO: Implement
    };
    
    Ok(Json(StatsResponse::new(stats)))
}