//! GraphQL schema definition

use async_graphql::{Schema, SchemaBuilder, EmptySubscription};
use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};

use crate::{
    context::{GraphQLContext, GraphQLConfig},
    resolvers::{Query, Mutation},
};

/// The main GraphQL schema type
pub type RatchetSchema = Schema<Query, Mutation, EmptySubscription>;

/// Create the GraphQL schema with all resolvers
pub fn create_schema() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation, EmptySubscription)
}

/// Configure the schema with limits and extensions
pub fn configure_schema(
    builder: SchemaBuilder<Query, Mutation, EmptySubscription>,
    config: &GraphQLConfig,
) -> RatchetSchema {
    let mut schema = builder;

    if let Some(depth) = config.max_query_depth {
        schema = schema.limit_depth(depth);
    }

    if let Some(complexity) = config.max_query_complexity {
        schema = schema.limit_complexity(complexity);
    }

    if !config.enable_introspection {
        schema = schema.disable_introspection();
    }

    if config.enable_apollo_tracing {
        schema = schema.extension(async_graphql::extensions::ApolloTracing);
    }

    schema.finish()
}

/// GraphQL handler for Axum
pub async fn graphql_handler(
    State(context): State<GraphQLContext>,
    schema: axum::extract::Extension<RatchetSchema>,
    req: GraphQLRequest,
) -> impl IntoResponse {
    let response = schema.execute(req.into_inner().data(context)).await;
    GraphQLResponse::from(response)
}

/// GraphQL playground handler for development
pub async fn graphql_playground() -> impl IntoResponse {
    use axum::response::Html;
    
    let playground_html = async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql")
    );
    
    Html(playground_html)
}

/// GraphQL introspection schema handler
pub async fn graphql_introspection(
    schema: axum::extract::Extension<RatchetSchema>,
) -> impl IntoResponse {
    let introspection = schema.sdl();
    Json(serde_json::json!({ "schema": introspection }))
}