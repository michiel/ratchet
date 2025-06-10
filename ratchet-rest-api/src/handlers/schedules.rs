//! Schedule management endpoints (placeholder)

use axum::{extract::State, response::IntoResponse, Json};

use crate::{context::SchedulesContext, errors::RestResult};

/// Placeholder for schedule endpoints
pub async fn placeholder_schedules_handler(
    State(_ctx): State<SchedulesContext>,
) -> RestResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Schedule endpoints not yet implemented",
        "status": "placeholder"
    })))
}