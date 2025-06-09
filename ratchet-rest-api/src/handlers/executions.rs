//! Execution management endpoints (placeholder)

use axum::{extract::State, response::IntoResponse, Json};

use crate::{context::ExecutionsContext, errors::RestResult};

/// Placeholder for execution endpoints
/// 
/// These will be implemented in future iterations following the same
/// patterns as the task endpoints.
pub async fn placeholder_executions_handler(
    State(_ctx): State<ExecutionsContext>,
) -> RestResult<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "message": "Execution endpoints not yet implemented",
        "status": "placeholder"
    })))
}