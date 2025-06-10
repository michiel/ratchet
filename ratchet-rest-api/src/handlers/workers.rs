//! Worker monitoring endpoints (placeholder)

use axum::{extract::State, response::IntoResponse, Json};

use crate::{context::WorkersContext, errors::RestResult};

/// Placeholder for worker endpoints
pub async fn placeholder_workers_handler(
    State(_ctx): State<WorkersContext>,
) -> RestResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Worker endpoints not yet implemented", 
        "status": "placeholder"
    })))
}