# Output Destinations Guide

## Overview

The Ratchet output destination system provides a flexible way to deliver task execution results to various destinations including filesystem, webhooks, databases, and cloud storage. This system supports template variables, retry policies, and concurrent delivery to multiple destinations.

This guide covers both REST API and GraphQL API usage, with specific syntax requirements and limitations noted for each.

## Supported Destination Types

### 1. Filesystem
Store task outputs as files on the local filesystem with configurable formats and permissions.

### 2. Webhook
Send task outputs to HTTP endpoints with authentication, retry policies, and custom headers.

### 3. Database (Coming Soon)
Store outputs directly in database tables with column mapping.

### 4. S3 (Coming Soon)
Upload outputs to AWS S3 buckets with metadata and storage classes.

## Configuration

### YAML Configuration

Add output destination settings to your `ratchet.yaml` configuration file:

```yaml
# Output destinations configuration
output:
  # Maximum concurrent deliveries (default: 10)
  max_concurrent_deliveries: 20
  
  # Default timeout for all deliveries (default: 30s)
  default_timeout: 45
  
  # Validate configurations on startup (default: true)
  validate_on_startup: true
  
  # Default retry policy for failed deliveries
  default_retry_policy:
    max_attempts: 5
    initial_delay_ms: 2000
    max_delay_ms: 60000
    backoff_multiplier: 2.5
  
  # Global destination templates for reuse
  global_destinations:
    - name: "production_logs"
      description: "Production log storage"
      destination:
        type: filesystem
        path: "/var/log/ratchet/outputs/{{environment}}/{{job_uuid}}.json"
        format: json
        permissions: "644"
        create_dirs: true
        overwrite: true
    
    - name: "analytics_webhook"
      description: "Analytics webhook for completed tasks"
      destination:
        type: webhook
        url: "https://analytics.company.com/webhook/task-completion"
        method: POST
        headers:
          "X-Environment": "{{environment}}"
          "X-Task-Name": "{{task_name}}"
        timeout_seconds: 30
        content_type: "application/json"
        auth:
          type: bearer
          token: "${ANALYTICS_WEBHOOK_TOKEN}"
```

### Environment Variables

Override configuration values using environment variables:

```bash
# Output system configuration
export RATCHET_OUTPUT_MAX_CONCURRENT=15
export RATCHET_OUTPUT_DEFAULT_TIMEOUT=60
export RATCHET_OUTPUT_VALIDATE_STARTUP=true

# Authentication tokens (referenced in templates)
export ANALYTICS_WEBHOOK_TOKEN="your-webhook-token"
export AWS_ACCESS_KEY_ID="your-aws-key"
export AWS_SECRET_ACCESS_KEY="your-aws-secret"
```

## Template Variables

Output destinations support template variables that are replaced at execution time:

### Job Context Variables
- `{{job_uuid}}` - Unique job identifier
- `{{job_id}}` - Database job ID
- `{{task_name}}` - Name of the executed task
- `{{task_version}}` - Version of the executed task
- `{{task_id}}` - Database task ID
- `{{execution_id}}` - Database execution ID
- `{{priority}}` - Job priority (low, normal, high, urgent)
- `{{schedule_id}}` - Schedule ID (if job was scheduled)
- `{{environment}}` - Environment name (from ENVIRONMENT env var)

### Execution Context Variables
- `{{timestamp}}` - Execution completion timestamp (ISO 8601)
- `{{date}}` - Execution completion date (YYYY-MM-DD)
- `{{time}}` - Execution completion time (HH:MM:SS)
- `{{year}}`, `{{month}}`, `{{day}}` - Date components
- `{{hour}}`, `{{minute}}`, `{{second}}` - Time components
- `{{duration_ms}}` - Execution duration in milliseconds
- `{{status}}` - Execution status (completed, failed)

### Output Data Variables
- `{{output_data}}` - Complete task output as JSON
- `{{output_size}}` - Size of output data in bytes

### Example Template Usage

