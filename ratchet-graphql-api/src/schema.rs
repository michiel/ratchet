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
  executions(taskId: $taskId) {
    items {
      id
      uuid
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
                        name: 'Execute Task',
                        endpoint: '/graphql',
                        query: `mutation ExecuteTask($input: ExecuteTaskInput!) {
  executeTask(input: $input) {
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
                        variables: '{"input": {"taskId": "1", "inputData": {}, "priority": "NORMAL"}}'
                    },
                    {
                        name: 'Execute Task Direct',
                        endpoint: '/graphql',
                        query: `mutation ExecuteTaskDirect($taskId: String!, $inputData: JSON!) {
  executeTaskDirect(taskId: $taskId, inputData: $inputData) {
    success
    output
    error
    durationMs
  }
}`,
                        variables: '{"taskId": "1", "inputData": {}}'
                    },
                    {
                        name: 'System Health',
                        endpoint: '/graphql',
                        query: `query SystemHealth {
  health {
    database
    jobQueue
    scheduler
    message
  }
  taskStats {
    totalTasks
    enabledTasks
    disabledTasks
  }
  executionStats {
    totalExecutions
    pending
    running
    completed
    failed
  }
  jobStats {
    totalJobs
    queued
    processing
    completed
    failed
    retrying
  }
}`
                    },
                    {
                        name: 'Jobs Queue',
                        endpoint: '/graphql',
                        query: `query JobsQueue($status: JobStatus) {
  jobs(status: $status) {
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
      outputDestinations {
        destinationType
        template
      }
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
                        name: 'Get Task by UUID',
                        endpoint: '/graphql',
                        query: `query GetTaskByUUID($uuid: UUID!, $version: String) {
  task(uuid: $uuid, version: $version) {
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
                        variables: '{"uuid": "00000000-0000-0000-0000-000000000000", "version": null}'
                    },
                    {
                        name: 'Update Task Status',
                        endpoint: '/graphql',
                        query: `mutation UpdateTaskStatus($id: String!, $enabled: Boolean!) {
  updateTaskStatus(id: $id, enabled: $enabled) {
    id
    uuid
    name
    enabled
    updatedAt
  }
}`,
                        variables: '{"id": "1", "enabled": true}'
                    },
                    {
                        name: 'Test Output Destinations',
                        endpoint: '/graphql',
                        query: `mutation TestOutputDestinations($input: TestOutputDestinationsInput!) {
  testOutputDestinations(input: $input) {
    index
    destinationType
    success
    error
    estimatedTimeMs
  }
}`,
                        variables: '{"input": {"destinations": [{"destinationType": "FILESYSTEM", "filesystem": {"path": "/tmp/test.json", "format": "JSON"}}]}}'
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