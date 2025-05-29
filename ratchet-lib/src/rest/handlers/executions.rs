use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    database::{
        entities::{Execution, ExecutionStatus},
        repositories::{ExecutionRepository, RepositoryFactory},
    },
    execution::job_queue::JobQueueManager,
    rest::{
        extractors::ListQueryExtractor,
        middleware::{
            error_handler::RestError,
            pagination::WithPaginationHeaders,
        },
        models::{
            common::ApiResponse,
            executions::{
                ExecutionResponse, ExecutionDetailResponse, ExecutionCreateRequest, 
                ExecutionUpdateRequest
            },
        },
    },
};

/// Context for execution handlers
#[derive(Clone)]
pub struct ExecutionsContext {
    pub repository: ExecutionRepository,
    pub job_queue: Arc<JobQueueManager>,
}

impl ExecutionsContext {
    pub fn new(repositories: RepositoryFactory, job_queue: Arc<JobQueueManager>) -> Self {
        Self {
            repository: repositories.execution_repository(),
            job_queue,
        }
    }
}

/// List executions with pagination, filtering, and sorting
pub async fn list_executions(
    State(ctx): State<ExecutionsContext>,
    ListQueryExtractor(query): ListQueryExtractor,
) -> Result<impl IntoResponse, RestError> {
    let _offset = query.pagination.offset();
    let limit = query.pagination.limit();
    
    // TODO: Apply filters and sorting - implement proper ExecutionFilters parsing from query.filter
    // For now, skip filtering and sorting to get basic functionality working
    let _sort_column = query.sort.sort_field().unwrap_or("queued_at");
    let _sort_direction = query.sort.sort_direction();
    
    // For now, use simple recent query - TODO: implement proper filtering and sorting
    let total = ctx.repository.count().await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    let executions = ctx.repository.find_recent(limit)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    let response_data: Vec<ExecutionResponse> = executions
        .into_iter()
        .map(ExecutionResponse::from)
        .collect();
    
    let response = ApiResponse { data: response_data };
    
    Ok(Json(response).with_pagination_headers(total, 0, limit, "executions"))
}

