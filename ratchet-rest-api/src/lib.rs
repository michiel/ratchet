//! # Ratchet REST API
//!
//! REST API implementation for the Ratchet task execution system. This crate provides
//! HTTP endpoints for managing tasks, executions, jobs, schedules, and workers using
//! dependency injection and clean interface segregation.
//!
//! ## Features
//!
//! - **Task Management**: CRUD operations and task registry integration
//! - **Execution Tracking**: Monitor and control task executions
//! - **Job Queue**: Manage queued tasks with priority and retry logic
//! - **Scheduling**: Cron-based task scheduling with monitoring
//! - **Worker Status**: Real-time worker monitoring and health checks
//! - **OpenAPI Documentation**: Interactive Swagger UI with comprehensive API docs
//!
//! ## Architecture
//!
//! The API uses dependency injection through context structs that implement
//! the repository and service traits from `ratchet-interfaces`. This enables
//! clean testing with mock implementations and flexibility in backend choices.
//!
//! ## Example
//!
//! ```rust,no_run
//! use axum::Router;
//! use ratchet_rest_api::{create_rest_app, AppConfig};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create your repository implementations
//! // let repositories = ...;
//!
//! // Configure the application
//! // let config = AppConfig::default();
//!
//! // Create the REST API router
//! // let app = create_rest_app(repositories, config).await?;
//!
//! // Serve the application
//! // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//! // axum::serve(listener, app).await?;
//! # Ok(())
//! # }
//! ```

pub mod app;
pub mod context;
pub mod errors;
pub mod handlers;
pub mod models;

// Re-export commonly used types
pub use app::{create_rest_app, AppConfig, AppContext};
pub use errors::{RestError, RestResult};
pub use models::*;

// OpenAPI Documentation - temporarily disabled due to ToSchema compilation issues
// use utoipa::OpenApi;

// /// OpenAPI 3.0 specification for the Ratchet REST API
// #[derive(OpenApi)]
// #[openapi(
//     info(
//         title = "Ratchet Task Execution API",
//         description = "Comprehensive REST API for managing and executing tasks in the Ratchet system. Provides full CRUD operations for tasks, executions, jobs, schedules, and workers with real-time monitoring capabilities.",
//         version = "1.0.0",
//         contact(
//             name = "Ratchet API Support",
//             email = "api-support@ratchet.dev"
//         ),
//         license(
//             name = "MIT",
//             url = "https://opensource.org/licenses/MIT"
//         )
//     ),
//     servers(
//         (url = "http://localhost:8080/api/v1", description = "Development server"),
//         (url = "https://api.ratchet.dev/v1", description = "Production server")
//     ),
//     paths(
//         // Task management endpoints
//         handlers::tasks::list_tasks,
//         handlers::tasks::get_task,
//         handlers::tasks::create_task,
//         handlers::tasks::update_task,
//         handlers::tasks::get_task_stats,

//         // Execution management endpoints
//         handlers::executions::list_executions,
//         handlers::executions::get_execution,
//         handlers::executions::create_execution,
//         handlers::executions::update_execution,
//         handlers::executions::cancel_execution,
//         handlers::executions::retry_execution,
//         handlers::executions::get_execution_logs,
//         handlers::executions::get_execution_stats,

//         // Job management endpoints
//         handlers::jobs::list_jobs,
//         handlers::jobs::get_job,
//         handlers::jobs::create_job,
//         handlers::jobs::update_job,
//         handlers::jobs::cancel_job,
//         handlers::jobs::retry_job,
//         handlers::jobs::get_job_stats,

//         // Schedule management endpoints
//         handlers::schedules::list_schedules,
//         handlers::schedules::get_schedule,
//         handlers::schedules::create_schedule,
//         handlers::schedules::update_schedule,
//         handlers::schedules::delete_schedule,
//         handlers::schedules::enable_schedule,
//         handlers::schedules::disable_schedule,
//         handlers::schedules::trigger_schedule,
//         handlers::schedules::get_schedule_stats,

//         // Health check
//         handlers::health::health_check,

//         // Monitoring and metrics
//         handlers::metrics::get_metrics,
//         handlers::metrics::get_prometheus_metrics,
//     ),
//     components(
//         schemas(
//             // Task request/response models
//             models::tasks::CreateTaskRequest,
//             models::tasks::UpdateTaskRequest,
//             models::tasks::ValidateTaskRequest,
//             models::tasks::ValidateTaskResponse,
//             models::tasks::ValidationErrorDetail,
//             models::tasks::ValidationWarningDetail,
//             models::tasks::SyncTasksResponse,
//             models::tasks::TaskSyncError,
//             models::tasks::TaskStats,

//             // Execution request/response models
//             models::executions::CreateExecutionRequest,
//             models::executions::UpdateExecutionRequest,
//             models::executions::RetryExecutionRequest,
//             models::executions::ExecutionStats,

//             // Job request/response models
//             models::jobs::CreateJobRequest,
//             models::jobs::UpdateJobRequest,
//             models::jobs::JobStats,

//             // Schedule request/response models
//             // models::schedules::CreateScheduleRequest, // Temporarily commented due to ToSchema issues with UnifiedOutputDestination
//             // models::schedules::UpdateScheduleRequest, // Temporarily commented due to ToSchema issues with UnifiedOutputDestination
//             models::schedules::ScheduleStats,

//             // Domain types from ratchet-api-types
//             ratchet_api_types::UnifiedOutputDestination,
//             ratchet_api_types::UnifiedWebhookConfig,
//             ratchet_api_types::UnifiedRetryPolicy,
//             ratchet_api_types::UnifiedWebhookAuth,
//             ratchet_api_types::UnifiedBearerAuth,
//             ratchet_api_types::UnifiedBasicAuth,
//             ratchet_api_types::UnifiedApiKeyAuth,
//             ratchet_api_types::UnifiedFilesystemConfig,

//             // Enum types from ratchet-api-types
//             ratchet_api_types::OutputFormat,
//             ratchet_api_types::CompressionType,
//             ratchet_api_types::HttpMethod,

//             // Metrics and monitoring models
//             handlers::metrics::SystemMetrics,
//             handlers::metrics::SystemInfo,
//             handlers::metrics::PerformanceMetrics,
//             handlers::metrics::ResourceMetrics,
//             handlers::metrics::ApplicationMetrics,
//             handlers::metrics::DatabaseMetrics,
//             handlers::metrics::TaskMetrics,
//             handlers::metrics::ExecutionMetrics,
//             handlers::metrics::JobMetrics,
//             handlers::metrics::ScheduleMetrics,
//         )
//     ),
//     tags(
//         (name = "tasks", description = "Task management operations"),
//         (name = "executions", description = "Task execution monitoring and control"),
//         (name = "jobs", description = "Job queue management"),
//         (name = "schedules", description = "Task scheduling operations"),
//         (name = "workers", description = "Worker monitoring and management"),
//         (name = "mcp", description = "MCP (Model Context Protocol) development tools"),
//         (name = "health", description = "System health and monitoring"),
//         (name = "monitoring", description = "System metrics and observability")
//     )
// )]
// pub struct ApiDoc;

// /// Create OpenAPI specification as JSON
// pub fn openapi_spec() -> utoipa::openapi::OpenApi {
//     ApiDoc::openapi()
// }