```yaml
destinations:
  - type: filesystem
    path: "/data/outputs/{{year}}/{{month}}/{{day}}/{{task_name}}_{{job_uuid}}.json"
    
  - type: webhook
    url: "https://api.{{environment}}.company.com/webhooks/task/{{task_name}}"
    headers:
      "X-Job-ID": "{{job_uuid}}"
      "X-Timestamp": "{{timestamp}}"
      "X-Duration": "{{duration_ms}}ms"
```

## Output Formats

### JSON (Default)
Pretty-printed JSON format:
```json
{
  "result": "success",
  "data": {...}
}
```

### JSON Compact
Minified JSON without whitespace:
```json
{"result":"success","data":{...}}
```

### YAML
YAML format output:
```yaml
result: success
data:
  key: value
```

### CSV
For array outputs, convert to CSV format:
```csv
id,name,value
1,"Item 1",100
2,"Item 2",200
```

### Raw
Output data as-is without formatting.

### Template
Custom template format using Handlebars syntax:
```handlebars
Task: {{task_name}} ({{task_version}})
Completed: {{timestamp}}
Duration: {{duration_ms}}ms

Results:
{{output_data}}
```

## REST API Usage

### Test Output Destinations

Before using destinations in production, test them to verify connectivity and configuration:

```bash
curl -X POST http://localhost:8080/api/v1/jobs/test-output-destinations \
  -H "Content-Type: application/json" \
  -d '{
    "destinations": [
      {
        "type": "filesystem",
        "path": "/tmp/test-output.json",
        "format": "json",
        "create_dirs": true
      },
      {
        "type": "webhook",
        "url": "https://httpbin.org/post",
        "method": "POST",
        "timeout_seconds": 10
      }
    ]
  }'
```

Response:
```json
{
  "data": [
    {
      "index": 0,
      "destination_type": "filesystem",
      "success": true,
      "error": null,
      "estimated_time_ms": 5
    },
    {
      "index": 1,
      "destination_type": "webhook",
      "success": true,
      "error": null,
      "estimated_time_ms": 150
    }
  ]
}
```

### Create Job with Output Destinations

```bash
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "task_id": 1,
    "input_data": {"name": "example"},
    "priority": "normal",
    "output_destinations": [
      {
        "type": "filesystem",
        "path": "/data/outputs/{{job_uuid}}.json",
        "format": "json",
        "permissions": "644",
        "create_dirs": true
      },
      {
        "type": "webhook",
        "url": "https://api.company.com/webhook/completion",
        "method": "POST",
        "headers": {
          "Authorization": "Bearer {{webhook_token}}",
          "X-Task": "{{task_name}}"
        },
        "timeout_seconds": 30,
        "retry_policy": {
          "max_attempts": 3,
          "initial_delay_ms": 1000,
          "max_delay_ms": 10000,
          "backoff_multiplier": 2.0
        }
      }
    ]
  }'
```

### List Jobs with Output Destinations

```bash
curl -X GET "http://localhost:8080/api/v1/jobs?_start=0&_end=10" \
  -H "Accept: application/json"
```

Response includes output_destinations field:
```json
{
  "data": [
    {
      "id": 1,
      "task_id": 1,
      "priority": "normal",
      "status": "completed",
      "output_destinations": [
        {
          "type": "filesystem",
          "path": "/data/outputs/job-123.json",
          "format": "json"
        }
      ],
      "queued_at": "2024-01-01T10:00:00Z"
    }
  ]
}
```

## GraphQL Usage

### Test Output Destinations

```graphql
mutation TestDestinations($input: TestOutputDestinationsInput!) {
  testOutputDestinations(input: $input) {
    index
    destinationType
    success
    error
    estimatedTimeMs
  }
}
```

Variables:
```json
{
  "input": {
    "destinations": [
      {
        "destinationType": "FILESYSTEM",
        "filesystem": {
          "path": "/tmp/test-{{timestamp}}.json",
          "format": "JSON",
          "createDirs": true,
          "overwrite": true
        }
      },
      {
        "destinationType": "WEBHOOK",
        "webhook": {
          "url": "https://httpbin.org/post",
          "method": "POST",
          "timeoutSeconds": 30,
          "retryPolicy": {
            "maxAttempts": 3,
            "initialDelayMs": 1000,
            "maxDelayMs": 10000,
            "backoffMultiplier": 2.0
          }
        }
      }
    ]
  }
}
```

