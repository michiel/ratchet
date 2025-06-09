//! Task management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::TaskFilters;
use ratchet_web::{QueryParams, ApiResponse};
use tracing::info;

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{CreateTaskRequest, UpdateTaskRequest, TaskStats, common::StatsResponse},
};

/// List all tasks with optional filtering and pagination
pub async fn list_tasks(
    State(ctx): State<TasksContext>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("Listing tasks with query: {:?}", query.0);
    
    let list_input = query.0.to_list_input();
    let pagination = list_input.pagination.unwrap_or_default();
    
    // Convert query filters to task filters
    let filters = TaskFilters {
        name: None, // TODO: Extract from query filters
        enabled: None,
        registry_source: None,
        validated_after: None,
    };
    
    let task_repo = ctx.repositories.task_repository();
    let list_response = task_repo
        .find_with_filters(filters, pagination.clone())
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific task by ID
pub async fn get_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting task with ID: {}", task_id);
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    let task = task_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(RestError::Database)?
        .ok_or_else(|| RestError::not_found("Task", &task_id))?;
    
    Ok(Json(ApiResponse::new(task)))
}

/// Create a new task
pub async fn create_task(
    State(ctx): State<TasksContext>,
    Json(request): Json<CreateTaskRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating task: {}", request.name);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate the request
    // 2. Create task in registry or database
    // 3. Sync with other systems
    // 4. Return the created task
    
    Err(RestError::InternalError(
        "Task creation not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Update an existing task
pub async fn update_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
    Json(request): Json<UpdateTaskRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating task with ID: {}", task_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate the request and task existence
    // 2. Update task in database
    // 3. Sync with registry if needed
    // 4. Return the updated task
    
    Err(RestError::InternalError(
        "Task update not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Delete a task
pub async fn delete_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting task with ID: {}", task_id);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate task existence and dependencies
    // 2. Delete from database
    // 3. Clean up related executions/jobs if needed
    // 4. Return success confirmation
    
    Err(RestError::InternalError(
        "Task deletion not yet implemented".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// Enable a task
pub async fn enable_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Enabling task with ID: {}", task_id);
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    task_repo
        .set_enabled(api_id, true)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Task {} enabled", task_id)
    })))
}

/// Disable a task
pub async fn disable_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Disabling task with ID: {}", task_id);
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    task_repo
        .set_enabled(api_id, false)
        .await
        .map_err(RestError::Database)?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Task {} disabled", task_id)
    })))
}

/// Sync tasks from registry
pub async fn sync_tasks(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Syncing tasks from registry");
    
    let sync_result = ctx
        .registry_manager
        .sync_with_database()
        .await
        .map_err(|e| RestError::InternalError(format!("Sync failed: {}", e)))?;
    
    Ok(Json(ApiResponse::new(sync_result)))
}

/// Get task statistics
pub async fn get_task_stats(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Getting task statistics");
    
    let task_repo = ctx.repositories.task_repository();
    
    // Get basic counts
    let total_tasks = task_repo.count().await.map_err(RestError::Database)?;
    
    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = TaskStats {
        total_tasks,
        enabled_tasks: 0,   // TODO: Implement
        disabled_tasks: 0,  // TODO: Implement
        registry_tasks: 0,  // TODO: Implement
        database_tasks: total_tasks,
        validation_errors: 0, // TODO: Implement
        last_sync: None,    // TODO: Implement
    };
    
    Ok(Json(StatsResponse::new(stats)))
}