//! Execution management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::ExecutionFilters;
use ratchet_web::{QueryParams, ApiResponse, extract_execution_filters};
use ratchet_core::validation::{InputValidator, ErrorSanitizer};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{
        executions::{CreateExecutionRequest, UpdateExecutionRequest, RetryExecutionRequest, ExecutionStats},
        common::StatsResponse
    },
};

/// List all executions with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/executions",
    tag = "executions",
    operation_id = "listExecutions",
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
        // Execution-specific filters
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("status" = Option<String>, Query, description = "Filter by execution status (PENDING, RUNNING, COMPLETED, FAILED, CANCELLED)"),
        ("status_ne" = Option<String>, Query, description = "Filter by execution status (not equal)"),
        ("has_error" = Option<bool>, Query, description = "Filter by executions with errors"),
        ("can_retry" = Option<bool>, Query, description = "Filter by executions that can be retried"),
        ("can_cancel" = Option<bool>, Query, description = "Filter by executions that can be cancelled"),
        ("has_progress" = Option<bool>, Query, description = "Filter by executions with progress data"),
        ("error_message_like" = Option<String>, Query, description = "Filter by error message (contains text)"),
        // Progress and duration filters
        ("progress_gte" = Option<f32>, Query, description = "Filter by progress greater than or equal to value"),
        ("progress_lte" = Option<f32>, Query, description = "Filter by progress less than or equal to value"),
        ("duration_gte" = Option<i32>, Query, description = "Filter by duration greater than or equal to value (ms)"),
        ("duration_lte" = Option<i32>, Query, description = "Filter by duration less than or equal to value (ms)"),
        // Date filters (ISO 8601 format)
        ("queued_after" = Option<String>, Query, description = "Filter by queue date (after this date)"),
        ("queued_before" = Option<String>, Query, description = "Filter by queue date (before this date)"),
        ("started_after" = Option<String>, Query, description = "Filter by start date (after this date)"),
        ("started_before" = Option<String>, Query, description = "Filter by start date (before this date)"),
        ("completed_after" = Option<String>, Query, description = "Filter by completion date (after this date)"),
        ("completed_before" = Option<String>, Query, description = "Filter by completion date (before this date)")
    ),
    responses(
        (status = 200, description = "List of executions retrieved successfully"),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_executions(
    State(ctx): State<TasksContext>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("Listing executions with query: {:?}", query.0);
    
    let list_input = query.0.to_list_input();
    
    // Extract filters from query parameters
    let filters = extract_execution_filters(&query.0.filters);
    
    let execution_repo = ctx.repositories.execution_repository();
    let list_response = execution_repo
        .find_with_list_input(filters, list_input)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific execution by ID
#[utoipa::path(
    get,
    path = "/executions/{execution_id}",
    tag = "executions",
    operation_id = "getExecution",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier")
    ),
    responses(
        (status = 200, description = "Execution retrieved successfully"),
        (status = 400, description = "Invalid execution ID"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_execution(
    State(ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting execution with ID: {}", execution_id);
    
    // Validate execution ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&execution_id, "execution_id") {
        warn!("Invalid execution ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(execution_id.clone());
    let execution_repo = ctx.repositories.execution_repository();
    
    let execution = execution_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Execution", &execution_id))?;
    
    Ok(Json(ApiResponse::new(execution)))
}

/// Create a new execution
#[utoipa::path(
    post,
    path = "/executions",
    tag = "executions",
    operation_id = "createExecution",
    request_body = CreateExecutionRequest,
    responses(
        (status = 201, description = "Execution created successfully"),
        (status = 400, description = "Invalid execution data"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_execution(
    State(ctx): State<TasksContext>,
    Json(request): Json<CreateExecutionRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating execution for task: {:?}", request.task_id);
    
    // Validate the request input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // Validate input JSON
    let input_str = serde_json::to_string(&request.input)
        .map_err(|e| RestError::BadRequest(format!("Invalid input JSON: {}", e)))?;
    if let Err(validation_err) = validator.validate_json(&input_str) {
        warn!("Invalid execution input provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
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
    
    // Create UnifiedExecution from request
    let unified_execution = ratchet_api_types::UnifiedExecution {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        uuid: uuid::Uuid::new_v4(),
        task_id: request.task_id,
        input: request.input,
        output: None,
        status: ratchet_api_types::ExecutionStatus::Pending,
        error_message: None,
        error_details: None,
        queued_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        http_requests: None,
        recording_path: None,
        can_retry: false,
        can_cancel: true,
        progress: None,
    };
    
    // Create the execution using the repository
    let execution_repo = ctx.repositories.execution_repository();
    let created_execution = execution_repo.create(unified_execution).await
        .map_err(|e| RestError::InternalError(format!("Failed to create execution: {}", e)))?;
    
    Ok(Json(ApiResponse::new(created_execution)))
}

/// Update an existing execution
#[utoipa::path(
    patch,
    path = "/executions/{execution_id}",
    tag = "executions",
    operation_id = "updateExecution",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier")
    ),
    request_body = UpdateExecutionRequest,
    responses(
        (status = 200, description = "Execution updated successfully"),
        (status = 400, description = "Invalid execution data"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_execution(
    State(ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
    Json(request): Json<UpdateExecutionRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating execution with ID: {}", execution_id);
    
    // Validate execution ID input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    if let Err(validation_err) = validator.validate_string(&execution_id, "execution_id") {
        warn!("Invalid execution ID provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate output JSON if provided
    if let Some(ref output) = request.output {
        let output_str = serde_json::to_string(output)
            .map_err(|e| RestError::BadRequest(format!("Invalid output JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&output_str) {
            warn!("Invalid execution output provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Validate error message if provided
    if let Some(ref error_message) = request.error_message {
        if let Err(validation_err) = validator.validate_string(error_message, "error_message") {
            warn!("Invalid error message provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    let api_id = ApiId::from_string(execution_id.clone());
    let execution_repo = ctx.repositories.execution_repository();
    
    // Get the existing execution
    let mut existing_execution = execution_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Execution", &execution_id))?;
    
    // Apply updates
    if let Some(output) = request.output {
        existing_execution.output = Some(output);
    }
    if let Some(status) = request.status {
        existing_execution.status = status;
        
        // Update timestamps based on status
        match status {
            ratchet_api_types::ExecutionStatus::Running => {
                if existing_execution.started_at.is_none() {
                    existing_execution.started_at = Some(chrono::Utc::now());
                }
            }
            ratchet_api_types::ExecutionStatus::Completed | 
            ratchet_api_types::ExecutionStatus::Failed | 
            ratchet_api_types::ExecutionStatus::Cancelled => {
                if existing_execution.completed_at.is_none() {
                    existing_execution.completed_at = Some(chrono::Utc::now());
                }
                // Calculate duration if we have started_at
                if let Some(started_at) = existing_execution.started_at {
                    let duration = chrono::Utc::now().signed_duration_since(started_at);
                    existing_execution.duration_ms = Some(duration.num_milliseconds() as i32);
                }
            }
            _ => {}
        }
    }
    if let Some(error_message) = request.error_message {
        existing_execution.error_message = Some(error_message);
    }
    if let Some(error_details) = request.error_details {
        existing_execution.error_details = Some(error_details);
    }
    if let Some(progress) = request.progress {
        existing_execution.progress = Some(progress);
    }
    
    // Update the execution using the repository
    let updated_execution = execution_repo.update(existing_execution).await
        .map_err(|e| RestError::InternalError(format!("Failed to update execution: {}", e)))?;
    
    Ok(Json(ApiResponse::new(updated_execution)))
}

/// Delete an execution
#[utoipa::path(
    delete,
    path = "/executions/{execution_id}",
    tag = "executions",
    operation_id = "deleteExecution",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier")
    ),
    responses(
        (status = 200, description = "Execution deleted successfully"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_execution(
    State(ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting execution with ID: {}", execution_id);
    
    // Validate execution ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&execution_id, "execution_id") {
        warn!("Invalid execution ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(execution_id.clone());
    let execution_repo = ctx.repositories.execution_repository();
    
    // Check if execution exists
    let _execution = execution_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Execution", &execution_id))?;
    
    // Delete the execution
    execution_repo
        .delete(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Execution {} deleted successfully", execution_id)
    })))
}

/// Cancel a running execution
#[utoipa::path(
    post,
    path = "/executions/{execution_id}/cancel",
    tag = "executions",
    operation_id = "cancelExecution",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier")
    ),
    responses(
        (status = 200, description = "Execution cancelled successfully"),
        (status = 400, description = "Execution cannot be cancelled"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cancel_execution(
    State(ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Cancelling execution with ID: {}", execution_id);
    
    let api_id = ApiId::from_string(execution_id.clone());
    let execution_repo = ctx.repositories.execution_repository();
    
    execution_repo
        .mark_failed(api_id, "Cancelled by user".to_string(), None)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Execution {} cancelled", execution_id)
    })))
}

/// Retry a failed execution
#[utoipa::path(
    post,
    path = "/executions/{execution_id}/retry",
    tag = "executions",
    operation_id = "retryExecution",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier")
    ),
    request_body = RetryExecutionRequest,
    responses(
        (status = 201, description = "Execution retry created successfully"),
        (status = 400, description = "Execution cannot be retried"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn retry_execution(
    State(ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
    Json(request): Json<RetryExecutionRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Retrying execution with ID: {}", execution_id);
    
    // Validate execution ID
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&execution_id, "execution_id") {
        warn!("Invalid execution ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(execution_id.clone());
    let execution_repo = ctx.repositories.execution_repository();
    
    // Find the original execution
    let original_execution = execution_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Execution", &execution_id))?;
    
    // Check if execution can be retried (only retry failed executions)
    if !matches!(original_execution.status, ratchet_api_types::ExecutionStatus::Failed) {
        return Err(RestError::BadRequest(
            "Only failed executions can be retried".to_string()
        ));
    }
    
    // Use new input if provided, otherwise use original input
    let input_data = request.input.unwrap_or(original_execution.input);
    
    // Create new execution from the original
    let new_execution = ratchet_api_types::UnifiedExecution {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        uuid: uuid::Uuid::new_v4(),
        task_id: original_execution.task_id,
        input: input_data,
        output: None,
        status: ratchet_api_types::ExecutionStatus::Pending,
        error_message: None,
        error_details: None,
        queued_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        http_requests: None,
        recording_path: None,
        can_retry: false,
        can_cancel: true,
        progress: None,
    };
    
    // Create the new execution
    let created_execution = execution_repo.create(new_execution).await
        .map_err(|e| RestError::InternalError(format!("Failed to create retry execution: {}", e)))?;
    
    info!("Created retry execution with ID: {}", created_execution.id);
    
    Ok(Json(ApiResponse::new(created_execution)))
}

/// Get execution logs
#[utoipa::path(
    get,
    path = "/executions/{execution_id}/logs",
    tag = "executions",
    operation_id = "getExecutionLogs",
    params(
        ("execution_id" = String, Path, description = "Unique execution identifier"),
        ("follow" = Option<bool>, Query, description = "Follow logs in real-time"),
        ("since" = Option<String>, Query, description = "Show logs since timestamp (ISO 8601)")
    ),
    responses(
        (status = 200, description = "Execution logs retrieved successfully"),
        (status = 404, description = "Execution not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_execution_logs(
    State(_ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting logs for execution: {}", execution_id);
    
    // For now, return placeholder logs
    // In a full implementation, this would:
    // 1. Validate execution exists
    // 2. Retrieve logs from logging system
    // 3. Support real-time streaming if requested
    // 4. Return formatted log entries
    
    Ok(Json(serde_json::json!({
        "execution_id": execution_id,
        "logs": [
            {
                "timestamp": "2023-12-07T14:30:15.123Z",
                "level": "info",
                "message": "Starting task execution",
                "source": "task_executor"
            },
            {
                "timestamp": "2023-12-07T14:30:15.145Z",
                "level": "info", 
                "message": "Processing input data",
                "source": "task_executor"
            }
        ],
        "has_more": false
    })))
}

/// Get execution statistics
#[utoipa::path(
    get,
    path = "/executions/stats",
    tag = "executions",
    operation_id = "getExecutionStats",
    responses(
        (status = 200, description = "Execution statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_execution_stats(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Getting execution statistics");
    
    let execution_repo = ctx.repositories.execution_repository();
    
    // Get basic counts
    let total_executions = execution_repo.count().await.map_err(RestError::Database)?;
    
    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = ExecutionStats {
        total_executions,
        pending_executions: 0,   // TODO: Implement
        running_executions: 0,   // TODO: Implement  
        completed_executions: 0, // TODO: Implement
        failed_executions: 0,    // TODO: Implement
        cancelled_executions: 0, // TODO: Implement
        average_duration_ms: None, // TODO: Implement
        success_rate: 0.0,       // TODO: Implement
        executions_last_24h: 0,  // TODO: Implement
    };
    
    Ok(Json(StatsResponse::new(stats)))
}