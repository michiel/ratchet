//! Health check endpoint

use axum::{response::Json, extract::State};
use std::sync::Arc;
use std::time::Instant;

use crate::HealthResponse;

/// Application state for tracking uptime
pub struct AppState {
    pub start_time: Instant,
}

/// Health check endpoint handler
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime_seconds = state.start_time.elapsed().as_secs();
    Json(HealthResponse::healthy(uptime_seconds))
}