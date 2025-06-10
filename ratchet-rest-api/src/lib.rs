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

pub mod handlers;
pub mod models;
pub mod context;
pub mod app;
pub mod errors;

// Re-export commonly used types
pub use app::{create_rest_app, AppConfig, AppContext};
pub use errors::{RestError, RestResult};
pub use models::*;