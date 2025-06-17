//! Job management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::JobFilters;
use ratchet_web::{QueryParams, ApiResponse, extract_job_filters};
use ratchet_core::validation::{InputValidator, ErrorSanitizer};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{
        jobs::{CreateJobRequest, UpdateJobRequest, JobStats},
        common::StatsResponse
    },
};

/// List all jobs with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/jobs",
    tag = "jobs",
    operation_id = "listJobs",
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
        // Job-specific filters
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("status" = Option<String>, Query, description = "Filter by job status (QUEUED, PROCESSING, COMPLETED, FAILED, CANCELLED, RETRYING)"),
        ("status_ne" = Option<String>, Query, description = "Filter by job status (not equal)"),
        ("status_in" = Option<String>, Query, description = "Filter by job status (comma-separated list)"),
        ("priority" = Option<String>, Query, description = "Filter by job priority (LOW, NORMAL, HIGH, CRITICAL)"),
        ("priority_gte" = Option<String>, Query, description = "Filter by job priority (greater than or equal)"),
        ("priority_in" = Option<String>, Query, description = "Filter by job priority (comma-separated list)"),
        // Retry and execution filters
        ("retry_count_gte" = Option<i32>, Query, description = "Filter by retry count (greater than or equal to value)"),
        ("retry_count_lte" = Option<i32>, Query, description = "Filter by retry count (less than or equal to value)"),
        ("max_retries_gte" = Option<i32>, Query, description = "Filter by max retries (greater than or equal to value)"),
        ("max_retries_lte" = Option<i32>, Query, description = "Filter by max retries (less than or equal to value)"),
        ("has_retries_remaining" = Option<bool>, Query, description = "Filter by jobs with retries remaining"),
        ("has_error" = Option<bool>, Query, description = "Filter by jobs with errors"),
        ("error_message_like" = Option<String>, Query, description = "Filter by error message (contains text)"),
        // Scheduling filters
        ("is_scheduled" = Option<bool>, Query, description = "Filter by scheduled jobs"),
        ("due_now" = Option<bool>, Query, description = "Filter by jobs due for execution now"),
        // Date filters (ISO 8601 format)
        ("queued_after" = Option<String>, Query, description = "Filter by queue date (after this date)"),
        ("queued_before" = Option<String>, Query, description = "Filter by queue date (before this date)"),
        ("scheduled_after" = Option<String>, Query, description = "Filter by scheduled date (after this date)"),
        ("scheduled_before" = Option<String>, Query, description = "Filter by scheduled date (before this date)"),
        // Advanced filtering
        ("task_id_in" = Option<String>, Query, description = "Filter by task IDs (comma-separated list)"),
        ("id_in" = Option<String>, Query, description = "Filter by job IDs (comma-separated list)")
    ),
    responses(
        (status = 200, description = "List of jobs retrieved successfully"),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_jobs(
    State(ctx): State<TasksContext>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("Listing jobs with query: {:?}", query.0);
    
    let list_input = query.0.to_list_input();
    let pagination = list_input.pagination.unwrap_or_default();
    
    // Extract filters from query parameters
    let filters = extract_job_filters(&query.0.filter().filters);
    
    let job_repo = ctx.repositories.job_repository();
    let list_response = job_repo
        .find_with_filters(filters, pagination.clone())
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific job by ID
#[utoipa::path(
    get,
    path = "/jobs/{job_id}",
    tag = "jobs",
    operation_id = "getJob",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job retrieved successfully"),
        (status = 400, description = "Invalid job ID"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_job(
    State(ctx): State<TasksContext>,
    Path(job_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting job with ID: {}", job_id);
    
    // Validate job ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&job_id, "job_id") {
        warn!("Invalid job ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();
    
    let job = job_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Job", &job_id))?;
    
    Ok(Json(ApiResponse::new(job)))
}

/// Create a new job
#[utoipa::path(
    post,
    path = "/jobs",
    tag = "jobs",
    operation_id = "createJob",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created successfully"),
        (status = 400, description = "Invalid job data"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_job(
    State(ctx): State<TasksContext>,
    Json(request): Json<CreateJobRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating job for task: {:?}", request.task_id);
    
    // Validate the request input
    let _validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
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
    
    // Create UnifiedJob from request
    let unified_job = ratchet_api_types::UnifiedJob {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        task_id: request.task_id,
        priority: request.priority.unwrap_or(ratchet_api_types::JobPriority::Normal),
        status: ratchet_api_types::JobStatus::Queued,
        retry_count: 0,
        max_retries: request.max_retries.unwrap_or(3),
        queued_at: chrono::Utc::now(),
        scheduled_for: request.scheduled_for,
        error_message: None,
        output_destinations: request.output_destinations,
    };
    
    // Create the job using the repository
    let job_repo = ctx.repositories.job_repository();
    let created_job = job_repo.create(unified_job).await
        .map_err(|e| RestError::InternalError(format!("Failed to create job: {}", e)))?;
    
    Ok(Json(ApiResponse::new(created_job)))
}

/// Update an existing job
#[utoipa::path(
    patch,
    path = "/jobs/{job_id}",
    tag = "jobs",
    operation_id = "updateJob",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    request_body = UpdateJobRequest,
    responses(
        (status = 200, description = "Job updated successfully"),
        (status = 400, description = "Invalid job data"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_job(
    State(ctx): State<TasksContext>,
    Path(job_id): Path<String>,
    Json(request): Json<UpdateJobRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating job with ID: {}", job_id);
    
    // Validate job ID input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    if let Err(validation_err) = validator.validate_string(&job_id, "job_id") {
        warn!("Invalid job ID provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate error message if provided
    if let Some(ref error_message) = request.error_message {
        if let Err(validation_err) = validator.validate_string(error_message, "error_message") {
            warn!("Invalid error message provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();
    
    // Get the existing job
    let mut existing_job = job_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Job", &job_id))?;
    
    // Apply updates
    if let Some(priority) = request.priority {
        existing_job.priority = priority;
    }
    if let Some(status) = request.status {
        existing_job.status = status;
    }
    if let Some(max_retries) = request.max_retries {
        existing_job.max_retries = max_retries;
    }
    if let Some(scheduled_for) = request.scheduled_for {
        existing_job.scheduled_for = Some(scheduled_for);
    }
    if let Some(error_message) = request.error_message {
        existing_job.error_message = Some(error_message);
    }
    
    // Update the job using the repository
    let updated_job = job_repo.update(existing_job).await
        .map_err(|e| RestError::InternalError(format!("Failed to update job: {}", e)))?;
    
    Ok(Json(ApiResponse::new(updated_job)))
}

/// Delete a job
#[utoipa::path(
    delete,
    path = "/jobs/{job_id}",
    tag = "jobs",
    operation_id = "deleteJob",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job deleted successfully"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_job(
    State(ctx): State<TasksContext>,
    Path(job_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting job with ID: {}", job_id);
    
    // Validate job ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&job_id, "job_id") {
        warn!("Invalid job ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();
    
    // Check if job exists
    let _job = job_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Job", &job_id))?;
    
    // Delete the job
    job_repo
        .delete(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Job {} deleted successfully", job_id)
    })))
}

/// Cancel a queued job
#[utoipa::path(
    post,
    path = "/jobs/{job_id}/cancel",
    tag = "jobs",
    operation_id = "cancelJob",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job cancelled successfully"),
        (status = 400, description = "Job cannot be cancelled"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cancel_job(
    State(ctx): State<TasksContext>,
    Path(job_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Cancelling job with ID: {}", job_id);
    
    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();
    
    job_repo
        .cancel(api_id)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Job {} cancelled", job_id)
    })))
}

/// Retry a failed job
#[utoipa::path(
    post,
    path = "/jobs/{job_id}/retry",
    tag = "jobs",
    operation_id = "retryJob",
    params(
        ("job_id" = String, Path, description = "Unique job identifier")
    ),
    responses(
        (status = 200, description = "Job retry scheduled successfully"),
        (status = 400, description = "Job cannot be retried"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn retry_job(
    State(ctx): State<TasksContext>,
    Path(job_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Retrying job with ID: {}", job_id);
    
    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();
    
    // Schedule retry with current timestamp
    let retry_at = chrono::Utc::now();
    job_repo
        .schedule_retry(api_id, retry_at)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Job {} scheduled for retry", job_id),
        "retry_at": retry_at
    })))
}

/// Get job statistics
#[utoipa::path(
    get,
    path = "/jobs/stats",
    tag = "jobs",
    operation_id = "getJobStats",
    responses(
        (status = 200, description = "Job statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_job_stats(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Getting job statistics");
    
    let job_repo = ctx.repositories.job_repository();
    
    // Get basic counts
    let total_jobs = job_repo.count().await.map_err(RestError::Database)?;
    
    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = JobStats {
        total_jobs,
        queued_jobs: 0,         // TODO: Implement
        processing_jobs: 0,     // TODO: Implement  
        completed_jobs: 0,      // TODO: Implement
        failed_jobs: 0,         // TODO: Implement
        cancelled_jobs: 0,      // TODO: Implement
        retrying_jobs: 0,       // TODO: Implement
        average_wait_time_ms: None, // TODO: Implement
        jobs_last_24h: 0,       // TODO: Implement
    };
    
    Ok(Json(StatsResponse::new(stats)))
}