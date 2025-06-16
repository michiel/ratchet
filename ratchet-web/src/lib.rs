//! # Ratchet Web Utilities
//!
//! Reusable web middleware and utilities for building robust HTTP APIs with Axum.
//! This crate provides common functionality needed for REST APIs including
//! error handling, rate limiting, CORS, request tracing, and parameter extraction.
//!
//! ## Features
//!
//! - **Middleware**: CORS, error handling, rate limiting, request IDs, pagination
//! - **Extractors**: Query parameter extraction with validation and filtering
//! - **Utilities**: Response helpers, error conversion, and common patterns
//!
//! ## Example
//!
//! ```rust,no_run
//! use axum::{Router, routing::get};
//! use ratchet_web::{
//!     middleware::{cors_layer, request_id_layer, error_handler_layer},
//!     extractors::QueryParams,
//! };
//!
//! async fn list_items(_query: QueryParams) -> &'static str {
//!     "items"
//! }
//!
//! # #[tokio::main]
//! # async fn main() {
//! let app: Router = Router::new()
//!     .route("/items", get(list_items))
//!     .layer(error_handler_layer())
//!     .layer(request_id_layer())
//!     .layer(cors_layer());
//! 
//! // Start the server (axum 0.7)
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//! axum::serve(listener, app)
//!     .await
//!     .unwrap();
//! # }
//! ```

pub mod middleware;
pub mod extractors;
pub mod utils;
pub mod errors;

// Re-export commonly used types and functions
pub use errors::{WebError, WebResult};
pub use middleware::{
    cors_layer, error_handler_layer, request_id_layer,
    pagination_response_layer, rate_limit_layer
};
pub use extractors::{
    QueryParams, PaginationQuery, SortQuery, FilterQuery,
    extract_task_filters, extract_execution_filters, 
    extract_job_filters, extract_schedule_filters
};
pub use utils::{ApiResponse, ResponseBuilder};