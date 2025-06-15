//! Task management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_interfaces::TaskFilters;
use ratchet_web::{QueryParams, ApiResponse};
use ratchet_core::validation::{InputValidator, ErrorSanitizer};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{CreateTaskRequest, UpdateTaskRequest, TaskStats, common::StatsResponse},
};

/// List all tasks with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/tasks",
    tag = "tasks",
    operation_id = "listTasks",
    params(
        ("page" = Option<u32>, Query, description = "Page number (0-based)"),
        ("limit" = Option<u32>, Query, description = "Number of items per page"),
        ("filter" = Option<String>, Query, description = "Filter expression"),
        ("sort" = Option<String>, Query, description = "Sort expression")
    ),
    responses(
        (status = 200, description = "List of tasks retrieved successfully"),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    )
)]
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
#[utoipa::path(
    get,
    path = "/tasks/{task_id}",
    tag = "tasks",
    operation_id = "getTask",
    params(
        ("task_id" = String, Path, description = "Unique task identifier")
    ),
    responses(
        (status = 200, description = "Task retrieved successfully"),
        (status = 400, description = "Invalid task ID"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Getting task with ID: {}", task_id);
    
    // Validate task ID input
    let validator = InputValidator::new();
    if let Err(validation_err) = validator.validate_string(&task_id, "task_id") {
        warn!("Invalid task ID provided: {}", validation_err);
        let sanitizer = ErrorSanitizer::default();
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    let task = task_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitizer = ErrorSanitizer::default();
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Task", &task_id))?;
    
    Ok(Json(ApiResponse::new(task)))
}

/// Create a new task
#[utoipa::path(
    post,
    path = "/tasks",
    tag = "tasks",
    operation_id = "createTask",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created successfully"),
        (status = 400, description = "Invalid task data"),
        (status = 409, description = "Task with same name already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_task(
    State(ctx): State<TasksContext>,
    Json(request): Json<CreateTaskRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Creating task: {}", request.name);
    
    // Validate the request input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // Validate task name
    if let Err(validation_err) = validator.validate_task_name(&request.name) {
        warn!("Invalid task name provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate description if provided
    if let Some(ref description) = request.description {
        if let Err(validation_err) = validator.validate_string(description, "description") {
            warn!("Invalid task description provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Validate version
    if let Err(validation_err) = validator.validate_semver(&request.version) {
        warn!("Invalid task version provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Note: Path validation would be done here if the request had a path field
    
    // Validate JSON schemas if provided
    if let Some(ref input_schema) = request.input_schema {
        let input_str = serde_json::to_string(input_schema)
            .map_err(|e| RestError::BadRequest(format!("Invalid input schema JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&input_str) {
            warn!("Invalid input schema provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref output_schema) = request.output_schema {
        let output_str = serde_json::to_string(output_schema)
            .map_err(|e| RestError::BadRequest(format!("Invalid output schema JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&output_str) {
            warn!("Invalid output schema provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Create UnifiedTask from request
    let unified_task = ratchet_api_types::UnifiedTask {
        id: ratchet_api_types::ApiId::from_i32(0), // Will be set by database
        uuid: uuid::Uuid::new_v4(),
        name: request.name,
        description: request.description,
        version: request.version.clone(),
        enabled: request.enabled.unwrap_or(true),
        registry_source: false, // Tasks created via API are not from registry
        available_versions: vec![request.version],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        validated_at: None,
        in_sync: true,
        input_schema: request.input_schema,
        output_schema: request.output_schema,
        metadata: request.metadata,
    };
    
    // Create the task using the repository
    let task_repo = ctx.repositories.task_repository();
    let created_task = task_repo.create(unified_task).await
        .map_err(|e| RestError::InternalError(format!("Failed to create task: {}", e)))?;
    
    Ok(Json(ApiResponse::new(created_task)))
}

/// Update an existing task
#[utoipa::path(
    put,
    path = "/tasks/{task_id}",
    tag = "tasks",
    operation_id = "updateTask",
    params(
        ("task_id" = String, Path, description = "Unique task identifier")
    ),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated successfully"),
        (status = 400, description = "Invalid task data"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
    Json(request): Json<UpdateTaskRequest>,
) -> RestResult<impl IntoResponse> {
    info!("Updating task with ID: {}", task_id);
    
    // Validate task ID input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    if let Err(validation_err) = validator.validate_string(&task_id, "task_id") {
        warn!("Invalid task ID provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    // Validate update request fields if provided
    if let Some(ref name) = request.name {
        if let Err(validation_err) = validator.validate_task_name(name) {
            warn!("Invalid task name provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref description) = request.description {
        if let Err(validation_err) = validator.validate_string(description, "description") {
            warn!("Invalid task description provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref version) = request.version {
        if let Err(validation_err) = validator.validate_semver(version) {
            warn!("Invalid task version provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Validate JSON schemas if provided
    if let Some(ref input_schema) = request.input_schema {
        let input_str = serde_json::to_string(input_schema)
            .map_err(|e| RestError::BadRequest(format!("Invalid input schema JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&input_str) {
            warn!("Invalid input schema provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    if let Some(ref output_schema) = request.output_schema {
        let output_str = serde_json::to_string(output_schema)
            .map_err(|e| RestError::BadRequest(format!("Invalid output schema JSON: {}", e)))?;
        if let Err(validation_err) = validator.validate_json(&output_str) {
            warn!("Invalid output schema provided: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    // Get the existing task
    let mut existing_task = task_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?
        .ok_or_else(|| RestError::not_found("Task", &task_id))?;
    
    // Apply updates
    if let Some(name) = request.name {
        existing_task.name = name;
    }
    if let Some(description) = request.description {
        existing_task.description = Some(description);
    }
    if let Some(version) = request.version {
        existing_task.version = version.clone();
        // Add to available versions if not already present
        if !existing_task.available_versions.contains(&version) {
            existing_task.available_versions.push(version);
        }
    }
    if let Some(enabled) = request.enabled {
        existing_task.enabled = enabled;
    }
    if let Some(input_schema) = request.input_schema {
        existing_task.input_schema = Some(input_schema);
    }
    if let Some(output_schema) = request.output_schema {
        existing_task.output_schema = Some(output_schema);
    }
    if let Some(metadata) = request.metadata {
        existing_task.metadata = Some(metadata);
    }
    
    // Update timestamp
    existing_task.updated_at = chrono::Utc::now();
    
    // Update the task using the repository
    let updated_task = task_repo.update(existing_task).await
        .map_err(|e| RestError::InternalError(format!("Failed to update task: {}", e)))?;
    
    Ok(Json(ApiResponse::new(updated_task)))
}

/// Delete a task
#[utoipa::path(
    delete,
    path = "/tasks/{task_id}",
    tag = "tasks",
    operation_id = "deleteTask",
    params(
        ("task_id" = String, Path, description = "Unique task identifier")
    ),
    responses(
        (status = 200, description = "Task deleted successfully"),
        (status = 400, description = "Invalid task ID"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Deleting task with ID: {}", task_id);
    
    // Validate task ID input
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    if let Err(validation_err) = validator.validate_string(&task_id, "task_id") {
        warn!("Invalid task ID provided: {}", validation_err);
        let sanitized_error = sanitizer.sanitize_error(&validation_err);
        return Err(RestError::BadRequest(sanitized_error.message));
    }
    
    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();
    
    // Check if task exists before deletion
    let existing_task = task_repo
        .find_by_id(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|db_err| {
            let sanitized_error = sanitizer.sanitize_error(&db_err);
            RestError::InternalError(sanitized_error.message)
        })?;
    
    if existing_task.is_none() {
        return Err(RestError::not_found("Task", &task_id));
    }
    
    // Delete the task using the repository
    task_repo.delete(api_id.as_i32().unwrap_or(0)).await
        .map_err(|e| RestError::InternalError(format!("Failed to delete task: {}", e)))?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Task {} deleted successfully", task_id)
    })))
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
#[utoipa::path(
    get,
    path = "/tasks/stats",
    tag = "tasks",
    operation_id = "getTaskStats",
    responses(
        (status = 200, description = "Task statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
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

/// MCP task development - create a new task with full JavaScript code and testing
pub async fn mcp_create_task(
    State(_ctx): State<TasksContext>,
    Json(request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Creating task with development features");
    
    // Validate the incoming JSON request
    let validator = InputValidator::new();
    let sanitizer = ErrorSanitizer::default();
    
    // First validate the JSON structure itself
    let validated_input = match validator.validate_task_input(&request) {
        Ok(input) => input,
        Err(validation_err) => {
            warn!("Invalid MCP task creation request: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    };
    
    // Extract and validate specific fields
    if let Some(name) = validated_input.get("name").and_then(|v| v.as_str()) {
        if let Err(validation_err) = validator.validate_task_name(name) {
            warn!("Invalid task name in MCP request: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    } else {
        return Err(RestError::BadRequest("Missing required field: name".to_string()));
    }
    
    if let Some(version) = validated_input.get("version").and_then(|v| v.as_str()) {
        if let Err(validation_err) = validator.validate_semver(version) {
            warn!("Invalid version in MCP request: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // Validate JavaScript code if provided
    if let Some(code) = validated_input.get("code").and_then(|v| v.as_str()) {
        if let Err(validation_err) = validator.validate_string(code, "javascript_code") {
            warn!("Invalid JavaScript code in MCP request: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
        
        // Additional checks for JavaScript-specific concerns
        if code.contains("eval(") || code.contains("Function(") {
            warn!("Potentially dangerous JavaScript code detected");
            return Err(RestError::BadRequest("JavaScript code contains potentially dangerous constructs".to_string()));
        }
    }
    
    // Validate test cases if provided
    if let Some(test_cases) = validated_input.get("testCases") {
        if let Err(validation_err) = validator.validate_task_input(test_cases) {
            warn!("Invalid test cases in MCP request: {}", validation_err);
            let sanitized_error = sanitizer.sanitize_error(&validation_err);
            return Err(RestError::BadRequest(sanitized_error.message));
        }
    }
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Create task in database with full metadata
    // 2. Run test cases if provided
    // 3. Return the created task with validation results
    
    Err(RestError::InternalError(
        "MCP task creation requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// MCP task development - edit an existing task
pub async fn mcp_edit_task(
    State(_ctx): State<TasksContext>,
    Path(task_name): Path<String>,
    Json(_request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Editing task: {}", task_name);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate the task exists
    // 2. Update JavaScript code and/or metadata
    // 3. Validate changes
    // 4. Run test cases if provided
    // 5. Return the updated task
    
    Err(RestError::InternalError(
        "MCP task editing requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// MCP task development - delete a task
pub async fn mcp_delete_task(
    State(_ctx): State<TasksContext>,
    Path(task_name): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Deleting task: {}", task_name);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate the task exists
    // 2. Remove from database and filesystem
    // 3. Clean up related executions
    // 4. Return deletion confirmation
    
    Err(RestError::InternalError(
        "MCP task deletion requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// MCP task development - test a task
pub async fn mcp_test_task(
    State(_ctx): State<TasksContext>,
    Path(task_name): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Testing task: {}", task_name);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Load task from database
    // 2. Run all test cases
    // 3. Execute JavaScript code
    // 4. Return detailed test results
    
    Err(RestError::InternalError(
        "MCP task testing requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// MCP task development - store execution result
pub async fn mcp_store_result(
    State(_ctx): State<TasksContext>,
    Json(_request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Storing task execution result");
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Validate the execution data
    // 2. Store in execution repository
    // 3. Update task statistics
    // 4. Return storage confirmation
    
    Err(RestError::InternalError(
        "MCP result storage requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}

/// MCP task development - get stored results for a task
pub async fn mcp_get_results(
    State(_ctx): State<TasksContext>,
    Path(task_name): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Getting results for task: {}", task_name);
    
    // For now, return a placeholder response
    // In a full implementation, this would:
    // 1. Query execution repository by task name
    // 2. Return paginated results with execution data
    
    Err(RestError::InternalError(
        "MCP result retrieval requires MCP service integration".to_string(),
    )) as RestResult<Json<serde_json::Value>>
}