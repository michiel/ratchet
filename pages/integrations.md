---
layout: default
title: Integrations
permalink: /integrations/
---

# Integrations

Ratchet provides multiple integration points for connecting with external systems, APIs, and services. This guide covers the various ways to integrate Ratchet into your infrastructure.

## API Integrations

### REST API

Ratchet provides a comprehensive REST API for all operations:

#### Base URL
```
http://localhost:8080/api/v1
```

#### Authentication
```bash
# Currently no authentication required
# Future releases will support JWT authentication
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/v1/tasks
```

#### Common Endpoints

**Tasks Management**
```bash
# List all tasks
GET /api/v1/tasks

# Get task details
GET /api/v1/tasks/{task_id}

# Execute a task
POST /api/v1/tasks/{task_id}/execute
{
  "input": {
    "param1": "value1",
    "param2": "value2"
  }
}

# Delete a task
DELETE /api/v1/tasks/{task_id}
```

**Job Management**
```bash
# List jobs
GET /api/v1/jobs?status=pending&limit=10

# Get job details
GET /api/v1/jobs/{job_id}

# Cancel a job
POST /api/v1/jobs/{job_id}/cancel

# Retry a failed job
POST /api/v1/jobs/{job_id}/retry
```

**Schedules**
```bash
# Create a schedule
POST /api/v1/schedules
{
  "task_id": "daily-report",
  "cron_expression": "0 9 * * *",
  "input": {},
  "enabled": true
}

# Update schedule
PATCH /api/v1/schedules/{schedule_id}
{
  "enabled": false
}
```

### GraphQL API

For more complex queries, use the GraphQL API:

#### Endpoint
```
POST http://localhost:8080/graphql
```

#### Example Queries

**Query Tasks with Executions**
```graphql
query {
  tasks(limit: 10, offset: 0) {
    items {
      id
      name
      description
      version
      executions(limit: 5) {
        items {
          id
          status
          created_at
          completed_at
        }
      }
    }
    total
  }
}
```

**Execute Task Mutation**
```graphql
mutation {
  executeTask(
    taskId: "weather-api",
    input: { city: "London", units: "metric" }
  ) {
    job {
      id
      status
      created_at
    }
  }
}
```

**Subscribe to Job Updates**
```graphql
subscription {
  jobUpdates(jobId: "550e8400-e29b-41d4-a716-446655440000") {
    id
    status
    progress
    result
    error
  }
}
```

### GraphQL Playground

Access the interactive GraphQL playground:
```
http://localhost:8080/graphql/playground
```

## Client Libraries

### JavaScript/TypeScript

```typescript
// Using fetch API
async function executeTask(taskId: string, input: any) {
  const response = await fetch(`http://localhost:8080/api/v1/tasks/${taskId}/execute`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ input })
  });
  
  return response.json();
}

