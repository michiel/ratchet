//! Schedule management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_core::validation::{ErrorSanitizer, InputValidator};
use ratchet_interfaces::ScheduleFilters;
use ratchet_web::{extract_schedule_filters, ApiResponse, QueryParams};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{
        common::StatsResponse,
        schedules::{CreateScheduleRequest, ScheduleStats, UpdateScheduleRequest},
    },
};
use ratchet_api_types::UnifiedOutputDestination;

/// Validate output destinations configuration
fn validate_output_destinations(destinations: &[UnifiedOutputDestination]) -> Result<(), RestError> {
    if destinations.is_empty() {
        return Err(RestError::BadRequest(
            "Output destinations array cannot be empty".to_string(),
        ));
    }

    if destinations.len() > 10 {
        return Err(RestError::BadRequest(
            "Maximum of 10 output destinations allowed per schedule".to_string(),
        ));
    }

    for (index, dest) in destinations.iter().enumerate() {
        let context = format!("destination[{}]", index);

        match dest.destination_type.as_str() {
            "webhook" => {
                if let Some(webhook) = &dest.webhook {
                    // Validate URL format
                    if webhook.url.is_empty() {
                        return Err(RestError::BadRequest(format!(
                            "{}: Webhook URL cannot be empty",
                            context
                        )));
                    }

                    // Enhanced URL validation
                    if !webhook.url.starts_with("http://") && !webhook.url.starts_with("https://") {
                        return Err(RestError::BadRequest(format!(
                            "{}: Webhook URL must be a valid HTTP/HTTPS URL",
                            context
                        )));
                    }

                    // Validate URL length
                    if webhook.url.len() > 2048 {
                        return Err(RestError::BadRequest(format!(
                            "{}: Webhook URL too long (max 2048 characters)",
                            context
                        )));
                    }

                    // Prevent localhost and private IPs in production
                    if webhook.url.contains("localhost")
                        || webhook.url.contains("127.0.0.1")
                        || webhook.url.contains("::1")
                    {
                        return Err(RestError::BadRequest(format!(
                            "{}: Localhost URLs not allowed for webhooks",
                            context
                        )));
                    }

                    // Validate timeout
                    if webhook.timeout_seconds <= 0 {
                        return Err(RestError::BadRequest(format!(
                            "{}: Webhook timeout must be greater than 0",
                            context
                        )));
                    }

                    if webhook.timeout_seconds > 300 {
                        return Err(RestError::BadRequest(format!(
                            "{}: Webhook timeout too long (max 300 seconds)",
                            context
                        )));
                    }

                    // Validate HTTP method
                    match webhook.method {
                        ratchet_api_types::HttpMethod::Get
                        | ratchet_api_types::HttpMethod::Post
                        | ratchet_api_types::HttpMethod::Put
                        | ratchet_api_types::HttpMethod::Patch => {
                            // Valid methods
                        }
                        _ => {
                            return Err(RestError::BadRequest(format!(
                                "{}: Unsupported HTTP method for webhook",
                                context
                            )));
                        }
                    }

                    // Validate content type if present
                    if let Some(ref content_type) = webhook.content_type {
                        if content_type.is_empty() || content_type.len() > 100 {
                            return Err(RestError::BadRequest(format!("{}: Invalid content type", context)));
                        }
                    }

                    // Validate retry policy if present
                    if let Some(ref retry_policy) = webhook.retry_policy {
                        if retry_policy.max_attempts == 0 || retry_policy.max_attempts > 10 {
                            return Err(RestError::BadRequest(format!(
                                "{}: Retry max_attempts must be between 1 and 10",
                                context
                            )));
                        }

                        if retry_policy.initial_delay_seconds > retry_policy.max_delay_seconds {
                            return Err(RestError::BadRequest(format!(
                                "{}: Initial delay cannot be greater than max delay",
                                context
                            )));
                        }

                        if retry_policy.backoff_multiplier < 1.0 || retry_policy.backoff_multiplier > 10.0 {
                            return Err(RestError::BadRequest(format!(
                                "{}: Backoff multiplier must be between 1.0 and 10.0",
                                context
                            )));
                        }
                    }

                    // Validate authentication if present
                    if let Some(ref auth) = webhook.authentication {
                        match auth.auth_type.as_str() {
                            "bearer" => {
                                if let Some(ref bearer) = auth.bearer {
                                    if bearer.token.is_empty() || bearer.token.len() > 1024 {
                                        return Err(RestError::BadRequest(format!(
                                            "{}: Bearer token invalid length",
                                            context
                                        )));
                                    }
                                } else {
                                    return Err(RestError::BadRequest(format!(
                                        "{}: Bearer authentication requires bearer configuration",
                                        context
                                    )));
                                }
                            }
                            "basic" => {
                                if let Some(ref basic) = auth.basic {
                                    if basic.username.is_empty() || basic.password.is_empty() {
                                        return Err(RestError::BadRequest(format!(
                                            "{}: Basic authentication credentials cannot be empty",
                                            context
                                        )));
                                    }
                                    if basic.username.len() > 255 || basic.password.len() > 255 {
                                        return Err(RestError::BadRequest(format!(
                                            "{}: Basic authentication credentials too long",
                                            context
                                        )));
                                    }
                                } else {
                                    return Err(RestError::BadRequest(format!(
                                        "{}: Basic authentication requires basic configuration",
                                        context
                                    )));
                                }
                            }
                            "api_key" => {
                                if let Some(ref api_key) = auth.api_key {
                                    if api_key.key.is_empty() || api_key.key.len() > 1024 {
                                        return Err(RestError::BadRequest(format!(
                                            "{}: API key invalid length",
                                            context
                                        )));
                                    }
                                    if api_key.header_name.is_empty() || api_key.header_name.len() > 100 {
                                        return Err(RestError::BadRequest(format!(
                                            "{}: API key header name invalid",
                                            context
                                        )));
                                    }
                                } else {
                                    return Err(RestError::BadRequest(format!(
                                        "{}: API key authentication requires api_key configuration",
                                        context
                                    )));
                                }
                            }
                            _ => {
                                return Err(RestError::BadRequest(format!(
                                    "{}: Unsupported authentication type",
                                    context
                                )));
                            }
                        }
                    }
                } else {
                    return Err(RestError::BadRequest(format!(
                        "{}: Webhook destination must include webhook configuration",
                        context
                    )));
                }
            }
            "filesystem" => {
                if let Some(fs) = &dest.filesystem {
                    if fs.path.is_empty() {
                        return Err(RestError::BadRequest(format!(
                            "{}: Filesystem path cannot be empty",
                            context
                        )));
                    }

                    // Validate path length
                    if fs.path.len() > 4096 {
                        return Err(RestError::BadRequest(format!(
                            "{}: Filesystem path too long (max 4096 characters)",
                            context
                        )));
                    }

                    // Basic path security validation
                    if fs.path.contains("..") {
                        return Err(RestError::BadRequest(format!(
                            "{}: Path traversal not allowed in filesystem paths",
                            context
                        )));
                    }

                    // Validate format (always present)
                    match fs.format {
                        ratchet_api_types::OutputFormat::Json
                        | ratchet_api_types::OutputFormat::Yaml
                        | ratchet_api_types::OutputFormat::Csv
                        | ratchet_api_types::OutputFormat::Xml => {
                            // Valid formats
                        }
                    }
                } else {
                    return Err(RestError::BadRequest(format!(
                        "{}: Filesystem destination must include filesystem configuration",
                        context
                    )));
                }
            }
            "database" => {
                // Basic validation for database destinations
                return Err(RestError::BadRequest(format!(
                    "{}: Database destinations not yet supported",
                    context
                )));
            }
            _ => {
                return Err(RestError::BadRequest(format!(
                    "{}: Unsupported destination type: {}",
                    context, dest.destination_type
                )));
            }
        }
    }
    Ok(())
}

