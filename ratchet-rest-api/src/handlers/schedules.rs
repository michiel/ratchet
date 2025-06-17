//! Schedule management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::ScheduleFilters;
use ratchet_web::{QueryParams, ApiResponse, extract_schedule_filters};
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
        // Refine.dev pagination
        ("_start" = Option<u64>, Query, description = "Starting index (0-based) - Refine.dev style"),
        ("_end" = Option<u64>, Query, description = "Ending index (exclusive) - Refine.dev style"),
        // Standard pagination (alternative)
        ("page" = Option<u32>, Query, description = "Page number (1-based) - standard style"),
        ("limit" = Option<u32>, Query, description = "Number of items per page (max 100)"),
        // Refine.dev sorting
        ("_sort" = Option<String>, Query, description = "Field to sort by - Refine.dev style"),
        ("_order" = Option<String>, Query, description = "Sort order: ASC or DESC - Refine.dev style"),
        // Schedule-specific filters
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("enabled" = Option<bool>, Query, description = "Filter by enabled status"),
        ("name" = Option<String>, Query, description = "Filter by schedule name (exact match)"),
        ("name_like" = Option<String>, Query, description = "Filter by schedule name (contains text)"),
        ("cron_expression" = Option<String>, Query, description = "Filter by cron expression (exact match)"),
        ("cron_expression_like" = Option<String>, Query, description = "Filter by cron expression (contains text)"),
        ("is_due" = Option<bool>, Query, description = "Filter by schedules that are due now"),
        ("overdue" = Option<bool>, Query, description = "Filter by overdue schedules"),
        ("has_next_run" = Option<bool>, Query, description = "Filter by schedules with next run date"),
        ("has_last_run" = Option<bool>, Query, description = "Filter by schedules with last run date"),
        // Date filters (ISO 8601 format)
        ("next_run_after" = Option<String>, Query, description = "Filter by next run date (after this date)"),
        ("next_run_before" = Option<String>, Query, description = "Filter by next run date (before this date)"),
        ("last_run_after" = Option<String>, Query, description = "Filter by last run date (after this date)"),
        ("last_run_before" = Option<String>, Query, description = "Filter by last run date (before this date)"),
        ("created_after" = Option<String>, Query, description = "Filter by creation date (after this date)"),
        ("created_before" = Option<String>, Query, description = "Filter by creation date (before this date)"),
        ("updated_after" = Option<String>, Query, description = "Filter by update date (after this date)"),
        ("updated_before" = Option<String>, Query, description = "Filter by update date (before this date)")
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
    
    // Extract filters from query parameters
    let filters = extract_schedule_filters(&query.0.filters);
    
    let schedule_repo = ctx.repositories.schedule_repository();
    let list_response = schedule_repo
        .find_with_list_input(filters, list_input)
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
    State(ctx): State<TasksContext>,
    Json(request): Json<CreateScheduleRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating schedule: {:?}", request.name);
    
    // Validate the request input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // Validate schedule name
    if let Err(validation_err) = validator.validate_string(&request.name, "name") {
        warn!("Invalid schedule name provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate cron expression format
    if let Err(validation_err) = validator.validate_string(&request.cron_expression, "cron_expression") {
        warn!("Invalid cron expression provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Basic cron expression validation
    if request.cron_expression.trim().is_empty() {
        return Err(RestError::BadRequest("Cron expression cannot be empty".to_string()));
    }
    
    // Validate description if provided
    if let Some(ref description) = request.description {
        if let Err(validation_err) = validator.validate_string(description, "description") {
            warn!("Invalid description provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Validate that task exists
    let task_repo = ctx.repositories.task_repository();
    let _task = task_repo
        .find_by_id(request.task_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Task", &request.task_id.to_string()))?;
    
    // Create UnifiedSchedule from request
    let unified_schedule = ratchet_api_types::UnifiedSchedule {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        task_id: request.task_id,
        name: request.name,
        description: request.description,
        cron_expression: request.cron_expression,
        enabled: request.enabled.unwrap_or(true),
        next_run: None, // Will be calculated by the scheduler
        last_run: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Create the schedule using the repository
    let schedule_repo = ctx.repositories.schedule_repository();
    let created_schedule = schedule_repo.create(unified_schedule).await
        .map_err(|e| RestError::InternalError(format!("Failed to create schedule: {}", e)))?;
    
    Ok(Json(ApiResponse::new(created_schedule)))
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
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
    Json(request): Json<UpdateScheduleRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating schedule with ID: {}", schedule_id);
    
    // Validate schedule ID input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    if let Err(validation_err) = validator.validate_string(&schedule_id, "schedule_id") {
        warn!("Invalid schedule ID provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate update request fields if provided
    if let Some(ref name) = request.name {
        if let Err(validation_err) = validator.validate_string(name, "name") {
            warn!("Invalid schedule name provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref description) = request.description {
        if let Err(validation_err) = validator.validate_string(description, "description") {
            warn!("Invalid description provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref cron_expression) = request.cron_expression {
        if let Err(validation_err) = validator.validate_string(cron_expression, "cron_expression") {
            warn!("Invalid cron expression provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
        
        // Basic cron expression validation
        if cron_expression.trim().is_empty() {
            return Err(RestError::BadRequest("Cron expression cannot be empty".to_string()));
        }
    }
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    // Get the existing schedule
    let mut existing_schedule = schedule_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Schedule", &schedule_id))?;
    
    // Apply updates
    if let Some(name) = request.name {
        existing_schedule.name = name;
    }
    if let Some(description) = request.description {
        existing_schedule.description = Some(description);
    }
    if let Some(cron_expression) = request.cron_expression {
        existing_schedule.cron_expression = cron_expression;
        // Reset next_run when cron expression changes (will be recalculated by scheduler)
        existing_schedule.next_run = None;
    }
    if let Some(enabled) = request.enabled {
        existing_schedule.enabled = enabled;
    }
    
    // Update timestamp
    existing_schedule.updated_at = chrono::Utc::now();
    
    // Update the schedule using the repository
    let updated_schedule = schedule_repo.update(existing_schedule).await
        .map_err(|e| RestError::InternalError(format!("Failed to update schedule: {}", e)))?;
    
    Ok(Json(ApiResponse::new(updated_schedule)))
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
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Triggering schedule with ID: {}", schedule_id);
    
    // Validate schedule ID
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&schedule_id, "schedule_id") {
        warn!("Invalid schedule ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();
    
    // Find the schedule
    let schedule = schedule_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Schedule", &schedule_id))?;
    
    // Check if schedule is enabled
    if !schedule.enabled {
        return Err(RestError::BadRequest(
            "Cannot trigger disabled schedule".to_string()
        ));
    }
    
    // Validate that the associated task exists
    let task_repo = ctx.repositories.task_repository();
    let _task = task_repo
        .find_by_id(schedule.task_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::BadRequest("Associated task not found".to_string()))?;
    
    // Create a job for immediate execution
    let job_repo = ctx.repositories.job_repository();
    let task_id_clone = schedule.task_id.clone();
    let new_job = ratchet_api_types::UnifiedJob {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        task_id: task_id_clone,
        priority: ratchet_api_types::JobPriority::Normal, // Manual triggers get normal priority
        status: ratchet_api_types::JobStatus::Queued,
        retry_count: 0,
        max_retries: 3, // Default retry count
        queued_at: chrono::Utc::now(),
        scheduled_for: None, // Immediate execution
        error_message: None,
        output_destinations: None,
    };
    
    // Create the job
    let created_job = job_repo.create(new_job).await
        .map_err(|e| RestError::InternalError(format!("Failed to create job for schedule trigger: {}", e)))?;
    
    // Update the schedule's last_run timestamp
    let mut updated_schedule = schedule;
    updated_schedule.last_run = Some(chrono::Utc::now());
    schedule_repo.update(updated_schedule).await
        .map_err(|e| RestError::InternalError(format!("Failed to update schedule last_run: {}", e)))?;
    
    info!("Created job {} for triggered schedule {}", created_job.id, schedule_id);
    
    Ok(Json(ApiResponse::new(serde_json::json!({
        "success": true,
        "message": "Schedule triggered successfully",
        "job": created_job
    }))))
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