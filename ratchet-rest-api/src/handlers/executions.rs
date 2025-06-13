//! Execution management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::ExecutionFilters;
use ratchet_web::{QueryParams, ApiResponse};
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
        ("page" = Option<u32>, Query, description = "Page number (0-based)"),
        ("limit" = Option<u32>, Query, description = "Number of items per page"),
        ("status" = Option<String>, Query, description = "Filter by execution status"),
        ("task_id" = Option<String>, Query, description = "Filter by task ID"),
        ("sort" = Option<String>, Query, description = "Sort expression")
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
    let pagination = list_input.pagination.unwrap_or_default();
    
    // Convert query filters to execution filters
    let filters = ExecutionFilters {
        task_id: None, // TODO: Extract from query filters
        status: None,
        queued_after: None,
        completed_after: None,
    };
    
    let execution_repo = ctx.repositories.execution_repository();
    let list_response = execution_repo
        .find_with_filters(filters, pagination.clone())
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
    State(_ctx): State<TasksContext>,
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
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate task exists and is enabled
    // 2. Validate input against task's input schema
    // 3. Create execution in database
    // 4. Queue for processing if not scheduled
    // 5. Return the created execution
    
    Err(RestError::InternalError(
        "Execution creation not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
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
    State(_ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
    Json(_request): Json<UpdateExecutionRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating execution with ID: {}", execution_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate execution exists
    // 2. Update execution status, output, or error information
    // 3. Trigger any necessary downstream actions
    // 4. Return the updated execution
    
    Err(RestError::InternalError(
        "Execution update not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
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
    State(_ctx): State<TasksContext>,
    Path(execution_id): Path<String>,
    Json(_request): Json<RetryExecutionRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Retrying execution with ID: {}", execution_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate original execution exists and can be retried
    // 2. Create new execution with same or updated input
    // 3. Queue for processing
    // 4. Return the new execution
    
    Err(RestError::InternalError(
        "Execution retry not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
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