// Using GraphQL
async function queryTasks() {
  const response = await fetch('http://localhost:8080/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      query: `
        query {
          tasks {
            items { id name version }
            total
          }
        }
      `
    })
  });
  
  return response.json();
}
```

### Python

```python
import requests
import json

class RatchetClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url
        self.session = requests.Session()
    
    def execute_task(self, task_id, input_data):
        url = f"{self.base_url}/api/v1/tasks/{task_id}/execute"
        response = self.session.post(url, json={"input": input_data})
        response.raise_for_status()
        return response.json()
    
    def list_jobs(self, status=None, limit=10):
        url = f"{self.base_url}/api/v1/jobs"
        params = {"limit": limit}
        if status:
            params["status"] = status
        response = self.session.get(url, params=params)
        response.raise_for_status()
        return response.json()

# Usage
client = RatchetClient()
result = client.execute_task("weather-api", {"city": "Paris"})
print(result)
```

### Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
)

type RatchetClient struct {
    BaseURL string
    Client  *http.Client
}

func NewRatchetClient(baseURL string) *RatchetClient {
    return &RatchetClient{
        BaseURL: baseURL,
        Client:  &http.Client{},
    }
}

func (c *RatchetClient) ExecuteTask(taskID string, input map[string]interface{}) (map[string]interface{}, error) {
    url := fmt.Sprintf("%s/api/v1/tasks/%s/execute", c.BaseURL, taskID)
    
    payload := map[string]interface{}{"input": input}
    jsonData, err := json.Marshal(payload)
    if err != nil {
        return nil, err
    }
    
    resp, err := c.Client.Post(url, "application/json", bytes.NewBuffer(jsonData))
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    var result map[string]interface{}
    err = json.NewDecoder(resp.Body).Decode(&result)
    return result, err
}
```

## Webhook Integrations

### Receiving Webhooks

Create tasks that process incoming webhooks:

```javascript
// webhook-processor.js
(function(input) {
    // Parse webhook payload
    const { headers, body, method, path } = input.webhook;
    
    // Validate webhook signature (if applicable)
    if (headers['x-webhook-signature']) {
        const expectedSignature = generateSignature(body, input.webhook_secret);
        if (headers['x-webhook-signature'] !== expectedSignature) {
            throw new ValidationError("Invalid webhook signature");
        }
    }
    
    // Process webhook based on type
    switch (headers['x-event-type']) {
        case 'payment.completed':
            return processPayment(body);
        case 'user.created':
            return processNewUser(body);
        default:
            throw new ValidationError(`Unknown event type: ${headers['x-event-type']}`);
    }
})
```

### Sending Webhooks

Configure output destinations to send webhooks:

```yaml
output:
  global_destinations:
    - name: "order_webhook"
      destination:
        type: webhook
        url: "https://partner-api.com/webhooks/orders"
        method: POST
        headers:
          "X-Webhook-Event": "order.processed"
          "X-Webhook-Signature": "{{signature}}"
        auth:
          type: bearer
          token: "${PARTNER_API_TOKEN}"
```

## Message Queue Integrations

### RabbitMQ Integration

Process messages from RabbitMQ:

```javascript
// rabbitmq-consumer.js
(function(input) {
    const { queue_name, message, headers } = input;
    
    // Parse message based on content type
    let parsedMessage;
    if (headers['content-type'] === 'application/json') {
        parsedMessage = JSON.parse(message);
    } else {
        parsedMessage = message;
    }
    
    // Process message
    const result = processMessage(parsedMessage);
    
    // Return acknowledgment
    return {
        ack: true,
        result: result,
        processed_at: new Date().toISOString()
    };
})
```

### Kafka Integration

Consume Kafka events:

```javascript
// kafka-event-processor.js
(function(input) {
    const { topic, partition, offset, key, value, headers } = input.kafka_event;
    
    // Process event based on topic
    switch (topic) {
        case 'user-events':
            return processUserEvent(value);
        case 'order-events':
            return processOrderEvent(value);
        default:
            throw new Error(`Unknown topic: ${topic}`);
    }
})
```

## Database Integrations

### Direct Database Access

While Ratchet uses SQLite internally, tasks can integrate with external databases:

```javascript
// database-sync.js
(function(input) {
    // Fetch data from external API representing database
    const users = fetch('https://api.example.com/users', {
        headers: {
            'Authorization': `Bearer ${input.api_token}`
        }
    }).body;
    
    // Transform data
    const transformedUsers = users.map(user => ({
        id: user.id,
        name: user.full_name,
        email: user.email_address,
        last_sync: new Date().toISOString()
    }));
    
    // Return for storage in Ratchet or forwarding
    return {
        synced_count: transformedUsers.length,
        users: transformedUsers
    };
})
```

## CI/CD Integrations

### GitHub Actions

```yaml
# .github/workflows/ratchet-task.yml
name: Execute Ratchet Task

on:
  push:
    branches: [ main ]
  schedule:
    - cron: '0 0 * * *'

jobs:
  execute-task:
    runs-on: ubuntu-latest
    steps:
    - name: Execute Ratchet Task
      run: |
        curl -X POST https://ratchet.example.com/api/v1/tasks/daily-report/execute \
          -H "Content-Type: application/json" \
          -H "Authorization: Bearer ${{ secrets.RATCHET_TOKEN }}" \
          -d '{
            "input": {
              "environment": "production",
              "date": "'$(date +%Y-%m-%d)'"
            }
          }'
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any
    
    stages {
        stage('Execute Ratchet Task') {
            steps {
                script {
                    def response = httpRequest(
                        url: 'https://ratchet.example.com/api/v1/tasks/build-report/execute',
                        httpMode: 'POST',
                        contentType: 'APPLICATION_JSON',
                        customHeaders: [[name: 'Authorization', value: "Bearer ${env.RATCHET_TOKEN}"]],
                        requestBody: """{
                            "input": {
                                "build_number": "${env.BUILD_NUMBER}",
                                "branch": "${env.BRANCH_NAME}"
                            }
                        }"""
                    )
                    
                    def result = readJSON text: response.content
                    echo "Task executed: ${result.job.id}"
                }
            }
        }
    }
}
```

## Monitoring Integrations

### Prometheus Metrics

Export metrics for Prometheus:

```javascript
// metrics-exporter.js
(function(input) {
    // Fetch Ratchet metrics
    const jobs = fetch('http://localhost:8080/api/v1/jobs?limit=1000').body;
    const tasks = fetch('http://localhost:8080/api/v1/tasks').body;
    
    // Calculate metrics
    const metrics = {
        ratchet_total_tasks: tasks.total,
        ratchet_jobs_by_status: {},
        ratchet_job_duration_seconds: []
    };
    
    // Group jobs by status
    jobs.items.forEach(job => {
        metrics.ratchet_jobs_by_status[job.status] = 
            (metrics.ratchet_jobs_by_status[job.status] || 0) + 1;
        
        if (job.completed_at && job.created_at) {
            const duration = (new Date(job.completed_at) - new Date(job.created_at)) / 1000;
            metrics.ratchet_job_duration_seconds.push(duration);
        }
    });
    
    // Format as Prometheus metrics
    let output = '';
    output += `# HELP ratchet_total_tasks Total number of tasks\n`;
    output += `# TYPE ratchet_total_tasks gauge\n`;
    output += `ratchet_total_tasks ${metrics.ratchet_total_tasks}\n\n`;
    
    output += `# HELP ratchet_jobs_total Total number of jobs by status\n`;
    output += `# TYPE ratchet_jobs_total counter\n`;
    Object.entries(metrics.ratchet_jobs_by_status).forEach(([status, count]) => {
        output += `ratchet_jobs_total{status="${status}"} ${count}\n`;
    });
    
    return {
        metrics: output,
        format: "prometheus"
    };
})
```

### Datadog Integration

Send metrics to Datadog:

```javascript
// datadog-metrics.js
(function(input) {
    const executions = fetch('http://localhost:8080/api/v1/executions?limit=100').body;
    
    const metrics = executions.items.map(exec => ({
        metric: "ratchet.execution.duration",
        points: [[
            Math.floor(new Date(exec.created_at).getTime() / 1000),
            exec.duration_ms / 1000
        ]],
        tags: [
            `task:${exec.task_id}`,
            `status:${exec.status}`,
            `environment:${input.environment}`
        ]
    }));
    
    // Send to Datadog API
    const response = fetch('https://api.datadoghq.com/api/v1/series', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'DD-API-KEY': input.datadog_api_key
        },
        body: JSON.stringify({ series: metrics })
    });
    
    return {
        metrics_sent: metrics.length,
        status: response.status
    };
})
```

## OpenAPI Integration

### Generate Client Code

Use the OpenAPI specification to generate client libraries:

```bash
# Download OpenAPI spec
curl -o ratchet-openapi.yaml http://localhost:8080/openapi.yaml