/// List all schedules with optional filtering and pagination

pub async fn list_schedules(State(ctx): State<TasksContext>, query: QueryParams) -> RestResult<impl IntoResponse> {
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

    // Validate output destinations if provided
    if let Some(ref destinations) = request.output_destinations {
        if let Err(validation_err) = validate_output_destinations(destinations) {
            warn!("Invalid output destinations provided: {}", validation_err);
            return Err(validation_err);
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
        output_destinations: request.output_destinations,
    };

    // Create the schedule using the repository
    let schedule_repo = ctx.repositories.schedule_repository();
    let created_schedule = schedule_repo
        .create(unified_schedule)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to create schedule: {}", e)))?;

    // Add schedule to running scheduler if available and enabled
    if let Some(scheduler) = &ctx.scheduler_service {
        if created_schedule.enabled {
            if let Err(scheduler_err) = scheduler.add_schedule(created_schedule.clone()).await {
                warn!("Failed to add schedule to running scheduler: {}", scheduler_err);
                // Don't fail the request - schedule is created in database
                // Scheduler will pick it up on next restart
            } else {
                info!(
                    "Successfully added schedule {} to running scheduler",
                    created_schedule.name
                );
            }
        }
    }

    Ok((StatusCode::CREATED, Json(ApiResponse::new(created_schedule))))
}

/// Update an existing schedule

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
    if let Some(destinations) = request.output_destinations {
        // Validate the new output destinations
        if let Err(validation_err) = validate_output_destinations(&destinations) {
            warn!("Invalid output destinations provided in update: {}", validation_err);
            return Err(validation_err);
        }
        existing_schedule.output_destinations = Some(destinations);
    }

    // Update timestamp
    existing_schedule.updated_at = chrono::Utc::now();

    // Update the schedule using the repository
    let updated_schedule = schedule_repo
        .update(existing_schedule)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to update schedule: {}", e)))?;

    // Update schedule in running scheduler if available
    if let Some(scheduler) = &ctx.scheduler_service {
        if let Err(scheduler_err) = scheduler.update_schedule(updated_schedule.clone()).await {
            warn!("Failed to update schedule in running scheduler: {}", scheduler_err);
            // Don't fail the request - schedule is updated in database
            // Scheduler will pick up changes on next restart
        } else {
            info!(
                "Successfully updated schedule {} in running scheduler",
                updated_schedule.name
            );
        }
    }

    Ok(Json(ApiResponse::new(updated_schedule)))
}

