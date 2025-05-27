use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    http::StatusCode,
};

// use crate::database::repositories::Repository; // Unused due to Send/Sync constraints
use super::app::ServerState;

/// GraphQL endpoint handler
pub async fn graphql_handler(
    State(state): State<ServerState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

/// GraphQL playground handler (for development)
pub async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Health check endpoint (simplified)
pub async fn health_handler(_state: State<ServerState>) -> impl IntoResponse {
    // TODO: Re-add database health check when Send/Sync issues are resolved
    (StatusCode::OK, "OK")
}

/// API version information
pub async fn version_handler() -> impl IntoResponse {
    let version_info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
    });
    
    axum::Json(version_info)
}