# Generate Python client
openapi-generator generate \
  -i ratchet-openapi.yaml \
  -g python \
  -o ./ratchet-python-client

# Generate TypeScript client
openapi-generator generate \
  -i ratchet-openapi.yaml \
  -g typescript-axios \
  -o ./ratchet-ts-client
```

### Import into API Gateways

Import the OpenAPI spec into API management platforms:
- AWS API Gateway
- Azure API Management
- Kong
- Tyk

## Best Practices

### 1. Error Handling

Always handle API errors gracefully:

```javascript
try {
    const result = await ratchetClient.executeTask(taskId, input);
    // Process result
} catch (error) {
    if (error.status === 429) {
        // Handle rate limiting
        await sleep(error.retryAfter * 1000);
        return retry();
    } else if (error.status === 404) {
        // Handle not found
        console.error(`Task ${taskId} not found`);
    } else {
        // Handle other errors
        throw error;
    }
}
```

### 2. Pagination

Use pagination for large result sets:

```javascript
async function* getAllJobs(client) {
    let offset = 0;
    const limit = 100;
    
    while (true) {
        const response = await client.get('/api/v1/jobs', {
            params: { limit, offset }
        });
        
        yield* response.data.items;
        
        if (response.data.items.length < limit) {
            break;
        }
        
        offset += limit;
    }
}
```

### 3. Rate Limiting

Respect rate limits:

```javascript
class RateLimitedClient {
    constructor(baseURL, requestsPerMinute = 60) {
        this.baseURL = baseURL;
        this.requestsPerMinute = requestsPerMinute;
        this.requestTimes = [];
    }
    
    async request(method, path, data) {
        await this.waitForRateLimit();
        
        const response = await fetch(`${this.baseURL}${path}`, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });
        
        this.requestTimes.push(Date.now());
        return response;
    }
    
    async waitForRateLimit() {
        const now = Date.now();
        const oneMinuteAgo = now - 60000;
        
        this.requestTimes = this.requestTimes.filter(time => time > oneMinuteAgo);
        
        if (this.requestTimes.length >= this.requestsPerMinute) {
            const oldestRequest = this.requestTimes[0];
            const waitTime = 60000 - (now - oldestRequest);
            await new Promise(resolve => setTimeout(resolve, waitTime));
        }
    }
}
```

## Next Steps

- Review [Logging & Error Handling]({{ "/logging-error-handling" | relative_url }}) for debugging integrations
- Check [Server Configuration]({{ "/server-configuration" | relative_url }}) for API settings
- Explore [Example Uses]({{ "/examples" | relative_url }}) for integration patterns