### Execute Task with Output Destinations

```graphql
mutation ExecuteTask($input: ExecuteTaskInput!) {
  executeTask(input: $input) {
    id
    taskId
    status
    priority
    outputDestinations {
      ... on FilesystemDestination {
        path
        format
        permissions
        createDirs
        overwrite
      }
      ... on WebhookDestination {
        url
        method
        timeoutSeconds
        retryPolicy {
          maxAttempts
          initialDelayMs
          maxDelayMs
          backoffMultiplier
        }
      }
    }
  }
}
```

Variables:
```json
{
  "input": {
    "taskId": 1,
    "inputData": {"message": "Hello World"},
    "priority": "NORMAL",
    "outputDestinations": [
      {
        "destinationType": "FILESYSTEM",
        "filesystem": {
          "path": "/data/outputs/{{task_name}}/{{job_uuid}}.yaml",
          "format": "YAML",
          "permissions": "644",
          "createDirs": true,
          "overwrite": true,
          "backupExisting": false
        }
      },
      {
        "destinationType": "WEBHOOK",
        "webhook": {
          "url": "https://api.company.com/webhooks/task-completed",
          "method": "POST",
          "headers": {
            "Authorization": "Bearer your-token",
            "X-Source": "ratchet",
            "X-Task": "{{task_name}}"
          },
          "timeoutSeconds": 30,
          "contentType": "application/json",
          "retryPolicy": {
            "maxAttempts": 5,
            "initialDelayMs": 2000,
            "maxDelayMs": 30000,
            "backoffMultiplier": 2.0
          }
        }
      }
    ]
  }
}
```

### Query Jobs with Output Destinations

```graphql
query GetJobs($pagination: PaginationInput) {
  jobs(pagination: $pagination) {
    jobs {
      id
      taskId
      status
      priority
      queuedAt
      outputDestinations {
        ... on FilesystemDestination {
          path
          format
        }
        ... on WebhookDestination {
          url
          method
        }
      }
    }
    total
    page
    limit
  }
}
```

## Retry Policies

Configure how failed deliveries are retried:

```yaml
retry_policy:
  max_attempts: 5              # Maximum retry attempts
  initial_delay_ms: 1000       # Initial delay between retries
  max_delay_ms: 60000         # Maximum delay between retries
  backoff_multiplier: 2.0     # Exponential backoff multiplier
```

### Retry Logic

1. **Initial Attempt**: First delivery attempt
2. **Exponential Backoff**: Each retry waits longer: `initial_delay * (multiplier ^ attempt)`
3. **Maximum Delay**: Delays are capped at `max_delay_ms`
4. **Jitter**: Random jitter is added to prevent thundering herd
5. **Status Codes**: Webhooks retry on 429, 500, 502, 503, 504 by default

### Example Retry Sequence

With `initial_delay_ms: 1000`, `backoff_multiplier: 2.0`, `max_delay_ms: 30000`:

- Attempt 1: Immediate
- Attempt 2: ~1 second delay
- Attempt 3: ~2 second delay  
- Attempt 4: ~4 second delay
- Attempt 5: ~8 second delay
- Attempt 6: ~16 second delay
- Attempt 7: ~30 second delay (capped)

## Authentication

### Webhook Authentication

#### Bearer Token
```yaml
auth:
  type: bearer
  token: "your-bearer-token"
```

#### Basic Authentication
```yaml
auth:
  type: basic
  username: "your-username"
  password: "your-password"
```

#### API Key in Header
```yaml
auth:
  type: api_key
  header: "X-API-Key"
  value: "your-api-key"
```

#### HMAC Signature
```yaml
auth:
  type: signature
  secret: "your-hmac-secret"
  algorithm: "sha256"
```

## Error Handling

### Common Error Scenarios

