//! Job management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::JobFilters;
use ratchet_web::{QueryParams, ApiResponse};
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
        ("page" = Option<u32>, Query, description = "Page number (0-based)"),
        ("limit" = Option<u32>, Query, description = "Number of items per page"),
        ("status" = Option<String>, Query, description = "Filter by job status"),
        ("priority" = Option<String>, Query, description = "Filter by job priority"),
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("sort" = Option<String>, Query, description = "Sort expression")
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
    
    // Convert query filters to job filters
    let filters = JobFilters {
        task_id: None, // TODO: Extract from query filters
        status: None,
        priority: None,
        queued_after: None,
        scheduled_before: None,
    };
    
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
    State(_ctx): State<TasksContext>,
    Json(request): Json<CreateJobRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating job for task: {:?}", request.task_id);
    
    // Validate the request input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // Validate input JSON
    let input_str = serde_json::to_string(&request.input)
        .map_err(|e| RestError::BadRequest(format!("Invalid input JSON: {}", e)))?;
    if let Err(validation_err) = validator.validate_json(&input_str) {
        warn!("Invalid job input provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate task exists and is enabled
    // 2. Validate input against task's input schema
    // 3. Create job in database with priority and retry settings
    // 4. Add to job queue for processing
    // 5. Return the created job
    
    Err(RestError::InternalError(
        "Job creation not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
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
    State(_ctx): State<TasksContext>,
    Path(job_id): Path<String>,
    Json(_request): Json<UpdateJobRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating job with ID: {}", job_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate job exists
    // 2. Update job status, priority, or retry settings
    // 3. Handle queue position changes if priority updated
    // 4. Return the updated job
    
    Err(RestError::InternalError(
        "Job update not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
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