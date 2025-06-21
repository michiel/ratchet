//! Job management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_core::validation::{ErrorSanitizer, InputValidator};
use ratchet_interfaces::JobFilters;
use ratchet_web::{extract_job_filters, ApiResponse, QueryParams};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{
        common::StatsResponse,
        jobs::{CreateJobRequest, JobStats, UpdateJobRequest},
    },
};

/// List all jobs with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/jobs",
    responses(
        (status = 200, description = "List of jobs", body = Vec<ratchet_api_types::UnifiedJob>)
    ),
    tag = "jobs"
)]
pub async fn list_jobs(State(ctx): State<TasksContext>, query: QueryParams) -> RestResult<impl IntoResponse> {
    info!("Listing jobs with query: {:?}", query.0);

    let list_input = query.0.to_list_input();

    // Extract filters from query parameters
    let filters = extract_job_filters(&query.0.filters);

    let job_repo = ctx.repositories.job_repository();
    let list_response = job_repo
        .find_with_list_input(filters, list_input)
        .await
        .map_err(RestError::Database)?;

    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific job by ID
#[utoipa::path(
    get,
    path = "/jobs/{id}",
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job details", body = ratchet_api_types::UnifiedJob),
        (status = 404, description = "Job not found")
    ),
    tag = "jobs"
)]
pub async fn get_job(State(ctx): State<TasksContext>, Path(job_id): Path<String>) -> RestResult<impl IntoResponse> {
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
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created successfully", body = ratchet_api_types::UnifiedJob),
        (status = 400, description = "Invalid request")
    ),
    tag = "jobs"
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
    let created_job = job_repo
        .create(unified_job)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to create job: {}", e)))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::new(created_job))))
}

/// Update an existing job
#[utoipa::path(
    patch,
    path = "/jobs/{id}",
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    request_body = UpdateJobRequest,
    responses(
        (status = 200, description = "Job updated successfully", body = ratchet_api_types::UnifiedJob),
        (status = 404, description = "Job not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "jobs"
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
    let updated_job = job_repo
        .update(existing_job)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to update job: {}", e)))?;

    Ok(Json(ApiResponse::new(updated_job)))
}

/// Delete a job
#[utoipa::path(
    delete,
    path = "/jobs/{id}",
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job deleted successfully"),
        (status = 404, description = "Job not found")
    ),
    tag = "jobs"
)]
pub async fn delete_job(State(ctx): State<TasksContext>, Path(job_id): Path<String>) -> RestResult<impl IntoResponse> {
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
    job_repo.delete(api_id.as_i32().unwrap_or(0)).await.map_err(|db_err| {
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
    path = "/jobs/{id}/cancel",
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job cancelled successfully"),
        (status = 404, description = "Job not found")
    ),
    tag = "jobs"
)]
pub async fn cancel_job(State(ctx): State<TasksContext>, Path(job_id): Path<String>) -> RestResult<impl IntoResponse> {
    info!("Cancelling job with ID: {}", job_id);

    let api_id = ApiId::from_string(job_id.clone());
    let job_repo = ctx.repositories.job_repository();

    job_repo.cancel(api_id).await.map_err(RestError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Job {} cancelled", job_id)
    })))
}

/// Retry a failed job
#[utoipa::path(
    post,
    path = "/jobs/{id}/retry",
    params(
        ("id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job retry scheduled successfully"),
        (status = 404, description = "Job not found")
    ),
    tag = "jobs"
)]
pub async fn retry_job(State(ctx): State<TasksContext>, Path(job_id): Path<String>) -> RestResult<impl IntoResponse> {
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
    responses(
        (status = 200, description = "Job statistics", body = JobStats)
    ),
    tag = "jobs"
)]
pub async fn get_job_stats(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Getting job statistics");

    let job_repo = ctx.repositories.job_repository();

    // Get basic counts
    let total_jobs = job_repo.count().await.map_err(RestError::Database)?;

    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = JobStats {
        total_jobs,
        queued_jobs: 0,             // TODO: Implement
        processing_jobs: 0,         // TODO: Implement
        completed_jobs: 0,          // TODO: Implement
        failed_jobs: 0,             // TODO: Implement
        cancelled_jobs: 0,          // TODO: Implement
        retrying_jobs: 0,           // TODO: Implement
        average_wait_time_ms: None, // TODO: Implement
        jobs_last_24h: 0,           // TODO: Implement
    };

    Ok(Json(StatsResponse::new(stats)))
}
