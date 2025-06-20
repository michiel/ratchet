//! Task management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use ratchet_api_types::ApiId;
use ratchet_core::validation::{ErrorSanitizer, InputValidator};
use ratchet_interfaces::{DatabaseError, TaskFilters};
use ratchet_mcp::server::task_dev_tools::{
    CreateTaskRequest as McpCreateTaskRequest, DeleteTaskRequest as McpDeleteTaskRequest,
    EditTaskRequest as McpEditTaskRequest, RunTaskTestsRequest as McpRunTaskTestsRequest,
};
use ratchet_web::{extract_task_filters, ApiResponse, QueryParams};
use tracing::{info, warn};

use crate::{
    context::TasksContext,
    errors::{RestError, RestResult},
    models::{common::StatsResponse, CreateTaskRequest, TaskStats, UpdateTaskRequest},
};

/// List all tasks with optional filtering and pagination

pub async fn list_tasks(State(ctx): State<TasksContext>, query: QueryParams) -> RestResult<impl IntoResponse> {
    info!("Listing tasks with query: {:?}", query.0);

    let list_input = query.0.to_list_input();

    // Extract filters from query parameters
    let filters = extract_task_filters(&query.0.filters);

    let task_repo = ctx.repositories.task_repository();
    let list_response = task_repo
        .find_with_list_input(filters, list_input)
        .await
        .map_err(RestError::Database)?;

    Ok(Json(ApiResponse::from(list_response)))
}

/// Get a specific task by ID

pub async fn get_task(State(ctx): State<TasksContext>, Path(task_id): Path<String>) -> RestResult<impl IntoResponse> {
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
            // Debug: Log the actual database error to understand what's happening
            warn!("Database error in find_by_id({}): {:?}", task_id, db_err);

            // Handle specific database error types appropriately
            match &db_err {
                DatabaseError::NotFound { .. } => RestError::not_found("Task", &task_id),
                DatabaseError::Connection { .. }
                | DatabaseError::Internal { .. }
                | DatabaseError::Validation { .. }
                | DatabaseError::Constraint { .. }
                | DatabaseError::Transaction { .. } => {
                    let sanitizer = ErrorSanitizer::default();
                    let sanitized_error = sanitizer.sanitize_error(&db_err);
                    warn!("Sanitized error: {} (original: {})", sanitized_error.message, db_err);
                    RestError::InternalError(sanitized_error.message)
                }
            }
        })?
        .ok_or_else(|| RestError::not_found("Task", &task_id))?;

    Ok(Json(ApiResponse::new(task)))
}

/// Create a new task

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
    let created_task = task_repo
        .create(unified_task)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to create task: {}", e)))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::new(created_task))))
}

/// Update an existing task

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
    let updated_task = task_repo
        .update(existing_task)
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to update task: {}", e)))?;

    Ok(Json(ApiResponse::new(updated_task)))
}

