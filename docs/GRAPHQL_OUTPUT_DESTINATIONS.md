# GraphQL Output Destinations Guide

This guide covers how to use output destinations with Ratchet's GraphQL API, including syntax requirements, limitations, and best practices.

## Table of Contents

- [Overview](#overview)
- [GraphQL Syntax Requirements](#graphql-syntax-requirements)
- [Output Destination Types](#output-destination-types)
- [API Operations](#api-operations)
- [Examples](#examples)
- [Limitations](#limitations)
- [Error Handling](#error-handling)

## Overview

Output destinations allow you to configure where and how task execution results are delivered. Ratchet supports multiple destination types including filesystem storage and webhooks.

## GraphQL Syntax Requirements

### JSON Input Data

When passing JSON data in GraphQL mutations, you must use GraphQL object syntax, not JSON string syntax:

```graphql
# ✅ Correct - GraphQL object syntax
inputData: {message: "hello", count: 42}

# ❌ Incorrect - JSON string syntax
inputData: {"message": "hello", "count": 42}
```

### Field Names

GraphQL requires unquoted field names:

```graphql
# ✅ Correct
filesystem: {
  path: "/tmp/output.json"
  format: JSON
}

# ❌ Incorrect
filesystem: {
  "path": "/tmp/output.json"
  "format": "JSON"
}
```

## Output Destination Types

### Filesystem Destination

Stores output to the local filesystem with various format options.

#### Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | String | Yes | Absolute path where the output will be stored |
| `format` | Enum | No | Output format: `JSON`, `YAML`, `CSV`, `XML` (default: `JSON`) |
| `compression` | Enum | No | Compression type: `GZIP`, `ZSTD` (optional) |
| `permissions` | String | No | Unix file permissions (e.g., "0644") |

#### Example

```graphql
{
  destinationType: FILESYSTEM
  filesystem: {
    path: "/var/data/outputs/result.json"
    format: JSON
    compression: GZIP
    permissions: "0644"
  }
  template: "{{task_name}}_{{timestamp}}.json"
}
```

### Webhook Destination

Sends output to an HTTP endpoint.

#### Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | String | Yes | Full URL of the webhook endpoint |
| `method` | Enum | No | HTTP method: `GET`, `POST`, `PUT`, `PATCH` (default: `POST`) |
| `timeoutSeconds` | Int | No | Request timeout in seconds (default: 30) |
| `contentType` | String | No | Content-Type header (default: "application/json") |

#### Example

```graphql
{
  destinationType: WEBHOOK
  webhook: {
    url: "https://api.example.com/webhook"
    method: POST
    timeoutSeconds: 60
    contentType: "application/json"
  }
}
```

## API Operations

### Execute Task with Output Destinations

```graphql
mutation ExecuteTaskWithDestinations {
  executeTask(input: {
    taskId: 123
    inputData: {key: "value"}
    priority: NORMAL
    outputDestinations: [
      {
        destinationType: FILESYSTEM
        filesystem: {
          path: "/tmp/output.json"
          format: JSON
        }
      }
      {
        destinationType: WEBHOOK
        webhook: {
          url: "https://webhook.site/your-uuid"
          method: POST
        }
      }
    ]
  }) {
    id
    taskId
    status
    priority
    outputDestinations {
      destinationType
      template
    }
  }
}
```

### Test Output Destinations

Use this mutation to validate destination configurations without executing a task:

```graphql
mutation TestDestinations {
  testOutputDestinations(input: {
    destinations: [
      {
        destinationType: FILESYSTEM
        filesystem: {
          path: "/tmp/test.json"
          format: JSON
        }
      }
    ]
  }) {
    destinationType
    success
    error
    webhookResponse {
      statusCode
      headers
      body
    }
  }
}
```

### Query Jobs with Output Destinations

```graphql
query GetJobsWithDestinations {
  jobs(first: 10) {
    edges {
      node {
        id
        taskId
        status
        outputDestinations {
          destinationType
          filesystem {
            path
            format
          }
          webhook {
            url
            method
          }
        }
      }
    }
  }
}
```

## Examples

### Example 1: Multiple Format Outputs

```graphql
mutation ExecuteWithMultipleFormats {
  executeTask(input: {
    taskId: 456
    inputData: {operation: "process"}
    outputDestinations: [
      {
        destinationType: FILESYSTEM
        filesystem: {
          path: "/data/outputs/result.json"
          format: JSON
        }
      }
      {
        destinationType: FILESYSTEM
        filesystem: {
          path: "/data/outputs/result.yaml"
          format: YAML
        }
      }
      {
        destinationType: FILESYSTEM
        filesystem: {
          path: "/data/outputs/result.csv"
          format: CSV
        }
      }
    ]
  }) {
    id
    status
  }
}
```

### Example 2: Webhook with Retry

```graphql
mutation ExecuteWithWebhookRetry {
  executeTask(input: {
    taskId: 789
    inputData: {data: "important"}
    outputDestinations: [{
      destinationType: WEBHOOK
      webhook: {
        url: "https://api.partner.com/receive"
        method: POST
        timeoutSeconds: 30
        retryPolicy: {
          maxAttempts: 3
          backoffMultiplier: 2.0
          initialDelaySeconds: 1
        }
      }
    }]
  }) {
    id
    status
  }
}
```

### Example 3: Template Variables

Output destinations support template variables for dynamic naming:

```graphql
{
  destinationType: FILESYSTEM
  filesystem: {
    path: "/data/outputs/"
    format: JSON
  }
  template: "{{task_name}}/{{date}}/{{job_id}}_{{timestamp}}.json"
}
```

Available template variables:
- `{{job_id}}` - Unique job identifier
- `{{task_name}}` - Name of the executed task
- `{{task_id}}` - Task identifier
- `{{timestamp}}` - ISO 8601 timestamp
- `{{date}}` - Date in YYYY-MM-DD format
- `{{time}}` - Time in HH-MM-SS format
- `{{env}}` - Current environment

## Limitations

### GraphQL Type Limitations

1. **No HashMap Support**: GraphQL doesn't have a native HashMap type, so webhook headers cannot be specified directly in mutations. Headers must be configured through other means (e.g., authentication configuration).

2. **JSON Input Syntax**: Complex JSON objects must use GraphQL object syntax. Nested quotes are not supported.

3. **Enum Values**: Enum values (like `POST`, `JSON`, `FILESYSTEM`) must be unquoted in GraphQL.

### Webhook Limitations

1. **Headers**: Custom headers are not directly supported in GraphQL mutations due to type system limitations.

2. **Authentication**: Currently supports:
   - Bearer token authentication
   - Basic authentication
   - API key authentication

3. **Response Size**: Webhook responses are limited to 10MB.

### Filesystem Limitations

1. **Path Restrictions**: Paths must be absolute and within allowed directories.
2. **Permissions**: Only applies to Unix-like systems.
3. **Format Support**: Some formats may have limitations on data structure (e.g., CSV requires flat data).

## Error Handling

### Validation Errors

The `testOutputDestinations` mutation returns validation results as data, not GraphQL errors:

```graphql
{
  testOutputDestinations(input: {...}) {
    success      # false if validation failed
    error        # Error message if validation failed
    destinationType
  }
}
```

### Common Error Scenarios

1. **Invalid Path**
   ```json
   {
     "success": false,
     "error": "Invalid path: must be absolute",
     "destinationType": "FILESYSTEM"
   }
   ```

2. **Invalid URL**
   ```json
   {
     "success": false,
     "error": "Invalid URL format",
     "destinationType": "WEBHOOK"
   }
   ```

3. **Network Timeout**
   ```json
   {
     "success": false,
     "error": "Request timeout after 30 seconds",
     "destinationType": "WEBHOOK"
   }
   ```

### Best Practices

1. **Always Test First**: Use `testOutputDestinations` before configuring production jobs.

2. **Handle Failures Gracefully**: Output destination failures don't fail the job by default. Monitor delivery results.

3. **Use Templates**: Leverage template variables for organized output structure.

4. **Set Appropriate Timeouts**: Configure webhook timeouts based on endpoint performance.

5. **Validate Permissions**: Ensure the Ratchet process has write permissions for filesystem destinations.

## Migration from REST API

If migrating from the REST API, note these key differences:

| Feature | REST API | GraphQL API |
|---------|----------|-------------|
| Headers | Supported as object | Not directly supported |
| JSON Input | Standard JSON syntax | GraphQL object syntax |
| Error Handling | HTTP status codes | Success/error in response data |
| Batch Operations | Separate requests | Single mutation with multiple destinations |

## Related Documentation

- [Output Destinations Overview](./OUTPUT_DESTINATIONS.md)
- [REST API Guide](./REST_API_README.md)
- [Task Execution Guide](./README.md)