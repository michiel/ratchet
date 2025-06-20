//! Metrics collection and monitoring endpoints

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::info;
// use utoipa::ToSchema; // temporarily disabled

use crate::{context::TasksContext, errors::RestResult, models::common::StatsResponse};

/// System metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    // System information
    pub system_info: SystemInfo,
    // Performance metrics
    pub performance: PerformanceMetrics,
    // Resource utilization
    pub resources: ResourceMetrics,
    // Application metrics
    pub application: ApplicationMetrics,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub version: String,
    pub build_timestamp: String,
    pub rust_version: String,
    pub target_triple: String,
    pub uptime_seconds: u64,
    pub git_commit: Option<String>,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub success_rate_percent: f64,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetrics {
    pub memory_usage_mb: u64,
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub heap_size_mb: u64,
    pub heap_used_mb: u64,
    pub gc_count: u64,
    pub thread_count: u32,
    pub file_descriptors: u32,
}

/// Application-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationMetrics {
    pub database: DatabaseMetrics,
    pub tasks: TaskMetrics,
    pub executions: ExecutionMetrics,
    pub jobs: JobMetrics,
    pub schedules: ScheduleMetrics,
}

/// Database metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseMetrics {
    pub connection_pool_size: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub total_queries: u64,
    pub slow_queries: u64,
    pub average_query_time_ms: f64,
    pub connection_errors: u64,
}

/// Task metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetrics {
    pub total_tasks: u64,
    pub enabled_tasks: u64,
    pub validated_tasks: u64,
    pub tasks_with_errors: u64,
    pub registry_sync_count: u64,
    pub last_sync_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionMetrics {
    pub total_executions: u64,
    pub running_executions: u64,
    pub completed_executions: u64,
    pub failed_executions: u64,
    pub cancelled_executions: u64,
    pub average_execution_time_ms: f64,
    pub success_rate_percent: f64,
}

/// Job metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobMetrics {
    pub total_jobs: u64,
    pub pending_jobs: u64,
    pub processing_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub average_queue_time_ms: f64,
    pub retry_count: u64,
}

/// Schedule metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMetrics {
    pub total_schedules: u64,
    pub enabled_schedules: u64,
    pub overdue_schedules: u64,
    pub next_execution_in_seconds: Option<i64>,
    pub successful_triggers: u64,
    pub failed_triggers: u64,
}

/// Get comprehensive system metrics
///
/// Returns detailed system and application metrics for monitoring and observability.

pub async fn get_metrics(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Metrics collection requested");

    let start_time = std::time::Instant::now();

    // Collect system information
    let system_info = collect_system_info();

    // Collect performance metrics (placeholder for now)
    let performance = collect_performance_metrics();

    // Collect resource metrics
    let resources = collect_resource_metrics();

    // Collect application metrics
    let application = collect_application_metrics(&ctx).await;

    let metrics = SystemMetrics {
        system_info,
        performance,
        resources,
        application,
    };

    let collection_time = start_time.elapsed().as_millis();
    tracing::debug!("Metrics collection completed in {}ms", collection_time);

    Ok(Json(StatsResponse::new(metrics)))
}

/// Get Prometheus-formatted metrics
///
/// Returns metrics in Prometheus exposition format for integration with monitoring systems.

pub async fn get_prometheus_metrics(State(ctx): State<TasksContext>) -> RestResult<impl IntoResponse> {
    info!("Prometheus metrics requested");

    let metrics = collect_application_metrics(&ctx).await;
    let prometheus_output = format_prometheus_metrics(&metrics);

    Ok(axum::response::Response::builder()
        .header("content-type", "text/plain; version=0.0.4")
        .body(prometheus_output)
        .unwrap())
}

// Helper functions for metrics collection

fn collect_system_info() -> SystemInfo {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_timestamp: chrono::Utc::now().to_rfc3339(),
        rust_version: option_env!("RUSTC_VERSION").unwrap_or("unknown").to_string(),
        target_triple: std::env::consts::ARCH.to_string(),
        uptime_seconds: uptime,
        git_commit: None, // TODO: Add git information through build script
    }
}

fn collect_performance_metrics() -> PerformanceMetrics {
    // TODO: Implement actual performance tracking
    // For now, return placeholder metrics
    PerformanceMetrics {
        requests_per_second: 0.0,
        average_response_time_ms: 0.0,
        p95_response_time_ms: 0.0,
        p99_response_time_ms: 0.0,
        error_rate_percent: 0.0,
        success_rate_percent: 100.0,
    }
}

