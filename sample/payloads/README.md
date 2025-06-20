# Ratchet REST API Payload Examples

This directory contains example JSON payloads for the Ratchet REST API endpoints.

## Schedule Creation

### Current Implementation

**Endpoint:** `POST /api/v1/schedules`

**File:** `schedule-create-current.json`

This shows the **currently implemented** schedule creation format. Note that schedules do not currently support output destinations directly.

```json
{
  "taskId": "task_12345",
  "name": "daily-analytics-report",
  "description": "Generate daily analytics report and email to stakeholders",
  "cronExpression": "0 9 * * 1-5",
  "enabled": true
}
```

**Fields:**
- `taskId` (required): ID of the task to schedule
- `name` (required): Human-readable name for the schedule
- `description` (optional): Description of the schedule purpose
- `cronExpression` (required): Cron expression (5 or 6 fields supported)
- `enabled` (optional): Whether the schedule is active (defaults to true)

**Cron Expression Examples:**
- `"0 9 * * 1-5"` - 9 AM Monday through Friday (5 fields)
- `"0 0 9 * * 1-5"` - 9 AM Monday through Friday (6 fields)
- `"*/15 * * * *"` - Every 15 minutes
- `"0 0 * * *"` - Daily at midnight
- `"0 0 1 * *"` - First day of every month

### Future/Aspirational Implementation

**File:** `schedule-create-with-webhook.json`

This shows the **expected future format** based on test files. This functionality is not yet implemented but shows the intended API design for schedules with output destinations.

## Job Creation with Output Destinations

**Endpoint:** `POST /api/v1/jobs`

**File:** `job-create-with-webhook.json`

This shows the **working implementation** for creating jobs with webhook output destinations. This is the current workaround for webhook delivery of scheduled task results.

```json
{
  "taskId": "task_12345",
  "input": { /* task input data */ },
  "priority": "NORMAL",
  "maxRetries": 3,
  "scheduledFor": "2023-12-07T15:30:00Z",
  "output_destinations": [
    {
      "destination_type": "webhook",
      "webhook": {
        "url": "https://your-webhook-endpoint.com/api/notifications",
        "method": "POST",
        "timeout_seconds": 30,
        "authentication": {
          "auth_type": "bearer",
          "bearer": {
            "token": "your-webhook-auth-token"
          }
        }
      }
    }
  ]
}
```

## Webhook Configuration Options

### Authentication Types

**Bearer Token:**
```json
{
  "authentication": {
    "auth_type": "bearer",
    "bearer": {
      "token": "your-bearer-token"
    }
  }
}
```

**Basic Authentication:**
```json
{
  "authentication": {
    "auth_type": "basic",
    "basic": {
      "username": "your-username",
      "password": "your-password"
    }
  }
}
```

**API Key:**
```json
{
  "authentication": {
    "auth_type": "api_key",
    "api_key": {
      "header_name": "X-API-Key",
      "api_key": "your-api-key"
    }
  }
}
```

### HTTP Methods

Supported methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`

### Retry Policy

```json
{
  "retry_policy": {
    "max_attempts": 3,
    "initial_delay_seconds": 1,
    "max_delay_seconds": 5,
    "backoff_multiplier": 2.0
  }
}
```

## Output Destination Types

### Webhook Destination

```json
{
  "destination_type": "webhook",
  "webhook": {
    "url": "https://your-endpoint.com/webhook",
    "method": "POST",
    "timeout_seconds": 30,
    "content_type": "application/json",
    "headers": {
      "Custom-Header": "value"
    }
  }
}
```

### Filesystem Destination

```json
{
  "destination_type": "filesystem",
  "filesystem": {
    "path": "/var/log/ratchet/outputs/{date}/{task_name}-{execution_id}.json",
    "format": "json",
    "permissions": "644",
    "create_dirs": true,
    "overwrite": false,
    "backup_existing": true
  }
}
```

## Usage Examples

### Create a Schedule (Current API)

```bash
curl -X POST http://localhost:8080/api/v1/schedules \
  -H "Content-Type: application/json" \
  -d @schedule-create-current.json
```

### Create a Job with Webhook (Working Implementation)

```bash
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Content-Type: application/json" \
  -d @job-create-with-webhook.json
```

### Manual Schedule Trigger

```bash
curl -X POST http://localhost:8080/api/v1/schedules/{schedule_id}/trigger \
  -H "Content-Type: application/json" \
  -d '{"inputData": {"param": "value"}}'
```

## Current Limitations

1. **Schedules do not support output destinations** - This is a current limitation where schedules can be created but do not directly support webhook configuration
2. **Workaround**: Use the jobs API with `scheduledFor` timestamp and `output_destinations` for webhook delivery
3. **Future enhancement**: The schedule API is expected to support output destinations in a future release

## Environment Variables

For production deployments, use environment variables for sensitive data:

```json
{
  "webhook": {
    "url": "${WEBHOOK_URL}",
    "authentication": {
      "auth_type": "bearer",
      "bearer": {
        "token": "${WEBHOOK_TOKEN}"
      }
    }
  }
}
```