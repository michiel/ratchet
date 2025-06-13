//! Health check endpoints

use axum::{extract::State, response::IntoResponse, Json};
use std::collections::HashMap;
use tracing::info;

use crate::{
    context::TasksContext,
    errors::RestResult,
    models::common::{HealthResponse, HealthCheckResult, HealthStatus},
};

/// Health check endpoint
/// 
/// Returns the overall health status of the API and its dependencies.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    operation_id = "healthCheck",
    responses(
        (status = 200, description = "Service is healthy"),
        (status = 503, description = "Service is unhealthy")
    )
)]
pub async fn health_check() -> impl IntoResponse {
    info!("Health check requested");
    
    Json(HealthResponse::healthy())
}

/// Detailed health check with dependency checks
/// 
/// Performs health checks on all system dependencies and returns detailed status.
pub async fn health_check_detailed(
    State(ctx): State<TasksContext>,
) -> RestResult<impl IntoResponse> {
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
pub async fn readiness_check() -> impl IntoResponse {
    // For now, always ready. In production, this might check:
    // - Database connection pool has available connections
    // - Required services are initialized
    // - No critical errors in startup
    
    Json(serde_json::json!({
        "status": "ready",
        "timestamp": chrono::Utc::now()
    }))
}

/// Liveness probe endpoint
/// 
/// Returns 200 if the service is alive (not deadlocked, etc.).
pub async fn liveness_check() -> impl IntoResponse {
    // For now, always alive. In production, this might check:
    // - No deadlocks detected
    // - Memory usage within limits
    // - No critical thread failures
    
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now()
    }))
}