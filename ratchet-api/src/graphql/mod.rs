//! GraphQL API implementation for Ratchet

use async_graphql::{
    http::GraphiQLSource,
    EmptySubscription,
    Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

pub mod schema;
pub mod resolvers;
pub mod context;

use crate::config::ApiConfig;
use crate::errors::ApiResult;
use schema::{QueryRoot, MutationRoot};
pub use context::GraphQLContext;

/// GraphQL schema type
pub type RatchetSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Create the GraphQL router with playground
pub fn create_graphql_router(config: &ApiConfig, schema: RatchetSchema) -> ApiResult<Router> {
    let router = Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .with_state(schema);

    Ok(router)
}

/// GraphQL playground handler
async fn graphql_playground() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

/// GraphQL request handler
async fn graphql_handler(
    State(schema): State<RatchetSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// Create the GraphQL schema
pub fn create_schema(context: GraphQLContext) -> RatchetSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(context)
        .finish()
}