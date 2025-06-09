//! Job management endpoints (placeholder)

use axum::{extract::State, response::IntoResponse, Json};

use crate::{context::JobsContext, errors::RestResult};

/// Placeholder for job endpoints
pub async fn placeholder_jobs_handler(
    State(_ctx): State<JobsContext>,
) -> RestResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Job endpoints not yet implemented",
        "status": "placeholder"
    })))
}