# Ratchet RBAC Role Management Guide

This guide covers creating, managing, and customizing roles in Ratchet's RBAC system, including built-in roles, custom role creation, and advanced permission patterns.

## Table of Contents

1. [Role Concepts](#role-concepts)
2. [Built-in Roles](#built-in-roles)
3. [Custom Role Creation](#custom-role-creation)
4. [Permission System](#permission-system)
5. [Role Inheritance](#role-inheritance)
6. [Tenant-Specific Roles](#tenant-specific-roles)
7. [Role Management API](#role-management-api)
8. [Common Patterns](#common-patterns)
9. [Best Practices](#best-practices)

## Role Concepts

### Role Types

Ratchet supports two types of roles:

1. **Platform Roles**: System-wide roles that provide access across all tenants
2. **Tenant Roles**: Roles specific to individual tenants with scoped access

### Role Structure

```json
{
  "name": "role_identifier",
  "display_name": "Human Readable Name",
  "description": "Role description",
  "permissions": ["resource:action", "..."],
  "inherits_from": ["parent_role"],
  "is_platform_role": false,
  "tenant_id": 1
}
```

### Permission Format

Permissions follow the format `{resource}:{action}`:
- `tasks:read` - Read task information
- `executions:create` - Create new executions
- `users:manage` - Full user management capabilities
- `*:*` - All permissions (superuser)

## Built-in Roles

### Platform Roles

#### Platform Administrator
**Scope**: Global access across all tenants

```json
{
  "name": "platform_admin",
  "display_name": "Platform Administrator",
  "description": "Full administrative access across all tenants",
  "permissions": ["*:*"],
  "is_platform_role": true,
  "tenant_id": null
}
```

**Use Cases**:
- System administrators
- DevOps teams
- Platform maintenance

**Permissions**:
- All operations on all resources
- Cross-tenant access
- System configuration
- User and tenant management

#### Platform Operator
**Scope**: Read-only monitoring across all tenants

```json
{
  "name": "platform_operator",
  "display_name": "Platform Operator",
  "description": "Read-only monitoring and system health access",
  "permissions": [
    "metrics:read",
    "configurations:read",
    "tasks:read",
    "executions:read",
    "jobs:read",
    "schedules:read"
  ],
  "is_platform_role": true,
  "tenant_id": null
}
```

**Use Cases**:
- Monitoring teams
- Support staff
- System health checking

### Tenant Roles

#### Tenant Administrator
**Scope**: Full administrative access within a single tenant

```json
{
  "name": "admin",
  "display_name": "Administrator",
  "description": "Full administrative access within the tenant",
  "permissions": [
    "tasks:*",
    "executions:*",
    "jobs:*",
    "schedules:*",
    "users:*",
    "roles:*"
  ],
  "is_platform_role": false,
  "tenant_id": 1
}
```

**Use Cases**:
- Team leads
- Project managers
- Department administrators

#### Developer
**Scope**: Development and execution capabilities

```json
{
  "name": "developer",
  "display_name": "Developer",
  "description": "Task development, execution, and monitoring",
  "permissions": [
    "tasks:create",
    "tasks:read",
    "tasks:update",
    "tasks:execute",
    "executions:read",
    "executions:cancel",
    "jobs:create",
    "jobs:read",
    "jobs:update",
    "schedules:create",
    "schedules:read",
    "schedules:update"
  ],
  "is_platform_role": false,
  "tenant_id": 1
}
```

**Use Cases**:
- Software developers
- Automation engineers
- DevOps engineers

#### Operator
**Scope**: Execution and monitoring capabilities

```json
{
  "name": "operator",
  "display_name": "Operator",
  "description": "Task execution and system monitoring",
  "permissions": [
    "tasks:read",
    "tasks:execute",
    "executions:read",
    "executions:cancel",
    "jobs:read",
    "jobs:execute",
    "schedules:read",
    "metrics:read"
  ],
  "is_platform_role": false,
  "tenant_id": 1
}
```

**Use Cases**:
- Operations teams
- CI/CD systems
- Production operators

#### Viewer
**Scope**: Read-only access to resources

```json
{
  "name": "viewer",
  "display_name": "Viewer",
  "description": "Read-only access to tasks, jobs, and executions",
  "permissions": [
    "tasks:read",
    "executions:read",
    "jobs:read",
    "schedules:read",
    "metrics:read"
  ],
  "is_platform_role": false,
  "tenant_id": 1
}
```

**Use Cases**:
- Stakeholders
- Reporting systems
- External auditors

## Custom Role Creation

### API-Based Role Creation

#### Create Custom Role
```bash
curl -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "data_scientist",
    "display_name": "Data Scientist",
    "description": "Execute data analysis tasks and view results",
    "permissions": [
      "tasks:read",
      "tasks:execute",
      "executions:read",
      "jobs:read",
      "metrics:read"
    ],
    "tenant_id": 1
  }'
```

#### Create Role with Inheritance
```bash
curl -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "senior_developer",
    "display_name": "Senior Developer",
    "description": "Extended developer permissions with user management",
    "inherits_from": ["developer"],
    "permissions": [
      "users:read",
      "users:create",
      "roles:read"
    ],
    "tenant_id": 1
  }'
```

### Configuration-Based Role Creation

Add custom roles to your configuration file:

```yaml
# config.yaml
rbac:
  custom_roles:
    - name: "task_reviewer"
      display_name: "Task Reviewer"
      description: "Review and approve task changes"
      tenant_id: 1
      permissions:
        - "tasks:read"
        - "tasks:update"
        - "executions:read"
      conditions:
        - "task.status == 'pending_review'"
    
    - name: "api_client"
      display_name: "API Client"
      description: "Programmatic access for external systems"
      tenant_id: 1
      permissions:
        - "tasks:read"
        - "tasks:execute"
        - "executions:read"
      rate_limits:
        requests_per_minute: 100
```

### CLI-Based Role Creation

```bash
# Create role using CLI
ratchet role create \
  --name "backup_operator" \
  --display-name "Backup Operator" \
  --description "Backup and restore operations" \
  --tenant-id 1 \
  --permissions "tasks:read,executions:read,jobs:read" \
  --inherits-from "viewer"

# List roles
ratchet role list --tenant-id 1

# Update role
ratchet role update backup_operator \
  --add-permission "schedules:read" \
  --remove-permission "jobs:read"
```

## Permission System

### Resource Types

| Resource | Description | Examples |
|----------|-------------|----------|
| `tasks` | Task definitions and templates | `tasks:create`, `tasks:execute` |
| `executions` | Task execution instances | `executions:read`, `executions:cancel` |
| `jobs` | Job definitions and instances | `jobs:create`, `jobs:execute` |
| `schedules` | Scheduled task configurations | `schedules:create`, `schedules:update` |
| `users` | User account management | `users:create`, `users:delete` |
| `roles` | Role definitions | `roles:create`, `roles:assign` |
| `tenants` | Tenant management | `tenants:create`, `tenants:delete` |
| `metrics` | System metrics and monitoring | `metrics:read`, `metrics:export` |
| `configurations` | System configuration | `configurations:read`, `configurations:update` |
| `api_keys` | API key management | `api_keys:create`, `api_keys:revoke` |
| `sessions` | Session management | `sessions:read`, `sessions:invalidate` |

### Action Types

| Action | Description | Scope |
|--------|-------------|-------|
| `create` | Create new resources | Write operation |
| `read` | View resource information | Read operation |
| `update` | Modify existing resources | Write operation |
| `delete` | Remove resources | Destructive operation |
| `execute` | Execute tasks or jobs | Action operation |
| `manage` | Full administrative access | Administrative operation |
| `list` | List resources with filtering | Read operation |

### Wildcard Permissions

Use wildcards for broader permissions:

```json
{
  "permissions": [
    "tasks:*",        // All task operations
    "*:read",         // Read all resources
    "*:*"             // All operations (superuser)
  ]
}
```

### Conditional Permissions

Add conditions to permissions for fine-grained control:

```json
{
  "name": "conditional_role",
  "permissions": [
    {
      "resource": "tasks",
      "action": "execute",
      "conditions": [
        "task.owner_id == user.id",
        "task.environment == 'development'"
      ]
    }
  ]
}
```

## Role Inheritance

### Basic Inheritance

```json
{
  "name": "senior_developer",
  "display_name": "Senior Developer",
  "inherits_from": ["developer"],
  "permissions": [
    "users:read",
    "roles:read"
  ]
}
```

The `senior_developer` role inherits all permissions from `developer` and adds additional permissions.

### Multiple Inheritance

```json
{
  "name": "team_lead",
  "display_name": "Team Lead",
  "inherits_from": ["developer", "operator"],
  "permissions": [
    "users:manage",
    "roles:assign"
  ]
}
```

### Inheritance Chain

```
viewer → developer → senior_developer → team_lead
```

Each role in the chain inherits permissions from its parent and can add additional permissions.

## Tenant-Specific Roles

### Creating Tenant-Specific Roles

```bash
# Create role for tenant 1
curl -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "marketing_analyst",
    "display_name": "Marketing Analyst",
    "description": "Analyze marketing campaign data",
    "tenant_id": 1,
    "permissions": [
      "tasks:read",
      "tasks:execute",
      "executions:read",
      "metrics:read"
    ]
  }'

# Create role for tenant 2
curl -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sales_operator",
    "display_name": "Sales Operator",
    "description": "Manage sales automation tasks",
    "tenant_id": 2,
    "permissions": [
      "tasks:read",
      "tasks:execute",
      "jobs:create",
      "jobs:read"
    ]
  }'
```

### Cross-Tenant Role Templates

Create role templates that can be applied across tenants:

```yaml
# role_templates.yaml
templates:
  data_analyst:
    display_name: "Data Analyst"
    description: "Analyze data and generate reports"
    permissions:
      - "tasks:read"
      - "tasks:execute"
      - "executions:read"
      - "metrics:read"
    
  automation_engineer:
    display_name: "Automation Engineer"
    description: "Develop and maintain automation workflows"
    permissions:
      - "tasks:create"
      - "tasks:read"
      - "tasks:update"
      - "tasks:execute"
      - "schedules:create"
      - "schedules:read"
      - "schedules:update"
```

Apply templates to tenants:

```bash
# Apply template to tenant
ratchet role apply-template \
  --template data_analyst \
  --tenant-id 1 \
  --name "marketing_data_analyst"
```

## Role Management API

### List Roles

```bash
# List all roles
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/roles

# List tenant-specific roles
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/roles?tenant_id=1"

# List platform roles
curl -H "Authorization: Bearer <token>" \
  "http://localhost:8080/api/v1/roles?platform=true"
```

### Get Role Details

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/roles/developer
```

Response:
```json
{
  "success": true,
  "data": {
    "name": "developer",
    "display_name": "Developer",
    "description": "Task development and execution",
    "permissions": [
      "tasks:create",
      "tasks:read",
      "tasks:update",
      "tasks:execute",
      "executions:read",
      "jobs:create",
      "jobs:read"
    ],
    "inherits_from": [],
    "is_platform_role": false,
    "tenant_id": 1,
    "user_count": 15,
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-20T14:20:00Z"
  }
}
```

### Update Role

```bash
curl -X PATCH http://localhost:8080/api/v1/roles/developer \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Enhanced developer role with additional permissions",
    "permissions": [
      "tasks:create",
      "tasks:read",
      "tasks:update",
      "tasks:execute",
      "executions:read",
      "executions:cancel",
      "jobs:create",
      "jobs:read",
      "jobs:update",
      "schedules:read"
    ]
  }'
```

### Delete Role

```bash
curl -X DELETE http://localhost:8080/api/v1/roles/custom_role \
  -H "Authorization: Bearer <admin-token>"
```

### Assign Role to User

```bash
curl -X POST http://localhost:8080/api/v1/users/123/roles \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "role_name": "developer",
    "tenant_id": 1
  }'
```

### Remove Role from User

```bash
curl -X DELETE http://localhost:8080/api/v1/users/123/roles/developer \
  -H "Authorization: Bearer <admin-token>" \
  -d '{
    "tenant_id": 1
  }'
```

## Common Patterns

### Department-Specific Roles

```json
{
  "name": "finance_analyst",
  "display_name": "Finance Analyst",
  "permissions": [
    "tasks:read",
    "tasks:execute",
    "executions:read"
  ],
  "conditions": [
    "task.tags.includes('finance')",
    "task.category == 'reporting'"
  ]
}
```

### Time-Based Roles

```json
{
  "name": "business_hours_operator",
  "display_name": "Business Hours Operator",
  "permissions": [
    "tasks:execute",
    "jobs:create"
  ],
  "conditions": [
    "current_time >= '09:00'",
    "current_time <= '17:00'",
    "current_day in ['monday', 'tuesday', 'wednesday', 'thursday', 'friday']"
  ]
}
```

### Environment-Specific Roles

```json
{
  "name": "staging_developer",
  "display_name": "Staging Developer",
  "permissions": [
    "tasks:create",
    "tasks:update",
    "tasks:execute"
  ],
  "conditions": [
    "environment == 'staging'",
    "task.production_ready == false"
  ]
}
```

### Resource Owner Roles

```json
{
  "name": "task_owner",
  "display_name": "Task Owner",
  "permissions": [
    "tasks:read",
    "tasks:update",
    "tasks:delete",
    "executions:read"
  ],
  "conditions": [
    "resource.owner_id == user.id"
  ]
}
```

## Best Practices

### 1. Principle of Least Privilege
Grant the minimum permissions necessary for each role:

```json
// Good: Specific permissions
{
  "name": "report_generator",
  "permissions": [
    "tasks:execute",
    "executions:read"
  ]
}

// Avoid: Overly broad permissions
{
  "name": "report_generator",
  "permissions": ["*:*"]
}
```

### 2. Use Descriptive Names
Choose clear, descriptive names for roles:

```json
// Good: Clear purpose
{
  "name": "marketing_campaign_manager",
  "display_name": "Marketing Campaign Manager"
}

// Avoid: Ambiguous names
{
  "name": "user1",
  "display_name": "User Type 1"
}
```

### 3. Leverage Inheritance
Use role inheritance to reduce duplication:

```json
{
  "name": "senior_operator",
  "inherits_from": ["operator"],
  "permissions": [
    "schedules:create",
    "schedules:update"
  ]
}
```

### 4. Document Role Purposes
Provide clear descriptions for each role:

```json
{
  "name": "data_processor",
  "display_name": "Data Processor",
  "description": "Executes data processing tasks, monitors execution status, and generates processing reports. Cannot modify task definitions or system configuration."
}
```

### 5. Regular Permission Audits
Periodically review role permissions:

```bash
# Generate role audit report
curl -H "Authorization: Bearer <admin-token>" \
  "http://localhost:8080/api/v1/roles/audit?tenant_id=1&format=detailed"
```

### 6. Test Role Permissions
Verify role permissions before deployment:

```bash
# Test role permissions
curl -X POST http://localhost:8080/api/v1/roles/test \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "role_name": "new_role",
    "test_cases": [
      {
        "resource": "tasks",
        "action": "create",
        "expected": true
      },
      {
        "resource": "users",
        "action": "delete",
        "expected": false
      }
    ]
  }'
```

### 7. Monitor Role Usage
Track role assignment and usage patterns:

```bash
# Get role usage statistics
curl -H "Authorization: Bearer <admin-token>" \
  "http://localhost:8080/api/v1/roles/statistics?tenant_id=1"
```

### 8. Version Control Roles
Track changes to role definitions:

```json
{
  "name": "developer",
  "version": "1.2.0",
  "changelog": [
    {
      "version": "1.2.0",
      "date": "2024-01-20",
      "changes": ["Added schedules:read permission"]
    },
    {
      "version": "1.1.0",
      "date": "2024-01-15",
      "changes": ["Added executions:cancel permission"]
    }
  ]
}
```

For advanced role management patterns and enterprise configurations, see the [Advanced RBAC Configuration](RBAC_ADVANCED_CONFIG.md) guide.