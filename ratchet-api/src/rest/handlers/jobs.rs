//! Job-related REST API handlers

use axum::{
    extract::{Path, Query},
    response::Json,
    http::StatusCode,
};
use serde::Deserialize;

use crate::{
    errors::{ApiError, ApiResult},
    pagination::{PaginationInput, ListResponse},
    types::{ApiId, UnifiedJob},
};

/// Job query parameters
#[derive(Debug, Deserialize)]
pub struct JobQuery {
    #[serde(flatten)]
    pub pagination: PaginationInput,
    pub status: Option<String>,
    pub priority: Option<i32>,
}

/// Create job request
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub task_id: ApiId,
    pub input: serde_json::Value,
    pub priority: Option<i32>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Update job request
#[derive(Debug, Deserialize)]
pub struct UpdateJobRequest {
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// List jobs handler
pub async fn list_jobs(
    Query(query): Query<JobQuery>,
) -> ApiResult<Json<ListResponse<UnifiedJob>>> {
    // TODO: Implement job listing from storage
    Err(ApiError::not_implemented("Job listing"))
}

/// Get job by ID handler
pub async fn get_job(
    Path(id): Path<ApiId>,
) -> ApiResult<Json<UnifiedJob>> {
    // TODO: Implement job retrieval from storage
    Err(ApiError::not_implemented("Job retrieval"))
}

/// Create job handler
pub async fn create_job(
    Json(request): Json<CreateJobRequest>,
) -> ApiResult<(StatusCode, Json<UnifiedJob>)> {
    // TODO: Implement job creation
    Err(ApiError::not_implemented("Job creation"))
}

/// Update job handler
pub async fn update_job(
    Path(id): Path<ApiId>,
    Json(request): Json<UpdateJobRequest>,
) -> ApiResult<Json<UnifiedJob>> {
    // TODO: Implement job update
    Err(ApiError::not_implemented("Job update"))
}

/// Delete job handler
pub async fn delete_job(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement job deletion
    Err(ApiError::not_implemented("Job deletion"))
}

/// Cancel job handler
pub async fn cancel_job(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement job cancellation
    Err(ApiError::not_implemented("Job cancellation"))
}

/// Retry job handler
pub async fn retry_job(
    Path(id): Path<ApiId>,
) -> ApiResult<(StatusCode, Json<UnifiedJob>)> {
    // TODO: Implement job retry
    Err(ApiError::not_implemented("Job retry"))
}

/// Pause job handler
pub async fn pause_job(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement job pause
    Err(ApiError::not_implemented("Job pause"))
}

/// Resume job handler
pub async fn resume_job(
    Path(id): Path<ApiId>,
) -> ApiResult<StatusCode> {
    // TODO: Implement job resume
    Err(ApiError::not_implemented("Job resume"))
}