1. **Network Timeouts**: Webhook requests that exceed timeout
2. **Authentication Failures**: Invalid credentials or tokens
3. **Filesystem Permissions**: Insufficient permissions to write files
4. **Template Errors**: Invalid template variables or syntax
5. **Format Errors**: Unable to serialize output in specified format

### Error Logging

Failed deliveries are logged with detailed error information:

```
2024-01-01T10:00:00Z ERROR [output] Failed to deliver to filesystem destination
  Job: job-123
  Destination: /data/outputs/result.json
  Error: Permission denied (os error 13)
  
2024-01-01T10:00:05Z WARN [output] Webhook delivery failed, retrying (attempt 2/3)
  Job: job-123
  URL: https://api.company.com/webhook
  Status: 500 Internal Server Error
  Next retry in: 2.1s
```

### Monitoring Delivery Success

Monitor delivery metrics and success rates:

```bash
# Check delivery logs
tail -f /var/log/ratchet/ratchet.log | grep output

# Monitor delivery success rate
grep "Output delivered" /var/log/ratchet/ratchet.log | wc -l
grep "delivery failed" /var/log/ratchet/ratchet.log | wc -l
```

## Best Practices

### 1. Use Template Variables Effectively

```yaml
# Good: Organized by date and task
path: "/data/outputs/{{year}}/{{month}}/{{day}}/{{task_name}}/{{job_uuid}}.json"

# Avoid: Flat structure that becomes difficult to manage
path: "/data/outputs/{{job_uuid}}.json"
```

### 2. Configure Appropriate Timeouts

```yaml
# Short timeout for fast internal APIs
timeout_seconds: 5

# Longer timeout for external services
timeout_seconds: 30

# Very long timeout for slow processing webhooks
timeout_seconds: 120
```

### 3. Use Retry Policies Wisely

```yaml
# Conservative policy for critical deliveries
retry_policy:
  max_attempts: 5
  initial_delay_ms: 2000
  max_delay_ms: 60000
  backoff_multiplier: 2.0

# Aggressive policy for non-critical notifications
retry_policy:
  max_attempts: 2
  initial_delay_ms: 500
  max_delay_ms: 5000
  backoff_multiplier: 1.5
```

### 4. Secure Authentication

```yaml
# Use environment variables for secrets
auth:
  type: bearer
  token: "${WEBHOOK_TOKEN}"

# Avoid hardcoding credentials
auth:
  type: bearer
  token: "hardcoded-secret"  # DON'T DO THIS
```

### 5. Test Before Production

Always test your destination configurations:

```bash
# Test all destinations before deploying
curl -X POST http://localhost:8080/api/v1/jobs/test-output-destinations \
  -d @destination-config.json
```

### 6. Monitor and Alert

Set up monitoring for:
- Delivery success rates
- Retry attempt frequencies  
- Error patterns
- Delivery latency

### 7. Plan for Growth

```yaml
# Start conservative, increase as needed
max_concurrent_deliveries: 5

# Monitor resource usage and scale up
max_concurrent_deliveries: 20
```

## Troubleshooting

### Common Issues

#### Template Variable Not Found
```
Error: Template variable 'unknown_var' not found
Solution: Check available variables list and fix template
```

#### Permission Denied
```
Error: Permission denied writing to /data/outputs/
Solution: Check file permissions and user/group ownership
```

#### Webhook Timeout
```
Error: Request timeout after 30s
Solution: Increase timeout or optimize webhook endpoint
```

#### Invalid JSON Format
```
Error: Cannot serialize output to JSON
Solution: Check that output data is JSON-serializable
```

### Debug Mode

Enable debug logging for detailed output delivery information:

```yaml
logging:
  level: debug
  
# Or via environment variable
export RATCHET_LOG_LEVEL=debug
```

### Validation Errors

If validation fails on startup, check:

1. Template syntax is correct
2. Destination types are supported
3. Required fields are present
4. File paths are absolute
5. URLs are well-formed
6. Retry policy values are positive

## Roadmap

### Planned Features