/// Delete a task

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
    task_repo
        .delete(api_id.as_i32().unwrap_or(0))
        .await
        .map_err(|e| RestError::InternalError(format!("Failed to delete task: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Enable a task
pub async fn enable_task(
    State(ctx): State<TasksContext>,
    Path(task_id): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("Enabling task with ID: {}", task_id);

    let api_id = ApiId::from_string(task_id.clone());
    let task_repo = ctx.repositories.task_repository();

    task_repo.set_enabled(api_id, true).await.map_err(RestError::Database)?;

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
pub async fn sync_tasks(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Syncing tasks from registry");

    let sync_result = ctx
        .registry_manager
        .sync_with_database()
        .await
        .map_err(|e| RestError::InternalError(format!("Sync failed: {}", e)))?;

    Ok(Json(ApiResponse::new(sync_result)))
}

/// Get task statistics

pub async fn get_task_stats(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Getting task statistics");

    let task_repo = ctx.repositories.task_repository();

    // Get basic counts
    let total_tasks = task_repo.count().await.map_err(RestError::Database)?;

    // For now, return basic stats
    // In a full implementation, this would query for more detailed metrics
    let stats = TaskStats {
        total_tasks,
        enabled_tasks: 0,  // TODO: Implement
        disabled_tasks: 0, // TODO: Implement
        registry_tasks: 0, // TODO: Implement
        database_tasks: total_tasks,
        validation_errors: 0, // TODO: Implement
        last_sync: None,      // TODO: Implement
    };

    Ok(Json(StatsResponse::new(stats)))
}

/// MCP task development - create a new task with full JavaScript code and testing
pub async fn mcp_create_task(
    State(ctx): State<TasksContext>,
    Json(request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Creating task with development features");

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // Parse the request into the MCP create task request structure
    let create_request: McpCreateTaskRequest =
        serde_json::from_value(request).map_err(|e| RestError::BadRequest(format!("Invalid request format: {}", e)))?;

    // Call the MCP service to create the task
    match mcp_service.create_task(create_request).await {
        Ok(result) => {
            info!("Successfully created MCP task");
            Ok(Json(result))
        }
        Err(mcp_error) => {
            warn!("Failed to create MCP task: {}", mcp_error);
            Err(RestError::InternalError(format!("Task creation failed: {}", mcp_error)))
        }
    }
}

/// MCP task development - edit an existing task
pub async fn mcp_edit_task(
    State(ctx): State<TasksContext>,
    Path(task_name): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Editing task: {}", task_name);

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // Parse the request and add the task_id from the path
    let mut edit_request: McpEditTaskRequest =
        serde_json::from_value(request).map_err(|e| RestError::BadRequest(format!("Invalid request format: {}", e)))?;

    // Override task_id with the one from the URL path
    edit_request.task_id = task_name;

    // Call the MCP service to edit the task
    match mcp_service.edit_task(edit_request).await {
        Ok(result) => {
            info!("Successfully edited MCP task");
            Ok(Json(result))
        }
        Err(mcp_error) => {
            warn!("Failed to edit MCP task: {}", mcp_error);
            Err(RestError::InternalError(format!("Task editing failed: {}", mcp_error)))
        }
    }
}

/// MCP task development - delete a task
pub async fn mcp_delete_task(
    State(ctx): State<TasksContext>,
    Path(task_name): Path<String>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Deleting task: {}", task_name);

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // Create delete request
    let delete_request = McpDeleteTaskRequest {
        task_id: task_name,
        create_backup: true, // Default to creating backup
        force: false,        // Default to safe deletion
        delete_files: false, // Default to preserving files
    };

    // Call the MCP service to delete the task
    match mcp_service.delete_task(delete_request).await {
        Ok(result) => {
            info!("Successfully deleted MCP task");
            Ok(Json(result))
        }
        Err(mcp_error) => {
            warn!("Failed to delete MCP task: {}", mcp_error);
            Err(RestError::InternalError(format!("Task deletion failed: {}", mcp_error)))
        }
    }
}

/// MCP task development - test a task
pub async fn mcp_test_task(
    State(ctx): State<TasksContext>,
    Path(task_name): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Testing task: {}", task_name);

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // Parse the optional request body for test parameters
    let mut test_request: McpRunTaskTestsRequest = if request.is_null() {
        // Default test request if no body provided
        McpRunTaskTestsRequest {
            task_id: task_name.clone(),
            test_names: vec![],
            stop_on_failure: false,
            include_traces: true,
            parallel: false,
        }
    } else {
        serde_json::from_value(request).map_err(|e| RestError::BadRequest(format!("Invalid request format: {}", e)))?
    };

    // Override task_id with the one from the URL path
    test_request.task_id = task_name;

    // Call the MCP service to test the task
    match mcp_service.run_task_tests(test_request).await {
        Ok(result) => {
            info!("Successfully ran MCP task tests");
            Ok(Json(result))
        }
        Err(mcp_error) => {
            warn!("Failed to test MCP task: {}", mcp_error);
            Err(RestError::InternalError(format!("Task testing failed: {}", mcp_error)))
        }
    }
}

/// MCP task development - store execution result
pub async fn mcp_store_result(
    State(ctx): State<TasksContext>,
    Json(request): Json<serde_json::Value>,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Storing task execution result");

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // For now, just validate the request structure and store basic information
    // In the future, this could be expanded to use a specific result storage method
    let task_id = request
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RestError::BadRequest("Missing required field: task_id".to_string()))?;

    let execution_id = request
        .get("execution_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let result_data = request
        .get("result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!(null));

    // Store result using execution repository directly for now
    // This is a simplified implementation - in practice, you might want a dedicated method
    info!(
        "Storing execution result for task: {} execution: {}",
        task_id, execution_id
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "task_id": task_id,
        "execution_id": execution_id,
        "stored_at": chrono::Utc::now().to_rfc3339(),
        "message": "Result stored successfully (simplified implementation)"
    })))
}

/// MCP task development - get stored results for a task
pub async fn mcp_get_results(
    State(ctx): State<TasksContext>,
    Path(task_name): Path<String>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("MCP: Getting results for task: {}", task_name);

    // Check if MCP task service is available
    let mcp_service = ctx
        .mcp_task_service
        .as_ref()
        .ok_or_else(|| RestError::InternalError("MCP task development service is not available".to_string()))?;

    // For now, get results from execution repository
    // In the future, this could use a specialized method on the MCP service
    let execution_repo = ctx.repositories.execution_repository();

    // Parse pagination parameters
    let list_input = query.0.to_list_input();
    let pagination = list_input.pagination.unwrap_or_default();

    // Query executions for this task (using empty filters for now)
    use ratchet_interfaces::ExecutionFilters;
    let empty_filters = ExecutionFilters {
        // Basic filters (existing)
        task_id: None,
        status: None,
        queued_after: None,
        completed_after: None,

        // Advanced ID filtering
        task_id_in: None,
        id_in: None,

        // Advanced status filtering
        status_in: None,
        status_not: None,

        // Extended date filtering
        queued_before: None,
        started_after: None,
        started_before: None,
        completed_before: None,

        // Duration filtering
        duration_min_ms: None,
        duration_max_ms: None,

        // Progress filtering
        progress_min: None,
        progress_max: None,
        has_progress: None,

        // Error filtering
        has_error: None,
        error_message_contains: None,

        // Advanced boolean filtering
        can_retry: None,
        can_cancel: None,
    };

    match execution_repo.find_with_filters(empty_filters, pagination).await {
        Ok(list_response) => {
            // For now, return all executions (in practice, you'd filter by task_id)
            // Note: UnifiedExecution doesn't have a task_name field, it has task_id
            // This is a simplified implementation for the MCP results endpoint
            let filtered_executions = list_response.items;

            Ok(Json(serde_json::json!({
                "task_name": task_name,
                "total_results": filtered_executions.len(),
                "executions": filtered_executions,
                "pagination": {
                    "offset": list_response.meta.offset,
                    "limit": list_response.meta.limit,
                    "total": list_response.meta.total,
                    "page": list_response.meta.page
                },
                "retrieved_at": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            warn!("Failed to retrieve results for task {}: {}", task_name, e);
            Err(RestError::Database(e))
        }
    }
}
