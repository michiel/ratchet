use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;
use std::sync::Arc;

use crate::{
    rest::{
        middleware::{RestError, WithPaginationHeaders},
        models::{
            common::{ApiResponse, ApiError},
            tasks::{TaskResponse, TaskDetailResponse, TaskUpdateRequest, TaskFilters},
        },
        extractors::ListQueryExtractor,
    },
    services::TaskSyncService,
    registry::TaskRegistry,
};

/// REST API context for tasks
#[derive(Clone)]
pub struct TasksContext {
    pub sync_service: Option<Arc<TaskSyncService>>,
    pub registry: Option<Arc<TaskRegistry>>,
}

/// GET /api/v1/tasks - List all tasks
pub async fn list_tasks(
    State(ctx): State<TasksContext>,
    ListQueryExtractor(query): ListQueryExtractor,
) -> Result<impl IntoResponse, RestError> {
    let sync_service = ctx.sync_service
        .ok_or_else(|| RestError::InternalError("Task sync service not available".to_string()))?;

    // Get all tasks from unified service
    let all_tasks = sync_service.list_all_tasks().await
        .map_err(|e| RestError::InternalError(format!("Failed to list tasks: {}", e)))?;

    // Parse filters from query
    let filters = TaskFilters {
        uuid: query.filter.get_filter("uuid").cloned(),
        label: query.filter.get_filter("label").cloned(),
        version: query.filter.get_filter("version").cloned(),
        enabled: query.filter.get_filter("enabled")
            .and_then(|s| s.parse().ok()),
        registry_source: query.filter.get_filter("registrySource")
            .and_then(|s| s.parse().ok()),
        label_like: query.filter.get_like_filter("label").cloned(),
    };

    // Apply filters
    let filtered_tasks: Vec<_> = all_tasks
        .into_iter()
        .filter(|task| {
            filters.matches_uuid(&task.uuid) &&
            filters.matches_label(&task.label) &&
            filters.matches_version(&task.version) &&
            filters.matches_enabled(task.enabled) &&
            filters.matches_registry_source(task.registry_source)
        })
        .collect();

    let total = filtered_tasks.len() as u64;

    // Apply sorting
    let mut sorted_tasks = filtered_tasks;
    if let Some(sort_field) = query.sort.sort_field() {
        match sort_field {
            "label" => {
                sorted_tasks.sort_by(|a, b| match query.sort.sort_direction() {
                    crate::rest::models::common::SortDirection::Asc => a.label.cmp(&b.label),
                    crate::rest::models::common::SortDirection::Desc => b.label.cmp(&a.label),
                });
            },
            "version" => {
                sorted_tasks.sort_by(|a, b| match query.sort.sort_direction() {
                    crate::rest::models::common::SortDirection::Asc => a.version.cmp(&b.version),
                    crate::rest::models::common::SortDirection::Desc => b.version.cmp(&a.version),
                });
            },
            "createdAt" => {
                sorted_tasks.sort_by(|a, b| match query.sort.sort_direction() {
                    crate::rest::models::common::SortDirection::Asc => a.created_at.cmp(&b.created_at),
                    crate::rest::models::common::SortDirection::Desc => b.created_at.cmp(&a.created_at),
                });
            },
            "updatedAt" => {
                sorted_tasks.sort_by(|a, b| match query.sort.sort_direction() {
                    crate::rest::models::common::SortDirection::Asc => a.updated_at.cmp(&b.updated_at),
                    crate::rest::models::common::SortDirection::Desc => b.updated_at.cmp(&a.updated_at),
                });
            },
            _ => {
                // Default sort by UUID
                sorted_tasks.sort_by(|a, b| match query.sort.sort_direction() {
                    crate::rest::models::common::SortDirection::Asc => a.uuid.cmp(&b.uuid),
                    crate::rest::models::common::SortDirection::Desc => b.uuid.cmp(&a.uuid),
                });
            },
        }
    }

    // Apply pagination
    let offset = query.pagination.offset();
    let limit = query.pagination.limit();
    let paginated_tasks: Vec<TaskResponse> = sorted_tasks
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(TaskResponse::from)
        .collect();

    let response = Json(ApiResponse::new(paginated_tasks));
    Ok(response.with_pagination_headers(total, offset, limit, "tasks"))
}

/// GET /api/v1/tasks/{id} - Get a specific task
pub async fn get_task(
    State(ctx): State<TasksContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {
    let sync_service = ctx.sync_service
        .ok_or_else(|| RestError::InternalError("Task sync service not available".to_string()))?;

    // Parse UUID
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| RestError::BadRequest("Invalid task ID format".to_string()))?;

    // Get task from unified service
    let unified_task = sync_service.get_task(uuid, None).await
        .map_err(|e| RestError::InternalError(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| RestError::NotFound("Task not found".to_string()))?;

    // Try to get detailed information from registry if available
    let mut response = TaskDetailResponse {
        task: TaskResponse::from(unified_task.clone()),
        input_schema: None,
        output_schema: None,
    };

    if let Some(registry) = &ctx.registry {
        if let Ok(Some(registry_task)) = registry.get_task(uuid, None).await {
            response.input_schema = Some(registry_task.input_schema.clone());
            response.output_schema = Some(registry_task.output_schema.clone());
        }
    }

    Ok(Json(ApiResponse::new(response)))
}

/// PATCH /api/v1/tasks/{id} - Update a task (limited fields)
pub async fn update_task(
    State(ctx): State<TasksContext>,
    Path(id): Path<String>,
    Json(update): Json<TaskUpdateRequest>,
) -> Result<axum::Json<ApiError>, RestError> {
    let _sync_service = ctx.sync_service
        .ok_or_else(|| RestError::InternalError("Task sync service not available".to_string()))?;

    // Parse UUID
    let _uuid = Uuid::parse_str(&id)
        .map_err(|_| RestError::BadRequest("Invalid task ID format".to_string()))?;

    // For now, we only support enabling/disabling tasks
    // In a full implementation, this would update the database record
    if update.enabled.is_some() {
        return Err(RestError::MethodNotAllowed(
            "Task modification not yet implemented. Tasks are managed through the registry system.".to_string()
        ));
    }

    return Err(RestError::BadRequest("No valid updates provided".to_string()));
}

/// POST /api/v1/tasks - Create a task (not supported for registry tasks)
pub async fn create_task() -> Result<axum::Json<ApiError>, RestError> {
    Err(RestError::MethodNotAllowed(
        "Tasks are managed through the registry system. Use the file system or registry sources to add tasks.".to_string()
    ))
}

/// DELETE /api/v1/tasks/{id} - Delete a task (not supported for registry tasks)
pub async fn delete_task(Path(_id): Path<String>) -> Result<axum::Json<ApiError>, RestError> {
    Err(RestError::MethodNotAllowed(
        "Tasks are managed through the registry system. Remove tasks from the source registry.".to_string()
    ))
}