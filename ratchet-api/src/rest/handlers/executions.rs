//! Execution-related REST API handlers

use axum::{
    extract::{Path, Query},
    response::Json,
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    errors::{ApiError, ApiResult},
    pagination::{PaginationInput, ListResponse},
    types::{ApiId, UnifiedExecution},
};

/// Execution query parameters
#[derive(Debug, Deserialize)]
pub struct ExecutionQuery {
    #[serde(flatten)]
    pub pagination: PaginationInput,
    pub task_id: Option<ApiId>,
    pub status: Option<String>,
}

/// Create execution request
#[derive(Debug, Deserialize)]
pub struct CreateExecutionRequest {
    pub task_id: ApiId,
    pub input: serde_json::Value,
}

/// Update execution request
#[derive(Debug, Deserialize)]
pub struct UpdateExecutionRequest {
    pub status: Option<String>,
    pub output: Option<serde_json::Value>,
}

/// List executions handler
pub async fn list_executions(
    Query(query): Query<ExecutionQuery>,
) -> ApiResult<Json<ListResponse<UnifiedExecution>>> {
    // TODO: Implement execution listing from storage
    Err(ApiError::not_implemented("Execution listing"))
}

/// Get execution by ID handler
pub async fn get_execution(
    Path(id): Path<ApiId>,
) -> ApiResult<Json<UnifiedExecution>> {
    // TODO: Implement execution retrieval from storage
    Err(ApiError::not_implemented("Execution retrieval"))
}

/// Create execution handler
pub async fn create_execution(
    Json(request): Json<CreateExecutionRequest>,
) -> ApiResult<(StatusCode, Json<UnifiedExecution>)> {
    // TODO: Implement execution creation
    Err(ApiError::not_implemented("Execution creation"))
}

/// Update execution handler
pub async fn update_execution(
    Path(id): Path<ApiId>,
    Json(request): Json<UpdateExecutionRequest>,
) -> ApiResult<Json<UnifiedExecution>> {
    // TODO: Implement execution update
    Err(ApiError::not_implemented("Execution update"))
}

/// Delete execution handler
pub async fn delete_execution(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement execution deletion
    Err(ApiError::not_implemented("Execution deletion"))
}

/// Cancel execution handler
pub async fn cancel_execution(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement execution cancellation
    Err(ApiError::not_implemented("Execution cancellation"))
}

/// Get execution logs handler
pub async fn get_execution_logs(
    Path(id): Path<ApiId>,
) -> ApiResult<Json<Vec<String>>> {
    // TODO: Implement execution logs retrieval
    Err(ApiError::not_implemented("Execution logs"))
}