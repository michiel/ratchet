//! Worker monitoring endpoints

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use ratchet_web::{QueryParams, ApiResponse};
use tracing::info;

use crate::{
    context::TasksContext,
    errors::RestResult,
    models::{WorkersListResponse, WorkerStats, common::StatsResponse},
};

/// List all active workers
#[utoipa::path(
    get,
    path = "/workers",
    tag = "workers",
    operation_id = "listWorkers",
    params(
        ("page" = Option<u32>, Query, description = "Page number (0-based)"),
        ("limit" = Option<u32>, Query, description = "Number of items per page"),
        ("status" = Option<String>, Query, description = "Filter by worker status")
    ),
    responses(
        (status = 200, description = "List of workers retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_workers(
    State(_ctx): State<TasksContext>,
    query: QueryParams,
) -> RestResult<impl IntoResponse> {
    info!("Listing workers with query: {:?}", query.0);
    
    // For now, return a mock list of workers
    // In a full implementation, this would query the worker registry
    let workers = vec![
        serde_json::json!({
            "id": "worker-001",
            "status": "running",
            "task_count": 5,
            "current_task": "task-123",
            "uptime_seconds": 3600,
            "memory_usage_mb": 128,
            "cpu_usage_percent": 45.2,
            "last_heartbeat": chrono::Utc::now().to_rfc3339()
        }),
        serde_json::json!({
            "id": "worker-002", 
            "status": "idle",
            "task_count": 0,
            "current_task": null,
            "uptime_seconds": 7200,
            "memory_usage_mb": 64,
            "cpu_usage_percent": 12.1,
            "last_heartbeat": chrono::Utc::now().to_rfc3339()
        })
    ];
    
    let list_input = query.0.to_list_input();
    let pagination = list_input.pagination.unwrap_or_default();
    let response = WorkersListResponse {
        workers,
        total: 2,
        page: pagination.get_page(),
        limit: pagination.get_limit(),
    };
    
    Ok(Json(ApiResponse::new(response)))
}

/// Get worker statistics
#[utoipa::path(
    get,
    path = "/workers/stats",
    tag = "workers",
    operation_id = "getWorkerStats",
    responses(
        (status = 200, description = "Worker statistics retrieved successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_worker_stats(
    State(_ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
    info!("Getting worker statistics");
    
    // For now, return mock statistics
    // In a full implementation, this would aggregate from worker registry
    let stats = WorkerStats {
        total_workers: 2,
        active_workers: 1,
        idle_workers: 1,
        running_workers: 1,
        stopping_workers: 0,
        error_workers: 0,
        total_tasks: 5,
        average_uptime_seconds: Some(5400.0),
        total_memory_usage_mb: Some(192),
    };
    
    Ok(Json(ApiResponse::new(StatsResponse::new(stats))))
}