fn collect_resource_metrics() -> ResourceMetrics {
    // TODO: Implement actual resource monitoring
    // For now, return placeholder metrics
    ResourceMetrics {
        memory_usage_mb: 0,
        memory_usage_percent: 0.0,
        cpu_usage_percent: 0.0,
        heap_size_mb: 0,
        heap_used_mb: 0,
        gc_count: 0,
        thread_count: 0,
        file_descriptors: 0,
    }
}

async fn collect_application_metrics(ctx: &TasksContext) -> ApplicationMetrics {
    // Collect database metrics
    let database = collect_database_metrics(ctx).await;

    // Collect application-specific metrics
    let tasks = collect_task_metrics(ctx).await;
    let executions = collect_execution_metrics(ctx).await;
    let jobs = collect_job_metrics(ctx).await;
    let schedules = collect_schedule_metrics(ctx).await;

    ApplicationMetrics {
        database,
        tasks,
        executions,
        jobs,
        schedules,
    }
}

async fn collect_database_metrics(_ctx: &TasksContext) -> DatabaseMetrics {
    // TODO: Implement actual database metrics collection
    DatabaseMetrics {
        connection_pool_size: 10,
        active_connections: 2,
        idle_connections: 8,
        total_queries: 0,
        slow_queries: 0,
        average_query_time_ms: 0.0,
        connection_errors: 0,
    }
}

async fn collect_task_metrics(ctx: &TasksContext) -> TaskMetrics {
    let total_tasks = ctx.repositories.task_repository().count().await.unwrap_or(0);

    // TODO: Collect more detailed task metrics
    TaskMetrics {
        total_tasks,
        enabled_tasks: 0,
        validated_tasks: 0,
        tasks_with_errors: 0,
        registry_sync_count: 0,
        last_sync_timestamp: None,
    }
}

async fn collect_execution_metrics(ctx: &TasksContext) -> ExecutionMetrics {
    let total_executions = ctx.repositories.execution_repository().count().await.unwrap_or(0);

    // TODO: Collect more detailed execution metrics
    ExecutionMetrics {
        total_executions,
        running_executions: 0,
        completed_executions: 0,
        failed_executions: 0,
        cancelled_executions: 0,
        average_execution_time_ms: 0.0,
        success_rate_percent: 0.0,
    }
}

async fn collect_job_metrics(ctx: &TasksContext) -> JobMetrics {
    let total_jobs = ctx.repositories.job_repository().count().await.unwrap_or(0);

    // TODO: Collect more detailed job metrics
    JobMetrics {
        total_jobs,
        pending_jobs: 0,
        processing_jobs: 0,
        completed_jobs: 0,
        failed_jobs: 0,
        average_queue_time_ms: 0.0,
        retry_count: 0,
    }
}

async fn collect_schedule_metrics(ctx: &TasksContext) -> ScheduleMetrics {
    let total_schedules = ctx.repositories.schedule_repository().count().await.unwrap_or(0);

    // TODO: Collect more detailed schedule metrics
    ScheduleMetrics {
        total_schedules,
        enabled_schedules: 0,
        overdue_schedules: 0,
        next_execution_in_seconds: None,
        successful_triggers: 0,
        failed_triggers: 0,
    }
}

fn format_prometheus_metrics(metrics: &ApplicationMetrics) -> String {
    let mut output = String::new();

    // Add help and type annotations
    output.push_str("# HELP ratchet_tasks_total Total number of tasks\n");
    output.push_str("# TYPE ratchet_tasks_total gauge\n");
    output.push_str(&format!("ratchet_tasks_total {}\n", metrics.tasks.total_tasks));

    output.push_str("# HELP ratchet_executions_total Total number of executions\n");
    output.push_str("# TYPE ratchet_executions_total gauge\n");
    output.push_str(&format!(
        "ratchet_executions_total {}\n",
        metrics.executions.total_executions
    ));

    output.push_str("# HELP ratchet_jobs_total Total number of jobs\n");
    output.push_str("# TYPE ratchet_jobs_total gauge\n");
    output.push_str(&format!("ratchet_jobs_total {}\n", metrics.jobs.total_jobs));

    output.push_str("# HELP ratchet_schedules_total Total number of schedules\n");
    output.push_str("# TYPE ratchet_schedules_total gauge\n");
    output.push_str(&format!(
        "ratchet_schedules_total {}\n",
        metrics.schedules.total_schedules
    ));

    output.push_str("# HELP ratchet_database_connections_active Active database connections\n");
    output.push_str("# TYPE ratchet_database_connections_active gauge\n");
    output.push_str(&format!(
        "ratchet_database_connections_active {}\n",
        metrics.database.active_connections
    ));

    output
}