/// Delete a schedule

pub async fn delete_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting schedule with ID: {}", schedule_id);

    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();

    // Remove from running scheduler first if available
    if let Some(scheduler) = &ctx.scheduler_service {
        if let Err(scheduler_err) = scheduler.remove_schedule(api_id.clone()).await {
            warn!("Failed to remove schedule from running scheduler: {}", scheduler_err);
            // Continue with database deletion even if scheduler removal fails
        } else {
            info!("Successfully removed schedule {} from running scheduler", schedule_id);
        }
    }

    // Delete from database
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

pub async fn enable_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Enabling schedule with ID: {}", schedule_id);

    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();

    // Update the schedule in the database
    schedule_repo
        .set_enabled(api_id.clone(), true)
        .await
        .map_err(RestError::Database)?;

    // Add to running scheduler if available
    if let Some(scheduler) = &ctx.scheduler_service {
        // Get the updated schedule to add to scheduler
        if let Ok(Some(updated_schedule)) = schedule_repo.find_by_id(api_id.as_i32().unwrap_or(0)).await {
            if let Err(scheduler_err) = scheduler.add_schedule(updated_schedule).await {
                warn!("Failed to add enabled schedule to running scheduler: {}", scheduler_err);
                // Don't fail the request - schedule is enabled in database
            } else {
                info!(
                    "Successfully added enabled schedule {} to running scheduler",
                    schedule_id
                );
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Schedule {} enabled", schedule_id)
    })))
}

/// Disable a schedule

pub async fn disable_schedule(
    State(ctx): State<TasksContext>,
    Path(schedule_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Disabling schedule with ID: {}", schedule_id);

    let api_id = ApiId::from_string(schedule_id.clone());
    let schedule_repo = ctx.repositories.schedule_repository();

    // Remove from running scheduler first if available
    if let Some(scheduler) = &ctx.scheduler_service {
        if let Err(scheduler_err) = scheduler.remove_schedule(api_id.clone()).await {
            warn!("Failed to remove schedule from running scheduler: {}", scheduler_err);
            // Continue with database update even if scheduler removal fails
        } else {
            info!("Successfully removed schedule {} from running scheduler", schedule_id);
        }
    }

    // Update the schedule in the database
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
        return Err(RestError::BadRequest("Cannot trigger disabled schedule".to_string()));
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
    let output_destinations_clone = schedule.output_destinations.clone();
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
        output_destinations: output_destinations_clone,
    };

    // Create the job
    let created_job = job_repo
        .create(new_job)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to create job for schedule trigger: {}", e)))?;

    // Update the schedule's last_run timestamp
    let mut updated_schedule = schedule;
    updated_schedule.last_run = Some(chrono::Utc::now());
    schedule_repo
        .update(updated_schedule)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to update schedule last_run: {}", e)))?;

    info!("Created job {} for triggered schedule {}", created_job.id, schedule_id);

    Ok(Json(ApiResponse::new(serde_json::json!({
        "success": true,
        "message": "Schedule triggered successfully",
        "job": created_job
    }))))
}

/// Get schedule statistics

pub async fn get_schedule_stats(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Getting schedule statistics");

    let schedule_repo = ctx.repositories.schedule_repository();

    // Get basic counts
    let total_schedules = schedule_repo.count().await.map_err(RestError::Database)?;
    let enabled_schedules = schedule_repo.find_enabled().await.map_err(RestError::Database)?.len() as u64;
    let schedules_ready = schedule_repo
        .find_ready_to_run()
        .await
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
        last_execution: None,                     // TODO: Implement
        next_execution: None,                     // TODO: Implement
    };

    Ok(Json(StatsResponse::new(stats)))
}
