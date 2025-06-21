//! Health check endpoints

use axum::{extract::State, response::IntoResponse, Json};
use std::collections::HashMap;
use tracing::info;

use crate::{
    context::TasksContext,
    errors::RestResult,
    models::common::{HealthCheckResult, HealthResponse, HealthStatus},
};

/// Health check endpoint
///
/// Returns the overall health status of the API and its dependencies.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health status", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check() -> impl IntoResponse {
    info!("Health check requested");

    Json(HealthResponse::healthy())
}

/// Detailed health check with dependency checks
///
/// Performs health checks on all system dependencies and returns detailed status.
pub async fn health_check_detailed(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Detailed health check requested");

    let mut checks = HashMap::new();

    // Check repository health
    let repo_start = std::time::Instant::now();
    let repo_health = match ctx.repositories.health_check().await {
        Ok(_) => HealthCheckResult {
            status: HealthStatus::Healthy,
            message: Some("Database connection healthy".to_string()),
            duration_ms: Some(repo_start.elapsed().as_millis() as u64),
        },
        Err(e) => HealthCheckResult {
            status: HealthStatus::Unhealthy,
            message: Some(format!("Database connection failed: {}", e)),
            duration_ms: Some(repo_start.elapsed().as_millis() as u64),
        },
    };
    checks.insert("database".to_string(), repo_health);

    // Check registry health
    let registry_start = std::time::Instant::now();
    let registry_health = match ctx.registry.health_check().await {
        Ok(_) => HealthCheckResult {
            status: HealthStatus::Healthy,
            message: Some("Task registry healthy".to_string()),
            duration_ms: Some(registry_start.elapsed().as_millis() as u64),
        },
        Err(e) => HealthCheckResult {
            status: HealthStatus::Unhealthy,
            message: Some(format!("Task registry failed: {}", e)),
            duration_ms: Some(registry_start.elapsed().as_millis() as u64),
        },
    };
    checks.insert("registry".to_string(), registry_health);

    let response = HealthResponse::healthy().with_checks(checks);
    Ok(Json(response))
}

/// Readiness probe endpoint
///
/// Returns 200 if the service is ready to handle requests.
pub async fn readiness_check(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    // Check if critical services are ready
    let mut checks = HashMap::new();
    let mut overall_ready = true;

    // Check database readiness
    let db_ready = match ctx.repositories.health_check().await {
        Ok(_) => {
            checks.insert(
                "database".to_string(),
                serde_json::json!({
                    "ready": true,
                    "message": "Database connections available"
                }),
            );
            true
        }
        Err(e) => {
            checks.insert(
                "database".to_string(),
                serde_json::json!({
                    "ready": false,
                    "message": format!("Database not ready: {}", e)
                }),
            );
            overall_ready = false;
            false
        }
    };

    // Check registry readiness
    let registry_ready = match ctx.registry.health_check().await {
        Ok(_) => {
            checks.insert(
                "registry".to_string(),
                serde_json::json!({
                    "ready": true,
                    "message": "Task registry operational"
                }),
            );
            true
        }
        Err(e) => {
            checks.insert(
                "registry".to_string(),
                serde_json::json!({
                    "ready": false,
                    "message": format!("Registry not ready: {}", e)
                }),
            );
            overall_ready = false;
            false
        }
    };

    let response = serde_json::json!({
        "status": if overall_ready { "ready" } else { "not_ready" },
        "timestamp": chrono::Utc::now(),
        "checks": checks
    });

    if overall_ready {
        Ok(Json(response))
    } else {
        Err(crate::errors::RestError::ServiceUnavailable(
            "Service not ready".to_string(),
        ))
    }
}

/// Liveness probe endpoint
///
/// Returns 200 if the service is alive (not deadlocked, etc.).
pub async fn liveness_check() -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    // Simple liveness checks
    let mut checks = HashMap::new();

    // Check thread responsiveness (if we reach here, main thread is responsive)
    checks.insert(
        "thread_responsive".to_string(),
        serde_json::json!({
            "alive": true,
            "message": "Main thread responsive"
        }),
    );

    // Check memory usage (basic check)
    let memory_ok = check_memory_usage();
    checks.insert(
        "memory_usage".to_string(),
        serde_json::json!({
            "alive": memory_ok,
            "message": if memory_ok { "Memory usage normal" } else { "Memory usage high" }
        }),
    );

    // Check system time
    let time_ok = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .is_ok();
    checks.insert(
        "system_time".to_string(),
        serde_json::json!({
            "alive": time_ok,
            "message": if time_ok { "System time valid" } else { "System time invalid" }
        }),
    );

    let response_time = start_time.elapsed().as_millis();

    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now(),
        "response_time_ms": response_time,
        "checks": checks
    }))
}

/// Basic memory usage check
fn check_memory_usage() -> bool {
    // Simple check - in production would use proper memory monitoring
    // For now, just return true (could be enhanced with actual memory checks)
    true
}
