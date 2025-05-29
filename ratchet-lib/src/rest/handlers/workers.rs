use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::{
    rest::{
        middleware::error_handler::RestError,
        models::{
            common::ApiResponse,
            workers::{WorkerResponse, WorkerDetailResponse, WorkerPoolStats, WorkerStatus},
        },
    },
};

#[derive(Clone)]
pub struct WorkersContext {
    // TODO: Add worker tracking when implemented
}

pub async fn list_workers(
    State(_ctx): State<WorkersContext>,
) -> Result<impl IntoResponse, RestError> {

    // For now, return mock data since worker tracking isn't fully implemented
    // TODO: Implement actual worker tracking in WorkerPool
    let workers = vec![
        WorkerResponse {
            id: "worker-1".to_string(),
            status: WorkerStatus::Idle,
            current_task: None,
            current_execution_id: None,
            started_at: chrono::Utc::now() - chrono::Duration::hours(1),
            last_heartbeat: chrono::Utc::now(),
            tasks_completed: 42,
            tasks_failed: 2,
        },
        WorkerResponse {
            id: "worker-2".to_string(),
            status: WorkerStatus::Running,
            current_task: Some("process-data".to_string()),
            current_execution_id: Some("exec-123".to_string()),
            started_at: chrono::Utc::now() - chrono::Duration::hours(2),
            last_heartbeat: chrono::Utc::now(),
            tasks_completed: 38,
            tasks_failed: 1,
        },
    ];

    Ok(Json(ApiResponse { data: workers }))
}

pub async fn get_worker(
    State(_ctx): State<WorkersContext>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, RestError> {

    // For now, return mock data
    // TODO: Implement actual worker tracking
    if id == "worker-1" || id == "worker-2" {
        let response = WorkerDetailResponse {
            id: id.clone(),
            status: if id == "worker-1" { WorkerStatus::Idle } else { WorkerStatus::Running },
            current_task: if id == "worker-2" { Some("process-data".to_string()) } else { None },
            current_task_name: if id == "worker-2" { Some("Process Customer Data".to_string()) } else { None },
            current_execution_id: if id == "worker-2" { Some("exec-123".to_string()) } else { None },
            started_at: chrono::Utc::now() - chrono::Duration::hours(if id == "worker-1" { 1 } else { 2 }),
            last_heartbeat: chrono::Utc::now(),
            tasks_completed: if id == "worker-1" { 42 } else { 38 },
            tasks_failed: if id == "worker-1" { 2 } else { 1 },
            memory_usage_mb: Some(256.5),
            cpu_usage_percent: Some(if id == "worker-2" { 75.2 } else { 2.1 }),
        };
        Ok(Json(response))
    } else {
        Err(RestError::NotFound("Worker not found".to_string()))
    }
}

pub async fn get_worker_pool_stats(
    State(_ctx): State<WorkersContext>,
) -> Result<impl IntoResponse, RestError> {

    // For now, return mock data
    // TODO: Implement actual statistics tracking
    let stats = WorkerPoolStats {
        total_workers: 2,
        idle_workers: 1,
        running_workers: 1,
        total_tasks_completed: 80,
        total_tasks_failed: 3,
        average_task_duration_ms: Some(1250.5),
    };

    Ok(Json(stats))
}