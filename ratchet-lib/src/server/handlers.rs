// use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
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
    // Custom HTML with GraphQL Playground and preloaded tabs
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