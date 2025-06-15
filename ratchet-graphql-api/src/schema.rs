//! GraphQL schema definition

use async_graphql::{Schema, SchemaBuilder, EmptySubscription};
use axum::{
    response::IntoResponse,
    Json,
};

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

/// GraphQL handler for Axum 0.7 compatibility
pub async fn graphql_handler(
    axum::extract::Extension(context): axum::extract::Extension<GraphQLContext>,
    axum::extract::Extension(schema): axum::extract::Extension<RatchetSchema>,
    axum::extract::Json(request): axum::extract::Json<async_graphql::Request>,
) -> axum::response::Json<async_graphql::Response> {
    let response = schema.execute(request.data(context)).await;
    axum::response::Json(response)
}

/// GraphQL playground handler for development
pub async fn graphql_playground() -> impl IntoResponse {
    use axum::response::Html;
    
    // Custom HTML with GraphQL Playground and preloaded tabs with predefined queries
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset=utf-8/>
    <meta name="viewport" content="user-scalable=no, initial-scale=1.0, minimum-scale=1.0, maximum-scale=1.0, minimal-ui">
    <title>Ratchet GraphQL Playground</title>
    <link rel="stylesheet" href="//cdn.jsdelivr.net/npm/graphql-playground-react/build/static/css/index.css" />
    <link rel="shortcut icon" href="//cdn.jsdelivr.net/npm/graphql-playground-react/build/favicon.png" />
    <script src="//cdn.jsdelivr.net/npm/graphql-playground-react/build/static/js/middleware.js"></script>
</head>
<body>
    <div id="root"></div>
    <script>
        window.addEventListener('load', function (event) {
            GraphQLPlayground.init(document.getElementById('root'), {
                endpoint: '/graphql',
                settings: {
                    'editor.theme': 'dark',
                    'schema.polling.enable': false,
                    'request.credentials': 'omit'
                },
                tabs: [
                    {
                        name: 'List All Tasks',
                        endpoint: '/graphql',
                        query: `query ListAllTasks {
  tasks {
    items {
      id
      uuid
      name
      description
      version
      availableVersions
      registrySource
      enabled
      createdAt
      updatedAt
      validatedAt
      inSync
    }
    meta {
      page
      limit
      total
      totalPages
      hasNext
      hasPrevious
    }
  }
}`
                    },
                    {
                        name: 'Task Executions',
                        endpoint: '/graphql',
                        query: `query TaskExecutions($taskId: String) {
  executions(filters: { taskId: $taskId }) {
    items {
      id
      taskId
      input
      output
      status
      errorMessage
      queuedAt
      startedAt
      completedAt
      durationMs
    }
    meta {
      page
      limit
      total
      totalPages
      hasNext
      hasPrevious
    }
  }
}`,
                        variables: '{"taskId": null}'
                    },
                    {
                        name: 'Task Statistics',
                        endpoint: '/graphql',
                        query: `query TaskStatistics {
  taskStats {
    totalTasks
    enabledTasks
    disabledTasks
    totalExecutions
    successfulExecutions
    failedExecutions
    averageExecutionTimeMs
  }
}`
                    },
                    {
                        name: 'Jobs Queue',
                        endpoint: '/graphql',
                        query: `query JobsQueue($status: JobStatusGraphQL) {
  jobs(filters: { status: $status }) {
    items {
      id
      taskId
      priority
      status
      retryCount
      maxRetries
      queuedAt
      scheduledFor
      errorMessage
    }
    meta {
      page
      limit
      total
      totalPages
      hasNext
      hasPrevious
    }
  }
}`,
                        variables: '{"status": null}'
                    },
                    {
                        name: 'Get Task by ID',
                        endpoint: '/graphql',
                        query: `query GetTaskById($id: String!) {
  task(id: $id) {
    id
    uuid
    name
    description
    version
    availableVersions
    registrySource
    enabled
    createdAt
    updatedAt
    validatedAt
    inSync
    inputSchema
    outputSchema
    metadata
  }
}`,
                        variables: '{"id": "1"}'
                    },
                    {
                        name: 'Get Single Job by ID',
                        endpoint: '/graphql',
                        query: `query GetJobById($id: String!) {
  job(id: $id) {
    id
    taskId
    priority
    status
    retryCount
    maxRetries
    queuedAt
    scheduledFor
    errorMessage
  }
}`,
                        variables: '{"id": "1"}'
                    },
                    {
                        name: 'Get Single Execution by ID',
                        endpoint: '/graphql',
                        query: `query GetExecutionById($id: String!) {
  execution(id: $id) {
    id
    taskId
    status
    input
    output
    errorMessage
    queuedAt
    startedAt
    completedAt
    durationMs
  }
}`,
                        variables: '{"id": "1"}'
                    }
                ]
            })
        })
    </script>
</body>
</html>"#;

    Html(html)
}

/// GraphQL introspection schema handler
pub async fn graphql_introspection(
    schema: axum::extract::Extension<RatchetSchema>,
) -> impl IntoResponse {
    let introspection = schema.sdl();
    Json(serde_json::json!({ "schema": introspection }))
}