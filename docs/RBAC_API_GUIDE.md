# Ratchet RBAC API Guide

This guide provides comprehensive examples for using Ratchet's RBAC-protected APIs, including authentication methods, authorization patterns, and practical usage scenarios.

## Table of Contents

1. [Authentication Methods](#authentication-methods)
2. [API Endpoints](#api-endpoints)
3. [User Management](#user-management)
4. [Role Management](#role-management)
5. [Tenant Management](#tenant-management)
6. [Resource Access Examples](#resource-access-examples)
7. [Error Handling](#error-handling)
8. [SDK Examples](#sdk-examples)

## Authentication Methods

### JWT Token Authentication

#### Login and Get Token
```bash
# Login to get JWT token
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "developer",
    "password": "secure-password"
  }'
```

Response:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600,
    "user": {
      "id": "123",
      "username": "developer",
      "email": "dev@example.com",
      "roles": ["developer"],
      "tenant_id": 1
    }
  }
}
```

#### Using JWT Token
```bash
# Use JWT token in subsequent requests
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  http://localhost:8080/api/v1/tasks
```

### API Key Authentication

#### Create API Key
```bash
# Create API key (requires authentication)
curl -X POST http://localhost:8080/api/v1/auth/api-keys \
  -H "Authorization: Bearer <jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "CI/CD Pipeline",
    "description": "API key for automated deployments",
    "permissions": ["tasks:read", "tasks:execute", "executions:read"],
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

Response:
```json
{
  "success": true,
  "data": {
    "key": "rk_live_1234567890abcdef1234567890abcdef",
    "id": "api_key_456",
    "name": "CI/CD Pipeline",
    "prefix": "rk_live_",
    "permissions": ["tasks:read", "tasks:execute", "executions:read"],
    "created_at": "2024-01-15T10:30:00Z",
    "expires_at": "2024-12-31T23:59:59Z"
  }
}
```

#### Using API Key
```bash
# Use API key for authentication
curl -H "X-API-Key: rk_live_1234567890abcdef1234567890abcdef" \
  http://localhost:8080/api/v1/tasks
```

### Session Authentication

#### Login with Session
```bash
# Login to create session
curl -X POST http://localhost:8080/api/v1/auth/session \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{
    "username": "developer",
    "password": "secure-password"
  }'
```

#### Using Session Cookie
```bash
# Use session cookie for subsequent requests
curl -b cookies.txt http://localhost:8080/api/v1/tasks
```

## API Endpoints

### Authentication Endpoints

#### POST /api/v1/auth/login
Authenticate user and get JWT token.

**Request:**
```json
{
  "username": "string",
  "password": "string",
  "tenant_id": "integer (optional)"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "token": "string",
    "expires_in": "integer",
    "user": {
      "id": "string",
      "username": "string",
      "email": "string",
      "roles": ["string"],
      "tenant_id": "integer"
    }
  }
}
```

#### POST /api/v1/auth/logout
Invalidate current session or token.

```bash
curl -X POST http://localhost:8080/api/v1/auth/logout \
  -H "Authorization: Bearer <jwt-token>"
```

#### GET /api/v1/auth/me
Get current user information.

```bash
curl -H "Authorization: Bearer <jwt-token>" \
  http://localhost:8080/api/v1/auth/me
```

Response:
```json
{
  "success": true,
  "data": {
    "id": "123",
    "username": "developer",
    "email": "dev@example.com",
    "roles": [
      {
        "name": "developer",
        "tenant_id": 1,
        "permissions": ["tasks:read", "tasks:create", "tasks:execute"]
      }
    ],
    "tenant_id": 1,
    "last_login_at": "2024-01-15T10:30:00Z"
  }
}
```

#### POST /api/v1/auth/change-password
Change user password.

```bash
curl -X POST http://localhost:8080/api/v1/auth/change-password \
  -H "Authorization: Bearer <jwt-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "old-password",
    "new_password": "new-secure-password"
  }'
```

## User Management

### Create User

```bash
# Create new user (requires user:create permission)
curl -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "email": "newuser@example.com",
    "password": "secure-password",
    "roles": ["developer"],
    "tenant_id": 1
  }'
```

### List Users

```bash
# List users (requires user:read permission)
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/users?tenant_id=1&limit=20&offset=0"
```

### Update User

```bash
# Update user (requires user:update permission)
curl -X PATCH http://localhost:8080/api/v1/users/123 \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "updated@example.com",
    "is_active": true
  }'
```

### Assign Role to User

```bash
# Assign role (requires user:manage permission)
curl -X POST http://localhost:8080/api/v1/users/123/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "role_name": "operator",
    "tenant_id": 1
  }'
```

### Remove Role from User

```bash
# Remove role (requires user:manage permission)
curl -X DELETE http://localhost:8080/api/v1/users/123/roles/operator \
  -H "Authorization: Bearer <admin-token>" \
  -d '{
    "tenant_id": 1
  }'
```

## Role Management

### List Available Roles

```bash
# List roles (requires role:read permission)
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/roles?tenant_id=1"
```

Response:
```json
{
  "success": true,
  "data": {
    "roles": [
      {
        "name": "admin",
        "display_name": "Administrator",
        "description": "Full administrative access",
        "permissions": ["*:*"],
        "is_platform_role": false,
        "tenant_id": 1
      },
      {
        "name": "developer",
        "display_name": "Developer",
        "description": "Task development and execution",
        "permissions": [
          "tasks:read", "tasks:create", "tasks:execute",
          "executions:read", "jobs:read"
        ],
        "is_platform_role": false,
        "tenant_id": 1
      }
    ]
  }
}
```

### Create Custom Role

```bash
# Create custom role (requires role:create permission)
curl -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "task_executor",
    "display_name": "Task Executor",
    "description": "Can only execute existing tasks",
    "permissions": [
      "tasks:read",
      "tasks:execute",
      "executions:read"
    ],
    "tenant_id": 1
  }'
```

### Update Role

```bash
# Update role (requires role:update permission)
curl -X PATCH http://localhost:8080/api/v1/roles/task_executor \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "Task Executor Plus",
    "permissions": [
      "tasks:read",
      "tasks:execute",
      "executions:read",
      "jobs:read"
    ]
  }'
```

### Delete Role

```bash
# Delete role (requires role:delete permission)
curl -X DELETE http://localhost:8080/api/v1/roles/task_executor \
  -H "Authorization: Bearer <admin-token>"
```

## Tenant Management

### Create Tenant

```bash
# Create tenant (requires tenant:create permission)
curl -X POST http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer <platform-admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "marketing_team",
    "display_name": "Marketing Team",
    "description": "Marketing automation and campaigns",
    "settings": {
      "max_users": 50,
      "max_tasks": 1000
    }
  }'
```

### List Tenants

```bash
# List tenants (requires tenant:read permission)
curl -H "Authorization: Bearer <platform-admin-token>" \
  http://localhost:8080/api/v1/tenants
```

### Add User to Tenant

```bash
# Add user to tenant (requires tenant:manage permission)
curl -X POST http://localhost:8080/api/v1/tenants/2/users \
  -H "Authorization: Bearer <tenant-admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": 123,
    "role": "developer"
  }'
```

### Remove User from Tenant

```bash
# Remove user from tenant (requires tenant:manage permission)
curl -X DELETE http://localhost:8080/api/v1/tenants/2/users/123 \
  -H "Authorization: Bearer <tenant-admin-token>"
```

## Resource Access Examples

### Tasks

#### Create Task
```bash
# Create task (requires tasks:create permission)
curl -X POST http://localhost:8080/api/v1/tasks \
  -H "Authorization: Bearer <developer-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "data_processing",
    "description": "Process customer data",
    "content": "console.log(\"Processing data...\");",
    "type": "javascript",
    "enabled": true
  }'
```

#### List Tasks
```bash
# List tasks (requires tasks:read permission)
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/tasks?limit=10&offset=0"
```

#### Execute Task
```bash
# Execute task (requires tasks:execute permission)
curl -X POST http://localhost:8080/api/v1/tasks/task_123/execute \
  -H "Authorization: Bearer <developer-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "customer_id": "12345",
      "process_type": "standard"
    }
  }'
```

### Executions

#### List Executions
```bash
# List executions (requires executions:read permission)
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/executions?status=running&limit=20"
```

#### Get Execution Details
```bash
# Get execution (requires executions:read permission)
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/executions/exec_456
```

#### Cancel Execution
```bash
# Cancel execution (requires executions:update permission)
curl -X POST http://localhost:8080/api/v1/executions/exec_456/cancel \
  -H "Authorization: Bearer <operator-token>"
```

### Jobs

#### Create Job
```bash
# Create job (requires jobs:create permission)
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Authorization: Bearer <developer-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "daily_report",
    "task_id": "task_123",
    "input": {
      "report_type": "daily",
      "recipients": ["admin@example.com"]
    },
    "priority": "normal"
  }'
```

#### List Jobs
```bash
# List jobs (requires jobs:read permission)
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/jobs?status=pending&limit=20"
```

### Schedules

#### Create Schedule
```bash
# Create schedule (requires schedules:create permission)
curl -X POST http://localhost:8080/api/v1/schedules \
  -H "Authorization: Bearer <developer-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hourly_sync",
    "task_id": "task_123",
    "cron_expression": "0 * * * *",
    "input": {
      "sync_type": "incremental"
    },
    "enabled": true
  }'
```

## Error Handling

### Authentication Errors

#### 401 Unauthorized
```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Authentication required",
    "details": "No valid authentication credentials provided"
  }
}
```

#### 403 Forbidden
```json
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Insufficient permissions",
    "details": "User lacks 'tasks:create' permission for tenant 1"
  }
}
```

### Permission Errors

#### Missing Permission
```json
{
  "success": false,
  "error": {
    "code": "PERMISSION_DENIED",
    "message": "Access denied",
    "details": "Required permission 'users:delete' not granted",
    "required_permission": "users:delete",
    "user_permissions": ["users:read", "users:update"]
  }
}
```

#### Tenant Access Denied
```json
{
  "success": false,
  "error": {
    "code": "TENANT_ACCESS_DENIED",
    "message": "Access to tenant denied",
    "details": "User not a member of tenant 2",
    "tenant_id": 2
  }
}
```

## SDK Examples

### JavaScript/Node.js

```javascript
const RatchetClient = require('@ratchet/client');

// Initialize client with JWT token
const client = new RatchetClient({
  baseURL: 'http://localhost:8080',
  auth: {
    type: 'jwt',
    token: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...'
  }
});

// Create task
async function createTask() {
  try {
    const task = await client.tasks.create({
      name: 'email_notification',
      content: 'console.log("Sending email...");',
      type: 'javascript'
    });
    console.log('Task created:', task.id);
  } catch (error) {
    if (error.code === 'FORBIDDEN') {
      console.error('Permission denied:', error.details);
    } else {
      console.error('Error:', error.message);
    }
  }
}

// Execute task
async function executeTask(taskId, input) {
  try {
    const execution = await client.tasks.execute(taskId, input);
    console.log('Execution started:', execution.id);
    
    // Poll for completion
    const result = await client.executions.wait(execution.id);
    console.log('Execution completed:', result.status);
  } catch (error) {
    console.error('Execution failed:', error.message);
  }
}
```

### Python

```python
from ratchet_client import RatchetClient, AuthenticationError, PermissionError

# Initialize client with API key
client = RatchetClient(
    base_url='http://localhost:8080',
    api_key='rk_live_1234567890abcdef1234567890abcdef'
)

# Create and execute task
try:
    # Create task
    task = client.tasks.create(
        name='data_analysis',
        content='print("Analyzing data...")',
        type='python'
    )
    print(f'Task created: {task.id}')
    
    # Execute task
    execution = client.tasks.execute(
        task.id,
        input={'dataset': 'customer_data.csv'}
    )
    print(f'Execution started: {execution.id}')
    
    # Wait for completion
    result = client.executions.wait(execution.id, timeout=300)
    print(f'Execution completed: {result.status}')
    
except PermissionError as e:
    print(f'Permission denied: {e.required_permission}')
except AuthenticationError as e:
    print(f'Authentication failed: {e}')
except Exception as e:
    print(f'Error: {e}')
```

### cURL Script

```bash
#!/bin/bash

# Configuration
API_BASE="http://localhost:8080/api/v1"
JWT_TOKEN="your-jwt-token-here"

# Helper function for authenticated requests
api_call() {
    local method=$1
    local endpoint=$2
    local data=$3
    
    if [ -n "$data" ]; then
        curl -X "$method" \
            -H "Authorization: Bearer $JWT_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$API_BASE$endpoint"
    else
        curl -X "$method" \
            -H "Authorization: Bearer $JWT_TOKEN" \
            "$API_BASE$endpoint"
    fi
}

# Create task
create_task() {
    local task_data='{
        "name": "batch_process",
        "description": "Batch processing task",
        "content": "console.log(\"Processing batch...\");",
        "type": "javascript"
    }'
    
    api_call POST "/tasks" "$task_data"
}

# List tasks
list_tasks() {
    api_call GET "/tasks?limit=10"
}

# Execute task
execute_task() {
    local task_id=$1
    local input_data='{
        "batch_size": 100,
        "source": "database"
    }'
    
    api_call POST "/tasks/$task_id/execute" "$input_data"
}

# Main execution
echo "Creating task..."
task_response=$(create_task)
task_id=$(echo "$task_response" | jq -r '.data.id')

echo "Task created with ID: $task_id"

echo "Executing task..."
execution_response=$(execute_task "$task_id")
execution_id=$(echo "$execution_response" | jq -r '.data.id')

echo "Execution started with ID: $execution_id"
```

## Best Practices

1. **Token Management**: Store tokens securely and implement refresh logic
2. **Error Handling**: Always check for authentication and authorization errors
3. **Permission Checking**: Verify permissions before attempting operations
4. **Tenant Context**: Ensure requests are made in the correct tenant context
5. **Rate Limiting**: Implement backoff strategies for rate-limited operations
6. **Logging**: Log authentication and authorization events for debugging
7. **Security**: Use HTTPS in production and validate SSL certificates

For more advanced examples and integration patterns, see the [Integration Examples](RBAC_INTEGRATION_EXAMPLES.md) guide.