/// Get a specific execution by ID
pub async fn get_execution(
    State(ctx): State<ExecutionsContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {
    let execution_id = id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid execution ID format".to_string()))?;
    
    let execution = ctx.repository
        .find_by_id(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound(format!("Execution {} not found", id)))?;
    
    let response_data = ExecutionDetailResponse::from(execution);
    let response = ApiResponse { data: response_data };
    
    Ok(Json(response))
}

/// Create a new execution
pub async fn create_execution(
    State(ctx): State<ExecutionsContext>,
    Json(request): Json<ExecutionCreateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let task_id = request.task_id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid task ID format".to_string()))?;
    
    // Create new execution
    let execution = Execution::new(task_id, request.input);
    
    let created_execution = ctx.repository
        .create(execution)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    // Queue the execution for processing
    // Note: This would typically integrate with the job queue system
    // For now, we'll just create the execution record
    
    let response_data = ExecutionDetailResponse::from(created_execution);
    let response = ApiResponse { data: response_data };
    
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update an execution
pub async fn update_execution(
    State(ctx): State<ExecutionsContext>,
    Path(id): Path<String>,
    Json(request): Json<ExecutionUpdateRequest>,
) -> Result<impl IntoResponse, RestError> {
    let execution_id = id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid execution ID format".to_string()))?;
    
    let mut execution = ctx.repository
        .find_by_id(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound(format!("Execution {} not found", id)))?;
    
    // Update fields that are allowed to be modified
    let mut updated = false;
    
    // Status updates
    if let Some(new_status) = request.status {
        // Validate status transitions
        match (&execution.status, &new_status) {
            // Allow cancellation of pending/running executions
            (ExecutionStatus::Pending | ExecutionStatus::Running, ExecutionStatus::Cancelled) => {
                execution.status = new_status;
                execution.completed_at = Some(chrono::Utc::now());
                updated = true;
            }
            // Allow manual completion with output
            (ExecutionStatus::Running, ExecutionStatus::Completed) => {
                execution.status = new_status;
                if let Some(output) = request.output {
                    execution.output = Some(sea_orm::prelude::Json::from(output));
                }
                execution.completed_at = Some(chrono::Utc::now());
                
                // Calculate duration if we have start time
                if let Some(started) = execution.started_at {
                    let duration = chrono::Utc::now().signed_duration_since(started);
                    execution.duration_ms = Some(duration.num_milliseconds() as i32);
                }
                updated = true;
            }
            // Allow manual failure
            (ExecutionStatus::Running, ExecutionStatus::Failed) => {
                execution.status = new_status;
                if let Some(error_msg) = request.error_message {
                    execution.error_message = Some(error_msg);
                }
                if let Some(error_details) = request.error_details {
                    execution.error_details = Some(sea_orm::prelude::Json::from(error_details));
                }
                execution.completed_at = Some(chrono::Utc::now());
                
                // Calculate duration if we have start time
                if let Some(started) = execution.started_at {
                    let duration = chrono::Utc::now().signed_duration_since(started);
                    execution.duration_ms = Some(duration.num_milliseconds() as i32);
                }
                updated = true;
            }
            _ => {
                return Err(RestError::BadRequest(
                    format!("Invalid status transition from {:?} to {:?}", execution.status, new_status)
                ));
            }
        }
    }
    
    if !updated {
        return Err(RestError::BadRequest("No valid updates provided".to_string()));
    }
    
    let updated_execution = ctx.repository
        .update(execution)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    let response_data = ExecutionDetailResponse::from(updated_execution);
    let response = ApiResponse { data: response_data };
    
    Ok(Json(response))
}

/// Delete an execution
pub async fn delete_execution(
    State(ctx): State<ExecutionsContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {
    let execution_id = id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid execution ID format".to_string()))?;
    
    // Check if execution exists
    let execution = ctx.repository
        .find_by_id(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound(format!("Execution {} not found", id)))?;
    
    // Don't allow deletion of running executions
    if matches!(execution.status, ExecutionStatus::Running) {
        return Err(RestError::BadRequest("Cannot delete running execution".to_string()));
    }
    
    ctx.repository
        .delete(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Retry a failed execution
pub async fn retry_execution(
    State(ctx): State<ExecutionsContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {
    let execution_id = id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid execution ID format".to_string()))?;
    
    let execution = ctx.repository
        .find_by_id(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound(format!("Execution {} not found", id)))?;
    
    // Only allow retry of failed or cancelled executions
    if !matches!(execution.status, ExecutionStatus::Failed | ExecutionStatus::Cancelled) {
        return Err(RestError::BadRequest("Can only retry failed or cancelled executions".to_string()));
    }
    
    // Create a new execution with the same input
    let new_execution = Execution::new(execution.task_id, execution.input.clone());
    
    let created_execution = ctx.repository
        .create(new_execution)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    // Queue the new execution for processing
    // Note: This would typically integrate with the job queue system
    
    let response_data = ExecutionDetailResponse::from(created_execution);
    let response = ApiResponse { data: response_data };
    
    Ok((StatusCode::CREATED, Json(response)))
}

/// Cancel a pending or running execution
pub async fn cancel_execution(
    State(ctx): State<ExecutionsContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {
    let execution_id = id.parse::<i32>()
        .map_err(|_| RestError::BadRequest("Invalid execution ID format".to_string()))?;
    
    let mut execution = ctx.repository
        .find_by_id(execution_id)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?
        .ok_or_else(|| RestError::NotFound(format!("Execution {} not found", id)))?;
    
    // Only allow cancellation of pending or running executions
    if !matches!(execution.status, ExecutionStatus::Pending | ExecutionStatus::Running) {
        return Err(RestError::BadRequest("Can only cancel pending or running executions".to_string()));
    }
    
    // Update execution status to cancelled
    execution.status = ExecutionStatus::Cancelled;
    execution.completed_at = Some(chrono::Utc::now());
    
    // Calculate duration if execution was running
    if let Some(started) = execution.started_at {
        let duration = chrono::Utc::now().signed_duration_since(started);
        execution.duration_ms = Some(duration.num_milliseconds() as i32);
    }
    
    let updated_execution = ctx.repository
        .update(execution)
        .await
        .map_err(|e| RestError::DatabaseError(e.to_string()))?;
    
    // TODO: Signal the job queue to cancel the execution if it's running
    
    let response_data = ExecutionDetailResponse::from(updated_execution);
    let response = ApiResponse { data: response_data };
    
    Ok(Json(response))
}