1. **Database Destinations**: PostgreSQL, MySQL, SQLite support
2. **S3 Destinations**: Full AWS S3 integration
3. **Message Queue Destinations**: RabbitMQ, Apache Kafka, Redis
4. **Email Destinations**: SMTP delivery for notifications
5. **Slack/Discord Destinations**: Chat notifications
6. **Compression**: Gzip compression for large outputs
7. **Encryption**: Encrypt sensitive outputs at rest and in transit
8. **Batch Delivery**: Group multiple outputs for efficiency
9. **Dead Letter Queues**: Handle permanently failed deliveries
10. **Delivery Scheduling**: Delay delivery until specific times

### Migration Notes

When upgrading to newer versions:
- Check changelog for configuration changes
- Test destination configurations after upgrade
- Monitor delivery success rates post-upgrade
- Update templates if new variables are available

## GraphQL API Usage

### GraphQL Syntax Requirements

When using output destinations with the GraphQL API, follow these syntax requirements:

#### JSON Input Data
Use GraphQL object syntax, not JSON string syntax:

```graphql
# ✅ Correct - GraphQL object syntax
inputData: {message: "hello", count: 42}

# ❌ Incorrect - JSON string syntax
inputData: {"message": "hello", "count": 42}
```

#### Field Names
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

### GraphQL Destination Configuration

#### Filesystem Destination

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

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | String | Yes | Absolute path where the output will be stored |
| `format` | Enum | No | Output format: `JSON`, `YAML`, `CSV`, `XML` (default: `JSON`) |
| `compression` | Enum | No | Compression type: `GZIP`, `ZSTD` (optional) |
| `permissions` | String | No | Unix file permissions (e.g., "0644") |

#### Webhook Destination

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

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | String | Yes | Full URL of the webhook endpoint |
| `method` | Enum | No | HTTP method: `GET`, `POST`, `PUT`, `PATCH` (default: `POST`) |
| `timeoutSeconds` | Int | No | Request timeout in seconds (default: 30) |
| `contentType` | String | No | Content-Type header (default: "application/json") |

### GraphQL API Operations

#### Execute Task with Output Destinations

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

#### Test Output Destinations

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

#### Query Jobs with Output Destinations

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

### GraphQL Examples

#### Multiple Format Outputs

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

#### Webhook with Retry

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

### GraphQL Limitations

#### Type System Limitations

1. **No HashMap Support**: Custom webhook headers cannot be specified directly in mutations due to GraphQL type limitations.

2. **JSON Input Syntax**: Complex JSON objects must use GraphQL object syntax. Nested quotes are not supported.

3. **Enum Values**: Enum values (like `POST`, `JSON`, `FILESYSTEM`) must be unquoted in GraphQL.

#### Webhook Limitations

1. **Headers**: Custom headers are not directly supported in GraphQL mutations.

2. **Authentication**: Currently supports:
   - Bearer token authentication
   - Basic authentication
   - API key authentication

3. **Response Size**: Webhook responses are limited to 10MB.

### GraphQL Error Handling

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

Common error scenarios:

```json
// Invalid Path
{
  "success": false,
  "error": "Invalid path: must be absolute",
  "destinationType": "FILESYSTEM"
}

// Invalid URL
{
  "success": false,
  "error": "Invalid URL format",
  "destinationType": "WEBHOOK"
}

// Network Timeout
{
  "success": false,
  "error": "Request timeout after 30 seconds",
  "destinationType": "WEBHOOK"
}
```

### GraphQL vs REST API Comparison

| Feature | REST API | GraphQL API |
|---------|----------|-------------|
| Headers | Supported as object | Not directly supported |
| JSON Input | Standard JSON syntax | GraphQL object syntax |
| Error Handling | HTTP status codes | Success/error in response data |
| Batch Operations | Separate requests | Single mutation with multiple destinations |

## Related Documentation

- [REST API Guide](./REST_API_README.md) - REST API documentation and examples
- [GraphQL Schema](../openapi.yaml) - Complete API schema definitions
- [Configuration Guide](./README.md) - Main Ratchet documentation