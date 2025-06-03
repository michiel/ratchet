//! REST API implementation for Ratchet
//!
//! This module provides the RESTful API endpoints for interacting with Ratchet.

use axum::Router;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub mod handlers;
pub mod middleware;
pub mod routes;

use crate::config::ApiConfig;
use crate::errors::ApiResult;
use std::sync::Arc;
use std::time::Instant;

use self::handlers::health::AppState;

/// Create the REST API router with all routes and middleware
pub fn create_rest_router(config: &ApiConfig) -> ApiResult<Router> {
    // Create application state
    let app_state = Arc::new(AppState {
        start_time: Instant::now(),
    });
    let router = Router::new()
        .merge(routes::create_api_routes())
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                // Add tracing/logging
                .layer(TraceLayer::new_for_http())
                // Add CORS support
                .layer(create_cors_layer(config))
                // Add request ID
                .layer(axum::middleware::from_fn(self::middleware::request_id))
                // Add error handling
                .layer(axum::middleware::from_fn(self::middleware::error_handler))
        );

    Ok(router)
}

/// Create CORS layer based on configuration
fn create_cors_layer(config: &ApiConfig) -> CorsLayer {
    let cors = CorsLayer::new();
    
    // Configure allowed origins
    let cors = if config.cors.allowed_origins.is_empty() {
        cors.allow_origin(tower_http::cors::Any)
    } else {
        config.cors.allowed_origins.iter().fold(cors, |cors, origin| {
            cors.allow_origin(origin.parse::<axum::http::HeaderValue>().unwrap())
        })
    };
    
    // Configure allowed methods
    let cors = config.cors.allowed_methods.iter().fold(cors, |cors, method| {
        cors.allow_methods([method.parse::<axum::http::Method>().unwrap()])
    });
    
    // Configure allowed headers
    let cors = config.cors.allowed_headers.iter().fold(cors, |cors, header| {
        cors.allow_headers([header.parse::<axum::http::HeaderName>().unwrap()])
    });
    
    // Configure exposed headers
    let cors = config.cors.exposed_headers.iter().fold(cors, |cors, header| {
        cors.expose_headers([header.parse::<axum::http::HeaderName>().unwrap()])
    });
    
    // Configure other CORS settings
    cors.allow_credentials(config.cors.allow_credentials)
        .max_age(std::time::Duration::from_secs(config.cors.max_age.as_secs()))
}